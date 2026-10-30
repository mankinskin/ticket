#[allow(unused_imports)]
use super::*;

impl TicketStore {
    pub(in super::super) fn normalize_path(path: PathBuf) -> PathBuf {
        #[cfg(windows)]
        {
            let raw = path.to_string_lossy().replace('\\', "/");
            let normalized = raw
                .strip_prefix("//?/")
                .or_else(|| raw.strip_prefix(r"\\?\"))
                .unwrap_or(&raw);
            PathBuf::from(normalized)
        }

        #[cfg(not(windows))]
        {
            path
        }
    }

    pub(in super::super) fn normalize_existing_path(path: &Path) -> PathBuf {
        std::fs::canonicalize(path)
            .map(Self::normalize_path)
            .unwrap_or_else(|_| Self::normalize_path(path.to_path_buf()))
    }

    pub(in super::super) fn resolved_candidate_matches(
        candidate: &Path,
        marker_file: Option<&str>,
    ) -> bool {
        match marker_file {
            Some(marker_file) => candidate.join(marker_file).is_file(),
            None => candidate.is_dir(),
        }
    }

    pub(in super::super) fn resolve_indexed_path(
        &self,
        path: &Path,
        marker_file: Option<&str>,
    ) -> PathBuf {
        if path.is_absolute() && Self::resolved_candidate_matches(path, marker_file) {
            return Self::normalize_path(path.to_path_buf());
        }

        if Self::resolved_candidate_matches(path, marker_file) {
            return Self::normalize_existing_path(path);
        }

        for base in self.index_root.ancestors() {
            let candidate = base.join(path);
            if Self::resolved_candidate_matches(&candidate, marker_file) {
                return Self::normalize_existing_path(&candidate);
            }
        }

        Self::normalize_path(path.to_path_buf())
    }

    pub(in super::super) fn resolve_ticket_path(&self, path: &Path) -> PathBuf {
        self.resolve_indexed_path(path, Some(TICKET_MANIFEST_FILE))
    }

    pub(in super::super) fn resolve_scan_root_path(&self, path: &Path) -> PathBuf {
        self.resolve_indexed_path(path, None)
    }

    pub(in super::super) fn normalize_indexed_ticket(
        &self,
        mut indexed: IndexedTicket,
    ) -> IndexedTicket {
        indexed.path = self.resolve_ticket_path(&indexed.path);
        indexed
    }

    pub(in super::super) fn normalize_indexed_tickets(
        &self,
        tickets: Vec<IndexedTicket>,
    ) -> Vec<IndexedTicket> {
        tickets
            .into_iter()
            .map(|ticket| self.normalize_indexed_ticket(ticket))
            .collect()
    }
}
