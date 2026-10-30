//! API request and response types for the ticket viewer.
//!
//! Kept separate from the HTTP client so components can import types without
//! pulling in the transport layer.

use serde::{Deserialize, Serialize};

#[path = "types/tickets.rs"]
mod tickets;
#[allow(unused_imports)]
pub use tickets::*;

#[path = "types/graph.rs"]
mod graph;
#[allow(unused_imports)]
pub use graph::*;

#[path = "types/workflow.rs"]
mod workflow;
#[allow(unused_imports)]
pub use workflow::*;

#[path = "types/schema.rs"]
mod schema;
#[allow(unused_imports)]
pub use schema::*;

#[path = "types/requests.rs"]
mod requests;
#[allow(unused_imports)]
pub use requests::*;

#[path = "types/state.rs"]
mod state;
#[allow(unused_imports)]
pub use state::*;
