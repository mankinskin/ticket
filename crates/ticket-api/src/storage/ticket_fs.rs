use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Component, Path, PathBuf},
};

use chrono::Utc;
use fs4::fs_std::FileExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    error::StorageError,
    model::{
        filesystem::{
            TICKET_ASSETS_DIR, TICKET_HISTORY_FILE, TICKET_LOCK_FILE, TICKET_MANIFEST_FILE,
            TICKET_PARTS_DIR, parse_ticket_manifest_toml,
        },
        parts::{classify_part_kind, ensure_nonblank_content, part_kind_slug, part_slug_filename},
        ticket::{TicketManifest, TicketManifestExt},
    },
};

mod part_layout;
#[cfg(test)]
mod tests;
mod transaction;

pub(crate) use part_layout::{PartFileFingerprint, PartLayoutSnapshot, sha256_bytes};

use transaction::Operation as TransactionOperation;

/// A single immutable revision snapshot stored in `history.ndjson`.
///
/// Revisions are append-only; `revert` creates a new revision with old state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryRevision {
    /// 1-based sequential revision number.
    pub rev: u64,
    /// ISO-8601 UTC timestamp of when this revision was written.
    pub ts: String,
    /// Complete snapshot of the manifest `extra` fields at this revision.
    pub fields: BTreeMap<String, Value>,
    /// Identity of the user or agent who made this change (optional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
}

fn write_manifest_transactional(dir: &Path, manifest: &TicketManifest) -> Result<(), StorageError> {
    let toml_str = crate::model::manifest_format::format_manifest_toml(manifest);
    transaction::publish(
        dir,
        TransactionOperation::ManifestOnly,
        Vec::new(),
        Vec::new(),
        Some(toml_str.as_bytes()),
    )
}

fn read_optional_text(path: &Path) -> Result<Option<String>, StorageError> {
    match fs::read_to_string(path) {
        Ok(contents) => Ok(Some(contents)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(StorageError::Io(error)),
    }
}

/// Low-level filesystem operations for ticket folders.
///
/// Each ticket lives in a folder named by its UUID:
///
/// ```text
/// <scan_root>/<uuid>/
///   ticket.toml         ← manifest (TOML)
///   .ticket-lock        ← advisory lock file (held during writes)
///   assets/             ← optional attachments
/// ```
pub struct TicketFs;

impl TicketFs {
    /// Create a new ticket folder under `target_root`.
    ///
    /// Protocol:
    /// 1. Write manifest to a temp folder `<uuid>.tmp/`
    /// 2. Rename temp → final `<uuid>/` (atomic on POSIX; best-effort on Windows)
    ///
    /// Returns the absolute path to the created ticket folder.
    pub fn create(
        manifest: &TicketManifest,
        target_root: &Path,
        body: Option<&str>,
    ) -> Result<PathBuf, StorageError> {
        if let Some(body) = body {
            ensure_nonblank_content(body, format!("ticket {} description", manifest.id))?;
        }
        let uuid_str = manifest.id.to_string();
        let final_dir = target_root.join(&uuid_str);
        let temp_dir = target_root.join(format!("{}.tmp", uuid_str));

        if final_dir.exists() {
            return Err(StorageError::Io(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("ticket folder already exists: {}", final_dir.display()),
            )));
        }

        // Write to temp dir first.
        fs::create_dir_all(&temp_dir)?;
        write_manifest(&temp_dir, manifest)?;
        if let Some(text) = body {
            fs::write(temp_dir.join("description.md"), text)?;
        }

        // Rename temp → final.
        fs::rename(&temp_dir, &final_dir).map_err(|e| {
            // Clean up temp on failure.
            let _ = fs::remove_dir_all(&temp_dir);
            StorageError::Io(e)
        })?;

        Ok(final_dir)
    }

    /// Read and parse the manifest from an existing ticket folder.
    pub fn read(ticket_path: &Path) -> Result<TicketManifest, StorageError> {
        let lock_path = ticket_path.join(TICKET_LOCK_FILE);
        let lock_file = acquire_lock(&lock_path)?;
        let result = Self::read_unlocked(ticket_path);
        release_lock(&lock_file, result)
    }

    fn read_unlocked(ticket_path: &Path) -> Result<TicketManifest, StorageError> {
        let manifest_path = ticket_path.join(TICKET_MANIFEST_FILE);
        let content = fs::read_to_string(&manifest_path)?;
        parse_ticket_manifest_toml(manifest_path.clone(), &content).map_err(|d| {
            StorageError::ParseError {
                path: d.path,
                reason: d.reason,
            }
        })
    }

    /// Apply a field patch to the manifest on disk.
    ///
    /// Protocol:
    /// 1. Acquire `.ticket-lock` (exclusive)
    /// 2. Read + merge patch
    /// 3. Write updated `ticket.toml`
    /// 4. Release lock
    ///
    /// Returns the updated manifest.
    pub fn update(
        ticket_path: &Path,
        patch: &std::collections::BTreeMap<String, Value>,
        new_state: Option<&str>,
    ) -> Result<TicketManifest, StorageError> {
        let lock_path = ticket_path.join(TICKET_LOCK_FILE);
        let lock_file = acquire_lock(&lock_path)?;

        let result = (|| -> Result<TicketManifest, StorageError> {
            let mut manifest = Self::read_unlocked(ticket_path)?;
            // Apply extra-field patches. A `Value::Null` in the patch means
            // "delete this key" rather than "set it to null" — the manifest
            // TOML format has no null literal, so storing it verbatim used
            // to corrupt the field into an empty string on every write.
            for (k, v) in patch {
                if v.is_null() {
                    manifest.extra.remove(k);
                } else {
                    manifest.extra.insert(k.clone(), v.clone());
                }
            }
            // Apply state change.
            if let Some(state) = new_state {
                manifest
                    .extra
                    .insert("state".to_string(), Value::String(state.to_string()));
            }
            write_manifest_transactional(ticket_path, &manifest)?;
            Ok(manifest)
        })();

        release_lock(&lock_file, result)
    }

    fn part_path_for_kind(
        ticket_path: &Path,
        manifest: &TicketManifest,
        kind: &str,
    ) -> Result<String, StorageError> {
        let parts = manifest.parts();
        let base = part_kind_slug(kind);
        let ordinal = parts
            .iter()
            .filter(|part| part_kind_slug(&part.kind) == base)
            .count()
            + 1;
        let filename = part_slug_filename(kind, ordinal);
        let rel_path = format!("{TICKET_PARTS_DIR}/{filename}");

        let parts_dir = ticket_path.join(TICKET_PARTS_DIR);
        match fs::symlink_metadata(&parts_dir) {
            Ok(metadata) if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() => {
                return Err(StorageError::PartPathConflict {
                    ticket: manifest.id,
                    path: TICKET_PARTS_DIR.to_string(),
                });
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(StorageError::Io(error)),
        }
        let file_path = ticket_path.join(&rel_path);
        let occupied = match fs::symlink_metadata(&file_path) {
            Ok(_) => true,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(error) => return Err(StorageError::Io(error)),
        };
        if parts.iter().any(|part| part.path == rel_path) || occupied {
            return Err(StorageError::PartPathConflict {
                ticket: manifest.id,
                path: rel_path,
            });
        }

        Ok(rel_path)
    }

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

    fn scan_root_dir(scan_root: &Path) -> Result<Option<fs::ReadDir>, StorageError> {
        match fs::read_dir(scan_root) {
            Ok(read_dir) => Ok(Some(read_dir)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(StorageError::Io(error)),
        }
    }

    fn scan_root_entry(
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

    fn read_history_unlocked(ticket_path: &Path) -> Result<Vec<HistoryRevision>, StorageError> {
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

    fn append_history_unlocked(
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

    fn read_description_unlocked(ticket_path: &Path) -> Result<Option<String>, StorageError> {
        let description_path = ticket_path.join("description.md");
        match fs::read_to_string(&description_path) {
            Ok(description) => Ok(Some(description)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(io_error_at(&description_path, error)),
        }
    }

    /// Load a ticket's content parts: the `[[parts]]` manifest entries
    /// joined with their file content, plus a report of orphan files under
    /// `parts/` not referenced by any manifest entry.
    ///
    /// A legacy ticket with no `[[parts]]` table (AC8) reads back a single
    /// synthetic `objective` part backed by `description.md`, with a
    /// deterministic `id` derived from the ticket id so it stays stable
    /// across reads without requiring a migration write.
    pub fn load_parts(
        ticket_path: &Path,
        manifest: &TicketManifest,
    ) -> Result<PartsLoadReport, StorageError> {
        let lock_path = ticket_path.join(TICKET_LOCK_FILE);
        let lock_file = acquire_lock(&lock_path)?;
        let result = (|| -> Result<PartsLoadReport, StorageError> {
            let current_manifest = Self::read_unlocked(ticket_path)?;
            if manifest.id != current_manifest.id {
                return Err(StorageError::Other(format!(
                    "ticket manifest id changed while loading parts at {}",
                    ticket_path.display()
                )));
            }
            let entries = current_manifest.parts();
            if entries.is_empty() {
                let content = Self::read_description_unlocked(ticket_path)?.unwrap_or_default();
                return Ok(PartsLoadReport {
                    parts: vec![LoadedPart {
                        id: implicit_objective_part_id(current_manifest.id),
                        kind: "objective".to_string(),
                        path: "description.md".to_string(),
                        frozen: false,
                        created_at: current_manifest.created_at,
                        supersedes: None,
                        content,
                        implicit: true,
                    }],
                    orphans: Vec::new(),
                });
            }

            let mut parts = Vec::with_capacity(entries.len());
            let mut referenced_paths: BTreeSet<String> = BTreeSet::new();
            for entry in entries {
                let part_path = ticket_path.join(&entry.path);
                let content = fs::read_to_string(&part_path)
                    .map_err(|error| io_error_at(&part_path, error))?;
                referenced_paths.insert(entry.path.clone());
                parts.push(LoadedPart {
                    id: entry.id,
                    kind: entry.kind,
                    path: entry.path,
                    frozen: entry.frozen,
                    created_at: entry.created_at,
                    supersedes: entry.supersedes,
                    content,
                    implicit: false,
                });
            }

            let orphans = find_orphan_part_files(ticket_path, &referenced_paths)?;
            Ok(PartsLoadReport { parts, orphans })
        })();
        release_lock(&lock_file, result)
    }

    /// Write content to a single content part addressed by its stable
    /// opaque `part_id` — never by `kind` or manifest index.
    ///
    /// If `part_id` matches an existing `[[parts]]` entry, only that
    /// entry's file is overwritten; its `kind` is left unchanged (`kind` is
    /// assigned once at creation and never reclassified by a content
    /// write). If `part_id` matches no existing entry, a new part is
    /// created with `kind` and appended to the manifest; `supersedes` is
    /// set on the new entry (used only by `amendment` parts, `None`
    /// otherwise — ignored when overwriting an existing part).
    ///
    /// A legacy ticket with no `[[parts]]` table addresses its implicit
    /// `objective` part (backed by `description.md`) via
    /// [`implicit_objective_part_id`]; any other `part_id` creates a new
    /// explicit part alongside it without disturbing `description.md`.
    ///
    /// Only the addressed part's file is ever read or written: writing a
    /// `review` or `validation` part never touches `objective` or
    /// `description.md` (AC4 of ticket 3d952036).
    ///
    /// Frozen-part rejection is enforced by the caller
    /// (`TicketStore::enforce_part_write_gate`) before this is reached —
    /// this is a crate-private low-level primitive with no external entry
    /// point (spec 24b3d22b, ticket f9e70385, AC7).
    ///
    /// Returns the updated manifest and the part's prior content (`None`
    /// if the write created the part).
    pub(crate) fn write_part(
        ticket_path: &Path,
        part_id: Uuid,
        kind: &str,
        content: &str,
        supersedes: Option<Uuid>,
    ) -> Result<(TicketManifest, Option<String>), StorageError> {
        ensure_nonblank_content(content, format!("ticket part {part_id}"))?;
        let lock_path = ticket_path.join(TICKET_LOCK_FILE);
        let lock_file = acquire_lock(&lock_path)?;

        let result = (|| -> Result<(TicketManifest, Option<String>), StorageError> {
            let mut manifest = Self::read_unlocked(ticket_path)?;
            let mut parts = manifest.parts();

            if let Some(entry) = parts.iter().find(|p| p.id == part_id) {
                let rel_path = entry.path.clone();
                let prior = read_optional_text(&ticket_path.join(&rel_path))?;
                let manifest_bytes = crate::model::manifest_format::format_manifest_toml(&manifest);
                transaction::publish(
                    ticket_path,
                    TransactionOperation::Overwrite,
                    vec![(rel_path, content.as_bytes().to_vec())],
                    Vec::new(),
                    Some(manifest_bytes.as_bytes()),
                )?;
                return Ok((manifest, prior));
            }

            if parts.is_empty() && part_id == implicit_objective_part_id(manifest.id) {
                let prior = Self::read_description_unlocked(ticket_path)?;
                transaction::publish(
                    ticket_path,
                    TransactionOperation::Overwrite,
                    vec![("description.md".to_string(), content.as_bytes().to_vec())],
                    Vec::new(),
                    None,
                )?;
                return Ok((manifest, prior));
            }

            // New part: create its file under parts/ and append a manifest
            // entry. Never touches any other part's file. Reject a
            // near-miss core-kind typo here — the only point where `kind`
            // is actually persisted (an existing part's `kind` is fixed at
            // creation and ignored above) — spec 24b3d22b AC2.
            classify_part_kind(kind)?;
            let rel_path = Self::part_path_for_kind(ticket_path, &manifest, kind)?;
            parts.push(crate::model::ticket::TicketPart {
                id: part_id,
                kind: kind.to_string(),
                path: rel_path.clone(),
                frozen: false,
                created_at: Utc::now(),
                supersedes,
            });
            manifest.set_parts(parts);
            let manifest_bytes = crate::model::manifest_format::format_manifest_toml(&manifest);
            transaction::publish(
                ticket_path,
                TransactionOperation::Create,
                vec![(rel_path, content.as_bytes().to_vec())],
                Vec::new(),
                Some(manifest_bytes.as_bytes()),
            )?;
            Ok((manifest, None))
        })();

        release_lock(&lock_file, result)
    }

    /// Append a new typed `[[refs]]` entry through the gated manifest write
    /// path, validating `kind` against the closed vocabulary and `urn`
    /// against the shape expected for that `kind` (spec 24b3d22b, ticket
    /// 9d69e93d, AC2/AC3). Rejects before touching the manifest on disk.
    pub(crate) fn write_ref(
        ticket_path: &Path,
        kind: &str,
        urn: &str,
        note: Option<String>,
    ) -> Result<TicketManifest, StorageError> {
        crate::model::refs::validate_new_ref(kind, urn)?;

        let lock_path = ticket_path.join(TICKET_LOCK_FILE);
        let lock_file = acquire_lock(&lock_path)?;

        let result = (|| -> Result<TicketManifest, StorageError> {
            let mut manifest = Self::read_unlocked(ticket_path)?;
            let mut refs = manifest.refs();
            refs.push(crate::model::ticket::TicketRefEntry {
                kind: kind.to_string(),
                urn: urn.to_string(),
                note,
            });
            manifest.set_refs(refs);
            write_manifest_transactional(ticket_path, &manifest)?;
            Ok(manifest)
        })();

        release_lock(&lock_file, result)
    }

    /// Remove a typed `[[refs]]` entry matching `kind` and `urn` exactly,
    /// through the same gated manifest write path as [`Self::write_ref`].
    /// A no-op (returns the unchanged manifest) when no entry matches.
    pub(crate) fn remove_ref(
        ticket_path: &Path,
        kind: &str,
        urn: &str,
    ) -> Result<TicketManifest, StorageError> {
        let lock_path = ticket_path.join(TICKET_LOCK_FILE);
        let lock_file = acquire_lock(&lock_path)?;

        let result = (|| -> Result<TicketManifest, StorageError> {
            let mut manifest = Self::read_unlocked(ticket_path)?;
            let mut refs = manifest.refs();
            refs.retain(|entry| !(entry.kind == kind && entry.urn == urn));
            manifest.set_refs(refs);
            write_manifest_transactional(ticket_path, &manifest)?;
            Ok(manifest)
        })();

        release_lock(&lock_file, result)
    }

    /// Remove `[[parts]]` entries matching any of `part_ids` and delete
    /// their backing files, used by the description migration's undo path
    /// (`TicketStore::migration_undo`) to restore the pre-migration layout.
    /// Ids with no matching entry are ignored. `description.md` itself is
    /// never touched — migration never overwrites it, so it already holds
    /// the pre-migration content.
    pub(crate) fn remove_parts(
        ticket_path: &Path,
        part_ids: &[Uuid],
    ) -> Result<TicketManifest, StorageError> {
        let lock_path = ticket_path.join(TICKET_LOCK_FILE);
        let lock_file = acquire_lock(&lock_path)?;

        let result = (|| -> Result<TicketManifest, StorageError> {
            let mut manifest = Self::read_unlocked(ticket_path)?;
            let mut parts = manifest.parts();
            let mut removed_paths = Vec::new();
            parts.retain(|p| {
                if part_ids.contains(&p.id) {
                    removed_paths.push(p.path.clone());
                    false
                } else {
                    true
                }
            });
            manifest.set_parts(parts);
            let manifest_bytes = crate::model::manifest_format::format_manifest_toml(&manifest);
            transaction::publish(
                ticket_path,
                TransactionOperation::RemoveParts,
                Vec::new(),
                removed_paths,
                Some(manifest_bytes.as_bytes()),
            )?;
            Ok(manifest)
        })();

        release_lock(&lock_file, result)
    }

    /// f9e70385, AC1/AC5), invoked exclusively from the state-transition
    /// path (`TicketStore::update_with_options`) whenever a ticket enters
    /// or leaves `ready`. This is the sanctioned freeze/unfreeze
    /// mechanism itself — not a content write — so it does not go through
    /// `TicketStore::enforce_part_write_gate`; there is no other privileged
    /// bypass.
    ///
    /// `freeze = true`: existing entries whose kinds are in
    /// [`crate::model::parts::PLANNING_PART_KINDS`] are set `frozen = true`.
    /// Missing kinds remain absent. `plan_revision` is incremented, which
    /// appends a plan revision to history via the caller's normal snapshot.
    ///
    /// `freeze = false`: every part's `frozen` flag is cleared, regardless
    /// of kind.
    pub(crate) fn apply_plan_freeze(
        ticket_path: &Path,
        freeze: bool,
    ) -> Result<TicketManifest, StorageError> {
        let lock_path = ticket_path.join(TICKET_LOCK_FILE);
        let lock_file = acquire_lock(&lock_path)?;

        let result = (|| -> Result<TicketManifest, StorageError> {
            let mut manifest = Self::read_unlocked(ticket_path)?;
            let mut parts = manifest.parts();

            if freeze {
                for part in &mut parts {
                    if crate::model::parts::is_planning_part_kind(&part.kind) {
                        part.frozen = true;
                    }
                }
                let revision = manifest
                    .extra
                    .get("plan_revision")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
                    + 1;
                manifest
                    .extra
                    .insert("plan_revision".to_string(), Value::Number(revision.into()));
            } else {
                for part in parts.iter_mut() {
                    part.frozen = false;
                }
            }

            manifest.set_parts(parts);
            write_manifest_transactional(ticket_path, &manifest)?;
            Ok(manifest)
        })();

        release_lock(&lock_file, result)
    }

    pub fn update_edge_field(
        ticket_path: &Path,
        edge_kind: &str,
        target: Uuid,
        present: bool,
    ) -> Result<(TicketManifest, bool), StorageError> {
        let lock_path = ticket_path.join(TICKET_LOCK_FILE);
        let lock_file = acquire_lock(&lock_path)?;

        let result = (|| -> Result<(TicketManifest, bool), StorageError> {
            let mut manifest = Self::read_unlocked(ticket_path)?;
            let mut targets = parse_edge_targets(manifest.extra.get(edge_kind), edge_kind)?;

            let changed = if present {
                targets.insert(target.to_string())
            } else {
                targets.remove(&target.to_string())
            };

            if changed {
                if targets.is_empty() {
                    manifest.extra.remove(edge_kind);
                } else {
                    manifest.extra.insert(
                        edge_kind.to_string(),
                        Value::Array(targets.into_iter().map(Value::String).collect()),
                    );
                }
                write_manifest_transactional(ticket_path, &manifest)?;
            }

            Ok((manifest, changed))
        })();

        release_lock(&lock_file, result)
    }
}

/// Identity used to derive the deterministic synthetic `id` of the implicit
/// `objective` part backfilled for legacy tickets with no `[[parts]]` table
/// (AC8/AC9). Never persisted; recomputed on every read so it stays stable
/// for a given ticket without requiring a one-time migration write.
const IMPLICIT_OBJECTIVE_PART_NAME: &[u8] = b"ticket-api:implicit-objective";

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

pub struct TicketScanEntry {
    pub id: Uuid,
    pub path: PathBuf,
    pub manifest: TicketManifest,
}

struct ScanCandidate {
    id: Uuid,
    manifest_path: PathBuf,
}

fn scan_candidate_path(
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

fn prune_empty_ticket_artifact_dir(path: &Path) -> bool {
    let mut entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return false,
    };

    if entries.next().is_some() {
        return false;
    }

    fs::remove_dir(path).is_ok()
}

fn load_scan_entry(
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

fn write_manifest(dir: &Path, manifest: &TicketManifest) -> Result<(), StorageError> {
    let toml_str = crate::model::manifest_format::format_manifest_toml(manifest);
    let path = dir.join(TICKET_MANIFEST_FILE);
    fs::write(&path, toml_str)?;
    Ok(())
}

/// Files under `parts/` with no matching entry in `referenced_paths`
/// (manifest-relative paths, e.g. `"parts/<id>.md"`). Absent `parts/`
/// yields no orphans rather than an error.
fn find_orphan_part_files(
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

fn validate_part_relative_path(path: &str) -> Result<(), StorageError> {
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

fn part_file_fingerprint(
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

fn parse_edge_targets(
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

/// Wrap an io error so its message names the path it failed on while keeping
/// its `ErrorKind`, so existing kind-based matches keep working.
fn io_error_at(path: &Path, error: std::io::Error) -> StorageError {
    StorageError::Io(std::io::Error::new(
        error.kind(),
        format!("{}: {error}", path.display()),
    ))
}

fn acquire_lock(lock_path: &Path) -> Result<File, StorageError> {
    // Refuse to lock inside a missing ticket dir: creating the lock file would fail with a
    // bare OS error, and recreating the directory would resurrect a deleted ticket.
    if let Some(ticket_path) = lock_path.parent() {
        if !ticket_path.is_dir() {
            return Err(StorageError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("ticket directory not found: {}", ticket_path.display()),
            )));
        }
    }
    // Keep the path stable; unlinking after unlock lets concurrent writers lock different inodes.
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(lock_path)
        .map_err(|error| io_error_at(lock_path, error))?;
    file.lock_exclusive().map_err(StorageError::Io)?;
    if let Some(ticket_path) = lock_path.parent() {
        transaction::recover(ticket_path)?;
    }
    Ok(file)
}

fn release_lock<T>(file: &File, result: Result<T, StorageError>) -> Result<T, StorageError> {
    let unlock = file.unlock().map_err(StorageError::Io);
    match (result, unlock) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) | (Ok(_), Err(error)) => Err(error),
        (Err(operation_error), Err(unlock_error)) => Err(StorageError::Other(format!(
            "{operation_error}; lock release failed: {unlock_error}"
        ))),
    }
}
