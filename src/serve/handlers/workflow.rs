use std::{collections::HashSet, sync::Arc};

use axum::{
    extract::{Extension, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde::{Deserialize, Serialize};
use ticket_api::{
    storage::store::TicketStore,
    workflow::{BoardExcludedCandidate, WorkflowModel, WorkflowTreeNode, apply_board_filter},
};
use uuid::Uuid;
use viewer_api::error::{ApiError, RequestIdExt};

use crate::serve::{
    AppState,
    error::{storage_err, task_join_err},
    handlers::tickets::{TicketRef, ticket_ref_from_indexed},
};

#[cfg(test)]
#[path = "workflow_tests.rs"]
mod tests;

#[path = "workflow/models.rs"]
mod models;
#[allow(unused_imports)]
pub use models::*;

#[path = "workflow/next_candidates.rs"]
mod next_candidates;
#[allow(unused_imports)]
pub use next_candidates::*;

#[path = "workflow/workflow_trees.rs"]
mod workflow_trees;
#[allow(unused_imports)]
pub use workflow_trees::*;
