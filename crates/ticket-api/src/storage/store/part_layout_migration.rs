mod execution;
mod integrity;
mod planning;
mod reports;
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
