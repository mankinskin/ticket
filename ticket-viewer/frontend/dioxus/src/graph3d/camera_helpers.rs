#[allow(unused_imports)]
use super::*;

/// Re-export so existing call sites (`crate::graph3d::can_use_webgpu`) keep
/// working unchanged.
pub fn can_use_webgpu() -> bool {
    can_use_webgpu_graph3d()
}

/// Build a shared [`Layout3D`] from the 2-D force-directed layout.
pub fn lift_2d(gl: GraphLayout, mode: LayoutMode) -> Layout3D {
    let nodes: Vec<Node3D> = gl
        .nodes
        .iter()
        .map(|gn| Node3D {
            id: gn.id.clone(),
            label: gn.title.clone(),
            state: gn.state.clone(),
            x: gn.x as f32 * LAYOUT_SCALE,
            y: -(gn.y as f32 * LAYOUT_SCALE),
            z: match mode {
                LayoutMode::Hierarchical3D | LayoutMode::KanbanTable => gn.z as f32 * LAYOUT_SCALE,
                LayoutMode::Flat2D | LayoutMode::Fixed2D => 0.0,
            },
        })
        .collect();

    let idx: HashMap<&str, usize> = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.as_str(), i))
        .collect();

    let edges: Vec<EdgeRef3D> = gl
        .edges
        .iter()
        .filter_map(|e| {
            let &from_idx = idx.get(e.from.as_str())?;
            let &to_idx = idx.get(e.to.as_str())?;
            Some(EdgeRef3D {
                from_idx,
                to_idx,
                kind: e.kind.clone(),
            })
        })
        .collect();

    Layout3D::new(nodes, edges).with_node_card_profile(NodeCardProfile::TicketWide)
}

pub(super) fn focus_context_ids(
    layout: &Layout3D,
    focus_id: Option<&str>,
) -> Option<HashSet<String>> {
    let focus_id = focus_id?;
    let focus_idx = layout.nodes.iter().position(|node| node.id == focus_id)?;
    let mut ids = HashSet::from([focus_id.to_string()]);
    for edge in &layout.edges {
        if edge.from_idx == focus_idx {
            if let Some(node) = layout.nodes.get(edge.to_idx) {
                ids.insert(node.id.clone());
            }
        }
        if edge.to_idx == focus_idx {
            if let Some(node) = layout.nodes.get(edge.from_idx) {
                ids.insert(node.id.clone());
            }
        }
    }
    Some(ids)
}

pub(super) fn focus_camera_command(
    layout: &Layout3D,
    focus_id: Option<&str>,
) -> Option<CameraCommand> {
    let focus_id = focus_id?;
    let focus_node = layout.nodes.iter().find(|node| node.id == focus_id)?;
    let focus_radius = layout
        .nodes
        .iter()
        .map(|node| {
            let dx = node.x - focus_node.x;
            let dy = node.y - focus_node.y;
            let dz = node.z - focus_node.z;
            (dx * dx + dy * dy + dz * dz).sqrt()
        })
        .fold(0.0_f32, f32::max)
        .max(1.0);
    Some(CameraCommand::FocusOn {
        target: [focus_node.x, focus_node.y, focus_node.z],
        distance: viewer_api_dioxus::graph3d::camera::frame_distance(focus_radius),
    })
}

pub(super) fn initial_camera_for_layout(
    layout: &Layout3D,
    kanban_overlay: Option<&KanbanOverlay>,
    layout_mode: LayoutMode,
) -> Camera {
    let mut camera = Camera::default();
    let framed_bounds = match (layout_mode, kanban_overlay) {
        (LayoutMode::KanbanTable, Some(overlay)) => kanban_camera_bounds(layout, overlay),
        (LayoutMode::KanbanTable, None)
        | (LayoutMode::Hierarchical3D, _)
        | (LayoutMode::Flat2D, _)
        | (LayoutMode::Fixed2D, _) => layout.bounds(),
    };
    camera.frame(framed_bounds.0, framed_bounds.1);
    let (yaw, pitch) = match layout_mode {
        // Look perfectly flat (head-on) at the grid so the whole layout is
        // visible without foreshortening.
        LayoutMode::Hierarchical3D => (0.0_f32, 0.0_f32),
        LayoutMode::Flat2D => (0.0_f32, 0.0_f32),
        LayoutMode::KanbanTable => (0.78_f32, 0.72_f32),
        LayoutMode::Fixed2D => (0.0_f32, 1.5708_f32), // Top-down
    };
    camera.apply_command_for_mode(
        &CameraCommand::ResetTo { yaw, pitch },
        viewer_api_dioxus::graph3d::camera::CameraMode::Orbit,
        framed_bounds,
    );
    camera
}

pub(super) fn kanban_camera_bounds(layout: &Layout3D, overlay: &KanbanOverlay) -> ([f32; 3], f32) {
    let mut points = layout
        .nodes
        .iter()
        .map(|node| [node.x, node.y, node.z])
        .collect::<Vec<_>>();

    for column in &overlay.columns {
        points.push([
            column.x as f32 * LAYOUT_SCALE,
            -(column.y as f32) * LAYOUT_SCALE,
            0.0,
        ]);
    }
    for row in &overlay.row_labels {
        points.push([
            row.x as f32 * LAYOUT_SCALE,
            -(row.y as f32) * LAYOUT_SCALE,
            0.0,
        ]);
    }
    for separator in &overlay.separators {
        points.push([
            separator.x as f32 * LAYOUT_SCALE,
            -(separator.top_y as f32) * LAYOUT_SCALE,
            0.0,
        ]);
        points.push([
            separator.x as f32 * LAYOUT_SCALE,
            -(separator.bottom_y as f32) * LAYOUT_SCALE,
            0.0,
        ]);
    }

    if points.is_empty() {
        return layout.bounds();
    }

    let mut min = points[0];
    let mut max = points[0];
    for point in &points[1..] {
        min[0] = min[0].min(point[0]);
        min[1] = min[1].min(point[1]);
        min[2] = min[2].min(point[2]);
        max[0] = max[0].max(point[0]);
        max[1] = max[1].max(point[1]);
        max[2] = max[2].max(point[2]);
    }

    let center = [
        (min[0] + max[0]) * 0.5,
        (min[1] + max[1]) * 0.5,
        (min[2] + max[2]) * 0.5,
    ];
    let radius = points
        .iter()
        .map(|point| {
            let dx = point[0] - center[0];
            let dy = point[1] - center[1];
            let dz = point[2] - center[2];
            (dx * dx + dy * dy + dz * dz).sqrt()
        })
        .fold(0.0_f32, f32::max)
        .max(1.0)
        + 1.2;

    (center, radius)
}

pub(super) fn state_label(state: &str) -> String {
    state
        .split('-')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let mut label = String::new();
                    label.push(first.to_ascii_uppercase());
                    label.push_str(chars.as_str());
                    label
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn layout_anchor_x(value: f64) -> String {
    format!("{}", value as f32 * LAYOUT_SCALE)
}

pub(super) fn layout_anchor_y(value: f64) -> String {
    format!("{}", -(value as f32) * LAYOUT_SCALE)
}

pub(super) fn layout_anchor_z(value: f64) -> String {
    format!("{}", value as f32 * LAYOUT_SCALE)
}
