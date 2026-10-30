use std::collections::HashMap;

use chrono::Utc;
use serde_json::{Value, json};
use uuid::Uuid;

use ticket_api::storage::{
    TicketStore,
    board::{
        BoardConfig, BoardEntry, BoardEntryStatus, BoardError, BoardHistorySnapshot, BoardSnapshot,
    },
};

use crate::cli::{BoardArgs, BoardCleanCommand, BoardCommand, CliRunError};

use super::resolve_uuid_prefix;

#[path = "board/render.rs"]
mod render;

pub(crate) use self::render::{BoardRecommendation, write_next_up};

use self::render::{
    BoardDisplay, BoardDisplayEntry, BoardHistoryDisplay, board_display_entry_to_json,
    config_to_json, entry_status, entry_to_json, heartbeat_age_secs, render_board_history_human,
    render_board_human,
};

// ── entry point ────────────────────────────────────────────────────────────────

pub(crate) fn cmd_board(args: BoardArgs, store: &TicketStore) -> Result<Value, CliRunError> {
    match args.command {
        BoardCommand::Show { agent } => cmd_board_show(agent.as_deref(), store),
        BoardCommand::History { agent } => cmd_board_history(agent.as_deref(), store),
        BoardCommand::Worktrees => cmd_board_worktrees(store),
        BoardCommand::CheckIn {
            id,
            agent,
            intent,
            files,
            ttl_secs,
            session_id,
            worktree_path,
            branch,
        } => cmd_board_check_in(
            id,
            agent,
            intent,
            files,
            ttl_secs,
            session_id,
            worktree_path,
            branch,
            store,
        ),
        BoardCommand::CheckOut { id, agent, reason } => {
            cmd_board_check_out(id, agent, reason, store)
        }
        BoardCommand::Heartbeat { entry_id } => cmd_board_heartbeat(entry_id, store),
        BoardCommand::Configure {
            max_wip,
            stale_after_secs,
            completed_audit_window_secs,
        } => cmd_board_configure(
            max_wip,
            stale_after_secs,
            completed_audit_window_secs,
            store,
        ),
        BoardCommand::Clean(clean_args) => match clean_args.command {
            BoardCleanCommand::Preview { include_stale } => {
                cmd_board_clean_preview(include_stale, store)
            }
            BoardCleanCommand::Apply {
                token,
                include_stale,
            } => cmd_board_clean_apply(token, include_stale, store),
        },
        BoardCommand::UpdateFiles {
            id,
            agent,
            add,
            remove,
        } => cmd_board_update_files(id, agent, add, remove, store),
        BoardCommand::RenameFile {
            id,
            agent,
            from,
            to,
        } => cmd_board_rename_file(id, agent, from, to, store),
    }
}

#[path = "board/board_read.rs"]
mod board_read;
#[allow(unused_imports)]
pub use board_read::*;

#[path = "board/board_write.rs"]
mod board_write;
#[allow(unused_imports)]
pub use board_write::*;
