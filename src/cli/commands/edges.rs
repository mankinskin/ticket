use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::Write,
};

use chrono::Utc;
use serde_json::{Value, json};
use uuid::Uuid;

use ticket_api::{model::edge::EdgeRecord, storage::TicketStore};

use crate::cli::{
    CliRunError, LinkArgs, LinksArgs, PruneDanglingArgs, SubgraphArgs, TopgraphArgs, UnlinkArgs,
};

use super::resolve_uuid_prefix;

#[path = "edges/edge_commands.rs"]
mod edge_commands;
#[allow(unused_imports)]
pub use edge_commands::*;

#[path = "edges/graph_traversal.rs"]
mod graph_traversal;
#[allow(unused_imports)]
pub use graph_traversal::*;

#[path = "edges/graph_rendering.rs"]
mod graph_rendering;
#[allow(unused_imports)]
pub use graph_rendering::*;
