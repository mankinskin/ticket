use super::*;
use crate::{
    error::StorageError,
    model::{
        filesystem::{TICKET_LOCK_FILE, TICKET_MANIFEST_FILE, TICKET_PARTS_DIR},
        ticket::TicketManifestExt,
    },
};
use std::{collections::BTreeSet, fs, path::Path};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PartFileFingerprint {
    pub path: String,
    pub size: Option<u64>,
    pub sha256: Option<String>,
    pub part_id: Option<Uuid>,
    pub orphan: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct PartLayoutSnapshot {
    pub manifest: TicketManifest,
    pub manifest_contents: String,
    pub manifest_sha256: String,
    pub files: Vec<PartFileFingerprint>,
}

pub(crate) fn sha256_bytes(contents: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(contents);
    format!("{:x}", hasher.finalize())
}

fn has_pending_transaction(ticket_path: &Path) -> Result<bool, StorageError> {
    match fs::symlink_metadata(ticket_path.join(".ticket-transaction.json")) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(StorageError::Io(error)),
    }
}

impl TicketFs {
    pub(crate) fn part_layout_snapshot(
        ticket_path: &Path,
    ) -> Result<PartLayoutSnapshot, StorageError> {
        let lock_path = ticket_path.join(TICKET_LOCK_FILE);
        let lock_file = acquire_lock(&lock_path)?;
        let result = Self::part_layout_snapshot_unlocked(ticket_path);
        release_lock(&lock_file, result)
    }

    pub(crate) fn part_layout_snapshot_read_only(
        ticket_path: &Path,
    ) -> Result<PartLayoutSnapshot, StorageError> {
        if has_pending_transaction(ticket_path)? {
            return Err(StorageError::Other(format!(
                "ticket has a pending filesystem transaction; dry-run will not recover or modify {}",
                ticket_path.display()
            )));
        }
        Self::part_layout_snapshot_unlocked(ticket_path)
    }

    fn part_layout_snapshot_unlocked(
        ticket_path: &Path,
    ) -> Result<PartLayoutSnapshot, StorageError> {
        let manifest_path = ticket_path.join(TICKET_MANIFEST_FILE);
        let manifest_bytes = fs::read(&manifest_path)?;
        let manifest_contents = String::from_utf8(manifest_bytes.clone()).map_err(|error| {
            StorageError::Other(format!(
                "ticket manifest is not UTF-8 at {}: {error}",
                manifest_path.display()
            ))
        })?;
        let manifest = Self::read_unlocked(ticket_path)?;
        let parts = manifest.parts();
        let mut referenced_paths = BTreeSet::new();
        let mut files = Vec::with_capacity(parts.len());

        let parts_dir = ticket_path.join(TICKET_PARTS_DIR);
        match fs::symlink_metadata(&parts_dir) {
            Ok(metadata) if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() => {
                return Err(StorageError::Other(format!(
                    "ticket parts path is not a real directory: {}",
                    parts_dir.display()
                )));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(StorageError::Io(error)),
        }

        for part in &parts {
            validate_part_relative_path(&part.path)?;
            referenced_paths.insert(part.path.clone());
            let (size, sha256) = part_file_fingerprint(ticket_path, &part.path)?;
            files.push(PartFileFingerprint {
                path: part.path.clone(),
                size,
                sha256,
                part_id: Some(part.id),
                orphan: false,
            });
        }

        for orphan in find_orphan_part_files(ticket_path, &referenced_paths)? {
            let relative = orphan
                .strip_prefix(ticket_path)
                .map_err(|error| StorageError::Other(error.to_string()))?;
            let path = relative.to_string_lossy().replace('\\', "/");
            validate_part_relative_path(&path)?;
            let (size, sha256) = part_file_fingerprint(ticket_path, &path)?;
            files.push(PartFileFingerprint {
                path,
                size,
                sha256,
                part_id: None,
                orphan: true,
            });
        }

        Ok(PartLayoutSnapshot {
            manifest,
            manifest_contents,
            manifest_sha256: sha256_bytes(&manifest_bytes),
            files,
        })
    }

    pub(crate) fn publish_part_layout(
        ticket_path: &Path,
        expected_manifest_sha256: &str,
        expected_files: &[PartFileFingerprint],
        expected_absent_paths: &[String],
        copies: &[(String, String)],
        empty_files: &[String],
        removals: &[String],
        manifest_contents: &[u8],
    ) -> Result<(), StorageError> {
        let lock_path = ticket_path.join(TICKET_LOCK_FILE);
        let lock_file = acquire_lock(&lock_path)?;
        let result = (|| -> Result<(), StorageError> {
            let manifest_path = ticket_path.join(TICKET_MANIFEST_FILE);
            let current_manifest = fs::read(&manifest_path)?;
            if sha256_bytes(&current_manifest) != expected_manifest_sha256 {
                return Err(StorageError::Other(format!(
                    "stale part-layout input: manifest changed at {}",
                    manifest_path.display()
                )));
            }

            for expected in expected_files {
                validate_part_relative_path(&expected.path)?;
                let (size, sha256) = part_file_fingerprint(ticket_path, &expected.path)?;
                if size != expected.size || sha256 != expected.sha256 {
                    return Err(StorageError::Other(format!(
                        "stale part-layout input: file changed at {}",
                        expected.path
                    )));
                }
            }

            for path in expected_absent_paths {
                validate_part_relative_path(path)?;
                match fs::symlink_metadata(ticket_path.join(path)) {
                    Ok(_) => {
                        return Err(StorageError::Other(format!(
                            "stale part-layout input: target path is now occupied: {path}"
                        )));
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(StorageError::Io(error)),
                }
            }

            let mut writes = Vec::with_capacity(copies.len() + empty_files.len());
            for (source, target) in copies {
                validate_part_relative_path(source)?;
                validate_part_relative_path(target)?;
                writes.push((target.clone(), fs::read(ticket_path.join(source))?));
            }
            for path in empty_files {
                validate_part_relative_path(path)?;
                writes.push((path.clone(), Vec::new()));
            }

            let removals = removals
                .iter()
                .cloned()
                .chain(copies.iter().map(|(source, _)| source.clone()))
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            transaction::publish(
                ticket_path,
                TransactionOperation::PathMigration,
                writes,
                removals,
                Some(manifest_contents),
            )
        })();
        release_lock(&lock_file, result)
    }
}
