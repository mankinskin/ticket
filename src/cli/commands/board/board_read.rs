#[allow(unused_imports)]
use super::*;

// ── show ──────────────────────────────────────────────────────────────────────

pub(super) fn cmd_board_show(
    agent: Option<&str>,
    store: &TicketStore,
) -> Result<Value, CliRunError> {
    let mut snap = store.board_show(agent)?;

    // When an agent is supplied, also refresh heartbeats for that agent's active
    // entries so the show itself acts as a heartbeat signal, then re-snapshot.
    if let Some(agent_id) = agent {
        let active_entry_ids: Vec<Uuid> = snap
            .caller_entries
            .iter()
            .filter(|entry| entry.status == BoardEntryStatus::Active)
            .map(|entry| entry.entry_id)
            .collect();

        for entry_id in &active_entry_ids {
            // Non-fatal: stale entries may already be gone.
            let _ = store.board_heartbeat(entry_id);
        }

        if !active_entry_ids.is_empty() {
            snap = store.board_show(Some(agent_id))?;
        }
    }

    let entries: Vec<Value> = snap
        .entries
        .iter()
        .map(|entry| entry_to_json(entry, &snap.config))
        .collect();
    let display = build_board_display(&snap, store)?;
    let current_work: Vec<Value> = display
        .current_work
        .iter()
        .map(board_display_entry_to_json)
        .collect();
    let actions = display.actions.clone();
    let human = render_board_human(&snap, &display);
    let file_ownership: Value = json!(snap.file_ownership);
    let active_index_root = store.index_root.display().to_string();

    Ok(json!({
        "command": "board_show",
        "status": "ok",
        "scope": {
            "active_index_root": active_index_root,
        },
        "captured_at": snap.captured_at,
        "active_count": snap.active_count,
        "stale_count": snap.stale_count,
        "conflict_count": snap.conflict_count,
        "wip_limit_reached": snap.wip_limit_reached,
        "config": config_to_json(&snap.config),
        "entries": entries,
        "current_work": current_work,
        "actions": actions,
        "warnings": snap.warnings,
        "file_ownership": file_ownership,
        "human": human,
    }))
}

pub(super) fn cmd_board_history(
    agent: Option<&str>,
    store: &TicketStore,
) -> Result<Value, CliRunError> {
    let snap = store.board_history(agent)?;
    let display = build_board_history_display(&snap, store)?;
    let entries: Vec<Value> = display
        .entries
        .iter()
        .map(board_display_entry_to_json)
        .collect();
    let human = render_board_history_human(&snap, &display);

    Ok(json!({
        "command": "board_history",
        "status": "ok",
        "captured_at": snap.captured_at,
        "completed_count": snap.completed_count,
        "hidden_completed_count": snap.hidden_completed_count,
        "history_window_secs": snap.config.completed_audit_window_secs,
        "config": config_to_json(&snap.config),
        "entries": entries,
        "human": human,
    }))
}

pub(super) fn cmd_board_worktrees(store: &TicketStore) -> Result<Value, CliRunError> {
    let snapshot = store.board_show(None)?;
    let worktrees = &snapshot.active_worktrees;
    let mut human = String::from("Active Worktrees:\n");
    if worktrees.is_empty() {
        human.push_str("  (no active worktrees)\n");
    } else {
        for worktree in worktrees {
            let branch = worktree.branch.as_deref().unwrap_or("-");
            let sessions = worktree.session_ids.join(", ");
            let agents = worktree.agent_ids.join(", ");
            let tickets = worktree
                .ticket_ids
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            human.push_str(&format!(
                "  {}  branch: {}  sessions: {}  agents: {}  tickets: {}{}\n",
                worktree.worktree_path,
                branch,
                sessions,
                agents,
                tickets,
                if worktree.conflicted {
                    "  CONFLICT"
                } else {
                    ""
                },
            ));
        }
    }

    Ok(json!({
        "command": "board_worktrees",
        "status": "ok",
        "active_worktrees": worktrees,
        "human": human,
    }))
}

pub(super) struct TicketSummary {
    pub(super) title: Option<String>,
    pub(super) state: Option<String>,
}

pub(super) fn build_board_display(
    snap: &BoardSnapshot,
    store: &TicketStore,
) -> Result<BoardDisplay, CliRunError> {
    let ticket_summaries = load_ticket_summaries(store)?;

    let mut current_work: Vec<&BoardEntry> = snap
        .entries
        .iter()
        .filter(|entry| is_current_work_status(&entry.status))
        .collect();
    current_work.sort_by(|left, right| {
        current_work_priority(&left.status)
            .cmp(&current_work_priority(&right.status))
            .then_with(|| right.checked_in_at.cmp(&left.checked_in_at))
    });

    let current_work: Vec<BoardDisplayEntry> = current_work
        .into_iter()
        .map(|entry| build_display_entry(entry, &snap.config, &ticket_summaries))
        .collect();

    let actions = build_actions(snap, &current_work);

    Ok(BoardDisplay {
        current_work,
        actions,
    })
}

pub(super) fn build_board_history_display(
    snap: &BoardHistorySnapshot,
    store: &TicketStore,
) -> Result<BoardHistoryDisplay, CliRunError> {
    let ticket_summaries = load_ticket_summaries(store)?;
    let entries = snap
        .entries
        .iter()
        .map(|entry| build_display_entry(entry, &snap.config, &ticket_summaries))
        .collect();

    Ok(BoardHistoryDisplay { entries })
}

pub(super) fn load_ticket_summaries(
    store: &TicketStore,
) -> Result<HashMap<Uuid, TicketSummary>, CliRunError> {
    Ok(store
        .list(None, None, None)?
        .into_iter()
        .map(|ticket| {
            (
                ticket.id,
                TicketSummary {
                    title: ticket.title,
                    state: ticket.state,
                },
            )
        })
        .collect())
}

pub(super) fn build_display_entry(
    entry: &BoardEntry,
    config: &BoardConfig,
    ticket_summaries: &HashMap<Uuid, TicketSummary>,
) -> BoardDisplayEntry {
    let age_secs = heartbeat_age_secs(entry, Utc::now());
    let summary = ticket_summaries.get(&entry.ticket_id);

    BoardDisplayEntry {
        entry_id: entry.entry_id,
        ticket_id: entry.ticket_id,
        title: summary
            .and_then(|ticket| ticket.title.clone())
            .unwrap_or_else(|| "(untitled ticket)".to_string()),
        state: summary.and_then(|ticket| ticket.state.clone()),
        agent_id: entry.agent_id.clone(),
        intent: non_empty_or_default(&entry.intent, "no intent recorded"),
        status: entry_status(entry, config, age_secs).to_string(),
        heartbeat_age_secs: age_secs,
        owned_files: entry.owned_files.clone(),
        handoff_reason: entry.handoff_reason.clone(),
        completed_at: history_completed_at(entry),
        session_id: entry.session_id.clone(),
        worktree_path: entry.worktree_path.clone(),
        branch: entry.branch.clone(),
    }
}

pub(crate) fn parse_board_recommendation(value: &Value) -> Option<BoardRecommendation> {
    Some(BoardRecommendation {
        rank: value.get("rank")?.as_u64()? as usize,
        ticket_id: value.get("id")?.as_str()?.to_string(),
        title: value
            .get("title")
            .and_then(Value::as_str)
            .filter(|title| !title.trim().is_empty())
            .unwrap_or("(untitled ticket)")
            .to_string(),
        state: value
            .get("state")
            .and_then(Value::as_str)
            .map(str::to_string),
        priority: value
            .get("priority")
            .and_then(Value::as_str)
            .unwrap_or("none")
            .to_string(),
        effort: value
            .get("effort")
            .and_then(Value::as_str)
            .map(str::to_string),
        dependency_count: value
            .get("dependency_count")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
        dependee_count: value
            .get("dependee_count")
            .or_else(|| value.get("dependees"))
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
        created_at: value
            .get("created_at")
            .and_then(Value::as_str)
            .unwrap_or("-")
            .to_string(),
    })
}

pub(super) fn build_actions(
    snap: &BoardSnapshot,
    current_work: &[BoardDisplayEntry],
) -> Vec<String> {
    let mut actions = Vec::new();

    if snap.conflict_count > 0 {
        actions.push(format!(
            "Resolve {} conflicting board entr{} before taking more work.",
            snap.conflict_count,
            plural_suffix(snap.conflict_count)
        ));
    }

    if snap.stale_count > 0 {
        actions.push(format!(
            "Review {} stale entr{} now. Heartbeat live work or run 'ticket board clean preview --include-stale' if the ownership is abandoned.",
            snap.stale_count,
            plural_suffix(snap.stale_count)
        ));
    }

    if snap.wip_limit_reached {
        actions.push(format!(
            "WIP is full at {}/{} active or stale entries. Reduce the board before starting additional work.",
            snap.active_count + snap.stale_count,
            snap.config.max_wip
        ));
    } else if current_work.is_empty() {
        actions.push("Board is clear. Run 'ticket next' to see unblocked tickets.".to_string());
    }

    actions
}

pub(super) fn is_current_work_status(status: &BoardEntryStatus) -> bool {
    matches!(
        status,
        BoardEntryStatus::Active | BoardEntryStatus::Stale | BoardEntryStatus::Conflict
    )
}

pub(super) fn current_work_priority(status: &BoardEntryStatus) -> u8 {
    match status {
        BoardEntryStatus::Conflict => 0,
        BoardEntryStatus::Stale => 1,
        BoardEntryStatus::Active => 2,
        BoardEntryStatus::Completed => 3,
    }
}

pub(super) fn non_empty_or_default(value: &str, default: &str) -> String {
    if value.trim().is_empty() {
        default.to_string()
    } else {
        value.to_string()
    }
}

pub(super) fn plural_suffix(count: u32) -> &'static str {
    if count == 1 { "y" } else { "ies" }
}

pub(super) fn history_completed_at(entry: &BoardEntry) -> Option<chrono::DateTime<Utc>> {
    (entry.status == BoardEntryStatus::Completed).then(|| {
        entry
            .completed_at
            .unwrap_or(entry.last_heartbeat.max(entry.checked_in_at))
    })
}
