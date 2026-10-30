#[allow(unused_imports)]
use super::*;

/// Show the legal state-transition graph for a ticket.
///
/// Returns the current state, the states reachable in a single hop
/// (`allowed_next_states`), the full transition edge list, and the schema's
/// declared states, required intermediate states, and terminal states. This is
/// the inspection surface paired with the invalid-transition recovery error.
pub(crate) fn cmd_transitions(args: IdArgs, store: &TicketStore) -> Result<Value, CliRunError> {
    let id = super::super::resolve_uuid_prefix(&args.id, store)?;
    let indexed = store.get_indexed(&id)?.ok_or_else(|| {
        CliRunError::BadRequest(format!(
            "ticket '{id}' was not found in the active workspace. Retry with --workspace-root <workspace-path> or --index-root <path-to-.ticket>."
        ))
    })?;
    let type_id = indexed.type_id.clone();
    let current_state = indexed.state.clone().unwrap_or_else(|| "open".to_string());
    let schema = store.schema_registry().get(&type_id).ok_or_else(|| {
        CliRunError::BadRequest(format!("no schema registered for ticket type '{type_id}'"))
    })?;

    let allowed_next = schema.allowed_next_states(&current_state);
    let transitions: Vec<Value> = schema
        .transitions
        .iter()
        .map(|t| json!({ "from": t.from, "to": t.to }))
        .collect();

    Ok(json!({
        "command": "transitions",
        "status": "ok",
        "id": id,
        "type": type_id,
        "current_state": current_state,
        "allowed_next_states": allowed_next,
        "states": schema.states,
        "transitions": transitions,
        "required_states": schema.required_states,
        "terminal_states": schema.terminal_states,
    }))
}

pub(crate) fn cmd_audit(store: &TicketStore) -> Result<Value, CliRunError> {
    let all = store.list(None, None, None)?;

    let mut state_counts = BTreeMap::new();
    for t in &all {
        let state = t.state.as_deref().unwrap_or("unknown");
        *state_counts.entry(state.to_string()).or_insert(0usize) += 1;
    }

    let mut type_counts = BTreeMap::new();
    for t in &all {
        *type_counts.entry(t.type_id.clone()).or_insert(0usize) += 1;
    }

    Ok(json!({
        "command": "audit",
        "status": "ok",
        "total": all.len(),
        "by_state": state_counts,
        "by_type": type_counts,
    }))
}
