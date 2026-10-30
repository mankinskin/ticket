#[allow(unused_imports)]
use super::*;

/// `POST /api/tickets/{id}/undo?workspace=<name>`
///
/// Undo the last state/field transition on a ticket by reverting to the
/// second-to-last history revision, bypassing state-machine validation.
pub async fn undo_ticket(
    State(state): State<AppState>,
    Extension(rid): Extension<RequestIdExt>,
    Path(id): Path<Uuid>,
    Query(params): Query<MutationWorkspaceParam>,
    headers: HeaderMap,
) -> Response {
    let (workspace, store) = match state.resolve_public_workspace_request(&params.workspace, &rid.0)
    {
        Ok(resolved) => resolved,
        Err(response) => return response,
    };

    let author = author_from_headers(&headers);
    let request_id = rid.0.clone();
    let task_request_id = request_id.clone();

    tokio::task::spawn_blocking(move || {
        let request_id = task_request_id.clone();
        let revisions = match store.get_history(&id) {
            Ok(revisions) => revisions,
            Err(e) => return storage_err(e, &request_id),
        };

        if revisions.len() < 2 {
            return ApiError::bad_request(
                "no_previous_revision",
                "ticket has no previous revision to undo",
                &request_id,
            )
            .into_response_with_status(StatusCode::UNPROCESSABLE_ENTITY);
        }

        let prev_fields = revisions[revisions.len() - 2].fields.clone();

        match store.apply_revert(&id, prev_fields, author.as_deref()) {
            Ok(_new_rev) => current_ticket_response(&store, &request_id, &workspace, &id),
            Err(e) => storage_err(e, &request_id),
        }
    })
    .await
    .unwrap_or_else(|_| task_join_err(&request_id, "ticket undo request"))
}

/// `DELETE /api/tickets/{id}?workspace=<name>`
///
/// Delete a ticket permanently, removing its folder from disk. Emits a `ticket.delete` SSE event.
pub async fn delete_ticket(
    State(state): State<AppState>,
    Extension(rid): Extension<RequestIdExt>,
    Path(id): Path<Uuid>,
    Query(params): Query<MutationWorkspaceParam>,
) -> Response {
    let (workspace, store) = match state.resolve_public_workspace_request(&params.workspace, &rid.0)
    {
        Ok(resolved) => resolved,
        Err(response) => return response,
    };
    let request_id = rid.0.clone();
    let task_request_id = request_id.clone();

    tokio::task::spawn_blocking(move || {
        // Capture ref before deletion while entity still exists in index.
        let ticket_ref = match ticket_ref_for_id(&store, &workspace, &id) {
            Ok(ticket_ref) => ticket_ref,
            Err(e) => return storage_err(e, &task_request_id),
        };
        match store.delete(&id) {
            Ok(()) => Json(DeleteResponse {
                request_id: task_request_id.clone(),
                active_workspace: workspace.clone(),
                workspace: workspace.clone(),
                id: id.to_string(),
                ticket_ref,
            })
            .into_response(),
            Err(e) => storage_err(e, &task_request_id),
        }
    })
    .await
    .unwrap_or_else(|_| task_join_err(&request_id, "ticket delete request"))
}
