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

#[path = "listing_followups/duplicate_workspace_followups.rs"]
mod duplicate_workspace_followups;
#[allow(unused_imports)]
pub use duplicate_workspace_followups::*;

#[path = "listing_followups/stale_hits_and_filters.rs"]
mod stale_hits_and_filters;
#[allow(unused_imports)]
pub use stale_hits_and_filters::*;

#[path = "listing_followups/public_workspace_ids.rs"]
mod public_workspace_ids;
#[allow(unused_imports)]
pub use public_workspace_ids::*;
