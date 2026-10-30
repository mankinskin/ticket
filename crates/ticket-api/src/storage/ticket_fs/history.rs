#[allow(unused_imports)]
use super::*;

impl TicketFs {
    // ── history ───────────────────────────────────────────────────────────────

    /// Read all history revisions for a ticket (oldest first).
    ///
    /// Returns an empty vec if no `history.ndjson` exists yet.
    pub fn read_history(ticket_path: &Path) -> Result<Vec<HistoryRevision>, StorageError> {
        let lock_path = ticket_path.join(TICKET_LOCK_FILE);
        let lock_file = acquire_lock(&lock_path)?;
        let result = Self::read_history_unlocked(ticket_path);
        release_lock(&lock_file, result)
    }

    pub(super) fn read_history_unlocked(
        ticket_path: &Path,
    ) -> Result<Vec<HistoryRevision>, StorageError> {
        let path = ticket_path.join(TICKET_HISTORY_FILE);
        let file = match File::open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Vec::new());
            }
            Err(error) => return Err(StorageError::Io(error)),
        };
        let reader = BufReader::new(file);
        let mut revisions = Vec::new();
        for (line_no, line) in reader.lines().enumerate() {
            let line = line?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            // A single malformed line must not permanently wedge every future
            // append (read_history is called to compute the next rev number
            // before each append); skip and report it loudly instead.
            match serde_json::from_str::<HistoryRevision>(trimmed) {
                Ok(rev) => revisions.push(rev),
                Err(error) => {
                    tracing::warn!(
                        path = %path.display(),
                        line = line_no + 1,
                        error = %error,
                        "skipping malformed history.ndjson line"
                    );
                }
            }
        }
        Ok(revisions)
    }

    /// Append one revision snapshot to `history.ndjson`.
    ///
    /// The revision number is `existing_count + 1`.
    pub fn append_history(
        ticket_path: &Path,
        fields: BTreeMap<String, Value>,
        author: Option<String>,
    ) -> Result<u64, StorageError> {
        let lock_path = ticket_path.join(TICKET_LOCK_FILE);
        let lock_file = acquire_lock(&lock_path)?;
        let result = (|| -> Result<u64, StorageError> {
            Self::append_history_unlocked(ticket_path, fields, author)
        })();
        release_lock(&lock_file, result)
    }

    pub(super) fn append_history_unlocked(
        ticket_path: &Path,
        fields: BTreeMap<String, Value>,
        author: Option<String>,
    ) -> Result<u64, StorageError> {
        let path = ticket_path.join(TICKET_HISTORY_FILE);
        // Count existing revisions to assign the next rev number.
        let existing_count = Self::read_history_unlocked(ticket_path)?.len() as u64;
        let rev = existing_count + 1;
        let entry = HistoryRevision {
            rev,
            ts: Utc::now().to_rfc3339(),
            fields,
            author,
        };
        let line = serde_json::to_string(&entry)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
        writeln!(file, "{}", line)?;
        Ok(rev)
    }
}
