#[allow(unused_imports)]
use super::*;

impl TicketFs {
    /// Walk `scan_root` and locate all valid ticket folders.
    ///
    /// A folder is considered a valid ticket folder if it:
    /// - Has a UUID-parseable name, **and**
    /// - Contains a `ticket.toml` file
    ///
    /// Returns `(valid_paths, parse_diagnostics)`.
    pub fn scan_root(
        scan_root: &Path,
    ) -> Result<
        (
            Vec<TicketScanEntry>,
            Vec<crate::model::filesystem::ParseDiagnostic>,
        ),
        StorageError,
    > {
        let mut valid = Vec::new();
        let mut diags = Vec::new();

        let Some(read_dir) = Self::scan_root_dir(scan_root)? else {
            return Ok((valid, diags));
        };

        for entry in read_dir.flatten() {
            Self::scan_root_entry(entry.path(), &mut valid, &mut diags);
        }

        Ok((valid, diags))
    }

    pub(super) fn scan_root_dir(scan_root: &Path) -> Result<Option<fs::ReadDir>, StorageError> {
        match fs::read_dir(scan_root) {
            Ok(read_dir) => Ok(Some(read_dir)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(StorageError::Io(error)),
        }
    }

    pub(super) fn scan_root_entry(
        path: PathBuf,
        valid: &mut Vec<TicketScanEntry>,
        diags: &mut Vec<crate::model::filesystem::ParseDiagnostic>,
    ) {
        let Some(candidate) = scan_candidate_path(&path, diags) else {
            return;
        };

        match load_scan_entry(candidate.id, path, candidate.manifest_path) {
            Ok(Some(entry)) => valid.push(entry),
            Ok(None) => {}
            Err(diag) => diags.push(diag),
        }
    }
}
