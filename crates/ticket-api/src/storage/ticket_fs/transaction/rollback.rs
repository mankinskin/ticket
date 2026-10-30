#[allow(unused_imports)]
use super::*;

pub(super) fn marker_backup_is_old(
    ticket_path: &Path,
    journal: &Journal,
) -> Result<bool, StorageError> {
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

pub(super) fn rollback_prepared(ticket_path: &Path, journal: &Journal) -> Result<(), StorageError> {
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

pub(super) fn finish_committed(ticket_path: &Path, journal: &Journal) -> Result<(), StorageError> {
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

pub(super) fn remove_journal(ticket_path: &Path, _journal: &Journal) -> Result<(), StorageError> {
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

pub(super) fn remove_owned_file(ticket_path: &Path, relative: &str) -> Result<(), StorageError> {
    let path = ticket_path.join(relative);
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(StorageError::Io(error)),
    }
}

pub(super) fn remove_stage_if_owned(
    ticket_path: &Path,
    relative: &str,
) -> Result<(), StorageError> {
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
