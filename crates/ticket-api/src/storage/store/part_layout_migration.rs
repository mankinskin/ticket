#[path = "part_layout_migration/execution.rs"]
mod execution;
#[path = "part_layout_migration/integrity.rs"]
mod integrity;
#[path = "part_layout_migration/planning.rs"]
mod planning;
#[path = "part_layout_migration/reports.rs"]
mod reports;
#[path = "part_layout_migration/verify_undo.rs"]
mod verify_undo;

pub use reports::{
    PartLayoutApplyReport, PartLayoutBaseCollision, PartLayoutCleanupCandidate,
    PartLayoutDryRunReport, PartLayoutFileFingerprint, PartLayoutPart, PartLayoutPathChange,
    PartLayoutSkip, PartLayoutTicketPlan, PartLayoutTicketStatus, PartLayoutUndoReport,
    PartLayoutVerifyFailure, PartLayoutVerifyReport,
};

#[cfg(test)]
#[path = "part_layout_migration/tests.rs"]
mod tests;
