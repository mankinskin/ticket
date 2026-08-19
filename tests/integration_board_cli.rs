//! Integration tests for the `ticket board` subcommand family.
//!
//! Each test runs against a fully isolated `Sandbox` and exercises the real
//! `ticket` binary (via `CARGO_BIN_EXE_ticket`). No internal Rust APIs are
//! called directly; all assertions are made on the JSON output.

mod common;

use std::process::Command;

use common::{
    TicketCommands,
    TicketSandbox as Sandbox,
    create_ticket,
};

const TICKET: &str = env!("CARGO_BIN_EXE_ticket");

// ---------------------------------------------------------------------------
// Full lifecycle: check-in → heartbeat → update-files → show → check-out → show
// ---------------------------------------------------------------------------

#[path = "integration_board_cli/tests_board_ops.rs"]
mod tests_board_ops;
#[path = "integration_board_cli/tests_dependency_trees.rs"]
mod tests_dependency_trees;
#[path = "integration_board_cli/tests_next_display.rs"]
mod tests_next_display;
#[path = "integration_board_cli/tests_prioritization_and_help.rs"]
mod tests_prioritization_and_help;
