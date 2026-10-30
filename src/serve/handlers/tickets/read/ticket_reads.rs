#[allow(unused_imports)]
use super::*;

pub async fn get_ticket(
    State(state): State<AppState>,
    Extension(rid): Extension<RequestIdExt>,
    Path(id): Path<Uuid>,
    Query(params): Query<TicketIdParam>,
) -> Response {
    let (workspace, store) = match resolve_workspace_request(&state, &params.workspace, &rid.0) {
        Ok(resolved) => resolved,
        Err(response) => return response,
    };
    let state = state.clone();
    let request_id = rid.0.clone();
    let task_request_id = request_id.clone();
    let view = params.view;
    let parts = params.parts;

    tokio::task::spawn_blocking(move || {
        let request_id = task_request_id.clone();
        let resolved =
            match resolve_ticket_with_preferred_source(&store, &state, &workspace, id, &request_id)
            {
                Ok(ticket) => ticket,
                Err(response) => return response,
            };

        let projection =
            match ticket_api::storage::ReadProjection::decode(view.as_deref(), parts.as_deref()) {
                Ok(projection) => projection,
                Err(error) => return storage_err(error, &request_id),
            };

        if let Some(projection) = projection {
            return match store.project(&id, &projection) {
                Ok(projected) => Json(serde_json::json!({
                    "request_id": request_id,
                    "active_workspace": workspace,
                    "workspace": workspace,
                    "ticket_ref": resolved.ticket_ref,
                    "ticket": projected,
                }))
                .into_response(),
                Err(error) => storage_err(error, &request_id),
            };
        }

        match TicketFs::read(&resolved.path) {
            Ok(manifest) => {
                let ticket_ref = resolved.ticket_ref;

                Json(TicketDetailResponse {
                    request_id: request_id.clone(),
                    active_workspace: workspace.clone(),
                    workspace: workspace.clone(),
                    ticket: TicketDetail {
                        id: manifest.id.to_string(),
                        ticket_ref,
                        created_at: manifest.created_at,
                        fields: manifest.extra.into_iter().collect(),
                    },
                })
                .into_response()
            }
            Err(e) => storage_err(e, &request_id),
        }
    })
    .await
    .unwrap_or_else(|_| task_join_err(&request_id, "ticket detail request"))
}

/// `GET /api/tickets/{id}/description?workspace=<name>`
///
/// Returns the raw Markdown content of `description.md` for a ticket, if it
/// exists. Returns `{ "description": null }` when no description has been
/// written, rather than 404, so the UI can show a placeholder without special-
/// casing the status code.
pub async fn get_ticket_description(
    State(state): State<AppState>,
    Extension(rid): Extension<RequestIdExt>,
    Path(id): Path<Uuid>,
    Query(params): Query<TicketIdParam>,
) -> Response {
    let (workspace, store) = match resolve_workspace_request(&state, &params.workspace, &rid.0) {
        Ok(resolved) => resolved,
        Err(response) => return response,
    };
    let state = state.clone();
    let request_id = rid.0.clone();
    let task_request_id = request_id.clone();

    tokio::task::spawn_blocking(move || {
        let request_id = task_request_id.clone();
        let resolved =
            match resolve_ticket_with_preferred_source(&store, &state, &workspace, id, &request_id)
            {
                Ok(ticket) => ticket,
                Err(response) => return response,
            };

        let description = match TicketFs::read_description(&resolved.path) {
            Ok(description) => description,
            Err(error) => return storage_err(error, &request_id),
        };
        Json(TicketDescriptionResponse {
            request_id: request_id.clone(),
            active_workspace: workspace.clone(),
            workspace: workspace.clone(),
            id: id.to_string(),
            ticket_ref: resolved.ticket_ref,
            description,
        })
        .into_response()
    })
    .await
    .unwrap_or_else(|_| task_join_err(&request_id, "ticket description request"))
}

/// `GET /api/tickets/{id}/history?workspace=<name>`
///
/// Return all history revisions for a ticket, oldest first.
pub async fn get_ticket_history(
    State(state): State<AppState>,
    Extension(rid): Extension<RequestIdExt>,
    Path(id): Path<Uuid>,
    Query(params): Query<TicketIdParam>,
) -> Response {
    let (workspace, store) = match resolve_workspace_request(&state, &params.workspace, &rid.0) {
        Ok(resolved) => resolved,
        Err(response) => return response,
    };
    let state = state.clone();
    let request_id = rid.0.clone();
    let task_request_id = request_id.clone();

    tokio::task::spawn_blocking(move || {
        let request_id = task_request_id.clone();
        let resolved =
            match resolve_ticket_with_preferred_source(&store, &state, &workspace, id, &request_id)
            {
                Ok(ticket) => ticket,
                Err(response) => return response,
            };
        match TicketFs::read_history(&resolved.path) {
            Ok(revisions) => {
                let ticket_ref = resolved.ticket_ref;

                let entries = revisions
                    .into_iter()
                    .map(|revision| HistoryEntry {
                        rev: revision.rev,
                        ts: revision.ts,
                        author: revision.author,
                        fields: revision.fields,
                    })
                    .collect::<Vec<_>>();
                Json(TicketHistoryResponse {
                    request_id: request_id.clone(),
                    active_workspace: workspace.clone(),
                    workspace: workspace.clone(),
                    id: id.to_string(),
                    ticket_ref,
                    count: entries.len() as u64,
                    entries,
                })
                .into_response()
            }
            Err(e) => storage_err(e, &request_id),
        }
    })
    .await
    .unwrap_or_else(|_| task_join_err(&request_id, "ticket history request"))
}
