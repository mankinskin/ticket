//! Hierarchical 2D layout engine for the ticket dependency graph.
//!
//! Nodes are arranged in depth-based rows (BFS depth 0 = root at top).
//! Within each row nodes are ordered by priority (critical → none) then title.
//! The resulting layout has larger / parent tickets at the top and smaller /
//! leaf tickets at the bottom, matching the conventional dependency-tree view.
//! Ticket-viewer then adds only a small Z stagger so the default graph reads
//! as a mostly planar isometric diagram instead of a deep 3-D cluster.
//!
//! Used by `graph3d::Layout3D::from_2d()` to compute initial node positions
//! before 3D projection.

use std::{cmp::Ordering, collections::HashMap};

use crate::types::{GraphEdgeItem, GraphNodeItem, TicketRef};

// ── Layout mode ────────────────────────────────────────────────────────────

// Re-exported from viewer-api so callers don't need to import two crates.
pub use viewer_api_dioxus::LayoutMode;

/// One node in the rendered graph.  Coordinates are in layout-space pixels
/// centred on (0, 0); the canvas/DOM layer applies pan + zoom on top.
#[derive(Debug, Clone)]
pub struct GraphNode {
    pub id: String,
    pub ticket_ref: TicketRef,
    pub title: Option<String>,
    pub state: Option<String>,
    /// BFS depth from the root ticket.
    pub depth: usize,
    /// Ticket type (e.g. "tracker-improvement").
    pub ticket_type: Option<String>,
    /// Priority field value (e.g. "high", "critical").
    pub priority: Option<String>,
    /// Layout position (centre of card), layout-space pixels.
    pub x: f64,
    pub y: f64,
    /// Z position — force-directed within each depth layer's XZ plane.
    pub z: f64,
    /// Physics velocities used by force simulations.
    pub vx: f64,
    pub vy: f64,
    pub vz: f64,
}

/// One directed dependency edge.
#[derive(Debug, Clone)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub kind: String,
}

/// Fully computed layout ready for rendering.
#[derive(Debug, Clone, Default)]
pub struct GraphLayout {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub kanban_overlay: Option<KanbanOverlay>,
}

#[derive(Debug, Clone, Default)]
pub struct KanbanOverlay {
    pub columns: Vec<KanbanColumnGuide>,
    pub row_labels: Vec<KanbanRowGuide>,
    pub separators: Vec<KanbanSeparatorGuide>,
}

#[derive(Debug, Clone)]
pub struct KanbanColumnGuide {
    pub state: String,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone)]
pub struct KanbanRowGuide {
    pub label: String,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone)]
pub struct KanbanSeparatorGuide {
    pub x: f64,
    pub top_y: f64,
    pub bottom_y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct KanbanPlacement {
    column: usize,
    row: usize,
    slot: usize,
    cell_count: usize,
}

#[derive(Debug, Clone)]
struct KanbanLayoutPlan {
    placements: HashMap<usize, KanbanPlacement>,
    column_states: Vec<String>,
    row_labels: Vec<String>,
}

// ── Priority ordering ──────────────────────────────────────────────────────

/// Maps a priority string to a sort key (lower = more important = further left).
fn priority_order(p: Option<&str>) -> u32 {
    match p {
        Some("critical") => 0,
        Some("high") => 1,
        Some("medium") => 2,
        Some("low") => 3,
        Some("none") | Some("") => 4,
        _ => 5,
    }
}

/// Aspect ratio (width / height) of the current viewport, used to shape the
/// hierarchical grid so its footprint matches the screen.
///
/// On wasm this reads the live browser window size; elsewhere (and as a safe
/// fallback when the window size is unavailable) it returns a 16:9 default.
fn viewport_aspect_ratio() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(win) = web_sys::window() {
            let w = win
                .inner_width()
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let h = win
                .inner_height()
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            if w > 0.0 && h > 0.0 {
                return w / h;
            }
        }
    }
    16.0 / 9.0
}

// ── Layout construction ────────────────────────────────────────────────────

impl GraphLayout {}

fn sorted_node_indices(nodes: &[GraphNode]) -> Vec<usize> {
    let mut indices = (0..nodes.len()).collect::<Vec<_>>();
    indices.sort_by(|left_idx, right_idx| kanban_node_cmp(nodes, *left_idx, *right_idx));
    indices
}

fn kanban_node_cmp(nodes: &[GraphNode], left_idx: usize, right_idx: usize) -> Ordering {
    let left = &nodes[left_idx];
    let right = &nodes[right_idx];
    left.depth
        .cmp(&right.depth)
        .then_with(|| {
            let left_state = kanban_state_key(left.state.as_deref());
            let right_state = kanban_state_key(right.state.as_deref());
            kanban_state_cmp(&left_state, &right_state)
        })
        .then_with(|| {
            let left_priority = priority_order(left.priority.as_deref());
            let right_priority = priority_order(right.priority.as_deref());
            left_priority.cmp(&right_priority)
        })
        .then_with(|| {
            let left_title = left.title.as_deref().unwrap_or("");
            let right_title = right.title.as_deref().unwrap_or("");
            left_title.cmp(right_title)
        })
        .then_with(|| left.id.cmp(&right.id))
}

fn parents_by_child(nodes: &[GraphNode], edges: &[GraphEdge]) -> HashMap<usize, Vec<usize>> {
    let id_to_idx = nodes
        .iter()
        .enumerate()
        .map(|(idx, node)| (node.id.as_str(), idx))
        .collect::<HashMap<_, _>>();
    let mut parents = HashMap::<usize, Vec<usize>>::new();
    for edge in edges {
        let Some(&from_idx) = id_to_idx.get(edge.from.as_str()) else {
            continue;
        };
        let Some(&to_idx) = id_to_idx.get(edge.to.as_str()) else {
            continue;
        };
        parents.entry(to_idx).or_default().push(from_idx);
    }
    parents
}

fn primary_parent_index(
    nodes: &[GraphNode],
    parents_by_child: &HashMap<usize, Vec<usize>>,
    node_idx: usize,
) -> Option<usize> {
    let parents = parents_by_child.get(&node_idx)?;
    parents
        .iter()
        .copied()
        .min_by(|left_idx, right_idx| kanban_node_cmp(nodes, *left_idx, *right_idx))
}

fn kanban_state_key(state: Option<&str>) -> String {
    let normalized = state.unwrap_or("open").trim();
    if normalized.is_empty() {
        return "open".to_string();
    }
    normalized.to_ascii_lowercase()
}

fn kanban_state_rank(state: &str) -> usize {
    match state {
        "open" => 0,
        "planned" => 1,
        "blocked" => 2,
        "in-implementation" => 3,
        "in-review" => 4,
        "done" => 5,
        "cancelled" | "canceled" => 6,
        _ => 7,
    }
}

fn kanban_state_cmp(left: &str, right: &str) -> Ordering {
    kanban_state_rank(left)
        .cmp(&kanban_state_rank(right))
        .then_with(|| left.cmp(right))
}

fn kanban_row_label(node: &GraphNode, row_idx: usize) -> String {
    let title = node.title.as_deref().unwrap_or(node.id.as_str()).trim();
    let short_title = if title.is_empty() {
        node.id.as_str()
    } else {
        title
    };
    format!("{:02}. {short_title}", row_idx + 1)
}

fn mean_axis(indices: &[usize], nodes: &[GraphNode], axis: impl Fn(&GraphNode) -> f64) -> f64 {
    if indices.is_empty() {
        return 0.0;
    }
    indices.iter().map(|idx| axis(&nodes[*idx])).sum::<f64>() / indices.len() as f64
}

fn kanban_cell_offset(
    slot: usize,
    cell_count: usize,
    cluster_x: f64,
    cluster_y: f64,
    cluster_z: f64,
) -> (f64, f64, f64) {
    if cell_count <= 1 {
        return (0.0, 0.0, 0.0);
    }

    let cluster_columns = match cell_count {
        0 | 1 => 1,
        2..=4 => 2,
        _ => 3,
    };
    let cluster_rows = (cell_count + cluster_columns - 1) / cluster_columns;
    let column = slot % cluster_columns;
    let row = slot / cluster_columns;
    let x = (column as f64 - (cluster_columns as f64 - 1.0) / 2.0) * cluster_x;
    let y = (row as f64 - (cluster_rows as f64 - 1.0) / 2.0) * cluster_y;
    let z = layer_stagger(slot, cluster_z);
    (x, y, z)
}

#[cfg(test)]
#[path = "layout/tests.rs"]
mod tests;

#[allow(dead_code)]
fn parent_anchor(
    nodes: &[GraphNode],
    parents_by_child: &HashMap<usize, Vec<usize>>,
    node_idx: usize,
) -> f64 {
    let Some(parent_indices) = parents_by_child.get(&node_idx) else {
        return 0.0;
    };

    let sum = parent_indices
        .iter()
        .map(|parent_idx| nodes[*parent_idx].x)
        .sum::<f64>();
    sum / parent_indices.len() as f64
}

fn layer_stagger(slot: usize, spacing: f64) -> f64 {
    const OFFSETS: [f64; 7] = [0.0, 1.0, -1.0, 2.0, -2.0, 3.0, -3.0];
    OFFSETS[slot % OFFSETS.len()] * spacing
}

#[path = "layout/layout_modes.rs"]
mod layout_modes;

#[path = "layout/spatial_layout.rs"]
mod spatial_layout;
