#[allow(unused_imports)]
use super::*;

/// Identity used to derive the deterministic synthetic `id` of the implicit
/// `objective` part backfilled for legacy tickets with no `[[parts]]` table
/// (AC8/AC9). Never persisted; recomputed on every read so it stays stable
/// for a given ticket without requiring a one-time migration write.
pub(super) const IMPLICIT_OBJECTIVE_PART_NAME: &[u8] = b"ticket-api:implicit-objective";

pub(crate) fn implicit_objective_part_id(ticket_id: Uuid) -> Uuid {
    Uuid::new_v5(&ticket_id, IMPLICIT_OBJECTIVE_PART_NAME)
}

/// A single loaded content part: manifest metadata joined with its file
/// content read from `parts/` (or `description.md` for the implicit
/// legacy `objective` part).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedPart {
    pub id: Uuid,
    pub kind: String,
    /// Path relative to the ticket directory.
    pub path: String,
    pub frozen: bool,
    pub created_at: chrono::DateTime<Utc>,
    pub supersedes: Option<Uuid>,
    pub content: String,
    /// `true` for the synthetic `objective` part backfilled from
    /// `description.md` on a legacy ticket with no `[[parts]]` table.
    pub implicit: bool,
}

/// Result of loading a ticket's parts: entries in manifest (display/creation)
/// order, plus any orphan files under `parts/` that exist on disk but are
/// not referenced by the manifest — reported, never silently adopted (AC4).
#[derive(Debug, Clone, Default)]
pub struct PartsLoadReport {
    pub parts: Vec<LoadedPart>,
    /// Absolute paths of files under `parts/` with no matching manifest entry.
    pub orphans: Vec<PathBuf>,
}

impl PartsLoadReport {
    /// Address a part by its stable `id` — the only addressing key; kind and
    /// manifest index are never used for lookup (AC3).
    pub fn find(&self, id: Uuid) -> Option<&LoadedPart> {
        self.parts.iter().find(|part| part.id == id)
    }

    /// All parts of a given `kind`, in manifest order.
    pub fn of_kind<'a>(&'a self, kind: &'a str) -> impl Iterator<Item = &'a LoadedPart> {
        self.parts.iter().filter(move |part| part.kind == kind)
    }
}

/// Files under `parts/` with no matching entry in `referenced_paths`
/// (manifest-relative paths, e.g. `"parts/<id>.md"`). Absent `parts/`
/// yields no orphans rather than an error.
pub(super) fn find_orphan_part_files(
    ticket_path: &Path,
    referenced_paths: &BTreeSet<String>,
) -> Result<Vec<PathBuf>, StorageError> {
    let parts_dir = ticket_path.join(TICKET_PARTS_DIR);
    let read_dir = match fs::read_dir(&parts_dir) {
        Ok(read_dir) => read_dir,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Vec::new());
        }
        Err(error) => return Err(StorageError::Io(error)),
    };

    let mut orphans = Vec::new();
    for entry in read_dir {
        let path = entry?.path();
        if !path.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let relative = format!("{TICKET_PARTS_DIR}/{file_name}");
        if !referenced_paths.contains(&relative) {
            orphans.push(path);
        }
    }
    orphans.sort();
    Ok(orphans)
}

pub(super) fn validate_part_relative_path(path: &str) -> Result<(), StorageError> {
    if path.contains('\\') || Path::new(path).is_absolute() {
        return Err(StorageError::Other(format!(
            "unsafe ticket part path: {path}"
        )));
    }
    let components = Path::new(path).components().collect::<Vec<_>>();
    let first_is_parts = matches!(
        components.first(),
        Some(Component::Normal(name)) if *name == std::ffi::OsStr::new(TICKET_PARTS_DIR)
    );
    if components.len() < 2
        || !first_is_parts
        || components
            .iter()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(StorageError::Other(format!(
            "ticket part path must stay under {TICKET_PARTS_DIR}/: {path}"
        )));
    }
    Ok(())
}

pub(super) fn part_file_fingerprint(
    ticket_path: &Path,
    relative_path: &str,
) -> Result<(Option<u64>, Option<String>), StorageError> {
    validate_part_relative_path(relative_path)?;
    let path = ticket_path.join(relative_path);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok((None, None)),
        Err(error) => return Err(StorageError::Io(error)),
    };
    if !metadata.file_type().is_file() {
        return Err(StorageError::Other(format!(
            "ticket part path is not a regular file: {}",
            path.display()
        )));
    }
    let contents = fs::read(&path)?;
    Ok((Some(contents.len() as u64), Some(sha256_bytes(&contents))))
}

pub(super) fn parse_edge_targets(
    value: Option<&Value>,
    edge_kind: &str,
) -> Result<BTreeSet<String>, StorageError> {
    let Some(value) = value else {
        return Ok(BTreeSet::new());
    };

    let Some(items) = value.as_array() else {
        return Err(StorageError::Other(format!(
            "edge field '{}' must be an array of strings",
            edge_kind
        )));
    };

    let mut targets = BTreeSet::new();
    for item in items {
        let Some(target) = item.as_str() else {
            return Err(StorageError::Other(format!(
                "edge field '{}' must contain only string ticket IDs",
                edge_kind
            )));
        };
        targets.insert(target.to_string());
    }

    Ok(targets)
}
