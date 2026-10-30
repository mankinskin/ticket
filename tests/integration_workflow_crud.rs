//! Sandboxed integration tests — ticket CRUD, search, scan, and lease workflows.
//!
//! Every test creates an isolated `Sandbox` backed by its own temp directory.
//! All operations go through the real `ticket` binary; no internal Rust APIs
//! are called directly.  JSON output is asserted via field access so tests
//! are independent of human-readable formatting.

mod common;

use std::process::Command;
use std::{fs, path::Path};

use common::{TicketCommands, TicketSandbox as Sandbox, create_ticket};
use ticket_api::storage::REQUIRED_DESCRIPTION_MODE_ERROR;

#[path = "integration_workflow_crud/crud_and_search.rs"]
mod crud_and_search;
#[allow(unused_imports)]
pub use crud_and_search::*;

#[path = "integration_workflow_crud/leases_and_batch.rs"]
mod leases_and_batch;
#[allow(unused_imports)]
pub use leases_and_batch::*;

#[path = "integration_workflow_crud/edge_unlink.rs"]
mod edge_unlink;
#[allow(unused_imports)]
pub use edge_unlink::*;
