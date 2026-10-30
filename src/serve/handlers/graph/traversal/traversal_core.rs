#[allow(unused_imports)]
use super::*;

pub(super) fn bfs_graph(state: AppState, request_id: &str, request: GraphRequest) -> Response {
    let total_timer = Instant::now();
    let store = match resolve_workspace_store(&state, &request.workspace, request_id) {
        Ok(store) => store,
        Err(response) => return response,
    };

    let phase_timer = Instant::now();
    let all_edges = match store.list_all_edges() {
        Ok(edges) => edges,
        Err(error) => return storage_err(error, request_id),
    };
    let adjacency = build_adjacency(&all_edges, request.edge_kind_filter());
    let phase1_edges_ms = phase_timer.elapsed().as_millis();

    let traversal = traverse_graph(
        &adjacency,
        request.root,
        request.direction.as_str(),
        request.depth_limit(),
        request.limit_nodes,
        request.limit_edges,
    );
    let phase2_end_ms = phase_timer.elapsed().as_millis();

    let resolved = match resolve_graph_tickets(&state, &request.workspace, &traversal, request_id) {
        Ok(resolved) => resolved,
        Err(response) => return response,
    };

    let nodes = match build_nodes(
        &resolved,
        &traversal.visited,
        &request.workspace,
        request_id,
    ) {
        Ok(nodes) => nodes,
        Err(response) => return response,
    };
    let edges = match build_edges(
        dedupe_edges(traversal.edges),
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
        max_depth_reached: traversal.max_depth_reached,
        phase1_edges_ms,
        phase2_bfs_ms: phase2_end_ms - phase1_edges_ms,
        phase3_meta_ms: phase3_end_ms - phase2_end_ms,
        total_ms: total_timer.elapsed().as_millis(),
    };

    log_subgraph_timing(
        &request.workspace,
        request.root,
        &nodes,
        &edges,
        traversal.truncated,
        &stats,
    );

    Json(SubgraphResponse {
        request_id: request_id.to_string(),
        active_workspace: request.workspace.clone(),
        workspace: request.workspace,
        nodes,
        edges,
        truncated: traversal.truncated,
        next_cursor: None,
        stats,
    })
    .into_response()
}

pub(super) fn build_adjacency(
    all_edges: &[EdgeRecord],
    edge_kind_filter: &str,
) -> HashMap<Uuid, Vec<AdjEntry>> {
    let mut adjacency = HashMap::new();
    for edge in all_edges {
        if edge_kind_filter != "all" && edge.kind != edge_kind_filter {
            continue;
        }

        adjacency.entry(edge.from).or_insert_with(Vec::new).push((
            edge.to,
            edge.from,
            edge.to,
            edge.kind.clone(),
        ));
        adjacency.entry(edge.to).or_insert_with(Vec::new).push((
            edge.from,
            edge.from,
            edge.to,
            edge.kind.clone(),
        ));
    }
    adjacency
}

pub(super) fn filter_graph_edges(
    all_edges: &[EdgeRecord],
    edge_kind_filter: &str,
) -> Vec<RawEdgeItem> {
    all_edges
        .iter()
        .filter(|edge| edge_kind_filter == "all" || edge.kind == edge_kind_filter)
        .map(|edge| RawEdgeItem {
            from: edge.from,
            to: edge.to,
            kind: edge.kind.clone(),
        })
        .collect()
}

pub(super) fn traverse_graph(
    adjacency: &HashMap<Uuid, Vec<AdjEntry>>,
    root: Uuid,
    direction: &str,
    depth_limit: usize,
    limit_nodes: usize,
    limit_edges: usize,
) -> TraversalResult {
    let mut visited = HashMap::new();
    let mut edges = Vec::new();
    let mut truncated = false;
    let mut max_depth_reached = 0;
    let mut queue = VecDeque::from([(root, 0)]);

    while let Some((current_id, depth)) = queue.pop_front() {
        if visited.contains_key(&current_id) {
            continue;
        }
        if visited.len() >= limit_nodes {
            truncated = true;
            break;
        }

        visited.insert(current_id, depth);
        max_depth_reached = max_depth_reached.max(depth);

        if depth >= depth_limit {
            continue;
        }

        if let Some(neighbors) = adjacency.get(&current_id) {
            for (neighbor, edge_from, edge_to, edge_kind) in neighbors {
                if !direction_allows(direction, *edge_from == current_id) {
                    continue;
                }
                if edges.len() < limit_edges {
                    edges.push(RawEdgeItem {
                        from: *edge_from,
                        to: *edge_to,
                        kind: edge_kind.clone(),
                    });
                }
                if !visited.contains_key(neighbor) {
                    queue.push_back((*neighbor, depth + 1));
                }
            }
        }
    }

    TraversalResult {
        visited,
        edges,
        truncated,
        max_depth_reached,
    }
}

pub(super) fn direction_allows(direction: &str, is_outbound: bool) -> bool {
    match direction {
        "out" => is_outbound,
        "in" => !is_outbound,
        _ => true,
    }
}

pub(super) fn resolve_graph_tickets(
    state: &AppState,
    active_workspace: &str,
    traversal: &TraversalResult,
    request_id: &str,
) -> Result<HashMap<Uuid, ResolvedIndexedTicket>, Response> {
    let mut ids: Vec<Uuid> = traversal.visited.keys().copied().collect();
    for edge in &traversal.edges {
        ids.push(edge.from);
        ids.push(edge.to);
    }
    ids.sort();
    ids.dedup();

    resolve_graph_tickets_by_ids(state, active_workspace, &ids, request_id)
}

pub(super) fn resolve_graph_tickets_by_ids(
    state: &AppState,
    active_workspace: &str,
    ids: &[Uuid],
    request_id: &str,
) -> Result<HashMap<Uuid, ResolvedIndexedTicket>, Response> {
    state
        .registry
        .resolve_indexed_many(active_workspace, &ids)
        .map_err(|error| storage_err(error, request_id))
}
