use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
    time::Instant,
};

use axum::response::{IntoResponse, Json, Response};
use ticket_api::{
    model::edge::EdgeRecord,
    storage::{indexed::IndexedTicket, store::TicketStore, ticket_fs::TicketFs},
};
use uuid::Uuid;

use crate::serve::{
    AppState,
    error::{storage_err, task_join_err},
    handlers::tickets::{TicketRef, ticket_ref_from_indexed},
    registry::ResolvedIndexedTicket,
};

use super::{
    EdgeItem, NodeItem, SubgraphQuery, SubgraphResponse, SubgraphStats, TopgraphQuery,
    WorkspaceGraphQuery,
};

#[path = "traversal/request_types.rs"]
mod request_types;
#[allow(unused_imports)]
pub use request_types::*;

#[path = "traversal/handlers.rs"]
mod handlers;
#[allow(unused_imports)]
pub use handlers::*;

#[path = "traversal/traversal_core.rs"]
mod traversal_core;
#[allow(unused_imports)]
pub use traversal_core::*;

#[path = "traversal/workspace_graph.rs"]
mod workspace_graph;
#[allow(unused_imports)]
pub use workspace_graph::*;
