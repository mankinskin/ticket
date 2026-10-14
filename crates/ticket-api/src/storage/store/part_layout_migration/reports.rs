use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::model::ticket::TicketPart;

pub(super) const PART_LAYOUT_REPORT_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
/// Outcome assigned to one ticket during the read-only layout scan.
pub enum PartLayoutTicketStatus {
    /// At least one path rename or eligible empty-part removal is planned.
    Planned,
    /// All retained paths already match the slug layout and no part is eligible for cleanup.
    Unchanged,
    /// One or more conflicts prevent applying this ticket's plan.
    Conflict,
}

pub(super) fn public_fingerprints_match(
    left: &[PartLayoutFileFingerprint],
    right: &[PartLayoutFileFingerprint],
) -> bool {
    let key = |file: &PartLayoutFileFingerprint| {
        (
            file.path.clone(),
            file.part_id,
            file.orphan,
            file.size,
            file.sha256.clone(),
        )
    };
    let mut left = left.iter().map(key).collect::<Vec<_>>();
    let mut right = right.iter().map(key).collect::<Vec<_>>();
    left.sort();
    right.sort();
    left == right
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// Path, size, and content fingerprint for a referenced part or orphan file.
pub struct PartLayoutFileFingerprint {
    /// Manifest-relative path.
    pub path: String,
    /// Observed byte length, or `None` when the path was missing.
    pub size: Option<u64>,
    /// Lowercase SHA-256 hex digest, or `None` when the path was missing.
    pub sha256: Option<String>,
    /// Owning part id for manifest-referenced files; `None` for an orphan.
    pub part_id: Option<Uuid>,
    /// Whether the file was discovered as an unreferenced part file.
    pub orphan: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// Manifest part metadata and its observed backing-file fingerprint.
pub struct PartLayoutPart {
    /// Original part metadata.
    pub part: TicketPart,
    /// Zero-based position in the original manifest.
    pub original_index: usize,
    /// Observed backing-file length.
    pub size: Option<u64>,
    /// Observed backing-file SHA-256.
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// A retained part whose manifest path will change.
pub struct PartLayoutPathChange {
    /// Stable part id; the id is never rewritten by the migration.
    pub part_id: Uuid,
    /// Original manifest-relative path.
    pub from: String,
    /// Deterministic slug path.
    pub to: String,
    /// Backing-file byte length.
    pub size: u64,
    /// Backing-file SHA-256, which must survive the rename.
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// A manifest-referenced empty frozen planning part eligible for removal.
pub struct PartLayoutCleanupCandidate {
    /// Full original manifest entry, including identity and metadata.
    pub part: TicketPart,
    /// Zero-based position to restore during undo.
    pub original_index: usize,
    /// Eligibility requires this to be zero.
    pub size: u64,
    /// Fingerprint of the empty backing file.
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// Parts sharing one normalized filename base and using ordinal suffixes.
pub struct PartLayoutBaseCollision {
    /// Normalized slug shared by multiple retained parts.
    pub normalized_base: String,
    /// Part ids assigned distinct ordinal filenames under the base.
    pub part_ids: Vec<Uuid>,
    /// Original kind strings sharing the normalized base.
    pub kinds: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// A part preserved because it does not satisfy the cleanup predicate.
pub struct PartLayoutSkip {
    /// Preserved part id.
    pub part_id: Uuid,
    /// Preserved manifest-relative path.
    pub path: String,
    /// Why the part does not satisfy cleanup eligibility.
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// Dry-run plan and complete preservation inventory for one ticket.
pub struct PartLayoutTicketPlan {
    /// Ticket id.
    pub id: Uuid,
    /// Ticket title at dry-run time.
    pub title: Option<String>,
    /// Canonical ticket directory used to bind later operations.
    pub ticket_path: String,
    /// Per-ticket result of path and cleanup analysis.
    pub status: PartLayoutTicketStatus,
    /// Exact pre-migration manifest bytes as UTF-8.
    pub manifest_before: String,
    /// SHA-256 of `manifest_before`.
    pub manifest_sha256: String,
    /// Canonical serialized manifest after the planned changes, when changed.
    pub manifest_after: Option<String>,
    /// SHA-256 of `manifest_after`, when changed.
    pub manifest_after_sha256: Option<String>,
    /// Every manifest part and its observed backing-file fingerprint.
    pub parts: Vec<PartLayoutPart>,
    /// Retained part entries in post-migration manifest order.
    pub parts_after: Vec<TicketPart>,
    /// Every discovered orphan file and its observed fingerprint.
    pub orphan_files: Vec<PartLayoutFileFingerprint>,
    /// Combined inventory of all referenced part files and discovered orphans.
    pub preservation: Vec<PartLayoutFileFingerprint>,
    /// Retained part path changes.
    pub path_changes: Vec<PartLayoutPathChange>,
    /// Parts satisfying the exact cleanup predicate.
    pub cleanup_candidates: Vec<PartLayoutCleanupCandidate>,
    /// Normalized slug bases that require ordinal suffixes.
    pub normalized_base_collisions: Vec<PartLayoutBaseCollision>,
    /// Excluded parts and their reasons.
    pub skipped_parts: Vec<PartLayoutSkip>,
    /// Unsafe or conflicting targets that block this ticket's apply.
    pub conflicts: Vec<String>,
    /// Proposed targets that were absent at dry-run time and must remain absent until apply.
    pub expected_absent_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// Read-only store-wide migration plan, suitable for saving and later apply.
pub struct PartLayoutDryRunReport {
    /// Serialized report schema version.
    pub version: u32,
    /// Id linking this report to its apply result.
    pub plan_id: Uuid,
    /// Canonical ticket index/store root.
    pub store_root: String,
    /// Number of tickets included in the scan.
    pub scanned_tickets: usize,
    /// Tickets with at least one planned change.
    pub planned_tickets: usize,
    /// Tickets requiring no changes.
    pub unchanged_tickets: usize,
    /// Tickets with one or more conflicts.
    pub conflict_tickets: usize,
    /// Total retained parts whose paths change.
    pub renamed_parts: usize,
    /// Total eligible empty planning parts.
    pub cleanup_candidates: usize,
    /// Total parts excluded from cleanup.
    pub skipped_parts: usize,
    /// Per-ticket plans and preservation inventories.
    pub tickets: Vec<PartLayoutTicketPlan>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// Apply result, including completed and unapplied tickets after a partial stop.
pub struct PartLayoutApplyReport {
    /// Serialized report schema version.
    pub version: u32,
    /// Id linking the apply, verify, and undo reports.
    pub apply_id: Uuid,
    /// Id of the consumed dry-run plan.
    pub plan_id: Uuid,
    /// Canonical ticket index/store root.
    pub store_root: String,
    /// Tickets whose transactions completed.
    pub completed_tickets: Vec<PartLayoutTicketPlan>,
    /// Planned ticket ids not applied before completion or stop.
    pub unapplied_ticket_ids: Vec<Uuid>,
    /// Unapplied tickets whose disk state matched neither the full old nor full new state.
    pub recovery_required_ticket_ids: Vec<Uuid>,
    /// Stop reason, if the store-wide operation did not finish.
    pub failure: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// Read-back failure for one ticket in a migration verify operation.
pub struct PartLayoutVerifyFailure {
    /// Ticket whose read-back did not match the expected result.
    pub ticket_id: Uuid,
    /// Observed mismatch or read error.
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// Verification obtained by reopening migrated manifests and backing files.
pub struct PartLayoutVerifyReport {
    /// Serialized report schema version.
    pub version: u32,
    /// Id of the apply report that was read back.
    pub apply_id: Uuid,
    /// Canonical ticket index/store root.
    pub store_root: String,
    /// True only when every completed ticket matched and apply was complete.
    pub passed: bool,
    /// Tickets whose manifest and files matched the applied plan.
    pub verified_ticket_ids: Vec<Uuid>,
    /// Tickets with read-back failures.
    pub failures: Vec<PartLayoutVerifyFailure>,
    /// Partial-apply failure propagated from the apply report, if any.
    pub apply_failure: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// Result of reversing only migration-owned changes from an apply report.
pub struct PartLayoutUndoReport {
    /// Serialized report schema version.
    pub version: u32,
    /// Id of the apply report that was undone.
    pub apply_id: Uuid,
    /// Canonical ticket index/store root.
    pub store_root: String,
    /// Tickets restored and read back to their pre-migration state.
    pub restored_ticket_ids: Vec<Uuid>,
    /// Completed apply tickets not restored because undo stopped or conflicted.
    pub not_restored_ticket_ids: Vec<Uuid>,
    /// Tickets whose state matched neither the full pre- nor post-migration state.
    pub recovery_required_ticket_ids: Vec<Uuid>,
    /// Tickets that had not completed apply and were not eligible for undo.
    pub apply_unapplied_ticket_ids: Vec<Uuid>,
    /// Stop reason, if any completed ticket could not be restored.
    pub failure: Option<String>,
}
