use std::{
    collections::BTreeSet,
    fs,
    path::{Component, Path, PathBuf},
};

use serde_json::{Value, json};

use ticket_api::{
    contracts::command_schema::{export_command_schema, export_command_schema_json},
    model::schema_registry::SchemaRegistry,
    storage::TicketStore,
};

use super::{CliRunError, TicketCommandCli, batch, commands};

#[cfg(test)]
#[path = "dispatch/tests.rs"]
mod tests;

#[path = "dispatch/payloads.rs"]
mod payloads;
#[allow(unused_imports)]
pub use payloads::*;

#[path = "dispatch/dispatch_commands.rs"]
mod dispatch_commands;
#[allow(unused_imports)]
pub use dispatch_commands::*;

#[path = "dispatch/index_roots.rs"]
mod index_roots;
#[allow(unused_imports)]
pub use index_roots::*;

#[path = "dispatch/git_worktrees.rs"]
mod git_worktrees;
#[allow(unused_imports)]
pub use git_worktrees::*;
