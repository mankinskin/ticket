#[allow(unused_imports)]
use super::*;

// ── check-in ──────────────────────────────────────────────────────────────────

pub(super) fn cmd_board_check_in(
    id: String,
    agent: String,
    intent: Option<String>,
    files: Vec<String>,
    ttl_secs: Option<u64>,
    session_id: Option<String>,
    worktree_path: Option<String>,
    branch: Option<String>,
    store: &TicketStore,
) -> Result<Value, CliRunError> {
    let ticket_id = resolve_uuid_prefix(&id, store)?;
    let ttl = ttl_secs.unwrap_or(3600);
    let intent_str = intent.as_deref().unwrap_or("");

    let entry = store
        .board_check_in(
            &ticket_id,
            &agent,
            ttl,
            intent_str,
            files,
            session_id,
            worktree_path,
            branch,
        )
        .map_err(board_err_to_cli)?;

    Ok(json!({
        "command": "board_check_in",
        "status": "ok",
        "entry_id": entry.entry_id,
        "ticket_id": entry.ticket_id,
        "agent_id": entry.agent_id,
        "intent": entry.intent,
        "owned_files": entry.owned_files,
        "session_id": entry.session_id,
        "worktree_path": entry.worktree_path,
        "branch": entry.branch,
        "checked_in_at": entry.checked_in_at,
        "ttl_secs": entry.ttl_secs,
    }))
}

// ── check-out ─────────────────────────────────────────────────────────────────

pub(super) fn cmd_board_check_out(
    id: String,
    agent: Option<String>,
    reason: Option<String>,
    store: &TicketStore,
) -> Result<Value, CliRunError> {
    let ticket_id = resolve_uuid_prefix(&id, store)?;

    // Resolve agent: use supplied agent or fall back to any active agent on the ticket.
    let resolved_agent = if let Some(agent_id) = agent {
        agent_id
    } else {
        let snap = store.board_show(None)?;
        snap.entries
            .into_iter()
            .find(|entry| entry.ticket_id == ticket_id && entry.status == BoardEntryStatus::Active)
            .map(|entry| entry.agent_id)
            .ok_or_else(|| {
                CliRunError::BadRequest(format!(
                    "no active board entry found for ticket {ticket_id}; \
                     use --agent to specify the agent to check out"
                ))
            })?
    };

    let entry = store
        .board_check_out(&ticket_id, &resolved_agent, reason.as_deref())
        .map_err(board_err_to_cli)?;

    Ok(json!({
        "command": "board_check_out",
        "status": "ok",
        "entry_id": entry.entry_id,
        "ticket_id": entry.ticket_id,
        "agent_id": entry.agent_id,
        "handoff_reason": entry.handoff_reason,
        "status_field": "completed",
    }))
}

// ── heartbeat ─────────────────────────────────────────────────────────────────

pub(super) fn cmd_board_heartbeat(
    entry_id: String,
    store: &TicketStore,
) -> Result<Value, CliRunError> {
    let entry_id = entry_id.parse::<Uuid>().map_err(|_| {
        CliRunError::BadRequest(format!("invalid entry_id '{entry_id}': expected a UUID"))
    })?;

    let entry = store.board_heartbeat(&entry_id).map_err(board_err_to_cli)?;

    Ok(json!({
        "command": "board_heartbeat",
        "status": "ok",
        "entry_id": entry.entry_id,
        "ticket_id": entry.ticket_id,
        "agent_id": entry.agent_id,
        "last_heartbeat": entry.last_heartbeat,
    }))
}

// ── configure ─────────────────────────────────────────────────────────────────

pub(super) fn cmd_board_configure(
    max_wip: Option<u32>,
    stale_after_secs: Option<u64>,
    completed_audit_window_secs: Option<u64>,
    store: &TicketStore,
) -> Result<Value, CliRunError> {
    let new_config =
        if max_wip.is_none() && stale_after_secs.is_none() && completed_audit_window_secs.is_none()
        {
            None
        } else {
            let current = store.board_configure(None).map_err(board_err_to_cli)?;
            Some(BoardConfig {
                max_wip: max_wip.unwrap_or(current.max_wip),
                stale_after_secs: stale_after_secs.unwrap_or(current.stale_after_secs),
                completed_audit_window_secs: completed_audit_window_secs
                    .unwrap_or(current.completed_audit_window_secs),
            })
        };

    let config = store
        .board_configure(new_config)
        .map_err(board_err_to_cli)?;

    Ok(json!({
        "command": "board_configure",
        "status": "ok",
        "config": config_to_json(&config),
    }))
}

// ── clean preview ─────────────────────────────────────────────────────────────

pub(super) fn cmd_board_clean_preview(
    include_stale: bool,
    store: &TicketStore,
) -> Result<Value, CliRunError> {
    let preview = store
        .board_clean_preview(include_stale)
        .map_err(board_err_to_cli)?;

    Ok(json!({
        "command": "board_clean_preview",
        "status": "ok",
        "token": preview.token,
        "entry_count": preview.entry_count,
        "entry_ids": preview.entry_ids,
        "include_stale": preview.include_stale,
        "generated_at": preview.generated_at,
    }))
}

// ── clean apply ───────────────────────────────────────────────────────────────

pub(super) fn cmd_board_clean_apply(
    token: String,
    include_stale: bool,
    store: &TicketStore,
) -> Result<Value, CliRunError> {
    let result = store
        .board_clean_apply(&token, include_stale)
        .map_err(board_err_to_cli)?;

    Ok(json!({
        "command": "board_clean_apply",
        "status": "ok",
        "removed_count": result.removed_count,
        "removed_entry_ids": result.removed_entry_ids,
    }))
}

// ── update-files ──────────────────────────────────────────────────────────────

pub(super) fn cmd_board_update_files(
    id: String,
    agent: String,
    add: Vec<String>,
    remove: Vec<String>,
    store: &TicketStore,
) -> Result<Value, CliRunError> {
    let ticket_id = resolve_uuid_prefix(&id, store)?;
    let entry = store
        .board_update_files(&ticket_id, &agent, add, remove)
        .map_err(board_err_to_cli)?;

    Ok(json!({
        "command": "board_update_files",
        "status": "ok",
        "entry_id": entry.entry_id,
        "ticket_id": entry.ticket_id,
        "agent_id": entry.agent_id,
        "owned_files": entry.owned_files,
    }))
}

// ── rename-file ───────────────────────────────────────────────────────────────

pub(super) fn cmd_board_rename_file(
    id: String,
    agent: String,
    from: String,
    to: String,
    store: &TicketStore,
) -> Result<Value, CliRunError> {
    let ticket_id = resolve_uuid_prefix(&id, store)?;
    let entry = store
        .board_rename_file(&ticket_id, &agent, &from, &to)
        .map_err(board_err_to_cli)?;

    Ok(json!({
        "command": "board_rename_file",
        "status": "ok",
        "entry_id": entry.entry_id,
        "ticket_id": entry.ticket_id,
        "agent_id": entry.agent_id,
        "owned_files": entry.owned_files,
    }))
}

// ── error mapping ─────────────────────────────────────────────────────────────

pub(super) fn board_err_to_cli(err: BoardError) -> CliRunError {
    match &err {
        BoardError::WipLimitReached { current, max } => CliRunError::BadRequest(format!(
            "WIP limit reached: {current}/{max} active entries — check out a ticket or raise the limit with `board configure --max-wip`"
        )),
        BoardError::FileConflict {
            files,
            conflicting_agent,
            conflicting_ticket,
        } => CliRunError::BadRequest(format!(
            "file conflict: {files:?} already owned by agent '{conflicting_agent}' on ticket {conflicting_ticket}"
        )),
        BoardError::WorktreeConflict {
            worktree_path,
            conflicting_agent,
            conflicting_ticket,
        } => CliRunError::BadRequest(format!(
            "worktree conflict: '{worktree_path}' already held by agent '{conflicting_agent}' on ticket {conflicting_ticket}"
        )),
        BoardError::WorktreeRequiresSession { worktree_path } => CliRunError::BadRequest(format!(
            "worktree path '{worktree_path}' requires a session id"
        )),
        BoardError::AlreadyCheckedIn {
            ticket_id,
            agent_id,
        } => CliRunError::BadRequest(format!(
            "agent '{agent_id}' is already checked in for ticket {ticket_id}"
        )),
        BoardError::NotCheckedIn {
            ticket_id,
            agent_id,
        } => CliRunError::BadRequest(format!(
            "agent '{agent_id}' is not checked in for ticket {ticket_id}"
        )),
        BoardError::TicketNotFound(ticket_id) => {
            CliRunError::BadRequest(format!("ticket not found: {ticket_id}"))
        }
        BoardError::EntryNotFound(entry_id) => {
            CliRunError::BadRequest(format!("board entry not found: {entry_id}"))
        }
        BoardError::StaleCleanToken => CliRunError::BadRequest(
            "clean token is stale: the board has changed since the preview was generated — \
             run `board clean preview` again to get a fresh token"
                .to_string(),
        ),
        BoardError::FileRenameConflict {
            path,
            conflicting_agent,
            conflicting_ticket,
        } => CliRunError::BadRequest(format!(
            "rename conflict: '{path}' is already owned by agent '{conflicting_agent}' on ticket {conflicting_ticket}"
        )),
        BoardError::Storage(_) => CliRunError::Board(err),
    }
}
