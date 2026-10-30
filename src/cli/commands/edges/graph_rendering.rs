#[allow(unused_imports)]
use super::*;

pub(super) fn graph_json_nodes(node_list: &[TraversalNode]) -> Vec<Value> {
    node_list
        .iter()
        .map(|(id, title, state, depth)| {
            json!({
                "id": id.to_string(),
                "title": title,
                "state": state,
                "depth": depth,
            })
        })
        .collect()
}

pub(super) fn graph_json_edges(unique_edges: &[EdgeRecord]) -> Vec<Value> {
    unique_edges
        .iter()
        .map(|edge| {
            json!({
                "from": edge.from.to_string(),
                "to": edge.to.to_string(),
                "kind": edge.kind,
            })
        })
        .collect()
}

pub(super) fn ticket_exists(store: &TicketStore, ticket_id: Uuid) -> bool {
    store.get_indexed(&ticket_id).ok().flatten().is_some()
}

/// Map an edge kind to its reverse-direction display label.
/// `depends_on` becomes `blocks`; unknown kinds get a `~` prefix.
pub(super) fn reverse_edge_label(kind: &str) -> String {
    match kind {
        "depends_on" => "blocks".to_string(),
        other => format!("~{other}"),
    }
}

/// Render an ASCII dependency tree from BFS-collected nodes and edges.
/// When `reverse_tree` is true, edges are displayed from `to → from` (dependents).
pub(super) fn render_ascii_tree(
    root: Uuid,
    nodes: &[(Uuid, Option<String>, Option<String>, usize)],
    edges: &[&EdgeRecord],
    reverse_tree: bool,
) -> String {
    // Build lookup: id -> (title, state)
    let node_info: HashMap<Uuid, (&Option<String>, &Option<String>)> = nodes
        .iter()
        .map(|(id, title, state, _)| (*id, (title, state)))
        .collect();

    // Build adjacency: tree-parent -> [(kind, tree-child)]
    // For subgraph (reverse_tree=false): from is parent, to is child
    // For topgraph (reverse_tree=true): to is parent, from is child
    let mut children: HashMap<Uuid, Vec<(&str, Uuid)>> = HashMap::new();
    for edge in edges {
        if reverse_tree {
            children
                .entry(edge.to)
                .or_default()
                .push((&edge.kind, edge.from));
        } else {
            children
                .entry(edge.from)
                .or_default()
                .push((&edge.kind, edge.to));
        }
    }

    let mut out = String::new();
    let short_id = &root.to_string()[..8];
    let (title, state) = node_info
        .get(&root)
        .map(|(t, s)| (t.as_deref().unwrap_or("?"), s.as_deref().unwrap_or("?")))
        .unwrap_or(("?", "?"));
    let _ = writeln!(out, "{title} ({short_id}) [{state}]");

    // Track visited to handle diamond dependencies
    let mut visited = HashSet::new();
    visited.insert(root);

    render_children(
        &mut out,
        &mut visited,
        root,
        &children,
        &node_info,
        "",
        reverse_tree,
    );
    out
}

pub(super) fn render_children(
    out: &mut String,
    visited: &mut HashSet<Uuid>,
    parent: Uuid,
    children: &HashMap<Uuid, Vec<(&str, Uuid)>>,
    node_info: &HashMap<Uuid, (&Option<String>, &Option<String>)>,
    prefix: &str,
    reverse_tree: bool,
) {
    let Some(kids) = children.get(&parent) else {
        return;
    };

    for (i, (kind, child_id)) in kids.iter().enumerate() {
        let is_last = i == kids.len() - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let child_prefix = if is_last { "    " } else { "│   " };

        let short_id = &child_id.to_string()[..8];
        let (title, state) = node_info
            .get(child_id)
            .map(|(t, s)| (t.as_deref().unwrap_or("?"), s.as_deref().unwrap_or("?")))
            .unwrap_or(("?", "?"));

        // In a topgraph the child actually depends on the parent, so flip the arrow.
        let edge_label = if reverse_tree {
            format!("{} →", reverse_edge_label(kind))
        } else {
            format!("{kind} →")
        };

        let already_visited = !visited.insert(*child_id);
        if already_visited {
            let _ = writeln!(
                out,
                "{prefix}{connector}{edge_label} {title} ({short_id}) [{state}] (→ see above)"
            );
        } else {
            let _ = writeln!(
                out,
                "{prefix}{connector}{edge_label} {title} ({short_id}) [{state}]"
            );
            let next_prefix = format!("{prefix}{child_prefix}");
            render_children(
                out,
                visited,
                *child_id,
                children,
                node_info,
                &next_prefix,
                reverse_tree,
            );
        }
    }
}
