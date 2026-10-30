#[allow(unused_imports)]
use super::*;

/// `POST /api/tickets/{id}/move?workspace=<name>`
///
/// Dry-run or apply a cross-workspace ticket move using the storage-layer
/// planner and journaled execution primitive.
pub async fn move_ticket(
    State(state): State<AppState>,
    Extension(rid): Extension<RequestIdExt>,
    Path(id): Path<Uuid>,
    Query(params): Query<MutationWorkspaceParam>,
    Json(body): Json<MoveTicketBody>,
) -> Response {
    let span = tracing::info_span!(
        target: "ticket_http::transport",
        "ticket_http_move_request",
        request_id = %rid.0,
        ticket_id = %id,
        requested_workspace = %params.workspace,
        dry_run = body.dry_run,
    );
    let (workspace, store) = match state.resolve_public_workspace_request(&params.workspace, &rid.0)
    {
        Ok(resolved) => resolved,
        Err(response) => return response,
    };

    let request_id = rid.0.clone();
    let task_request_id = request_id.clone();
    let target_selector = match ticket_api::workspace::normalize_explicit_workspace_selector(Some(
        &body.to_workspace_root,
    )) {
        Ok(path) => path,
        Err(error) => {
            return ApiError::new(
                "ticket.invalid_workspace_selector",
                &error.to_string(),
                &rid.0,
            )
            .into_response_with_status(StatusCode::BAD_REQUEST);
        }
    };
    let target_workspace = normalize_workspace_root(&target_selector);
    let dry_run = body.dry_run;

    tracing::debug!(
        target: "ticket_http::transport",
        parent: &span,
        request_id = %request_id,
        workspace = %workspace,
        target_workspace_root = %target_workspace.display(),
        "ticket_http_move_resolved"
    );

    tokio::task::spawn_blocking(move || {
        let span = tracing::info_span!(
            target: "ticket_http::transport",
            "ticket_http_move_execute",
            request_id = %task_request_id,
            ticket_id = %id,
            workspace = %workspace,
            dry_run,
            journal_id = tracing::field::Empty,
        );
        let request_id = task_request_id.clone();
        let plan = match store.plan_move_preflight(&id, &target_workspace) {
            Ok(report) => report,
            Err(error) => return storage_err(error, &request_id),
        };

        if dry_run || !plan.supported() {
            tracing::info!(
                target: "ticket_http::transport",
                parent: &span,
                request_id = %request_id,
                ticket_id = %id,
                mode = "plan",
                supported = plan.supported(),
                blockers = plan.blockers.len(),
                "ticket_http_move_complete"
            );
            let status = if plan.supported() {
                StatusCode::OK
            } else {
                StatusCode::CONFLICT
            };
            return (
                status,
                Json(MoveTicketResponse {
                    request_id,
                    active_workspace: workspace.clone(),
                    workspace: workspace.clone(),
                    id: id.to_string(),
                    status: if plan.supported() {
                        "ok".to_string()
                    } else {
                        "blocked".to_string()
                    },
                    mode: "plan".to_string(),
                    plan: move_plan_json(&plan),
                    outcome: None,
                    recovery: move_recovery_json(),
                }),
            )
                .into_response();
        }

        let outcome = match store.execute_move_with_journal(&plan) {
            Ok(outcome) => outcome,
            Err(error) => return storage_err(error, &request_id),
        };
        span.record("journal_id", outcome.journal.id.to_string());
        tracing::info!(
            target: "ticket_http::transport",
            parent: &span,
            request_id = %request_id,
            ticket_id = %id,
            journal_id = %outcome.journal.id,
            mode = "apply",
            phase = ?outcome.journal.phase,
            resumed = outcome.resumed,
            rolled_back = outcome.rolled_back,
            "ticket_http_move_complete"
        );

        Json(MoveTicketResponse {
            request_id,
            active_workspace: workspace.clone(),
            workspace: workspace.clone(),
            id: id.to_string(),
            status: "ok".to_string(),
            mode: "apply".to_string(),
            plan: move_plan_json(&plan),
            outcome: Some(move_outcome_json(&outcome)),
            recovery: move_recovery_json(),
        })
        .into_response()
    })
    .await
    .unwrap_or_else(|_| task_join_err(&request_id, "ticket move request"))
}

/// `POST /api/tickets/{id}/revert?workspace=<name>`
///
/// Revert a ticket to a specific historical revision, identified by its
/// 1-based `revision` number. The revert is forward-only: a new history entry
/// is appended; no history is erased.
pub async fn revert_ticket(
    State(state): State<AppState>,
    Extension(rid): Extension<RequestIdExt>,
    Path(id): Path<Uuid>,
    Query(params): Query<MutationWorkspaceParam>,
    headers: HeaderMap,
    Json(body): Json<RevertTicketBody>,
) -> Response {
    let (workspace, store) = match state.resolve_public_workspace_request(&params.workspace, &rid.0)
    {
        Ok(resolved) => resolved,
        Err(response) => return response,
    };

    let revision = body.revision;
    let author = author_from_headers(&headers);
    let request_id = rid.0.clone();
    let task_request_id = request_id.clone();

    tokio::task::spawn_blocking(move || {
        let request_id = task_request_id.clone();
        let revisions = match store.get_history(&id) {
            Ok(revisions) => revisions,
            Err(e) => return storage_err(e, &request_id),
        };

        let target_rev = match revisions
            .iter()
            .find(|revision_entry| revision_entry.rev == revision)
        {
            Some(revision_entry) => revision_entry.clone(),
            None => {
                return ApiError::bad_request(
                    "revision_not_found",
                    &format!("revision {} does not exist for this ticket", revision),
                    &request_id,
                )
                .into_response_with_status(StatusCode::BAD_REQUEST);
            }
        };

        match store.apply_revert(&id, target_rev.fields, author.as_deref()) {
            Ok(_new_rev) => current_ticket_response(&store, &request_id, &workspace, &id),
            Err(e) => storage_err(e, &request_id),
        }
    })
    .await
    .unwrap_or_else(|_| task_join_err(&request_id, "ticket revert request"))
}
