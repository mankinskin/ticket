use std::{borrow::Cow, collections::BTreeMap};

use rmcp::schemars::{self, JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ticket_api::storage::DescriptionUpdate;

#[path = "types/ticket_queries.rs"]
mod ticket_queries;
#[allow(unused_imports)]
pub use ticket_queries::*;

#[path = "types/mutations.rs"]
mod mutations;
#[allow(unused_imports)]
pub use mutations::*;

#[path = "types/workflow.rs"]
mod workflow;
#[allow(unused_imports)]
pub use workflow::*;

#[path = "types/board.rs"]
mod board;
#[allow(unused_imports)]
pub use board::*;

#[path = "types/move_parts.rs"]
mod move_parts;
#[allow(unused_imports)]
pub use move_parts::*;
