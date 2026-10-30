use super::GraphLayout;
use crate::{
    layout::LayoutMode,
    types::{GraphEdgeItem, GraphNodeItem, TicketRef},
};

fn node(id: &str, state: &str, depth: usize, title: &str) -> GraphNodeItem {
    GraphNodeItem {
        id: id.to_string(),
        ticket_ref: TicketRef::default(),
        title: Some(title.to_string()),
        state: Some(state.to_string()),
        depth,
        ticket_type: Some("tracker-improvement".to_string()),
        priority: Some("medium".to_string()),
    }
}

fn edge(from: &str, to: &str) -> GraphEdgeItem {
    GraphEdgeItem {
        from: from.to_string(),
        to: to.to_string(),
        from_ref: TicketRef::default(),
        to_ref: TicketRef::default(),
        kind: "depends_on".to_string(),
    }
}

fn position(layout: &GraphLayout, node_id: &str) -> (f64, f64, f64) {
    let node = layout
        .nodes
        .iter()
        .find(|node| node.id == node_id)
        .expect("node should exist in layout");
    (node.x, node.y, node.z)
}

#[test]
fn kanban_layout_orders_columns_by_state_progress() {
    let layout = GraphLayout::build_with_mode(
        "default",
        vec![
            node("root", "done", 0, "Root"),
            node("new-child", "open", 1, "New child"),
            node("ready-child", "planned", 1, "Ready child"),
            node("impl-child", "in-implementation", 1, "Impl child"),
            node("review-child", "in-review", 1, "Review child"),
        ],
        vec![
            edge("root", "new-child"),
            edge("root", "ready-child"),
            edge("root", "impl-child"),
            edge("root", "review-child"),
        ],
        LayoutMode::KanbanTable,
    );

    let (new_x, _, _) = position(&layout, "new-child");
    let (ready_x, _, _) = position(&layout, "ready-child");
    let (impl_x, _, _) = position(&layout, "impl-child");
    let (review_x, _, _) = position(&layout, "review-child");

    assert!(new_x < ready_x);
    assert!(ready_x < impl_x);
    assert!(impl_x < review_x);
    assert!((ready_x - new_x) > 700.0);
    assert!((impl_x - ready_x) > 700.0);
    assert!((review_x - impl_x) > 700.0);

    let overlay = layout
        .kanban_overlay
        .as_ref()
        .expect("kanban layout should expose overlay metadata");
    let states = overlay
        .columns
        .iter()
        .map(|column| column.state.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        states,
        vec!["open", "planned", "in-implementation", "in-review", "done"]
    );
    assert_eq!(
        overlay.separators.len(),
        overlay.columns.len().saturating_sub(1)
    );
}

#[test]
fn kanban_layout_groups_dependency_branches_into_rows() {
    let layout = GraphLayout::build_with_mode(
        "default",
        vec![
            node("root", "done", 0, "Root"),
            node("branch-a", "planned", 1, "Branch A"),
            node("branch-a-impl", "in-implementation", 2, "Branch A impl"),
            node("branch-b", "planned", 1, "Branch B"),
        ],
        vec![
            edge("root", "branch-a"),
            edge("branch-a", "branch-a-impl"),
            edge("root", "branch-b"),
        ],
        LayoutMode::KanbanTable,
    );

    let (_, branch_a_y, _) = position(&layout, "branch-a");
    let (_, branch_a_impl_y, _) = position(&layout, "branch-a-impl");
    let (_, branch_b_y, _) = position(&layout, "branch-b");

    assert!((branch_a_y - branch_a_impl_y).abs() < 1.0);
    assert!((branch_a_y - branch_b_y).abs() > 120.0);

    let overlay = layout
        .kanban_overlay
        .as_ref()
        .expect("kanban layout should expose row label metadata");
    let leftmost_column_x = overlay
        .columns
        .iter()
        .map(|column| column.x)
        .fold(f64::INFINITY, f64::min);
    assert!(
        overlay
            .row_labels
            .iter()
            .all(|row| row.x < leftmost_column_x - 220.0)
    );
    assert!(
        overlay
            .row_labels
            .iter()
            .any(|row| row.label.contains("Branch A"))
    );
    assert!(
        overlay
            .row_labels
            .iter()
            .any(|row| row.label.contains("Branch B"))
    );
}

#[test]
fn kanban_layout_soft_clusters_multiple_nodes_inside_one_cell() {
    let layout = GraphLayout::build_with_mode(
        "default",
        vec![
            node("root", "done", 0, "Root"),
            node("branch", "planned", 1, "Branch"),
            node("impl-a", "in-implementation", 2, "Impl A"),
            node("impl-b", "in-implementation", 2, "Impl B"),
        ],
        vec![
            edge("root", "branch"),
            edge("branch", "impl-a"),
            edge("branch", "impl-b"),
        ],
        LayoutMode::KanbanTable,
    );

    let (impl_a_x, impl_a_y, impl_a_z) = position(&layout, "impl-a");
    let (impl_b_x, impl_b_y, impl_b_z) = position(&layout, "impl-b");

    assert!((impl_a_y - impl_b_y).abs() < 1.0);
    assert!((impl_a_x - impl_b_x).abs() > 1.0 || (impl_a_z - impl_b_z).abs() > 1.0);
}
