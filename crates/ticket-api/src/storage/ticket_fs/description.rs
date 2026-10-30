#[allow(unused_imports)]
use super::*;

impl TicketFs {
    /// Ensure the `assets/` subdirectory exists inside `ticket_path`.
    pub fn ensure_assets_dir(ticket_path: &Path) -> Result<(), StorageError> {
        let lock_path = ticket_path.join(TICKET_LOCK_FILE);
        let lock_file = acquire_lock(&lock_path)?;
        let assets = ticket_path.join(TICKET_ASSETS_DIR);
        let result = if assets.exists() {
            Ok(())
        } else {
            fs::create_dir_all(&assets).map_err(StorageError::Io)
        };
        release_lock(&lock_file, result)
    }

    /// Reformat an existing ticket's `ticket.toml` to canonical field ordering.
    ///
    /// Acquires the advisory lock, reads the current manifest, and rewrites
    /// it through the canonical formatter.  This is idempotent.
    pub fn reformat(ticket_path: &Path) -> Result<(), StorageError> {
        let lock_path = ticket_path.join(TICKET_LOCK_FILE);
        let lock_file = acquire_lock(&lock_path)?;
        let result = (|| -> Result<(), StorageError> {
            let manifest = Self::read_unlocked(ticket_path)?;
            write_manifest_transactional(ticket_path, &manifest)?;
            Ok(())
        })();
        release_lock(&lock_file, result)
    }

    /// Write or overwrite the `description.md` file for a ticket.
    ///
    /// Frozen-part rejection is enforced by the caller
    /// (`TicketStore::enforce_description_write_gate`) before this is
    /// reached — this is a crate-private low-level primitive with no
    /// external entry point (spec 24b3d22b, ticket f9e70385, AC7).
    pub(crate) fn write_description(ticket_path: &Path, text: &str) -> Result<(), StorageError> {
        ensure_nonblank_content(text, "ticket description")?;
        let lock_path = ticket_path.join(TICKET_LOCK_FILE);
        let lock_file = acquire_lock(&lock_path)?;
        let result = transaction::publish(
            ticket_path,
            TransactionOperation::Overwrite,
            vec![("description.md".to_string(), text.as_bytes().to_vec())],
            Vec::new(),
            None,
        );
        release_lock(&lock_file, result)
    }

    pub(crate) fn remove_description(ticket_path: &Path) -> Result<(), StorageError> {
        let lock_path = ticket_path.join(TICKET_LOCK_FILE);
        let lock_file = acquire_lock(&lock_path)?;
        let result = transaction::publish(
            ticket_path,
            TransactionOperation::Overwrite,
            Vec::new(),
            vec!["description.md".to_string()],
            None,
        );
        release_lock(&lock_file, result)
    }

    /// Read the legacy description, returning `None` when no description exists.
    pub fn read_description(ticket_path: &Path) -> Result<Option<String>, StorageError> {
        let lock_path = ticket_path.join(TICKET_LOCK_FILE);
        let lock_file = acquire_lock(&lock_path)?;
        let result = Self::read_description_unlocked(ticket_path);
        release_lock(&lock_file, result)
    }

    pub(super) fn read_description_unlocked(
        ticket_path: &Path,
    ) -> Result<Option<String>, StorageError> {
        let description_path = ticket_path.join("description.md");
        match fs::read_to_string(&description_path) {
            Ok(description) => Ok(Some(description)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(io_error_at(&description_path, error)),
        }
    }
}
