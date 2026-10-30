use std::path::PathBuf;

use clap::{ArgGroup, Args, ValueEnum};
use uuid::Uuid;

#[path = "operations/workflow_commands.rs"]
mod workflow_commands;
#[allow(unused_imports)]
pub use workflow_commands::*;

#[path = "operations/ticket_operations.rs"]
mod ticket_operations;
#[allow(unused_imports)]
pub use ticket_operations::*;

#[path = "operations/edge_operations.rs"]
mod edge_operations;
#[allow(unused_imports)]
pub use edge_operations::*;

#[path = "operations/maintenance.rs"]
mod maintenance;
#[allow(unused_imports)]
pub use maintenance::*;

#[path = "operations/parts.rs"]
mod parts;
#[allow(unused_imports)]
pub use parts::*;
