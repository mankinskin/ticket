#[allow(unused_imports)]
use super::*;

pub(super) fn effort_from_fields(fields: &BTreeMap<String, serde_json::Value>) -> Option<u64> {
    fields
        .get("effort")
        .and_then(serde_json::Value::as_str)
        .and_then(ticket_api::workflow::parse_effort)
}

pub async fn list_tickets(
    State(state): State<AppState>,
    Extension(rid): Extension<RequestIdExt>,
    Query(params): Query<WorkspaceParam>,
) -> Response {
    let (workspace, store) = match resolve_workspace_request(&state, &params.workspace, &rid.0) {
        Ok(resolved) => resolved,
        Err(response) => return response,
    };
    let state = state.clone();
    let request_id = rid.0.clone();
    let task_request_id = request_id.clone();

    tokio::task::spawn_blocking(move || {
        let _span = tracing::debug_span!(
            "list_tickets",
            request_id = %task_request_id,
            workspace = %workspace,
            state_filter = ?params.state,
            query = ?params.query,
            limit = ?params.limit,
        )
        .entered();

        let request_id = task_request_id.clone();
        let requested_limit = params.limit.unwrap_or(100).min(1000);
        let state_filter = params.state.as_deref();
        let tickets = match params.query.as_deref() {
            Some(query) => collect_search_ticket_summaries(
                &state,
                &store,
                &workspace,
                &request_id,
                query,
                state_filter,
                requested_limit,
            ),
            None => collect_list_ticket_summaries(
                &state,
                &store,
                &workspace,
                &request_id,
                state_filter,
                requested_limit,
            ),
        };
        let tickets = match tickets {
            Ok(items) => items,
            Err(response) => return response,
        };

        Json(TicketsResponse {
            request_id: request_id.clone(),
            active_workspace: workspace.clone(),
            workspace: workspace.clone(),
            items: tickets,
            next_cursor: None,
        })
        .into_response()
    })
    .await
    .unwrap_or_else(|_| task_join_err(&request_id, "ticket list request"))
}

pub(super) fn collect_search_ticket_summaries(
    state: &AppState,
    store: &ticket_api::storage::store::TicketStore,
    workspace: &str,
    request_id: &str,
    query: &str,
    state_filter: Option<&str>,
    requested_limit: usize,
) -> Result<Vec<TicketSummary>, Response> {
    let search_limit = store
        .count_tickets()
        .map(|count| count.max(requested_limit))
        .map_err(|e| storage_err(e, request_id))?;

    let results = store
        .search_tickets(query, search_limit)
        .map_err(|e| storage_err(e, request_id))?;
    let ids = results.iter().map(|result| result.id).collect::<Vec<_>>();
    let resolved = resolve_tickets(state, workspace, &ids, request_id)?;

    let mut items = Vec::with_capacity(results.len());
    for result in results {
        let local_ticket = store
            .get_indexed(&result.id)
            .map_err(|e| storage_err(e, request_id))?;
        let local_ticket_ref = local_ticket
            .as_ref()
            .map(|indexed| ticket_ref_from_indexed(store, workspace, indexed))
            .transpose()
            .map_err(|e| storage_err(e, request_id))?;
        let resolved_ticket = resolved.get(&result.id);
        let summary = if should_prefer_local_ticket(
            store,
            workspace,
            local_ticket.as_ref(),
            local_ticket_ref.as_ref(),
            resolved_ticket,
        ) {
            let ticket = local_ticket.as_ref().expect("local ticket");
            let ticket_ref = local_ticket_ref.expect("local ticket ref");
            Some(ticket_summary_from_indexed(ticket_ref, ticket))
        } else {
            resolved_ticket.map(ticket_summary_from_resolved)
        };

        let Some(summary) = summary else {
            tracing::debug!(
                ticket_id = %result.id,
                active_workspace = %workspace,
                has_local = local_ticket.is_some(),
                has_resolved = resolved_ticket.is_some(),
                "dropping unresolved search hit"
            );
            continue;
        };
        if state_filter.map_or(true, |state| summary.state.as_deref() == Some(state)) {
            items.push(summary);
        }
        if items.len() >= requested_limit {
            break;
        }
    }
    Ok(items)
}

pub(super) fn collect_list_ticket_summaries(
    state: &AppState,
    store: &ticket_api::storage::store::TicketStore,
    workspace: &str,
    request_id: &str,
    state_filter: Option<&str>,
    requested_limit: usize,
) -> Result<Vec<TicketSummary>, Response> {
    let items = store
        .list(state_filter, None, Some(requested_limit))
        .map_err(|e| storage_err(e, request_id))?;
    let ids = items.iter().map(|ticket| ticket.id).collect::<Vec<_>>();
    let resolved = resolve_tickets(state, workspace, &ids, request_id)?;

    let mut summaries = Vec::with_capacity(items.len());
    for ticket in items {
        let resolved_ticket = resolved.get(&ticket.id);
        let local_ticket_ref = ticket_ref_from_indexed(store, workspace, &ticket)
            .map_err(|e| storage_err(e, request_id))?;
        let summary = if should_prefer_local_ticket(
            store,
            workspace,
            Some(&ticket),
            Some(&local_ticket_ref),
            resolved_ticket,
        ) {
            let mut summary = ticket_summary_from_indexed(local_ticket_ref, &ticket);
            summary.effort = store
                .get(&ticket.id)
                .ok()
                .and_then(|manifest| effort_from_fields(&manifest.extra));
            summary
        } else if let Some(resolved_ticket) = resolved_ticket {
            ticket_summary_from_resolved(resolved_ticket)
        } else {
            continue;
        };
        summaries.push(summary);
    }
    sort_ticket_summaries(&mut summaries);
    Ok(summaries)
}

pub(super) fn sort_ticket_summaries(summaries: &mut [TicketSummary]) {
    summaries.sort_by(|left, right| {
        left.effort
            .unwrap_or(u64::MAX)
            .cmp(&right.effort.unwrap_or(u64::MAX))
            .then_with(|| right.updated_at.cmp(&left.updated_at))
            .then_with(|| {
                left.title
                    .as_deref()
                    .unwrap_or("")
                    .cmp(right.title.as_deref().unwrap_or(""))
            })
            .then_with(|| left.id.cmp(&right.id))
    });
}
