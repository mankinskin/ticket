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

use std::{
    cmp::Ordering,
    collections::HashMap,
};

use crate::types::{
    GraphEdgeItem,
    GraphNodeItem,
    TicketRef,
};

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

impl GraphLayout {
    /// Build a hierarchical layout from raw API items using the default
    /// [`LayoutMode::Hierarchical3D`] algorithm.
    pub fn build(
        active_workspace: &str,
        api_nodes: Vec<GraphNodeItem>,
        api_edges: Vec<GraphEdgeItem>,
    ) -> Self {
        Self::build_with_mode(
            active_workspace,
            api_nodes,
            api_edges,
            LayoutMode::default(),
        )
    }

    /// Build a layout using the specified [`LayoutMode`].
    pub fn build_with_mode(
        active_workspace: &str,
        api_nodes: Vec<GraphNodeItem>,
        api_edges: Vec<GraphEdgeItem>,
        mode: LayoutMode,
    ) -> Self {
        let nodes: Vec<GraphNode> = api_nodes
            .into_iter()
            .map(|node| GraphNode {
                id: node.id.clone(),
                ticket_ref: node.resolved_ticket_ref(active_workspace),
                title: node.title,
                state: node.state,
                depth: node.depth,
                ticket_type: node.ticket_type,
                priority: node.priority,
                x: 0.0,
                y: 0.0,
                z: 0.0,
                vx: 0.0,
                vy: 0.0,
                vz: 0.0,
            })
            .collect();

        let edges: Vec<GraphEdge> = api_edges
            .into_iter()
            .map(|e| GraphEdge {
                from: e.from,
                to: e.to,
                kind: e.kind,
            })
            .collect();

        let mut layout = Self {
            nodes,
            edges,
            kanban_overlay: None,
        };
        layout.apply_layout_mode(mode);

        layout
    }

    pub fn with_mode(
        &self,
        mode: LayoutMode,
    ) -> Self {
        let mut layout = self.clone();
        for node in &mut layout.nodes {
            node.x = 0.0;
            node.y = 0.0;
            node.z = 0.0;
            node.vx = 0.0;
            node.vy = 0.0;
            node.vz = 0.0;
        }
        layout.kanban_overlay = None;
        layout.apply_layout_mode(mode);
        layout
    }

    fn fixed_2d_layout(&mut self) {
        self.hierarchical_layout();
        for nd in &mut self.nodes {
            nd.z = 0.0;
        }
    }

    fn apply_layout_mode(
        &mut self,
        mode: LayoutMode,
    ) {
        self.kanban_overlay = None;
        match mode {
            LayoutMode::Hierarchical3D => self.hierarchical_layout(),
            LayoutMode::Flat2D => {
                self.hierarchical_layout();
                // For Flat2D, zero out all Z coords so the graph is a pure
                // top-down 2-D projection (suitable for orthographic view).
                for nd in &mut self.nodes {
                    nd.z = 0.0;
                }
            },
            LayoutMode::KanbanTable => self.kanban_table_layout(),
            LayoutMode::Fixed2D => self.fixed_2d_layout(),
        }
    }

    fn kanban_table_layout(&mut self) {
        const COLUMN_SPACING: f64 = 760.0;
        const ROW_SPACING: f64 = 320.0;
        const CELL_CLUSTER_X: f64 = 132.0;
        const CELL_CLUSTER_Y: f64 = 96.0;
        const CELL_CLUSTER_Z: f64 = 24.0;

        if self.nodes.is_empty() {
            return;
        }

        let plan = self.kanban_layout_plan();
        for (node_idx, placement) in &plan.placements {
            let (local_x, local_y, local_z) = kanban_cell_offset(
                placement.slot,
                placement.cell_count,
                CELL_CLUSTER_X,
                CELL_CLUSTER_Y,
                CELL_CLUSTER_Z,
            );
            self.nodes[*node_idx].x =
                placement.column as f64 * COLUMN_SPACING + local_x;
            self.nodes[*node_idx].y =
                placement.row as f64 * ROW_SPACING + local_y;
            self.nodes[*node_idx].z = local_z;
        }

        self.centre_y();
        self.centre_xz();
        self.kanban_overlay = Some(self.build_kanban_overlay(&plan));
    }

    fn kanban_layout_plan(&self) -> KanbanLayoutPlan {
        let ordered_indices = sorted_node_indices(&self.nodes);
        let parents_by_child = parents_by_child(&self.nodes, &self.edges);
        let mut group_seed_by_idx = vec![None; self.nodes.len()];

        for &node_idx in &ordered_indices {
            let primary_parent =
                primary_parent_index(&self.nodes, &parents_by_child, node_idx);
            let seed = match primary_parent {
                None => node_idx,
                Some(parent_idx)
                    if !parents_by_child.contains_key(&parent_idx) =>
                    node_idx,
                Some(parent_idx) =>
                    group_seed_by_idx[parent_idx].unwrap_or(parent_idx),
            };
            group_seed_by_idx[node_idx] = Some(seed);
        }

        let mut group_row_by_seed = HashMap::new();
        for &node_idx in &ordered_indices {
            let Some(seed_idx) = group_seed_by_idx[node_idx] else {
                continue;
            };
            let next_row = group_row_by_seed.len();
            group_row_by_seed.entry(seed_idx).or_insert(next_row);
        }

        let mut column_states = self
            .nodes
            .iter()
            .map(|node| kanban_state_key(node.state.as_deref()))
            .collect::<Vec<_>>();
        column_states.sort_by(|left, right| kanban_state_cmp(left, right));
        column_states.dedup();
        let column_by_state = column_states
            .iter()
            .cloned()
            .enumerate()
            .map(|(column, state)| (state, column))
            .collect::<HashMap<_, _>>();

        let mut cells: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
        for &node_idx in &ordered_indices {
            let Some(seed_idx) = group_seed_by_idx[node_idx] else {
                continue;
            };
            let row = *group_row_by_seed
                .get(&seed_idx)
                .expect("kanban row should exist for dependency group");
            let state_key =
                kanban_state_key(self.nodes[node_idx].state.as_deref());
            let column = *column_by_state
                .get(&state_key)
                .expect("kanban column should exist for state");
            cells.entry((column, row)).or_default().push(node_idx);
        }

        let mut placements = HashMap::with_capacity(self.nodes.len());
        for ((column, row), cell_indices) in &mut cells {
            cell_indices.sort_by(|left_idx, right_idx| {
                kanban_node_cmp(&self.nodes, *left_idx, *right_idx)
            });
            let cell_count = cell_indices.len();
            for (slot, node_idx) in cell_indices.iter().enumerate() {
                placements.insert(
                    *node_idx,
                    KanbanPlacement {
                        column: *column,
                        row: *row,
                        slot,
                        cell_count,
                    },
                );
            }
        }

        let mut row_labels = vec![String::new(); group_row_by_seed.len()];
        for (seed_idx, row_idx) in group_row_by_seed {
            row_labels[row_idx] =
                kanban_row_label(&self.nodes[seed_idx], row_idx);
        }

        KanbanLayoutPlan {
            placements,
            column_states,
            row_labels,
        }
    }

    fn build_kanban_overlay(
        &self,
        plan: &KanbanLayoutPlan,
    ) -> KanbanOverlay {
        if plan.placements.is_empty()
            || plan.column_states.is_empty()
            || plan.row_labels.is_empty()
        {
            return KanbanOverlay::default();
        }

        let mut column_nodes = vec![Vec::new(); plan.column_states.len()];
        let mut row_nodes = vec![Vec::new(); plan.row_labels.len()];
        for (&node_idx, placement) in &plan.placements {
            column_nodes[placement.column].push(node_idx);
            row_nodes[placement.row].push(node_idx);
        }

        let column_centers = column_nodes
            .iter()
            .map(|indices| mean_axis(indices, &self.nodes, |node| node.x))
            .collect::<Vec<_>>();
        let row_centers = row_nodes
            .iter()
            .map(|indices| mean_axis(indices, &self.nodes, |node| node.y))
            .collect::<Vec<_>>();

        let min_y = self
            .nodes
            .iter()
            .map(|node| node.y)
            .fold(f64::INFINITY, f64::min);
        let max_y = self
            .nodes
            .iter()
            .map(|node| node.y)
            .fold(f64::NEG_INFINITY, f64::max);
        let min_x =
            column_centers.iter().copied().fold(f64::INFINITY, f64::min);

        let header_y = min_y - 160.0;
        let row_label_x = min_x - 260.0;
        let separator_top_y = header_y + 28.0;
        let separator_bottom_y = max_y + 138.0;

        let columns = plan
            .column_states
            .iter()
            .zip(column_centers.iter().copied())
            .map(|(state, x)| KanbanColumnGuide {
                state: state.clone(),
                x,
                y: header_y,
            })
            .collect::<Vec<_>>();
        let row_labels = plan
            .row_labels
            .iter()
            .zip(row_centers.iter().copied())
            .map(|(label, y)| KanbanRowGuide {
                label: label.clone(),
                x: row_label_x,
                y,
            })
            .collect::<Vec<_>>();
        let separators = column_centers
            .windows(2)
            .map(|pair| KanbanSeparatorGuide {
                x: (pair[0] + pair[1]) * 0.5,
                top_y: separator_top_y,
                bottom_y: separator_bottom_y,
            })
            .collect::<Vec<_>>();

        KanbanOverlay {
            columns,
            row_labels,
            separators,
        }
    }

    /// Place nodes on a flat, uniform grid whose overall footprint matches the
    /// viewport aspect ratio.
    ///
    /// Hierarchical order is preserved by laying nodes out in depth bands
    /// (depth 0 = top): each depth level fills full-width rows before the next
    /// depth begins on a fresh row, so a shallower ticket never sits below a
    /// deeper one.  Within a depth, nodes are ordered by priority then title.
    ///
    /// The number of grid columns is derived from the node count and the
    /// viewport aspect ratio so the resulting block reads as a rectangle that
    /// fills the screen instead of a few extremely wide rows.  Column / row
    /// spacing is generous enough that the largest ticket cards never overlap,
    /// even at the highest level of detail, and the grid is kept perfectly flat
    /// (z = 0) so the default head-on camera shows every node.
    fn hierarchical_layout(&mut self) {
        // Uniform grid spacing in layout-space pixels.  Ticket cards are wider
        // than they are tall (≈ 212×132 px at full detail), so columns are
        // spaced further apart than rows to keep equal visual gaps.
        const COL_SPACING: f64 = 380.0;
        const ROW_SPACING: f64 = 260.0;

        let total = self.nodes.len();
        if total == 0 {
            return;
        }

        // Order nodes by depth (hierarchy), then priority, then title.
        let mut order: Vec<usize> = (0..total).collect();
        order.sort_by(|&a, &b| {
            let na = &self.nodes[a];
            let nb = &self.nodes[b];
            na.depth
                .cmp(&nb.depth)
                .then_with(|| {
                    priority_order(na.priority.as_deref())
                        .cmp(&priority_order(nb.priority.as_deref()))
                })
                .then_with(|| {
                    na.title
                        .as_deref()
                        .unwrap_or("")
                        .cmp(nb.title.as_deref().unwrap_or(""))
                })
        });

        // Choose a column count so the overall grid bounding box matches the
        // viewport aspect ratio.  With `grid_w = cols * COL_SPACING`,
        // `grid_h = rows * ROW_SPACING` and `rows ≈ total / cols`, requiring
        // `grid_w / grid_h ≈ aspect` solves to
        // `cols = sqrt(total * aspect * ROW_SPACING / COL_SPACING)`.
        let aspect = viewport_aspect_ratio();
        let cols_f =
            (total as f64 * aspect * (ROW_SPACING / COL_SPACING)).sqrt();
        let cols = (cols_f.round() as usize).clamp(1, total);

        // Lay each depth band into full-width rows, starting every new depth on
        // a fresh row so hierarchy bands stay visually distinct.
        let mut row = 0usize;
        let mut col = 0usize;
        let mut last_depth: Option<usize> = None;
        for &node_idx in &order {
            let depth = self.nodes[node_idx].depth;
            if last_depth.is_some_and(|d| d != depth) && col != 0 {
                row += 1;
                col = 0;
            }
            last_depth = Some(depth);

            self.nodes[node_idx].x = col as f64 * COL_SPACING;
            self.nodes[node_idx].y = row as f64 * ROW_SPACING;
            self.nodes[node_idx].z = 0.0;

            col += 1;
            if col >= cols {
                row += 1;
                col = 0;
            }
        }

        // Centre the grid on the origin.
        self.centre_y();
        self.centre_xz();
    }

    #[allow(dead_code)]
    /// XZ-plane force simulation that keeps depth-layer Y positions fixed.
    ///
    /// * Same-layer nodes repel each other in XZ to prevent crowding.
    /// * Edge-connected nodes attract in XZ so linked subtrees cluster
    ///   toward each other across the 3-D space of their layers.
    /// * Cross-layer repulsion decays with Y distance so nodes primarily
    ///   push away neighbours on the same or adjacent layers.
    fn xz_force_refinement(
        &mut self,
        steps: usize,
    ) {
        let n = self.nodes.len();
        if n <= 1 {
            return;
        }

        // Pre-compute (from_idx, to_idx) for every edge.
        let edge_pairs: Vec<(usize, usize)> = {
            let id_to_idx: HashMap<&str, usize> = self
                .nodes
                .iter()
                .enumerate()
                .map(|(i, nd)| (nd.id.as_str(), i))
                .collect();
            self.edges
                .iter()
                .filter_map(|e| {
                    let i = id_to_idx.get(e.from.as_str()).copied()?;
                    let j = id_to_idx.get(e.to.as_str()).copied()?;
                    Some((i, j))
                })
                .collect()
        };

        // Spring constant: pulls connected nodes toward each other in XZ.
        const SPRING_K: f64 = 0.05;
        // Repulsion strength in the XZ plane.
        const REPULSION: f64 = 80_000.0;
        // Minimum XZ distance used in the repulsion denominator (px).
        const MIN_DIST: f64 = 15.0;
        // Velocity damping per step.
        const DAMPING: f64 = 0.75;
        const DT: f64 = 1.0;

        for _ in 0..steps {
            let mut fx = vec![0.0_f64; n];
            let mut fz = vec![0.0_f64; n];

            // XZ repulsion between every pair of nodes.
            // Repulsion decays for nodes far apart in Y (cross-layer) so the
            // force mainly separates nodes on the same or nearby layers.
            for i in 0..n {
                for j in (i + 1)..n {
                    let dx = self.nodes[i].x - self.nodes[j].x;
                    let dz = self.nodes[i].z - self.nodes[j].z;
                    let dy = (self.nodes[i].y - self.nodes[j].y).abs();
                    let layer_w = 1.0 / (1.0 + dy / 250.0);
                    let d_xz = (dx * dx + dz * dz).sqrt().max(MIN_DIST);
                    let f = REPULSION * layer_w / (d_xz * d_xz);
                    fx[i] += f * dx / d_xz;
                    fx[j] -= f * dx / d_xz;
                    fz[i] += f * dz / d_xz;
                    fz[j] -= f * dz / d_xz;
                }
            }

            // XZ spring attraction along dependency edges.
            // Pulls parent and child toward each other in XZ so connected
            // subtrees group together visually.
            for &(i, j) in &edge_pairs {
                let dx = self.nodes[j].x - self.nodes[i].x;
                let dz = self.nodes[j].z - self.nodes[i].z;
                fx[i] += SPRING_K * dx;
                fx[j] -= SPRING_K * dx;
                fz[i] += SPRING_K * dz;
                fz[j] -= SPRING_K * dz;
            }

            // Integrate X and Z; Y is frozen to preserve depth layers.
            for i in 0..n {
                self.nodes[i].vx = (self.nodes[i].vx + fx[i] * DT) * DAMPING;
                self.nodes[i].vz = (self.nodes[i].vz + fz[i] * DT) * DAMPING;
                self.nodes[i].x += self.nodes[i].vx * DT;
                self.nodes[i].z += self.nodes[i].vz * DT;
            }
        }

        // Zero velocities so the layout is stable when re-used.
        for nd in &mut self.nodes {
            nd.vx = 0.0;
            nd.vz = 0.0;
        }
    }

    /// Translate all nodes so the XZ centre of mass is at (0, 0).
    fn centre_xz(&mut self) {
        if self.nodes.is_empty() {
            return;
        }
        let n = self.nodes.len() as f64;
        let cx = self.nodes.iter().map(|nd| nd.x).sum::<f64>() / n;
        let cz = self.nodes.iter().map(|nd| nd.z).sum::<f64>() / n;
        for nd in &mut self.nodes {
            nd.x -= cx;
            nd.z -= cz;
        }
    }

    /// Return the centre of mass of all node positions.
    pub fn centre_of_mass(&self) -> (f64, f64) {
        if self.nodes.is_empty() {
            return (0.0, 0.0);
        }
        let cx = self.nodes.iter().map(|n| n.x).sum::<f64>()
            / self.nodes.len() as f64;
        let cy = self.nodes.iter().map(|n| n.y).sum::<f64>()
            / self.nodes.len() as f64;
        (cx, cy)
    }

    /// Translate all nodes so the vertical centre of mass is at y = 0.
    /// X is kept as-is (nodes are already horizontally centred per row).
    fn centre_y(&mut self) {
        if self.nodes.is_empty() {
            return;
        }
        let cy = self.nodes.iter().map(|n| n.y).sum::<f64>()
            / self.nodes.len() as f64;
        for n in &mut self.nodes {
            n.y -= cy;
        }
    }

    /// Run `steps` force-simulation iterations (kept for the 3-D GPU path).
    pub fn simulate(
        &mut self,
        steps: usize,
    ) {
        for _ in 0..steps {
            self.step();
        }
    }

    /// One force-directed simulation step (spring-electrical model).
    /// Only called from the 3-D GPU path (`graph3d::Layout3D::from_2d`).
    fn step(&mut self) {
        let n = self.nodes.len();
        if n == 0 {
            return;
        }

        let id_to_idx: HashMap<&str, usize> = self
            .nodes
            .iter()
            .enumerate()
            .map(|(i, node)| (node.id.as_str(), i))
            .collect();

        let mut fx = vec![0.0_f64; n];
        let mut fy = vec![0.0_f64; n];

        // Repulsion
        const REPULSION: f64 = 12_000.0;
        for i in 0..n {
            for j in (i + 1)..n {
                let dx = self.nodes[i].x - self.nodes[j].x;
                let dy = self.nodes[i].y - self.nodes[j].y;
                let dist2 = (dx * dx + dy * dy).max(1.0);
                let dist = dist2.sqrt();
                let force = REPULSION / dist2;
                let fxi = force * dx / dist;
                let fyi = force * dy / dist;
                fx[i] += fxi;
                fy[i] += fyi;
                fx[j] -= fxi;
                fy[j] -= fyi;
            }
        }

        // Spring attraction along edges
        const TARGET_LEN: f64 = 220.0;
        const SPRING_K: f64 = 0.06;
        for edge in &self.edges {
            let Some(&i) = id_to_idx.get(edge.from.as_str()) else {
                continue;
            };
            let Some(&j) = id_to_idx.get(edge.to.as_str()) else {
                continue;
            };
            let dx = self.nodes[j].x - self.nodes[i].x;
            let dy = self.nodes[j].y - self.nodes[i].y;
            let dist = (dx * dx + dy * dy).sqrt().max(0.1);
            let stretch = dist - TARGET_LEN;
            let sfx = SPRING_K * stretch * dx / dist;
            let sfy = SPRING_K * stretch * dy / dist;
            fx[i] += sfx;
            fy[i] += sfy;
            fx[j] -= sfx;
            fy[j] -= sfy;
        }

        // Integration
        const DAMPING: f64 = 0.82;
        const DT: f64 = 0.5;
        for i in 0..n {
            self.nodes[i].vx = (self.nodes[i].vx + fx[i] * DT) * DAMPING;
            self.nodes[i].vy = (self.nodes[i].vy + fy[i] * DT) * DAMPING;
            self.nodes[i].x += self.nodes[i].vx * DT;
            self.nodes[i].y += self.nodes[i].vy * DT;
        }
    }
}

fn sorted_node_indices(nodes: &[GraphNode]) -> Vec<usize> {
    let mut indices = (0..nodes.len()).collect::<Vec<_>>();
    indices.sort_by(|left_idx, right_idx| {
        kanban_node_cmp(nodes, *left_idx, *right_idx)
    });
    indices
}

fn kanban_node_cmp(
    nodes: &[GraphNode],
    left_idx: usize,
    right_idx: usize,
) -> Ordering {
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

fn parents_by_child(
    nodes: &[GraphNode],
    edges: &[GraphEdge],
) -> HashMap<usize, Vec<usize>> {
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
    parents.iter().copied().min_by(|left_idx, right_idx| {
        kanban_node_cmp(nodes, *left_idx, *right_idx)
    })
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

fn kanban_state_cmp(
    left: &str,
    right: &str,
) -> Ordering {
    kanban_state_rank(left)
        .cmp(&kanban_state_rank(right))
        .then_with(|| left.cmp(right))
}

fn kanban_row_label(
    node: &GraphNode,
    row_idx: usize,
) -> String {
    let title = node.title.as_deref().unwrap_or(node.id.as_str()).trim();
    let short_title = if title.is_empty() {
        node.id.as_str()
    } else {
        title
    };
    format!("{:02}. {short_title}", row_idx + 1)
}

fn mean_axis(
    indices: &[usize],
    nodes: &[GraphNode],
    axis: impl Fn(&GraphNode) -> f64,
) -> f64 {
    if indices.is_empty() {
        return 0.0;
    }
    indices.iter().map(|idx| axis(&nodes[*idx])).sum::<f64>()
        / indices.len() as f64
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
mod tests {
    use super::GraphLayout;
    use crate::{
        layout::LayoutMode,
        types::{
            GraphEdgeItem,
            GraphNodeItem,
            TicketRef,
        },
    };

    fn node(
        id: &str,
        state: &str,
        depth: usize,
        title: &str,
    ) -> GraphNodeItem {
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

    fn edge(
        from: &str,
        to: &str,
    ) -> GraphEdgeItem {
        GraphEdgeItem {
            from: from.to_string(),
            to: to.to_string(),
            from_ref: TicketRef::default(),
            to_ref: TicketRef::default(),
            kind: "depends_on".to_string(),
        }
    }

    fn position(
        layout: &GraphLayout,
        node_id: &str,
    ) -> (f64, f64, f64) {
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
        assert!(overlay
            .row_labels
            .iter()
            .all(|row| row.x < leftmost_column_x - 220.0));
        assert!(overlay
            .row_labels
            .iter()
            .any(|row| row.label.contains("Branch A")));
        assert!(overlay
            .row_labels
            .iter()
            .any(|row| row.label.contains("Branch B")));
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
        assert!(
            (impl_a_x - impl_b_x).abs() > 1.0
                || (impl_a_z - impl_b_z).abs() > 1.0
        );
    }
}

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

fn layer_stagger(
    slot: usize,
    spacing: f64,
) -> f64 {
    const OFFSETS: [f64; 7] = [0.0, 1.0, -1.0, 2.0, -2.0, 3.0, -3.0];
    OFFSETS[slot % OFFSETS.len()] * spacing
}
