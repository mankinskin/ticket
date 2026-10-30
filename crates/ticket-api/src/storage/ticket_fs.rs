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

#[path = "ticket_fs/part_layout.rs"]
mod part_layout;
#[cfg(test)]
#[path = "ticket_fs/tests.rs"]
mod tests;
#[path = "ticket_fs/transaction.rs"]
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

#[path = "ticket_fs/parts_support.rs"]
mod parts_support;
#[allow(unused_imports)]
pub use parts_support::*;

#[path = "ticket_fs/scan_support.rs"]
mod scan_support;
#[allow(unused_imports)]
pub use scan_support::*;

#[path = "ticket_fs/scan.rs"]
mod scan;

#[path = "ticket_fs/history.rs"]
mod history;

#[path = "ticket_fs/description.rs"]
mod description;

#[path = "ticket_fs/parts.rs"]
mod parts;
