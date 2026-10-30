#[allow(unused_imports)]
use super::*;

pub(super) fn resolve_tickets(
    state: &AppState,
    active_workspace: &str,
    ids: &[Uuid],
    request_id: &str,
) -> Result<HashMap<Uuid, ResolvedIndexedTicket>, Response> {
    state
        .registry
        .resolve_indexed_many(active_workspace, ids)
        .map_err(|error| storage_err(error, request_id))
}

pub(super) fn resolve_ticket(
    state: &AppState,
    active_workspace: &str,
    id: Uuid,
    request_id: &str,
) -> Result<ResolvedIndexedTicket, Response> {
    let mut resolved = resolve_tickets(state, active_workspace, &[id], request_id)?;
    resolved.remove(&id).ok_or_else(|| {
        ApiError::not_found("ticket", request_id).into_response_with_status(StatusCode::NOT_FOUND)
    })
}

pub(super) fn resolve_workspace_request(
    state: &AppState,
    requested_workspace: &str,
    request_id: &str,
) -> Result<
    (
        String,
        std::sync::Arc<ticket_api::storage::store::TicketStore>,
    ),
    Response,
> {
    state.resolve_public_workspace_request(requested_workspace, request_id)
}

pub(super) fn ticket_ref_from_resolved(ticket: &ResolvedIndexedTicket) -> TicketRef {
    TicketRef {
        workspace: ticket.workspace.clone(),
        id: ticket.ticket.id.to_string(),
    }
}

pub(super) fn ticket_summary_from_indexed(
    ticket_ref: TicketRef,
    ticket: &ticket_api::storage::indexed::IndexedTicket,
) -> TicketSummary {
    TicketSummary {
        id: ticket.id.to_string(),
        ticket_ref,
        type_id: ticket.type_id.clone(),
        title: ticket.title.clone(),
        state: ticket.state.clone(),
        effort: None,
        created_at: ticket.created_at,
        updated_at: ticket.updated_at,
        fields: BTreeMap::new(),
    }
}

pub(super) fn ticket_summary_from_resolved(ticket: &ResolvedIndexedTicket) -> TicketSummary {
    let effort = ticket
        .store
        .get(&ticket.ticket.id)
        .ok()
        .and_then(|manifest| effort_from_fields(&manifest.extra));
    TicketSummary {
        id: ticket.ticket.id.to_string(),
        ticket_ref: ticket_ref_from_resolved(ticket),
        type_id: ticket.ticket.type_id.clone(),
        title: ticket.ticket.title.clone(),
        state: ticket.ticket.state.clone(),
        effort,
        created_at: ticket.ticket.created_at,
        updated_at: ticket.ticket.updated_at,
        fields: BTreeMap::new(),
    }
}

pub(super) struct PreferredResolvedTicket {
    pub(super) path: std::path::PathBuf,
    pub(super) ticket_ref: TicketRef,
}

pub(super) fn resolve_ticket_with_preferred_source(
    store: &ticket_api::storage::store::TicketStore,
    state: &AppState,
    active_workspace: &str,
    id: Uuid,
    request_id: &str,
) -> Result<PreferredResolvedTicket, Response> {
    let local_ticket = match store.get_indexed(&id) {
        Ok(ticket) => ticket,
        Err(error) => return Err(storage_err(error, request_id)),
    };
    let local_ticket_ref = match local_ticket
        .as_ref()
        .map(|ticket| ticket_ref_from_indexed(store, active_workspace, ticket))
        .transpose()
    {
        Ok(ticket_ref) => ticket_ref,
        Err(error) => return Err(storage_err(error, request_id)),
    };

    let resolved = resolve_ticket(state, active_workspace, id, request_id)?;

    if should_prefer_local_ticket(
        store,
        active_workspace,
        local_ticket.as_ref(),
        local_ticket_ref.as_ref(),
        Some(&resolved),
    ) {
        let ticket = local_ticket.as_ref().expect("local ticket");
        let ticket_ref = local_ticket_ref.expect("local ticket ref");
        return Ok(PreferredResolvedTicket {
            path: ticket.path.clone(),
            ticket_ref,
        });
    }

    let ticket_ref = ticket_ref_from_resolved(&resolved);
    Ok(PreferredResolvedTicket {
        path: resolved.ticket.path,
        ticket_ref,
    })
}

pub(super) fn should_prefer_local_ticket(
    store: &ticket_api::storage::store::TicketStore,
    active_workspace: &str,
    local_ticket: Option<&ticket_api::storage::indexed::IndexedTicket>,
    local_ticket_ref: Option<&TicketRef>,
    resolved_ticket: Option<&ResolvedIndexedTicket>,
) -> bool {
    let (Some(ticket), Some(ticket_ref), Some(resolved_ticket)) =
        (local_ticket, local_ticket_ref, resolved_ticket)
    else {
        return false;
    };

    should_use_local_ticket(active_workspace, ticket, ticket_ref)
        && resolved_ticket.store.index_root == store.index_root
}

pub(super) fn should_use_local_ticket(
    active_workspace: &str,
    ticket: &ticket_api::storage::indexed::IndexedTicket,
    ticket_ref: &TicketRef,
) -> bool {
    ticket_ref.workspace != active_workspace && ticket.path.join("ticket.toml").is_file()
}
