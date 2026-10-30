#[allow(unused_imports)]
use super::*;

pub(in super::super) fn publish(
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

pub(super) fn publish_changes(
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

pub(super) fn apply_prepared(
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

pub(in super::super) fn recover(ticket_path: &Path) -> Result<(), StorageError> {
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
