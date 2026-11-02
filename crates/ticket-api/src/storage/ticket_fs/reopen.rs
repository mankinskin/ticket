use super::*;

impl TicketFs {
    /// Audited terminal reopen. Manifest, operation receipt and history commit
    /// through the existing recoverable Ticket transaction under one lock.
    pub(crate) fn reopen_done(
        path: &Path,
        expected_revision: u64,
        reason: &str,
        operation_id: Uuid,
        author: Option<&str>,
    ) -> Result<TicketManifest, StorageError> {
        let rejected =
            |message: &str| StorageError::Other(format!("ticket_transition_rejected: {message}"));
        ensure_nonblank_content(reason, "Ticket reopen reason")?;
        if operation_id.is_nil() {
            return Err(rejected("nil operation identity"));
        }
        let lock = acquire_lock(&path.join(TICKET_LOCK_FILE))?;
        let result = (|| {
            let manifest = Self::read_unlocked(path)?;
            let history = Self::read_history_unlocked(path)?;
            let receipt = format!(".reopen-{operation_id}.json");
            let binding = serde_json::json!({
                "version": 1, "operation_id": operation_id,
                "expected_revision": expected_revision, "reason": reason,
            });
            if let Some(text) = read_optional_text(&path.join(&receipt))? {
                let previous: Value = serde_json::from_str(&text)
                    .map_err(|e| StorageError::Serialization(e.to_string()))?;
                if previous != binding {
                    return Err(rejected("operation identity conflict"));
                }
                if manifest.extra.get("state").and_then(Value::as_str) != Some("ready") {
                    return Err(rejected("reopened Ticket changed since receipt"));
                }
                return Ok(manifest);
            }
            if manifest.extra.get("state").and_then(Value::as_str) != Some("done")
                || history.iter().map(|h| h.rev).max().unwrap_or(0) != expected_revision
            {
                return Err(rejected("expected done state and revision do not match"));
            }
            let mut manifest = manifest;
            manifest
                .extra
                .insert("state".into(), Value::String("ready".into()));
            let mut fields = manifest.extra.clone();
            fields.insert("__reopen__".into(), binding.clone());
            let revision = HistoryRevision {
                rev: expected_revision
                    .checked_add(1)
                    .ok_or_else(|| rejected("revision exhausted"))?,
                ts: Utc::now().to_rfc3339(),
                fields,
                author: author.map(str::to_string),
            };
            let mut bytes = fs::read(path.join(TICKET_HISTORY_FILE)).or_else(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Ok(vec![])
                } else {
                    Err(e)
                }
            })?;
            if !bytes.is_empty() && !bytes.ends_with(b"\n") {
                bytes.push(b'\n');
            }
            bytes.extend(
                serde_json::to_vec(&revision)
                    .map_err(|e| StorageError::Serialization(e.to_string()))?,
            );
            bytes.push(b'\n');
            let receipt_bytes = serde_json::to_vec(&binding)
                .map_err(|e| StorageError::Serialization(e.to_string()))?;
            let text = crate::model::manifest_format::format_manifest_toml(&manifest);
            transaction::publish(
                path,
                TransactionOperation::ManifestOnly,
                vec![
                    (TICKET_HISTORY_FILE.into(), bytes),
                    (receipt, receipt_bytes),
                ],
                vec![],
                Some(text.as_bytes()),
            )?;
            Ok(manifest)
        })();
        release_lock(&lock, result)
    }
}
