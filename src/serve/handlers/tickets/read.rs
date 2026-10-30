use std::collections::{BTreeMap, HashMap};

use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use uuid::Uuid;

use ticket_api::storage::ticket_fs::TicketFs;

use viewer_api::error::{ApiError, RequestIdExt};

use crate::serve::{
    AppState,
    error::{storage_err, task_join_err},
    registry::ResolvedIndexedTicket,
};

use super::types::{
    HistoryEntry, TicketDescriptionResponse, TicketDetail, TicketDetailResponse,
    TicketHistoryResponse, TicketIdParam, TicketRef, TicketSummary, TicketsResponse,
    WorkspaceParam, ticket_ref_from_indexed,
};

#[path = "read/ticket_lists.rs"]
mod ticket_lists;
#[allow(unused_imports)]
pub use ticket_lists::*;

#[path = "read/ticket_reads.rs"]
mod ticket_reads;
#[allow(unused_imports)]
pub use ticket_reads::*;

#[path = "read/ticket_resolution.rs"]
mod ticket_resolution;
#[allow(unused_imports)]
pub use ticket_resolution::*;
