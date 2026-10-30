#[allow(unused_imports)]
use super::*;

pub(super) fn write_journal(ticket_path: &Path, journal: &Journal) -> Result<(), StorageError> {
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

pub(super) fn validate_journal(ticket_path: &Path, journal: &Journal) -> Result<(), StorageError> {
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

pub(super) fn validate_target(relative: &str) -> Result<(), String> {
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

pub(super) fn owned_path(target: &str, transaction_id: Uuid, index: usize, suffix: &str) -> String {
    let path = Path::new(target);
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let filename = format!(".ticket-txn-{transaction_id}-{index}.{suffix}");
    if parent.as_os_str().is_empty() {
        filename
    } else {
        parent.join(filename).to_string_lossy().replace('\\', "/")
    }
}

pub(super) fn file_sha256_if_present(
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

pub(super) fn sha256(contents: &[u8]) -> String {
    format!("{:x}", Sha256::digest(contents))
}
