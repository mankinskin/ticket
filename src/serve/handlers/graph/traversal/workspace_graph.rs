#[allow(unused_imports)]
use super::*;

pub(super) fn workspace_graph(
    state: AppState,
    request_id: &str,
    request: WorkspaceGraphRequest,
) -> Response {
    let total_timer = Instant::now();
    let phase_timer = Instant::now();
    let workspace_data = match workspace_graph_data(
        &state,
        &request.workspace,
        request.edge_kind_filter(),
        request_id,
    ) {
        Ok(data) => data,
        Err(response) => return response,
    };
    let phase1_edges_ms = phase_timer.elapsed().as_millis();

    let (depths, max_depth_reached) =
        compute_workspace_depths(&workspace_data.node_ids, &workspace_data.raw_edges);
    let phase2_end_ms = phase_timer.elapsed().as_millis();

    let resolved = match resolve_graph_tickets_by_ids(
        &state,
        &request.workspace,
        &workspace_data.node_ids,
        request_id,
    ) {
        Ok(resolved) => resolved,
        Err(response) => return response,
    };

    let nodes = match build_nodes(&resolved, &depths, &request.workspace, request_id) {
        Ok(nodes) => nodes,
        Err(response) => return response,
    };
    let edges = match build_edges(
        workspace_data.raw_edges,
        &resolved,
        &request.workspace,
        request_id,
    ) {
        Ok(edges) => edges,
        Err(response) => return response,
    };
    let phase3_end_ms = phase_timer.elapsed().as_millis();

    let stats = SubgraphStats {
        nodes_returned: nodes.len(),
        edges_returned: edges.len(),
        max_depth_reached,
        phase1_edges_ms,
        phase2_bfs_ms: phase2_end_ms - phase1_edges_ms,
        phase3_meta_ms: phase3_end_ms - phase2_end_ms,
        total_ms: total_timer.elapsed().as_millis(),
    };

    log_workspace_graph_timing(&request.workspace, &nodes, &edges, &stats);

    Json(SubgraphResponse {
        request_id: request_id.to_string(),
        active_workspace: request.workspace.clone(),
        workspace: request.workspace,
        nodes,
        edges,
        truncated: false,
        next_cursor: None,
        stats,
    })
    .into_response()
}

pub(super) struct WorkspaceGraphData {
    pub(super) raw_edges: Vec<RawEdgeItem>,
    pub(super) node_ids: Vec<Uuid>,
}

pub(super) fn workspace_graph_data(
    state: &AppState,
    workspace: &str,
    edge_kind_filter: &str,
    request_id: &str,
) -> Result<WorkspaceGraphData, Response> {
    let store = resolve_workspace_store(state, workspace, request_id)?;
    let local_tickets = store
        .list(None, None, None)
        .map_err(|error| storage_err(error, request_id))?;
    let all_edges = store
        .list_all_edges()
        .map_err(|error| storage_err(error, request_id))?;
    let raw_edges = dedupe_edges(filter_graph_edges(&all_edges, edge_kind_filter));
    let node_ids = collect_workspace_node_ids(&local_tickets, &raw_edges);
    Ok(WorkspaceGraphData {
        raw_edges,
        node_ids,
    })
}

pub(super) fn resolve_workspace_store(
    state: &AppState,
    workspace: &str,
    request_id: &str,
) -> Result<Arc<TicketStore>, Response> {
    state
        .resolve_public_workspace_request(workspace, request_id)
        .map(|(_, store)| store)
}

pub(super) fn collect_workspace_node_ids(
    local_tickets: &[IndexedTicket],
    edges: &[RawEdgeItem],
) -> Vec<Uuid> {
    let mut ids: Vec<Uuid> = local_tickets.iter().map(|ticket| ticket.id).collect();
    for edge in edges {
        ids.push(edge.from);
        ids.push(edge.to);
    }
    ids.sort_unstable();
    ids.dedup();
    ids
}

pub(super) fn compute_workspace_depths(
    node_ids: &[Uuid],
    edges: &[RawEdgeItem],
) -> (HashMap<Uuid, usize>, usize) {
    let node_set: HashSet<Uuid> = node_ids.iter().copied().collect();
    let mut indegree: HashMap<Uuid, usize> = node_ids.iter().copied().map(|id| (id, 0)).collect();
    let mut outgoing: HashMap<Uuid, Vec<Uuid>> = HashMap::new();

    for edge in edges {
        if !node_set.contains(&edge.from) || !node_set.contains(&edge.to) {
            continue;
        }
        outgoing.entry(edge.from).or_default().push(edge.to);
        *indegree.entry(edge.to).or_default() += 1;
        indegree.entry(edge.from).or_default();
    }

    let mut roots: Vec<Uuid> = indegree
        .iter()
        .filter_map(|(id, degree)| (*degree == 0).then_some(*id))
        .collect();
    roots.sort_unstable();

    let mut queue: VecDeque<Uuid> = roots.iter().copied().collect();
    let mut depths: HashMap<Uuid, usize> = roots.into_iter().map(|id| (id, 0)).collect();

    while let Some(current) = queue.pop_front() {
        let current_depth = depths.get(&current).copied().unwrap_or(0);
        if let Some(neighbors) = outgoing.get(&current) {
            for neighbor in neighbors {
                let next_depth = current_depth + 1;
                depths
                    .entry(*neighbor)
                    .and_modify(|depth| *depth = (*depth).max(next_depth))
                    .or_insert(next_depth);

                if let Some(remaining) = indegree.get_mut(neighbor) {
                    if *remaining > 0 {
                        *remaining -= 1;
                        if *remaining == 0 {
                            queue.push_back(*neighbor);
                        }
                    }
                }
            }
        }
    }

    for id in node_ids {
        depths.entry(*id).or_insert(0);
    }

    let max_depth_reached = depths.values().copied().max().unwrap_or(0);
    (depths, max_depth_reached)
}

pub(super) fn build_nodes(
    resolved: &HashMap<Uuid, ResolvedIndexedTicket>,
    visited: &HashMap<Uuid, usize>,
    active_workspace: &str,
    request_id: &str,
) -> Result<Vec<NodeItem>, Response> {
    let mut nodes = visited
        .iter()
        .map(|(node_id, depth)| {
            build_node_item(
                *node_id,
                *depth,
                resolved.get(node_id),
                active_workspace,
                request_id,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    nodes.sort_by_key(|node| node.depth);
    Ok(nodes)
}

pub(super) fn build_node_item(
    node_id: Uuid,
    depth: usize,
    ticket: Option<&ResolvedIndexedTicket>,
    active_workspace: &str,
    request_id: &str,
) -> Result<NodeItem, Response> {
    if let Some(ticket) = ticket {
        let ticket_ref = ticket_ref_from_indexed(&ticket.store, &ticket.workspace, &ticket.ticket)
            .map_err(|error| storage_err(error, request_id))?;
        return Ok(NodeItem {
            id: node_id.to_string(),
            ticket_ref,
            title: ticket.ticket.title.clone(),
            state: ticket.ticket.state.clone(),
            depth,
            ticket_type: Some(ticket.ticket.type_id.clone()),
            priority: ticket_priority(&ticket.ticket),
        });
    }

    Ok(NodeItem {
        id: node_id.to_string(),
        ticket_ref: fallback_ticket_ref(active_workspace, node_id),
        title: None,
        state: None,
        depth,
        ticket_type: None,
        priority: None,
    })
}

pub(super) fn ticket_priority(ticket: &IndexedTicket) -> Option<String> {
    TicketFs::read(&ticket.path).ok().and_then(|manifest| {
        manifest
            .extra
            .get("priority")
            .and_then(|value| value.as_str())
            .map(|priority| priority.to_string())
    })
}

pub(super) fn build_edges(
    edges: Vec<RawEdgeItem>,
    resolved: &HashMap<Uuid, ResolvedIndexedTicket>,
    active_workspace: &str,
    request_id: &str,
) -> Result<Vec<EdgeItem>, Response> {
    edges
        .into_iter()
        .map(|edge| {
            Ok(EdgeItem {
                from: edge.from.to_string(),
                to: edge.to.to_string(),
                from_ref: resolve_edge_ref(resolved, active_workspace, edge.from, request_id)?,
                to_ref: resolve_edge_ref(resolved, active_workspace, edge.to, request_id)?,
                kind: edge.kind,
            })
        })
        .collect()
}

pub(super) fn resolve_edge_ref(
    resolved: &HashMap<Uuid, ResolvedIndexedTicket>,
    active_workspace: &str,
    id: Uuid,
    request_id: &str,
) -> Result<TicketRef, Response> {
    match resolved.get(&id) {
        Some(ticket) => ticket_ref_from_indexed(&ticket.store, &ticket.workspace, &ticket.ticket)
            .map_err(|error| storage_err(error, request_id)),
        None => Ok(fallback_ticket_ref(active_workspace, id)),
    }
}

pub(super) fn fallback_ticket_ref(active_workspace: &str, id: Uuid) -> TicketRef {
    TicketRef {
        workspace: active_workspace.to_string(),
        id: id.to_string(),
    }
}

pub(super) fn dedupe_edges(edges: Vec<RawEdgeItem>) -> Vec<RawEdgeItem> {
    let mut seen = HashSet::new();
    edges
        .into_iter()
        .filter(|edge| seen.insert((edge.from, edge.to, edge.kind.clone())))
        .collect()
}

pub(super) fn log_subgraph_timing(
    workspace: &str,
    root: Uuid,
    nodes: &[NodeItem],
    edges: &[EdgeItem],
    truncated: bool,
    stats: &SubgraphStats,
) {
    tracing::info!(
        workspace = %workspace,
        root = %root,
        nodes = nodes.len(),
        edges = edges.len(),
        truncated,
        elapsed_ms = stats.total_ms,
        phase1_edges_ms = stats.phase1_edges_ms,
        phase2_bfs_ms = stats.phase2_bfs_ms,
        phase3_meta_ms = stats.phase3_meta_ms,
        "subgraph timing"
    );
}

pub(super) fn log_workspace_graph_timing(
    workspace: &str,
    nodes: &[NodeItem],
    edges: &[EdgeItem],
    stats: &SubgraphStats,
) {
    tracing::info!(
        workspace = %workspace,
        nodes = nodes.len(),
        edges = edges.len(),
        elapsed_ms = stats.total_ms,
        phase1_edges_ms = stats.phase1_edges_ms,
        phase2_bfs_ms = stats.phase2_bfs_ms,
        phase3_meta_ms = stats.phase3_meta_ms,
        "workspace graph timing"
    );
}
