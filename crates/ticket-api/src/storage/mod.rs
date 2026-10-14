pub mod board;
pub mod domain_store;
pub mod index;
pub mod indexed;
pub mod move_execution;
pub mod move_planner;
pub mod schema;
pub mod search;
pub mod store;
pub mod ticket_fs;

#[cfg(test)]
mod tests;

pub use board::{
    BoardCleanPreview, BoardCleanResult, BoardConfig, BoardEntry, BoardEntryStatus, BoardError,
    BoardReconcileResult, BoardSnapshot, ReconcileAction,
};
pub use domain_store::{TicketCreateInput, TicketListQuery, TicketUpdatePatch};
pub use store::{
    DESCRIPTION_HISTORY_KEY, DescriptionUpdate, DescriptionUpdateMode, PART_HISTORY_CONTENT_KEY,
    PART_HISTORY_ID_KEY, PartLayoutApplyReport, PartLayoutBaseCollision,
    PartLayoutCleanupCandidate, PartLayoutDryRunReport, PartLayoutFileFingerprint, PartLayoutPart,
    PartLayoutPathChange, PartLayoutSkip, PartLayoutTicketPlan, PartLayoutTicketStatus,
    PartLayoutUndoReport, PartLayoutVerifyFailure, PartLayoutVerifyReport, ProjectedPart,
    REQUIRED_DESCRIPTION_MODE_ERROR, ReadProjection, TicketProjection, TicketStore,
};
