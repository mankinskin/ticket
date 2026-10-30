#[allow(unused_imports)]
use super::*;

impl GraphLayout {
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
    pub(super) fn hierarchical_layout(&mut self) {
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
        let cols_f = (total as f64 * aspect * (ROW_SPACING / COL_SPACING)).sqrt();
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
    pub(super) fn xz_force_refinement(&mut self, steps: usize) {
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
    pub(super) fn centre_xz(&mut self) {
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
        let cx = self.nodes.iter().map(|n| n.x).sum::<f64>() / self.nodes.len() as f64;
        let cy = self.nodes.iter().map(|n| n.y).sum::<f64>() / self.nodes.len() as f64;
        (cx, cy)
    }

    /// Translate all nodes so the vertical centre of mass is at y = 0.
    /// X is kept as-is (nodes are already horizontally centred per row).
    pub(super) fn centre_y(&mut self) {
        if self.nodes.is_empty() {
            return;
        }
        let cy = self.nodes.iter().map(|n| n.y).sum::<f64>() / self.nodes.len() as f64;
        for n in &mut self.nodes {
            n.y -= cy;
        }
    }

    /// Run `steps` force-simulation iterations (kept for the 3-D GPU path).
    pub fn simulate(&mut self, steps: usize) {
        for _ in 0..steps {
            self.step();
        }
    }

    /// One force-directed simulation step (spring-electrical model).
    /// Only called from the 3-D GPU path (`graph3d::Layout3D::from_2d`).
    pub(super) fn step(&mut self) {
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
