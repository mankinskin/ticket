use axum::{
    body::to_bytes,
    extract::{Extension, Path, Query, State},
    http::StatusCode,
};
use std::{collections::BTreeMap, sync::Arc};
use ticket_api::storage::search::TantivySearchIndex;
use uuid::Uuid;
use viewer_api::error::RequestIdExt;

use super::{
    super::{
        TicketAssetParam, TicketIdParam, WorkspaceParam, get_ticket, get_ticket_asset,
        get_ticket_description, get_ticket_history, list_ticket_files, list_tickets,
    },
    make_state, make_store,
};

#[path = "listing_workspace/workspace_labels.rs"]
mod workspace_labels;
#[allow(unused_imports)]
pub use workspace_labels::*;

#[path = "listing_workspace/ticket_refs_and_history.rs"]
mod ticket_refs_and_history;
#[allow(unused_imports)]
pub use ticket_refs_and_history::*;

#[path = "listing_workspace/label_collisions.rs"]
mod label_collisions;
#[allow(unused_imports)]
pub use label_collisions::*;
