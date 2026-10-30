#[allow(unused_imports)]
use super::*;

pub struct TicketScanEntry {
    pub id: Uuid,
    pub path: PathBuf,
    pub manifest: TicketManifest,
}

pub(super) struct ScanCandidate {
    pub(super) id: Uuid,
    pub(super) manifest_path: PathBuf,
}

pub(super) fn scan_candidate_path(
    path: &Path,
    diags: &mut Vec<crate::model::filesystem::ParseDiagnostic>,
) -> Option<ScanCandidate> {
    if !path.is_dir() {
        return None;
    }

    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    if name.ends_with(".tmp") {
        return None;
    }

    let id = name.parse().ok()?;
    let manifest_path = path.join(TICKET_MANIFEST_FILE);
    if !manifest_path.exists() {
        if prune_empty_ticket_artifact_dir(path) {
            return None;
        }
        diags.push(crate::model::filesystem::ParseDiagnostic {
            path: manifest_path,
            reason: "missing ticket.toml".to_string(),
        });
        return None;
    }

    Some(ScanCandidate { id, manifest_path })
}

pub(super) fn prune_empty_ticket_artifact_dir(path: &Path) -> bool {
    let mut entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return false,
    };

    if entries.next().is_some() {
        return false;
    }

    fs::remove_dir(path).is_ok()
}

pub(super) fn load_scan_entry(
    id: Uuid,
    path: PathBuf,
    manifest_path: PathBuf,
) -> Result<Option<TicketScanEntry>, crate::model::filesystem::ParseDiagnostic> {
    match TicketFs::read(&path) {
        Ok(manifest) => Ok(Some(TicketScanEntry { id, path, manifest })),
        Err(StorageError::ParseError { path, reason }) => {
            Err(crate::model::filesystem::ParseDiagnostic { path, reason })
        }
        Err(error) => Err(crate::model::filesystem::ParseDiagnostic {
            path: manifest_path,
            reason: error.to_string(),
        }),
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

pub(super) fn write_manifest(dir: &Path, manifest: &TicketManifest) -> Result<(), StorageError> {
    let toml_str = crate::model::manifest_format::format_manifest_toml(manifest);
    let path = dir.join(TICKET_MANIFEST_FILE);
    fs::write(&path, toml_str)?;
    Ok(())
}
