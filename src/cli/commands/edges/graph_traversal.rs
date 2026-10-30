#[allow(unused_imports)]
use super::*;

// ── subgraph / topgraph ────────────────────────────────────────────────────────

pub(crate) fn cmd_subgraph(args: SubgraphArgs, store: &TicketStore) -> Result<Value, CliRunError> {
    graph_traversal(
        "subgraph",
        &args.root,
        args.depth,
        &args.direction,
        &args.edge_kind,
        false,
        store,
    )
}

pub(crate) fn cmd_topgraph(args: TopgraphArgs, store: &TicketStore) -> Result<Value, CliRunError> {
    graph_traversal(
        "topgraph",
        &args.root,
        args.depth,
        &args.direction,
        &args.edge_kind,
        true,
        store,
    )
}

pub(super) type TraversalNode = (Uuid, Option<String>, Option<String>, usize);

pub(super) struct TraversalData {
    pub(super) node_list: Vec<TraversalNode>,
    pub(super) unique_edges: Vec<EdgeRecord>,
}

pub(super) fn graph_traversal(
    command_name: &str,
    root_str: &str,
    depth: usize,
    direction: &str,
    edge_kind_filter: &str,
    reverse_tree: bool,
    store: &TicketStore,
) -> Result<Value, CliRunError> {
    let root = resolve_uuid_prefix(root_str, store)?;
    let traversal = collect_graph_data(root, depth.min(8), direction, edge_kind_filter, store)?;
    let json_nodes = graph_json_nodes(&traversal.node_list);
    let json_edges = graph_json_edges(&traversal.unique_edges);
    let edge_refs: Vec<&EdgeRecord> = traversal.unique_edges.iter().collect();
    let tree = render_ascii_tree(root, &traversal.node_list, &edge_refs, reverse_tree);

    Ok(json!({
        "command": command_name,
        "status": "ok",
        "tree": tree,
        "nodes": json_nodes,
        "edges": json_edges,
        "truncated": false,
        "stats": {
            "nodes_returned": json_nodes.len(),
            "edges_returned": json_edges.len(),
        },
    }))
}

pub(super) fn collect_graph_data(
    root: Uuid,
    depth_limit: usize,
    direction: &str,
    edge_kind_filter: &str,
    store: &TicketStore,
) -> Result<TraversalData, CliRunError> {
    let all_edges = store.list_all_edges()?;
    let mut visited = HashSet::new();
    let mut node_list = Vec::new();
    let mut collected_edges = Vec::new();
    let mut queue = VecDeque::from([(root, 0)]);

    while let Some((current_id, depth)) = queue.pop_front() {
        if !visited.insert(current_id) {
            continue;
        }

        let (title, state) = indexed_ticket_state(store, current_id)?;
        node_list.push((current_id, title, state, depth));

        if depth >= depth_limit {
            continue;
        }

        for edge in &all_edges {
            let Some(neighbor) = matching_neighbor(edge, current_id, direction, edge_kind_filter)
            else {
                continue;
            };
            collected_edges.push(edge.clone());
            if !visited.contains(&neighbor) {
                queue.push_back((neighbor, depth + 1));
            }
        }
    }

    Ok(TraversalData {
        node_list,
        unique_edges: dedupe_edges(collected_edges),
    })
}

pub(super) fn indexed_ticket_state(
    store: &TicketStore,
    ticket_id: Uuid,
) -> Result<(Option<String>, Option<String>), CliRunError> {
    Ok(match store.get_indexed(&ticket_id)? {
        Some(ticket) => (ticket.title, ticket.state),
        None => (None, None),
    })
}

pub(super) fn matching_neighbor(
    edge: &EdgeRecord,
    current_id: Uuid,
    direction: &str,
    edge_kind_filter: &str,
) -> Option<Uuid> {
    if edge_kind_filter != "all" && edge.kind != edge_kind_filter {
        return None;
    }

    let (neighbor, is_outbound) = edge_neighbor(edge, current_id)?;
    if direction_allows(direction, is_outbound) {
        Some(neighbor)
    } else {
        None
    }
}

pub(super) fn edge_neighbor(edge: &EdgeRecord, current_id: Uuid) -> Option<(Uuid, bool)> {
    if edge.from == current_id {
        Some((edge.to, true))
    } else if edge.to == current_id {
        Some((edge.from, false))
    } else {
        None
    }
}

pub(super) fn direction_allows(direction: &str, is_outbound: bool) -> bool {
    match direction {
        "out" => is_outbound,
        "in" => !is_outbound,
        _ => true,
    }
}

pub(super) fn dedupe_edges(collected_edges: Vec<EdgeRecord>) -> Vec<EdgeRecord> {
    let mut seen_edges = HashSet::new();
    collected_edges
        .into_iter()
        .filter(|edge| seen_edges.insert((edge.from, edge.to, edge.kind.clone())))
        .collect()
}
