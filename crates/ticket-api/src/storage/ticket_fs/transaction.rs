use std::{
    collections::BTreeSet,
    fs::{self, OpenOptions},
    io::Write,
    path::{Component, Path},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::StorageError;

const JOURNAL_FILE: &str = ".ticket-transaction.json";
const GENERATION_FILE: &str = ".ticket-generation";
const JOURNAL_VERSION: u32 = 1;

// The generation marker is the durable commit point for content-only writes,
// whose manifest bytes can remain unchanged.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum Operation {
    Create,
    Overwrite,
    ManifestOnly,
    PathMigration,
    RemoveParts,
}

#[derive(Debug, Serialize, Deserialize)]
struct Journal {
    version: u32,
    transaction_id: Uuid,
    operation: Operation,
    old_generation: u64,
    new_generation: u64,
    changes: Vec<FileChange>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FileChange {
    target: String,
    stage: Option<String>,
    backup: String,
    old_sha256: Option<String>,
    new_sha256: Option<String>,
}

struct PlannedChange {
    target: String,
    contents: Option<Vec<u8>>,
}

pub(super) fn publish(
    ticket_path: &Path,
    operation: Operation,
    writes: Vec<(String, Vec<u8>)>,
    removals: Vec<String>,
    manifest: Option<&[u8]>,
) -> Result<(), StorageError> {
    let write_targets = writes
        .iter()
        .map(|(target, _)| target.clone())
        .collect::<BTreeSet<_>>();
    let removal_targets = removals.iter().cloned().collect::<BTreeSet<_>>();
    let mut planned = Vec::with_capacity(writes.len() + removals.len() + 2);
    for (target, contents) in writes {
        planned.push(PlannedChange {
            target,
            contents: Some(contents),
        });
    }
    for target in removals {
        if write_targets.contains(&target) {
            continue;
        }
        planned.push(PlannedChange {
            target,
            contents: None,
        });
    }
    if let Some(manifest) = manifest {
        planned.push(PlannedChange {
            target: "ticket.toml".to_string(),
            contents: Some(manifest.to_vec()),
        });
    }
    publish_changes(ticket_path, operation, planned, &removal_targets)
}

fn publish_changes(
    ticket_path: &Path,
    operation: Operation,
    mut planned: Vec<PlannedChange>,
    removal_targets: &BTreeSet<String>,
) -> Result<(), StorageError> {
    if planned.is_empty() {
        return Ok(());
    }

    let old_generation = read_generation(ticket_path)?;
    let new_generation = old_generation
        .checked_add(1)
        .ok_or_else(|| transaction_error(ticket_path, "ticket generation counter overflowed"))?;
    let transaction_id = Uuid::new_v4();
    let generation_contents = format!("{new_generation}\n").into_bytes();
    planned.push(PlannedChange {
        target: GENERATION_FILE.to_string(),
        contents: Some(generation_contents),
    });

    let mut targets = BTreeSet::new();
    let mut changes = Vec::with_capacity(planned.len());
    let mut staged_contents = Vec::with_capacity(planned.len());
    for change in planned {
        validate_target(&change.target).map_err(|reason| transaction_error(ticket_path, reason))?;
        if !targets.insert(change.target.clone()) {
            return Err(transaction_error(
                ticket_path,
                format!("duplicate target path {}", change.target),
            ));
        }

        let old_sha256 = file_sha256_if_present(ticket_path, &change.target)?;
        let new_sha256 = change.contents.as_deref().map(sha256);
        if change.contents.is_some() && change.target.starts_with("parts/") {
            match operation {
                Operation::Create if old_sha256.is_some() => {
                    return Err(transaction_error(
                        ticket_path,
                        format!("part create target is already occupied: {}", change.target),
                    ));
                }
                Operation::PathMigration
                    if old_sha256.is_some() && !removal_targets.contains(&change.target) =>
                {
                    return Err(transaction_error(
                        ticket_path,
                        format!(
                            "path migration target is not transaction-owned: {}",
                            change.target
                        ),
                    ));
                }
                _ => {}
            }
        }
        if old_sha256.is_none() && new_sha256.is_none() {
            continue;
        }
        let index = changes.len();
        let stage = change
            .contents
            .as_ref()
            .map(|_| owned_path(&change.target, transaction_id, index, "stage"));
        let backup = owned_path(&change.target, transaction_id, index, "backup");

        ensure_absent(ticket_path, &backup)?;
        if let Some(stage) = stage.as_deref() {
            ensure_absent(ticket_path, stage)?;
        }
        changes.push(FileChange {
            target: change.target,
            stage,
            backup,
            old_sha256,
            new_sha256,
        });
        staged_contents.push(change.contents);
    }

    if changes.is_empty() {
        return Ok(());
    }

    let journal = Journal {
        version: JOURNAL_VERSION,
        transaction_id,
        operation,
        old_generation,
        new_generation,
        changes,
    };
    validate_journal(ticket_path, &journal)?;
    if let Err(error) = write_journal(ticket_path, &journal) {
        match fs::symlink_metadata(ticket_path.join(JOURNAL_FILE)) {
            Ok(_) => {
                return match recover(ticket_path) {
                    Ok(()) => Err(error),
                    Err(recovery_error) => Err(StorageError::Other(format!(
                        "{error}; transaction recovery also failed: {recovery_error}"
                    ))),
                };
            }
            Err(metadata_error) if metadata_error.kind() == std::io::ErrorKind::NotFound => {}
            Err(metadata_error) => {
                return Err(StorageError::Other(format!(
                    "{error}; could not inspect transaction journal after write failure: {metadata_error}"
                )));
            }
        }
        return Err(error);
    }

    let apply = apply_prepared(ticket_path, &journal, &staged_contents);
    match apply {
        Ok(()) => recover(ticket_path),
        Err(error) if is_injected_failure(&error) => Err(error),
        Err(error) => match recover(ticket_path) {
            Ok(()) => Err(error),
            Err(recovery_error) => Err(StorageError::Other(format!(
                "{error}; transaction recovery also failed: {recovery_error}"
            ))),
        },
    }
}

fn apply_prepared(
    ticket_path: &Path,
    journal: &Journal,
    staged_contents: &[Option<Vec<u8>>],
) -> Result<(), StorageError> {
    maybe_fail(FailPoint::JournalPublished)?;

    for (index, change) in journal.changes.iter().enumerate() {
        let Some(stage) = &change.stage else {
            continue;
        };
        let contents = staged_contents[index]
            .as_deref()
            .ok_or_else(|| transaction_error(ticket_path, "staged content is missing"))?;
        if Some(sha256(contents)) != change.new_sha256 {
            return Err(transaction_error(
                ticket_path,
                format!("staged content checksum does not match for {stage}"),
            ));
        }
        let stage_path = ticket_path.join(stage);
        create_parent_directories(ticket_path, stage)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&stage_path)?;
        file.write_all(&contents)?;
        file.sync_all()?;
        sync_parent(&stage_path)?;
        maybe_fail(FailPoint::Staged(index))?;
    }

    for (index, change) in journal.changes.iter().enumerate() {
        if change.old_sha256.is_none() {
            continue;
        }
        let target = ticket_path.join(&change.target);
        let backup = ticket_path.join(&change.backup);
        let old_sha256 = change.old_sha256.as_deref().unwrap_or_default();
        if file_sha256_if_present(ticket_path, &change.target)?.as_deref() != Some(old_sha256) {
            return Err(transaction_error(
                ticket_path,
                format!("target changed before backup: {}", change.target),
            ));
        }
        durable_rename(&target, &backup)?;
        sync_parent(&target)?;
        maybe_fail(FailPoint::BackedUp(index))?;
    }

    for (index, change) in journal.changes.iter().enumerate() {
        let target = ticket_path.join(&change.target);
        if let Some(stage) = &change.stage {
            let stage_path = ticket_path.join(stage);
            durable_rename(&stage_path, &target)?;
            sync_parent(&target)?;
            maybe_fail(FailPoint::Published(index))?;
        }
    }
    Ok(())
}

pub(super) fn recover(ticket_path: &Path) -> Result<(), StorageError> {
    let journal_path = ticket_path.join(JOURNAL_FILE);
    if file_sha256_if_present(ticket_path, JOURNAL_FILE)?.is_none() {
        read_generation_marker(ticket_path)?;
        return Ok(());
    }
    let journal_bytes = fs::read(&journal_path)?;
    let journal: Journal = serde_json::from_slice(&journal_bytes).map_err(|error| {
        transaction_error(
            ticket_path,
            format!(
                "journal is malformed at {}: {error}",
                journal_path.display()
            ),
        )
    })?;
    validate_journal(ticket_path, &journal)?;

    let manifest_generation_published = match journal
        .changes
        .iter()
        .find(|change| change.target == "ticket.toml")
    {
        Some(change) if change.old_sha256 != change.new_sha256 && change.new_sha256.is_some() => {
            file_sha256_if_present(ticket_path, "ticket.toml")? == change.new_sha256
        }
        _ => false,
    };
    let generation = read_generation_marker(ticket_path)?;
    match generation {
        Some(generation) if generation == journal.new_generation => {
            finish_committed(ticket_path, &journal)
        }
        Some(generation) if generation == journal.old_generation => {
            if manifest_generation_published {
                finish_committed(ticket_path, &journal)
            } else {
                rollback_prepared(ticket_path, &journal)
            }
        }
        Some(generation) => Err(transaction_error(
            ticket_path,
            format!(
                "journal generation conflict: found {generation}, expected {} or {}",
                journal.old_generation, journal.new_generation
            ),
        )),
        None if manifest_generation_published => finish_committed(ticket_path, &journal),
        None if marker_backup_is_old(ticket_path, &journal)? => {
            rollback_prepared(ticket_path, &journal)
        }
        None => Err(transaction_error(
            ticket_path,
            "generation marker and its recorded prior state are missing",
        )),
    }
}

fn marker_backup_is_old(ticket_path: &Path, journal: &Journal) -> Result<bool, StorageError> {
    let marker = journal
        .changes
        .last()
        .ok_or_else(|| transaction_error(ticket_path, "journal has no generation marker"))?;
    let Some(old_sha256) = &marker.old_sha256 else {
        return Ok(journal.old_generation == 0);
    };
    if file_sha256_if_present(ticket_path, &marker.backup)?.as_deref() != Some(old_sha256.as_str())
    {
        return Ok(false);
    }
    let backup_generation = read_generation_file(&ticket_path.join(&marker.backup))?;
    Ok(backup_generation == journal.old_generation)
}

fn rollback_prepared(ticket_path: &Path, journal: &Journal) -> Result<(), StorageError> {
    for change in journal.changes.iter().rev() {
        let target = ticket_path.join(&change.target);
        let backup = ticket_path.join(&change.backup);
        let backup_hash = file_sha256_if_present(ticket_path, &change.backup)?;

        match (&change.old_sha256, backup_hash.as_deref()) {
            (Some(expected), Some(actual)) if expected == actual => {
                let target_hash = file_sha256_if_present(ticket_path, &change.target)?;
                if let Some(target_hash) = target_hash {
                    if Some(&target_hash) != change.new_sha256.as_ref()
                        && Some(&target_hash) != change.old_sha256.as_ref()
                    {
                        return Err(transaction_error(
                            ticket_path,
                            format!("refusing to overwrite conflicting target {}", change.target),
                        ));
                    }
                    if target_hash == *expected {
                        remove_owned_file(ticket_path, &change.backup)?;
                    } else {
                        remove_owned_file(ticket_path, &change.target)?;
                        durable_rename(&backup, &target)?;
                    }
                } else {
                    durable_rename(&backup, &target)?;
                }
                sync_parent(&target)?;
            }
            (Some(_), Some(_)) => {
                return Err(transaction_error(
                    ticket_path,
                    format!("backup checksum conflict at {}", change.backup),
                ));
            }
            (Some(expected), None) => {
                if file_sha256_if_present(ticket_path, &change.target)?.as_deref()
                    != Some(expected.as_str())
                {
                    return Err(transaction_error(
                        ticket_path,
                        format!("original and backup are both missing for {}", change.target),
                    ));
                }
            }
            (None, Some(_)) => {
                return Err(transaction_error(
                    ticket_path,
                    format!("unexpected backup exists at {}", change.backup),
                ));
            }
            (None, None) => {
                if let Some(target_hash) = file_sha256_if_present(ticket_path, &change.target)? {
                    if Some(&target_hash) == change.new_sha256.as_ref() {
                        remove_owned_file(ticket_path, &change.target)?;
                        sync_parent(&target)?;
                    } else {
                        return Err(transaction_error(
                            ticket_path,
                            format!("refusing to remove unowned target {}", change.target),
                        ));
                    }
                }
            }
        }

        if let Some(stage) = &change.stage {
            remove_stage_if_owned(ticket_path, stage)?;
        }
    }
    remove_journal(ticket_path, journal)?;
    Ok(())
}

fn finish_committed(ticket_path: &Path, journal: &Journal) -> Result<(), StorageError> {
    for change in &journal.changes {
        let target = ticket_path.join(&change.target);
        let current_hash = file_sha256_if_present(ticket_path, &change.target)?;
        match &change.new_sha256 {
            Some(expected) if current_hash.as_deref() == Some(expected.as_str()) => {}
            Some(expected) => {
                let stage = change.stage.as_ref().ok_or_else(|| {
                    transaction_error(
                        ticket_path,
                        format!("staged file is missing for {}", change.target),
                    )
                })?;
                if file_sha256_if_present(ticket_path, stage)?.as_deref() != Some(expected.as_str())
                {
                    return Err(transaction_error(
                        ticket_path,
                        format!("staged file checksum conflict at {stage}"),
                    ));
                }
                if let Some(current_hash) = current_hash {
                    if Some(&current_hash) != change.old_sha256.as_ref() {
                        return Err(transaction_error(
                            ticket_path,
                            format!("refusing to replace conflicting target {}", change.target),
                        ));
                    }
                    if let Some(old_hash) = &change.old_sha256 {
                        if file_sha256_if_present(ticket_path, &change.backup)?
                            .is_some_and(|backup_hash| backup_hash != *old_hash)
                        {
                            return Err(transaction_error(
                                ticket_path,
                                format!("backup checksum conflict for {}", change.target),
                            ));
                        }
                    }
                    remove_owned_file(ticket_path, &change.target)?;
                }

                let stage_path = ticket_path.join(stage);
                durable_rename(&stage_path, &target)?;
                sync_parent(&target)?;
            }
            None => {
                if let Some(current_hash) = current_hash {
                    if Some(&current_hash) != change.old_sha256.as_ref() {
                        return Err(transaction_error(
                            ticket_path,
                            format!("refusing to remove conflicting target {}", change.target),
                        ));
                    }
                    remove_owned_file(ticket_path, &change.target)?;
                    sync_parent(&target)?;
                }
            }
        }
    }

    for change in &journal.changes {
        if let Some(old_hash) = &change.old_sha256 {
            if let Some(backup_hash) = file_sha256_if_present(ticket_path, &change.backup)? {
                if backup_hash != *old_hash {
                    return Err(transaction_error(
                        ticket_path,
                        format!("backup checksum conflict at {}", change.backup),
                    ));
                }
                remove_owned_file(ticket_path, &change.backup)?;
                sync_parent(&ticket_path.join(&change.backup))?;
            }
        } else if file_sha256_if_present(ticket_path, &change.backup)?.is_some() {
            return Err(transaction_error(
                ticket_path,
                format!("unexpected backup exists at {}", change.backup),
            ));
        }
        if let Some(stage) = &change.stage {
            remove_stage_if_owned(ticket_path, stage)?;
        }
    }
    remove_journal(ticket_path, journal)
}

fn remove_journal(ticket_path: &Path, _journal: &Journal) -> Result<(), StorageError> {
    let journal_path = ticket_path.join(JOURNAL_FILE);
    let recorded = serde_json::to_vec(_journal)
        .map_err(|error| StorageError::Serialization(error.to_string()))?;
    if fs::read(&journal_path)? != recorded {
        return Err(transaction_error(
            ticket_path,
            "journal changed during recovery",
        ));
    }
    fs::remove_file(&journal_path)?;
    sync_parent(&journal_path)?;
    Ok(())
}

fn remove_owned_file(ticket_path: &Path, relative: &str) -> Result<(), StorageError> {
    let path = ticket_path.join(relative);
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(StorageError::Io(error)),
    }
}

fn remove_stage_if_owned(ticket_path: &Path, relative: &str) -> Result<(), StorageError> {
    let path = ticket_path.join(relative);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(StorageError::Io(error)),
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(transaction_error(
            ticket_path,
            format!("refusing to remove a non-file staging path: {relative}"),
        ));
    }
    fs::remove_file(&path)?;
    sync_parent(&path)
}

fn write_journal(ticket_path: &Path, journal: &Journal) -> Result<(), StorageError> {
    let journal_path = ticket_path.join(JOURNAL_FILE);
    ensure_absent(ticket_path, JOURNAL_FILE)?;
    let temp_relative = format!(".ticket-transaction-{}.tmp", journal.transaction_id);
    ensure_absent(ticket_path, &temp_relative)?;
    let temp_path = ticket_path.join(&temp_relative);
    let contents = serde_json::to_vec(journal)
        .map_err(|error| StorageError::Serialization(error.to_string()))?;
    let write_result = (|| -> Result<(), StorageError> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)?;
        file.write_all(&contents)?;
        file.sync_all()?;
        durable_rename(&temp_path, &journal_path)?;
        sync_parent(&journal_path)?;
        Ok(())
    })();
    match write_result {
        Ok(()) => Ok(()),
        Err(error) => match fs::remove_file(&temp_path) {
            Ok(()) => Err(error),
            Err(cleanup_error) if cleanup_error.kind() == std::io::ErrorKind::NotFound => {
                Err(error)
            }
            Err(cleanup_error) => Err(StorageError::Other(format!(
                "{error}; could not remove transaction journal staging file: {cleanup_error}"
            ))),
        },
    }
}

fn validate_journal(ticket_path: &Path, journal: &Journal) -> Result<(), StorageError> {
    if journal.version != JOURNAL_VERSION
        || journal.new_generation != journal.old_generation.saturating_add(1)
        || journal.changes.is_empty()
    {
        return Err(transaction_error(ticket_path, "journal header is invalid"));
    }

    let mut targets = BTreeSet::new();
    for (index, change) in journal.changes.iter().enumerate() {
        validate_target(&change.target).map_err(|reason| transaction_error(ticket_path, reason))?;
        if !targets.insert(change.target.as_str()) {
            return Err(transaction_error(
                ticket_path,
                format!("journal repeats target {}", change.target),
            ));
        }
        if change.backup != owned_path(&change.target, journal.transaction_id, index, "backup")
            || change.stage
                != change
                    .new_sha256
                    .as_ref()
                    .map(|_| owned_path(&change.target, journal.transaction_id, index, "stage"))
        {
            return Err(transaction_error(
                ticket_path,
                format!("journal ownership paths are invalid for {}", change.target),
            ));
        }
        for digest in [&change.old_sha256, &change.new_sha256]
            .into_iter()
            .flatten()
        {
            if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Err(transaction_error(
                    ticket_path,
                    format!("journal checksum is invalid for {}", change.target),
                ));
            }
        }
        if change.old_sha256.is_none() && change.new_sha256.is_none() {
            return Err(transaction_error(
                ticket_path,
                format!("journal change has no old or new file: {}", change.target),
            ));
        }
    }
    if journal.changes.last().map(|change| change.target.as_str()) != Some(GENERATION_FILE) {
        return Err(transaction_error(
            ticket_path,
            "journal generation marker is not the final change",
        ));
    }
    Ok(())
}

fn validate_target(relative: &str) -> Result<(), String> {
    if relative.is_empty() || relative.contains('\\') || relative.starts_with('/') {
        return Err(format!("unsafe transaction path: {relative}"));
    }
    let path = Path::new(relative);
    if path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!("unsafe transaction path: {relative}"));
    }
    let components = path.components().collect::<Vec<_>>();
    let allowed = matches!(relative, "ticket.toml" | "description.md" | GENERATION_FILE)
        || (components.len() >= 2
            && components[0].as_os_str() == "parts"
            && components
                .iter()
                .all(|component| matches!(component, Component::Normal(_))));
    if !allowed {
        return Err(format!(
            "transaction path is outside ticket content: {relative}"
        ));
    }
    Ok(())
}

fn owned_path(target: &str, transaction_id: Uuid, index: usize, suffix: &str) -> String {
    let path = Path::new(target);
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let filename = format!(".ticket-txn-{transaction_id}-{index}.{suffix}");
    if parent.as_os_str().is_empty() {
        filename
    } else {
        parent.join(filename).to_string_lossy().replace('\\', "/")
    }
}

fn file_sha256_if_present(
    ticket_path: &Path,
    relative: &str,
) -> Result<Option<String>, StorageError> {
    let path = ticket_path.join(relative);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(StorageError::Io(error)),
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(transaction_error(
            ticket_path,
            format!("transaction target is not a regular file: {relative}"),
        ));
    }
    let contents = fs::read(path)?;
    Ok(Some(sha256(&contents)))
}

fn sha256(contents: &[u8]) -> String {
    format!("{:x}", Sha256::digest(contents))
}

fn read_generation(ticket_path: &Path) -> Result<u64, StorageError> {
    Ok(read_generation_marker(ticket_path)?.unwrap_or(0))
}

fn read_generation_marker(ticket_path: &Path) -> Result<Option<u64>, StorageError> {
    if file_sha256_if_present(ticket_path, GENERATION_FILE)?.is_none() {
        return Ok(None);
    }
    read_generation_file(&ticket_path.join(GENERATION_FILE)).map(Some)
}

fn read_generation_file(path: &Path) -> Result<u64, StorageError> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(StorageError::Io(error));
        }
        Err(error) => return Err(StorageError::Io(error)),
    };
    contents.trim().parse::<u64>().map_err(|error| {
        transaction_error(
            path.parent().unwrap_or_else(|| Path::new(".")),
            format!(
                "generation marker is malformed at {}: {error}",
                path.display()
            ),
        )
    })
}

fn ensure_absent(ticket_path: &Path, relative: &str) -> Result<(), StorageError> {
    match fs::symlink_metadata(ticket_path.join(relative)) {
        Ok(_) => Err(transaction_error(
            ticket_path,
            format!("transaction-owned path already exists: {relative}"),
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(StorageError::Io(error)),
    }
}

fn create_parent_directories(ticket_path: &Path, relative: &str) -> Result<(), StorageError> {
    let parent = Path::new(relative)
        .parent()
        .unwrap_or_else(|| Path::new(""));
    let mut current = ticket_path.to_path_buf();
    for component in parent.components() {
        let Component::Normal(name) = component else {
            return Err(transaction_error(
                ticket_path,
                format!("unsafe parent in transaction path: {relative}"),
            ));
        };
        current.push(name);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
            Ok(_) => {
                return Err(transaction_error(
                    ticket_path,
                    format!(
                        "transaction parent is not a real directory: {}",
                        current.display()
                    ),
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&current)?;
                sync_parent(&current)?;
            }
            Err(error) => return Err(StorageError::Io(error)),
        }
    }
    Ok(())
}

fn transaction_error(ticket_path: &Path, detail: impl std::fmt::Display) -> StorageError {
    StorageError::Other(format!(
        "ticket transaction at {}: {detail}",
        ticket_path.display()
    ))
}

fn sync_parent(path: &Path) -> Result<(), StorageError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    sync_directory(parent)
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<(), StorageError> {
    fs::File::open(path)?.sync_all()?;
    Ok(())
}

#[cfg(windows)]
fn sync_directory(path: &Path) -> Result<(), StorageError> {
    use std::{os::windows::ffi::OsStrExt, ptr};
    use windows_sys::Win32::{
        Foundation::{CloseHandle, GENERIC_WRITE, INVALID_HANDLE_VALUE},
        Storage::FileSystem::{
            CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_SHARE_DELETE, FILE_SHARE_READ,
            FILE_SHARE_WRITE, FlushFileBuffers, OPEN_EXISTING,
        },
    };

    let path_wide = path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let handle = unsafe {
        CreateFileW(
            path_wide.as_ptr(),
            GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(StorageError::Io(std::io::Error::last_os_error()));
    }
    let flush_result = unsafe { FlushFileBuffers(handle) };
    let flush_error = (flush_result == 0).then(std::io::Error::last_os_error);
    unsafe {
        CloseHandle(handle);
    }
    match flush_error {
        Some(error) => Err(StorageError::Io(error)),
        None => Ok(()),
    }
}

#[cfg(not(any(unix, windows)))]
fn sync_directory(path: &Path) -> Result<(), StorageError> {
    Err(transaction_error(
        path,
        "durable directory synchronization is unsupported on this platform",
    ))
}

#[cfg(unix)]
fn durable_rename(source: &Path, destination: &Path) -> Result<(), StorageError> {
    fs::rename(source, destination)?;
    Ok(())
}

#[cfg(windows)]
fn durable_rename(source: &Path, destination: &Path) -> Result<(), StorageError> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{MOVEFILE_WRITE_THROUGH, MoveFileExW};

    let source_wide = source
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let destination_wide = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let result = unsafe {
        MoveFileExW(
            source_wide.as_ptr(),
            destination_wide.as_ptr(),
            MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        return Err(StorageError::Io(std::io::Error::last_os_error()));
    }
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn durable_rename(source: &Path, destination: &Path) -> Result<(), StorageError> {
    fs::rename(source, destination)?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FailPoint {
    JournalPublished,
    Staged(usize),
    BackedUp(usize),
    Published(usize),
}

#[cfg(test)]
thread_local! {
    static FAIL_AT: std::cell::Cell<Option<FailPoint>> = const { std::cell::Cell::new(None) };
}

#[cfg(test)]
pub(super) fn set_failpoint(failpoint: Option<FailPoint>) {
    FAIL_AT.with(|current| current.set(failpoint));
}

#[cfg(test)]
fn maybe_fail(failpoint: FailPoint) -> Result<(), StorageError> {
    if FAIL_AT.with(|current| current.get() == Some(failpoint)) {
        FAIL_AT.with(|current| current.set(None));
        Err(StorageError::Other(format!(
            "injected failure at transaction boundary {failpoint:?}"
        )))
    } else {
        Ok(())
    }
}

#[cfg(not(test))]
fn maybe_fail(_failpoint: FailPoint) -> Result<(), StorageError> {
    Ok(())
}

#[cfg(test)]
fn is_injected_failure(error: &StorageError) -> bool {
    error
        .to_string()
        .starts_with("injected failure at transaction boundary")
}

#[cfg(not(test))]
fn is_injected_failure(_error: &StorageError) -> bool {
    false
}

#[cfg(test)]
mod tests;
