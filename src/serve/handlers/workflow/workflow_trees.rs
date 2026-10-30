#[allow(unused_imports)]
use super::*;

pub async fn workflow_blockers(
    State(state): State<AppState>,
    Extension(rid): Extension<RequestIdExt>,
    Query(params): Query<WorkflowTreeQuery>,
) -> Response {
    workflow_tree_response(state, rid.0, params, TreeKind::Blockers).await
}

pub async fn workflow_unblocked_by(
    State(state): State<AppState>,
    Extension(rid): Extension<RequestIdExt>,
    Query(params): Query<WorkflowTreeQuery>,
) -> Response {
    workflow_tree_response(state, rid.0, params, TreeKind::UnblockedBy).await
}

pub(super) async fn workflow_tree_response(
    state: AppState,
    request_id: String,
    params: WorkflowTreeQuery,
    kind: TreeKind,
) -> Response {
    let (workspace, store) = match resolve_workspace_request(&state, &params.workspace, &request_id)
    {
        Ok(resolved) => resolved,
        Err(response) => return response,
    };
    let task_request_id = request_id.clone();

    tokio::task::spawn_blocking(move || {
        let request_id = task_request_id.clone();
        match workflow_tree_payload(&store, &workspace, &request_id, params.root, kind) {
            Ok(payload) => Json(payload).into_response(),
            Err(response) => response,
        }
    })
    .await
    .unwrap_or_else(|_| task_join_err(&request_id, "workflow tree request"))
}

pub(super) fn workflow_tree_payload(
    store: &TicketStore,
    workspace: &str,
    request_id: &str,
    root: Uuid,
    kind: TreeKind,
) -> Result<WorkflowTreeResponse, Response> {
    let tickets = store
        .list(None, None, None)
        .map_err(|error| storage_err(error, request_id))?;
    let all_edges = store
        .list_all_edges()
        .map_err(|error| storage_err(error, request_id))?;
    let model = WorkflowModel::build(store, tickets, all_edges)
        .map_err(|error| storage_err(error, request_id))?;

    if model.ticket(&root).is_none() {
        return Err(ApiError::not_found("ticket", request_id)
            .into_response_with_status(StatusCode::NOT_FOUND));
    }

    let tree_info = tree_kind_payload(&model, root, kind, request_id)?;
    let root_item = build_tree_item(tree_info.tree, &model, store, workspace)
        .map_err(|error| storage_err(error, request_id))?;
    let empty_satisfied = HashSet::new();
    let frontier_items = build_candidate_items(
        &tree_info.frontier_ids,
        &model,
        store,
        workspace,
        tree_info.satisfied_ids.as_ref().unwrap_or(&empty_satisfied),
    )
    .map_err(|error| storage_err(error, request_id))?;

    Ok(WorkflowTreeResponse {
        request_id: request_id.to_string(),
        active_workspace: workspace.to_string(),
        workspace: workspace.to_string(),
        kind: tree_info.kind_label.to_string(),
        root: root_item,
        frontier_count: frontier_items.len(),
        frontier_items,
        reachable_dependents: tree_info.reachable_dependents,
        blocked_dependents: tree_info.blocked_dependents,
    })
}

pub(super) fn tree_kind_payload<'a>(
    model: &'a WorkflowModel,
    root: Uuid,
    kind: TreeKind,
    request_id: &str,
) -> Result<TreePayload<'a>, Response> {
    match kind {
        TreeKind::Blockers => blockers_tree_payload(model, root, request_id),
        TreeKind::UnblockedBy => unblocked_by_tree_payload(model, root, request_id),
    }
}

pub(super) fn blockers_tree_payload<'a>(
    model: &'a WorkflowModel,
    root: Uuid,
    request_id: &str,
) -> Result<TreePayload<'a>, Response> {
    let tree = model.blocker_tree(root).ok_or_else(|| {
        ApiError::not_found("ticket", request_id).into_response_with_status(StatusCode::NOT_FOUND)
    })?;
    let frontier_ids = tree.frontier_leaf_ids.clone();
    Ok(TreePayload {
        tree,
        frontier_ids,
        reachable_dependents: None,
        blocked_dependents: None,
        kind_label: "blockers",
        satisfied_ids: None,
    })
}

pub(super) fn unblocked_by_tree_payload<'a>(
    model: &'a WorkflowModel,
    root: Uuid,
    request_id: &str,
) -> Result<TreePayload<'a>, Response> {
    let satisfied_ids = HashSet::from([root]);
    let tree = model
        .unlock_tree_with_satisfied(root, &satisfied_ids)
        .ok_or_else(|| {
            ApiError::not_found("ticket", request_id)
                .into_response_with_status(StatusCode::NOT_FOUND)
        })?;
    let dependent_ids = model.reverse_dependents(root);
    let blocked_dependents = dependent_ids
        .iter()
        .filter(|ticket_id| {
            !model
                .unresolved_dependencies_excluding(ticket_id, &satisfied_ids)
                .is_empty()
        })
        .count();

    Ok(TreePayload {
        tree,
        frontier_ids: model.unlock_frontier_leaf_ids_with_satisfied(root, &satisfied_ids),
        reachable_dependents: Some(dependent_ids.len()),
        blocked_dependents: Some(blocked_dependents),
        kind_label: "unblocked-by",
        satisfied_ids: Some(satisfied_ids),
    })
}

pub(super) fn intersect_scopes(
    primary: Option<HashSet<Uuid>>,
    secondary: Option<&HashSet<Uuid>>,
) -> Option<HashSet<Uuid>> {
    match (primary, secondary) {
        (Some(primary), Some(secondary)) => Some(
            primary
                .into_iter()
                .filter(|ticket_id| secondary.contains(ticket_id))
                .collect(),
        ),
        (Some(primary), None) => Some(primary),
        (None, Some(secondary)) => Some(secondary.iter().copied().collect()),
        (None, None) => None,
    }
}

pub(super) fn build_tree_item(
    node: WorkflowTreeNode,
    model: &WorkflowModel,
    store: &TicketStore,
    active_workspace: &str,
) -> Result<WorkflowTreeItem, ticket_api::error::StorageError> {
    let ticket = model
        .ticket(&node.ticket_id)
        .ok_or(ticket_api::error::StorageError::NotFound(node.ticket_id))?;
    let metrics = model.metrics(&node.ticket_id).cloned().unwrap_or_default();
    let children = node
        .children
        .into_iter()
        .map(|child| build_tree_item(child, model, store, active_workspace))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(WorkflowTreeItem {
        id: node.ticket_id.to_string(),
        ticket_ref: ticket_ref_from_indexed(store, active_workspace, ticket)?,
        title: node.title,
        state: node.state,
        ticket_type: ticket.type_id.clone(),
        priority: node.priority,
        remaining_blocker_count: node.remaining_blocker_count,
        unresolved_frontier_leaf_count: node.unresolved_frontier_leaf_count,
        frontier_leaf_ids: node
            .frontier_leaf_ids
            .into_iter()
            .map(|id| id.to_string())
            .collect(),
        blocker_distance: node.blocker_distance,
        is_frontier: node.is_frontier,
        dependency_count: node.dependency_count,
        immediate_dependees: node.immediate_dependees,
        transitive_reverse_dependents: node.transitive_reverse_dependents,
        affected_reverse_dependent_reach: node.affected_reverse_dependent_reach,
        dependency_state_gap: node.dependency_state_gap,
        became_actionable_at: metrics
            .became_actionable_at
            .map(|timestamp| timestamp.to_rfc3339()),
        last_blocker_progress_at: metrics
            .last_blocker_progress_at
            .map(|timestamp| timestamp.to_rfc3339()),
        children,
    })
}
