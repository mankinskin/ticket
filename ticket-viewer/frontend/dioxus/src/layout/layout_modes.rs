#[allow(unused_imports)]
use super::*;

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

    pub fn with_mode(&self, mode: LayoutMode) -> Self {
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

    pub(super) fn fixed_2d_layout(&mut self) {
        self.hierarchical_layout();
        for nd in &mut self.nodes {
            nd.z = 0.0;
        }
    }

    pub(super) fn apply_layout_mode(&mut self, mode: LayoutMode) {
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
            }
            LayoutMode::KanbanTable => self.kanban_table_layout(),
            LayoutMode::Fixed2D => self.fixed_2d_layout(),
        }
    }

    pub(super) fn kanban_table_layout(&mut self) {
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
            self.nodes[*node_idx].x = placement.column as f64 * COLUMN_SPACING + local_x;
            self.nodes[*node_idx].y = placement.row as f64 * ROW_SPACING + local_y;
            self.nodes[*node_idx].z = local_z;
        }

        self.centre_y();
        self.centre_xz();
        self.kanban_overlay = Some(self.build_kanban_overlay(&plan));
    }

    pub(super) fn kanban_layout_plan(&self) -> KanbanLayoutPlan {
        let ordered_indices = sorted_node_indices(&self.nodes);
        let parents_by_child = parents_by_child(&self.nodes, &self.edges);
        let mut group_seed_by_idx = vec![None; self.nodes.len()];

        for &node_idx in &ordered_indices {
            let primary_parent = primary_parent_index(&self.nodes, &parents_by_child, node_idx);
            let seed = match primary_parent {
                None => node_idx,
                Some(parent_idx) if !parents_by_child.contains_key(&parent_idx) => node_idx,
                Some(parent_idx) => group_seed_by_idx[parent_idx].unwrap_or(parent_idx),
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
            let state_key = kanban_state_key(self.nodes[node_idx].state.as_deref());
            let column = *column_by_state
                .get(&state_key)
                .expect("kanban column should exist for state");
            cells.entry((column, row)).or_default().push(node_idx);
        }

        let mut placements = HashMap::with_capacity(self.nodes.len());
        for ((column, row), cell_indices) in &mut cells {
            cell_indices
                .sort_by(|left_idx, right_idx| kanban_node_cmp(&self.nodes, *left_idx, *right_idx));
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
            row_labels[row_idx] = kanban_row_label(&self.nodes[seed_idx], row_idx);
        }

        KanbanLayoutPlan {
            placements,
            column_states,
            row_labels,
        }
    }

    pub(super) fn build_kanban_overlay(&self, plan: &KanbanLayoutPlan) -> KanbanOverlay {
        if plan.placements.is_empty() || plan.column_states.is_empty() || plan.row_labels.is_empty()
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
        let min_x = column_centers.iter().copied().fold(f64::INFINITY, f64::min);

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
}
