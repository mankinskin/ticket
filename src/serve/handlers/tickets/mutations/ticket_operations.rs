#[allow(unused_imports)]
use super::*;

/// `POST /api/tickets?workspace=<name>`
///
/// Create a new ticket. Returns `201 Created` with the new ticket detail.
pub async fn create_ticket(
    State(state): State<AppState>,
    Extension(rid): Extension<RequestIdExt>,
    Query(params): Query<MutationWorkspaceParam>,
    Json(body): Json<CreateTicketBody>,
) -> Response {
    let (workspace, store) = match state.resolve_public_workspace_request(&params.workspace, &rid.0)
    {
        Ok(resolved) => resolved,
        Err(response) => return response,
    };

    let extra = body.fields.unwrap_or_default();
    let type_id = body.type_id;
    let title = body.title;
    let description = body.description;
    let request_id = rid.0.clone();
    let task_request_id = request_id.clone();

    tokio::task::spawn_blocking(move || {
        let request_id = task_request_id.clone();
        let workspace = workspace.clone();
        let id = match store.create(
            None,
            &type_id,
            title.as_deref(),
            None,
            extra,
            None,
            description.as_deref(),
        ) {
            Ok(id) => id,
            Err(e) => return storage_err(e, &request_id),
        };

        let manifest = match store.get(&id) {
            Ok(manifest) => manifest,
            Err(e) => return storage_err(e, &request_id),
        };

        let created_at = indexed_created_at(&store, &id);
        let ticket_ref = match ticket_ref_for_id(&store, &workspace, &id) {
            Ok(ticket_ref) => ticket_ref,
            Err(e) => return storage_err(e, &request_id),
        };

        (
            StatusCode::CREATED,
            Json(MutationResponse {
                request_id,
                active_workspace: workspace.clone(),
                workspace,
                ticket: TicketDetail {
                    id: manifest.id.to_string(),
                    ticket_ref,
                    created_at,
                    fields: manifest.extra,
                },
            }),
        )
            .into_response()
    })
    .await
    .unwrap_or_else(|_| task_join_err(&request_id, "ticket create request"))
}

/// `PATCH /api/tickets/{id}?workspace=<name>`
///
/// Update fields, state, or description of an existing ticket.
pub async fn update_ticket(
    State(state): State<AppState>,
    Extension(rid): Extension<RequestIdExt>,
    Path(id): Path<Uuid>,
    Query(params): Query<MutationWorkspaceParam>,
    headers: HeaderMap,
    Json(body): Json<UpdateTicketBody>,
) -> Response {
    let (workspace, store) = match state.resolve_public_workspace_request(&params.workspace, &rid.0)
    {
        Ok(resolved) => resolved,
        Err(response) => return response,
    };

    let patch = body.fields.unwrap_or_default();
    let transition_states = body.transition_states;
    let to_state = body.state;
    let single_hop = body.single_hop;
    let author = author_from_headers(&headers);
    let request_id = rid.0.clone();
    let task_request_id = request_id.clone();

    let (description, description_mode) = body.description_update.as_parts();
    let description = description.map(str::to_string);

    tokio::task::spawn_blocking(move || {
        let request_id = task_request_id.clone();
        let workspace = workspace.clone();
        let manifest = match store.update_with_options(
            &id,
            patch,
            Some(transition_states.as_slice()),
            to_state.as_deref(),
            description.as_deref(),
            description_mode,
            author.as_deref(),
            single_hop,
        ) {
            Ok(manifest) => manifest,
            Err(e) => return storage_err(e, &request_id),
        };

        let ticket_ref = match ticket_ref_for_id(&store, &workspace, &id) {
            Ok(ticket_ref) => ticket_ref,
            Err(e) => return storage_err(e, &request_id),
        };

        Json(MutationResponse {
            request_id,
            active_workspace: workspace.clone(),
            workspace,
            ticket: TicketDetail {
                id: manifest.id.to_string(),
                ticket_ref,
                created_at: indexed_created_at(&store, &id),
                fields: manifest.extra,
            },
        })
        .into_response()
    })
    .await
    .unwrap_or_else(|_| task_join_err(&request_id, "ticket update request"))
}

/// `POST /api/tickets/{id}/close?workspace=<name>`
///
/// Fast-forward a ticket through all intermediate states to the target terminal
/// state. `target_state` defaults to `"done"`.
pub async fn close_ticket(
    State(state): State<AppState>,
    Extension(rid): Extension<RequestIdExt>,
    Path(id): Path<Uuid>,
    Query(params): Query<MutationWorkspaceParam>,
    headers: HeaderMap,
    Json(body): Json<CloseTicketBody>,
) -> Response {
    let (workspace, store) = match state.resolve_public_workspace_request(&params.workspace, &rid.0)
    {
        Ok(resolved) => resolved,
        Err(response) => return response,
    };

    let target = body.target_state.as_deref().unwrap_or("done").to_string();
    let author = author_from_headers(&headers);
    let request_id = rid.0.clone();
    let task_request_id = request_id.clone();

    tokio::task::spawn_blocking(move || {
        let request_id = task_request_id.clone();
        let (manifest, _path) = match store.close(&id, &target, author.as_deref()) {
            Ok(result) => result,
            Err(e) => return storage_err(e, &request_id),
        };

        Json(MutationResponse {
            request_id: request_id.clone(),
            active_workspace: workspace.clone(),
            workspace: workspace.clone(),
            ticket: TicketDetail {
                id: manifest.id.to_string(),
                ticket_ref: match ticket_ref_for_id(&store, &workspace, &id) {
                    Ok(ticket_ref) => ticket_ref,
                    Err(e) => return storage_err(e, &request_id),
                },
                created_at: indexed_created_at(&store, &id),
                fields: manifest.extra,
            },
        })
        .into_response()
    })
    .await
    .unwrap_or_else(|_| task_join_err(&request_id, "ticket close request"))
}

/// `POST /api/tickets/{id}/release-lease?workspace=<name>`
///
/// Release a ticket lease using owner/stale semantics.
pub async fn release_ticket_lease(
    State(state): State<AppState>,
    Extension(rid): Extension<RequestIdExt>,
    Path(id): Path<Uuid>,
    Query(params): Query<MutationWorkspaceParam>,
    Json(body): Json<ReleaseLeaseBody>,
) -> Response {
    let (workspace, store) = match state.resolve_public_workspace_request(&params.workspace, &rid.0)
    {
        Ok(resolved) => resolved,
        Err(response) => return response,
    };

    let requester = body.requester;
    let request_id = rid.0.clone();
    let task_request_id = request_id.clone();

    tokio::task::spawn_blocking(move || {
        let request_id = task_request_id.clone();
        let workspace = workspace.clone();

        if let Err(e) = store.release_lease(&id, &requester) {
            return storage_err(e, &request_id);
        }

        let ticket_ref = match ticket_ref_for_id(&store, &workspace, &id) {
            Ok(ticket_ref) => ticket_ref,
            Err(e) => return storage_err(e, &request_id),
        };

        Json(ReleaseLeaseResponse {
            request_id,
            active_workspace: workspace.clone(),
            workspace,
            id: id.to_string(),
            ticket_ref,
            requester,
        })
        .into_response()
    })
    .await
    .unwrap_or_else(|_| task_join_err(&request_id, "ticket release-lease request"))
}

/// `POST /api/tickets/{id}/cancel?workspace=<name>`
///
/// Transition a ticket to the `cancelled` state. Optional `reason` field is
/// stored as a ticket field update.
pub async fn cancel_ticket(
    State(state): State<AppState>,
    Extension(rid): Extension<RequestIdExt>,
    Path(id): Path<Uuid>,
    Query(params): Query<MutationWorkspaceParam>,
    headers: HeaderMap,
    Json(body): Json<CancelTicketBody>,
) -> Response {
    let (workspace, store) = match state.resolve_public_workspace_request(&params.workspace, &rid.0)
    {
        Ok(resolved) => resolved,
        Err(response) => return response,
    };

    let author = author_from_headers(&headers);
    let mut patch = BTreeMap::new();
    if let Some(reason) = body.reason {
        patch.insert("cancel_reason".to_string(), Value::String(reason));
    }
    let request_id = rid.0.clone();
    let task_request_id = request_id.clone();

    tokio::task::spawn_blocking(move || {
        let request_id = task_request_id.clone();
        let manifest = match store.update(
            &id,
            patch,
            Some(&[]),
            Some("cancelled"),
            None,
            author.as_deref(),
        ) {
            Ok(manifest) => manifest,
            Err(e) => return storage_err(e, &request_id),
        };

        Json(MutationResponse {
            request_id: request_id.clone(),
            active_workspace: workspace.clone(),
            workspace: workspace.clone(),
            ticket: TicketDetail {
                id: manifest.id.to_string(),
                ticket_ref: match ticket_ref_for_id(&store, &workspace, &id) {
                    Ok(ticket_ref) => ticket_ref,
                    Err(e) => return storage_err(e, &request_id),
                },
                created_at: indexed_created_at(&store, &id),
                fields: manifest.extra,
            },
        })
        .into_response()
    })
    .await
    .unwrap_or_else(|_| task_join_err(&request_id, "ticket cancel request"))
}
