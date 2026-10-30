use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use chrono::Utc;
use serde_json::{Value, json};
use uuid::Uuid;

use ticket_api::{
    TICKET_INDEX_AGENT_HOOK_PATH, TicketCatalogSource,
    error::StorageError,
    generate_ticket_catalog,
    model::ticket::TicketManifestExt,
    storage::{TicketStore, ticket_fs::TicketFs},
    workspace,
};

use crate::cli::{
    AddRootArgs, AttachArgs, BlockersArgs, CliRunError, FmtArgs, HealthArgs, IdArgs, NextArgs,
    ReadyOverviewArgs, ScanArgs, ServeCliArgs, StatusArgs, StoreIndexArgs, UnblockedByArgs,
    WatchArgs,
};

const STORE_DIR: &str = ".ticket";

#[path = "ops/health.rs"]
mod health;
#[path = "ops/next.rs"]
mod next;
#[path = "ops/status.rs"]
mod status;

#[cfg(test)]
#[path = "ops/validate_links_tests.rs"]
mod validate_links_tests;

#[path = "ops/scan_commands.rs"]
mod scan_commands;
#[allow(unused_imports)]
pub use scan_commands::*;

#[path = "ops/transition_commands.rs"]
mod transition_commands;
#[allow(unused_imports)]
pub use transition_commands::*;

#[path = "ops/store_index.rs"]
mod store_index;
#[allow(unused_imports)]
pub use store_index::*;

#[path = "ops/link_validation.rs"]
mod link_validation;
#[allow(unused_imports)]
pub use link_validation::*;

#[path = "ops/serve_commands.rs"]
mod serve_commands;
#[allow(unused_imports)]
pub use serve_commands::*;

#[path = "ops/status_reports.rs"]
mod status_reports;
#[allow(unused_imports)]
pub use status_reports::*;

#[path = "ops/format_command.rs"]
mod format_command;
#[allow(unused_imports)]
pub use format_command::*;
