use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    path::{Path, PathBuf},
    sync::OnceLock,
    time::Instant,
};

use chrono::{DateTime, Utc};
use memory_kernel::{model::filesystem::ScanRoot, storage::ensure_sqlite_index_root};
use serde_json::Value;
use tracing::field::Empty;
use uuid::Uuid;

use crate::{
    error::StorageError,
    model::{
        filesystem::TICKET_MANIFEST_FILE,
        parts::ensure_nonblank_content,
        schema::TicketTypeSchemaExt,
        schema_registry::SchemaRegistry,
        ticket::{TicketId, TicketManifest},
    },
    storage::{
        index::RedbIndexStore, indexed::IndexedTicket, search::TantivySearchIndex,
        ticket_fs::TicketFs,
    },
    workspace,
};

#[path = "store/board.rs"]
mod board;
#[path = "store/lifecycle.rs"]
mod lifecycle;
#[path = "store/migration.rs"]
mod migration;
#[path = "store/part_layout_migration.rs"]
mod part_layout_migration;
#[path = "store/parts.rs"]
mod parts;
#[path = "store/projection.rs"]
mod projection;
#[path = "store/query.rs"]
mod query;
#[path = "store/release.rs"]
mod release;
#[path = "store/scan.rs"]
mod scan;
#[path = "store/store_open.rs"]
mod store_open;
#[path = "store/workflow_facts.rs"]
mod workflow_facts;

pub use self::{
    migration::{
        MIGRATION_CREATED_PART_IDS_KEY, MigrationApplyReport, MigrationDryRunReport,
        MigrationSegment, TicketMigrationPlan, split_description,
    },
    part_layout_migration::{
        PartLayoutApplyReport, PartLayoutBaseCollision, PartLayoutCleanupCandidate,
        PartLayoutDryRunReport, PartLayoutFileFingerprint, PartLayoutPart, PartLayoutPathChange,
        PartLayoutSkip, PartLayoutTicketPlan, PartLayoutTicketStatus, PartLayoutUndoReport,
        PartLayoutVerifyFailure, PartLayoutVerifyReport,
    },
    parts::{PART_HISTORY_CONTENT_KEY, PART_HISTORY_ID_KEY},
    projection::{ProjectedPart, ReadProjection, TicketProjection},
    release::{GateCheckOutcome, GateStatus, PromoteOutcome, ValidationResultOutcome},
    scan::ScanReport,
};

const STORE_TRACE_TARGET: &str = "ticket_api::storage::store";

/// History field key under which the pre-update `description.md` content is
/// captured whenever a description change is applied, regardless of
/// [`DescriptionUpdateMode`]. Used by [`TicketStore::apply_revert`] to
/// restore the description on undo.
pub const DESCRIPTION_HISTORY_KEY: &str = "__previous_description__";

/// How a caller-supplied `description` value should be applied to an
/// existing ticket's `description.md`.
///
/// Has no default: every caller supplying `description` must state a mode
/// explicitly (see [`REQUIRED_DESCRIPTION_MODE_ERROR`]). The historical
/// silent `Replace` default was the direct cause of destructive overwrites.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DescriptionUpdateMode {
    /// Overwrite `description.md` with the supplied text.
    Replace,
    /// Concatenate the supplied text onto the existing description.
    Append,
}

/// Error text returned when `description` is supplied with no
/// `description_mode` (AC1/AC2 of ticket 3d952036). Names both modes and
/// states which one preserves existing content.
pub const REQUIRED_DESCRIPTION_MODE_ERROR: &str = "update_ticket with `description` requires an explicit `description_mode`: 'replace' (overwrites description.md) or 'append' (preserves existing content by concatenating the new text onto it). There is no default; omitting description_mode is rejected.";

/// A caller's requested change to `description.md`, bundling the mode with
/// its content so "content supplied without a mode" is unrepresentable at
/// the type level: there is no separate `description_mode` field to omit
/// once a value of this type exists (AC5 of ticket 3d952036). Boundary
/// transports (HTTP/MCP JSON, CLI flags) decode two raw wire fields into
/// this type via [`DescriptionUpdate::decode`]; every Rust construction
/// downstream of that single decode point must pick one of these variants
/// explicitly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DescriptionUpdate {
    /// Leave `description.md` untouched.
    Unchanged,
    /// Overwrite `description.md` with the given text.
    Replace(String),
    /// Concatenate the given text onto the existing description.
    Append(String),
}

impl DescriptionUpdate {
    /// Boundary decoder: the one place a raw wire `description_mode` string
    /// is interpreted. Returns [`REQUIRED_DESCRIPTION_MODE_ERROR`] when
    /// `description` is set without a recognized mode string, and a named
    /// error for an unrecognized mode string.
    pub fn decode(
        description: Option<String>,
        description_mode: Option<&str>,
    ) -> Result<Self, String> {
        match description {
            None => Ok(DescriptionUpdate::Unchanged),
            Some(text) => match description_mode {
                Some("replace") => Ok(DescriptionUpdate::Replace(text)),
                Some("append") => Ok(DescriptionUpdate::Append(text)),
                Some(other) => Err(format!(
                    "invalid description_mode '{other}': expected 'replace' or 'append'"
                )),
                None => Err(REQUIRED_DESCRIPTION_MODE_ERROR.to_string()),
            },
        }
    }

    /// Split back into the `(content, mode)` pair
    /// [`TicketStore::update_with_options`] expects. Infallible: every
    /// variant already encodes a valid combination, so this can never hit
    /// the runtime `description_mode.ok_or_else(..)` guard in
    /// `apply_manifest_update` — that guard remains only as defense in depth
    /// for any caller that bypasses [`DescriptionUpdate`] entirely.
    pub fn as_parts(&self) -> (Option<&str>, Option<DescriptionUpdateMode>) {
        match self {
            DescriptionUpdate::Unchanged => (None, None),
            DescriptionUpdate::Replace(text) => {
                (Some(text.as_str()), Some(DescriptionUpdateMode::Replace))
            }
            DescriptionUpdate::Append(text) => {
                (Some(text.as_str()), Some(DescriptionUpdateMode::Append))
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct StoreOpenReport {
    pub initialized_store: bool,
    pub phase_timings_ms: BTreeMap<String, u64>,
    pub scan_reports: BTreeMap<String, ScanReport>,
}

/// Trait for receiving mutation events from the store (e.g. for SSE streaming).
///
/// Implement this in the HTTP layer and attach it via [`TicketStore::set_hook`].
pub trait StoreHook: Send + Sync + 'static {
    fn ticket_upsert(
        &self,
        id: Uuid,
        state: Option<String>,
        title: Option<String>,
        updated_at: chrono::DateTime<chrono::Utc>,
    );
    fn ticket_delete(&self, id: Uuid);
    fn edge_upsert(&self, from: Uuid, to: Uuid, kind: String);
    fn edge_delete(&self, from: Uuid, to: Uuid, kind: String);
}

/// The central ticket store: filesystem source-of-truth + SQLite metadata index +
/// Tantivy full-text search index.
///
/// Ticket manifests persist graph edges in file-backed fields such as
/// `depends_on` and `linked`, while SQLite caches the queryable edge table.
/// A forced scan backfills those file-backed edge fields for legacy stores and
/// then rebuilds the cached edge table from the tracked manifests.
pub struct TicketStore {
    index: RedbIndexStore,
    search: TantivySearchIndex,
    schema_registry: SchemaRegistry,
    /// Root directory for the SQLite database and Tantivy index files.
    pub index_root: PathBuf,
    /// Optional mutation hook. Set by the HTTP layer when streaming is active.
    /// Not used in CLI mode.
    hook: OnceLock<Box<dyn StoreHook>>,
}

const FILE_BACKED_EDGE_FIELDS: &[&str] = &["depends_on", "linked"];

impl TicketStore {
    /// Attach a mutation hook. May only be called once; subsequent calls
    /// are silently ignored (the first hook wins).
    pub fn set_hook(&self, hook: impl StoreHook) {
        let _ = self.hook.set(Box::new(hook));
    }

    /// Return a reference to the hook if one has been set.
    fn hook(&self) -> Option<&dyn StoreHook> {
        self.hook.get().map(|b| b.as_ref())
    }

    pub(crate) fn with_search_repair<T, F>(&self, mut op: F) -> Result<T, StorageError>
    where
        F: FnMut() -> Result<T, StorageError>,
    {
        // Proactively enforce the search-index structural invariants before a
        // write instead of catching a failure afterwards. A rebuild leaves the
        // index empty; the completeness invariant is restored by re-indexing the
        // on-disk tickets. Writes use the structural (rebuild-only) check rather
        // than the document-count check so an in-progress mutation that has
        // already updated the metadata index does not trigger a full reindex.
        if self.search.heal_if_needed()? {
            self.scan(true)?;
        }
        op()
    }

    /// Enforce the search-index completeness invariant before a read.
    ///
    /// Heals structural corruption (via [`TantivySearchIndex::num_docs`]) and
    /// repopulates the index from the on-disk tickets when its document count
    /// does not match the metadata index — the filesystem-backed source of
    /// truth that survives Tantivy corruption.
    pub(crate) fn ensure_search_complete(&self) -> Result<(), StorageError> {
        if self.search_needs_rebuild()? {
            self.scan(true)?;
        }
        Ok(())
    }

    /// Whether the search index must be rebuilt before it can be trusted.
    ///
    /// Returns `true` when the index cannot be opened/counted (structural or
    /// segment-content corruption) or when its document count differs from the
    /// metadata index. Calling this also heals the cheap structural invariants.
    fn search_needs_rebuild(&self) -> Result<bool, StorageError> {
        let indexed = self.index.list_tickets()?.len() as u64;
        match self.search.num_docs() {
            Ok(docs) => Ok(docs != indexed),
            Err(_) => Ok(true),
        }
    }

    pub fn schema_registry(&self) -> &SchemaRegistry {
        &self.schema_registry
    }
}

#[derive(Debug)]
struct EdgePatchPlan {
    kind: String,
    to_add: Vec<Uuid>,
    to_remove: Vec<Uuid>,
}

#[path = "store_helpers.rs"]
mod store_helpers;
use store_helpers::*;

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;

#[path = "store/creation.rs"]
mod creation;

#[path = "store/access.rs"]
mod access;

#[path = "store/mutation.rs"]
mod mutation;

#[path = "store/update_application.rs"]
mod update_application;

#[path = "store/transition_rules.rs"]
mod transition_rules;
