#[allow(unused_imports)]
use super::*;

pub async fn workflow_next(
    State(state): State<AppState>,
    Extension(rid): Extension<RequestIdExt>,
    Query(params): Query<WorkflowNextQuery>,
) -> Response {
    let (workspace, store) = match resolve_workspace_request(&state, &params.workspace, &rid.0) {
        Ok(resolved) => resolved,
        Err(response) => return response,
    };
    let request_id = rid.0.clone();
    let task_request_id = request_id.clone();

    tokio::task::spawn_blocking(move || {
        let request_id = task_request_id.clone();
        match workflow_next_payload(&store, &workspace, &params, &request_id) {
            Ok(payload) => Json(payload).into_response(),
            Err(response) => response,
        }
    })
    .await
    .unwrap_or_else(|_| task_join_err(&request_id, "workflow next request"))
}

pub(super) fn workflow_next_payload(
    store: &TicketStore,
    workspace: &str,
    params: &WorkflowNextQuery,
    request_id: &str,
) -> Result<WorkflowNextResponse, Response> {
    let scope_root = params.root.map(|id| id.to_string());
    let scope_filter = params.filter.clone();
    let active_index_root = store.index_root.display().to_string();
    let tickets = store
        .list(None, None, None)
        .map_err(|error| storage_err(error, request_id))?;
    let all_edges = store
        .list_all_edges()
        .map_err(|error| storage_err(error, request_id))?;
    let model = WorkflowModel::build(store, tickets.clone(), all_edges)
        .map_err(|error| storage_err(error, request_id))?;
    ensure_next_root_exists(&model, params.root, request_id)?;

    let next_scope = build_optional_next_scope(params.root, &model, store, workspace)
        .map_err(|error| storage_err(error, request_id))?;
    let (mut candidates, excluded_by_board, warnings, frontier_count) =
        collect_board_filtered_candidates(&tickets, &model, params, next_scope.as_ref(), store);
    apply_next_limit(&mut candidates, params);

    let empty_satisfied = HashSet::new();
    let items = build_candidate_items(&candidates, &model, store, workspace, &empty_satisfied)
        .map_err(|error| storage_err(error, request_id))?;
    Ok(WorkflowNextResponse {
        request_id: request_id.to_string(),
        active_workspace: workspace.to_string(),
        workspace: workspace.to_string(),
        scope: ScopeMetadata {
            workspace: workspace.to_string(),
            active_index_root,
            filter: scope_filter,
            root: scope_root,
        },
        root: next_scope.as_ref().map(|scope| scope.root.clone()),
        reachable_dependencies: next_scope
            .as_ref()
            .map(|scope| scope.reachable_dependencies),
        blocked_dependencies: next_scope.as_ref().map(|scope| scope.blocked_dependencies),
        remaining_blocker_count: next_scope
            .as_ref()
            .map(|scope| scope.remaining_blockers.len()),
        blocker_tree: next_scope.as_ref().map(|scope| scope.blocker_tree.clone()),
        frontier_count: next_scope.as_ref().map(|_| frontier_count),
        count: items.len(),
        items,
        excluded_by_board,
        warnings,
    })
}

pub(super) fn ensure_next_root_exists(
    model: &WorkflowModel,
    root: Option<Uuid>,
    request_id: &str,
) -> Result<(), Response> {
    if root.is_some_and(|root_id| model.ticket(&root_id).is_none()) {
        return Err(ApiError::not_found("ticket", request_id)
            .into_response_with_status(StatusCode::NOT_FOUND));
    }
    Ok(())
}

pub(super) fn build_optional_next_scope(
    root: Option<Uuid>,
    model: &WorkflowModel,
    store: &TicketStore,
    workspace: &str,
) -> Result<Option<NextScope>, ticket_api::error::StorageError> {
    root.map(|root_id| build_next_scope(root_id, model, store, workspace))
        .transpose()
}

pub(super) fn collect_board_filtered_candidates(
    tickets: &[ticket_api::storage::indexed::IndexedTicket],
    model: &WorkflowModel,
    params: &WorkflowNextQuery,
    next_scope: Option<&NextScope>,
    store: &TicketStore,
) -> (Vec<Uuid>, Vec<BoardExcludedCandidate>, Vec<String>, usize) {
    let filtered_scope = WorkflowModel::filter_scope(tickets, params.filter.as_deref());
    let candidate_scope = intersect_scopes(
        filtered_scope,
        next_scope.map(|scope| &scope.remaining_blockers),
    );
    let mut candidates = model.actionable_candidate_ids(candidate_scope.as_ref());
    model.sort_candidate_ids(&mut candidates);
    let board_filtered =
        apply_board_filter(candidates, store.board_show(None).ok().as_ref(), false);
    let frontier_count = board_filtered.candidates.len();
    (
        board_filtered.candidates,
        board_filtered.excluded_by_board,
        board_filtered.warnings,
        frontier_count,
    )
}

pub(super) fn apply_next_limit(candidates: &mut Vec<Uuid>, params: &WorkflowNextQuery) {
    match params.limit {
        Some(limit) => candidates.truncate(limit.min(100)),
        None if params.root.is_none() => candidates.truncate(20),
        None => {}
    }
}

pub(super) fn resolve_workspace_request(
    state: &AppState,
    requested_workspace: &str,
    request_id: &str,
) -> Result<(String, Arc<TicketStore>), Response> {
    state.resolve_public_workspace_request(requested_workspace, request_id)
}

pub(super) fn build_next_scope(
    root_id: Uuid,
    model: &WorkflowModel,
    store: &TicketStore,
    active_workspace: &str,
) -> Result<NextScope, ticket_api::error::StorageError> {
    let scope = model
        .root_blocker_scope(root_id)
        .ok_or(ticket_api::error::StorageError::NotFound(root_id))?;

    Ok(NextScope {
        root: build_root_summary(root_id, model, store, active_workspace)?,
        reachable_dependencies: scope.reachable_dependencies,
        blocked_dependencies: scope.blocked_dependencies,
        remaining_blockers: scope.remaining_blockers,
        blocker_tree: build_tree_item(scope.tree, model, store, active_workspace)?,
    })
}

pub(super) fn build_root_summary(
    ticket_id: Uuid,
    model: &WorkflowModel,
    store: &TicketStore,
    active_workspace: &str,
) -> Result<WorkflowRootSummary, ticket_api::error::StorageError> {
    let ticket = model
        .ticket(&ticket_id)
        .ok_or(ticket_api::error::StorageError::NotFound(ticket_id))?;
    Ok(WorkflowRootSummary {
        id: ticket_id.to_string(),
        ticket_ref: ticket_ref_from_indexed(store, active_workspace, ticket)?,
        title: ticket.title.clone(),
        state: ticket.state.clone(),
    })
}

pub(super) fn build_candidate_items(
    ids: &[Uuid],
    model: &WorkflowModel,
    store: &TicketStore,
    active_workspace: &str,
    satisfied_ids: &HashSet<Uuid>,
) -> Result<Vec<WorkflowCandidateItem>, ticket_api::error::StorageError> {
    let mut items = Vec::with_capacity(ids.len());
    for (rank, ticket_id) in ids.iter().enumerate() {
        let Some(ticket) = model.ticket(ticket_id) else {
            continue;
        };
        let metrics = model.metrics(ticket_id).cloned().unwrap_or_default();
        items.push(WorkflowCandidateItem {
            rank: rank + 1,
            id: ticket.id.to_string(),
            ticket_ref: ticket_ref_from_indexed(store, active_workspace, ticket)?,
            title: ticket.title.clone(),
            state: ticket.state.clone(),
            ticket_type: ticket.type_id.clone(),
            priority: model.priority_or_none(ticket_id).to_string(),
            effort: model.effort(ticket_id),
            dependency_count: model.dependency_count(ticket_id),
            remaining_blocker_count: model
                .unresolved_dependencies_excluding(ticket_id, satisfied_ids)
                .len(),
            dependee_count: model.dependee_count(ticket_id),
            transitive_reverse_dependents: metrics.transitive_reverse_dependents,
            affected_reverse_dependent_reach: metrics.affected_reverse_dependent_reach,
            max_affected_dependent_state: metrics.max_affected_dependent_state,
            dependency_state_gap: metrics.dependency_state_gap,
            became_actionable_at: metrics
                .became_actionable_at
                .map(|timestamp| timestamp.to_rfc3339()),
            last_blocker_progress_at: metrics
                .last_blocker_progress_at
                .map(|timestamp| timestamp.to_rfc3339()),
            created_at: ticket.created_at.to_rfc3339(),
        });
    }
    Ok(items)
}
