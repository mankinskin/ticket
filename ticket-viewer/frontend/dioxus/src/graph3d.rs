//! Ticket dependency graph — thin wrapper around the shared
//! [`viewer_api_dioxus::Graph3D`] component.
//!
//! Fetches the ticket subgraph from the backend, runs the 2-D force-directed
//! layout, lifts the result to a shared [`Layout3D`] (depth → Z), and renders
//! ticket cards as children of `<Graph3D>` so the shared renderer can position
//! them every frame via `data-node-idx` lookups.
//!
//! # Debug logging
//!
//! Every lifecycle event (mount, unmount, cache hit, debounce, fetch) is
//! emitted as a `tracing::debug!` record with `target = "ticket_viewer::graph3d"`.
//! Open the viewer with `?log=debug` to route them to the browser console:
//!
//! ```text
//! http://localhost:3002/workspace/default?log=ticket_viewer=debug
//! ```

use std::collections::{
    HashMap,
    HashSet,
};

use dioxus::prelude::*;

use viewer_api_dioxus::{
    can_use_webgpu_graph3d,
    Camera,
    CameraCommand,
    EdgeRef3D,
    Layout3D,
    Node3D,
    NodeCardProfile,
    Projection,
};

use crate::{
    components::ticket_card,
    layout::{
        GraphLayout,
        KanbanOverlay,
        LayoutMode,
    },
    types::TicketRef,
};

/// Tracing target used by all events in this module.
const T: &str = "ticket_viewer::graph3d";
const LAYOUT_SCALE: f32 = 1.0 / 100.0_f32;

/// Re-export so existing call sites (`crate::graph3d::can_use_webgpu`) keep
/// working unchanged.
pub fn can_use_webgpu() -> bool {
    can_use_webgpu_graph3d()
}

/// Build a shared [`Layout3D`] from the 2-D force-directed layout.
pub fn lift_2d(
    gl: GraphLayout,
    mode: LayoutMode,
) -> Layout3D {
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
                LayoutMode::Hierarchical3D | LayoutMode::KanbanTable =>
                    gn.z as f32 * LAYOUT_SCALE,
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

    Layout3D::new(nodes, edges)
        .with_node_card_profile(NodeCardProfile::TicketWide)
}

fn focus_context_ids(
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

fn focus_camera_command(
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
        distance: viewer_api_dioxus::graph3d::camera::frame_distance(
            focus_radius,
        ),
    })
}

fn initial_camera_for_layout(
    layout: &Layout3D,
    kanban_overlay: Option<&KanbanOverlay>,
    layout_mode: LayoutMode,
) -> Camera {
    let mut camera = Camera::default();
    let framed_bounds = match (layout_mode, kanban_overlay) {
        (LayoutMode::KanbanTable, Some(overlay)) =>
            kanban_camera_bounds(layout, overlay),
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

fn kanban_camera_bounds(
    layout: &Layout3D,
    overlay: &KanbanOverlay,
) -> ([f32; 3], f32) {
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

fn state_label(state: &str) -> String {
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
                },
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn layout_anchor_x(value: f64) -> String {
    format!("{}", value as f32 * LAYOUT_SCALE)
}

fn layout_anchor_y(value: f64) -> String {
    format!("{}", -(value as f32) * LAYOUT_SCALE)
}

fn layout_anchor_z(value: f64) -> String {
    format!("{}", value as f32 * LAYOUT_SCALE)
}

fn render_kanban_overlay(overlay: &KanbanOverlay) -> Element {
    rsx! {
        div {
            id: "graph3d-kanban-guides",
            style: "position: absolute; inset: 0; pointer-events: none;",
            for (index, separator) in overlay.separators.iter().enumerate() {
                div {
                    key: "kanban-separator-{index}",
                    "data-kanban-column-separator": "{index}",
                    "data-layout-line-x1": "{layout_anchor_x(separator.x)}",
                    "data-layout-line-y1": "{layout_anchor_y(separator.top_y)}",
                    "data-layout-line-z1": "0",
                    "data-layout-line-x2": "{layout_anchor_x(separator.x)}",
                    "data-layout-line-y2": "{layout_anchor_y(separator.bottom_y)}",
                    "data-layout-line-z2": "0",
                    "data-layout-z-index": "12",
                    style: "position: absolute; display: none; pointer-events: none; height: 2px; background: linear-gradient(180deg, rgba(122, 162, 255, 0.18) 0%, rgba(122, 162, 255, 0.42) 45%, rgba(122, 162, 255, 0.18) 100%); box-shadow: 0 0 0 1px rgba(122, 162, 255, 0.08); transform-origin: 0 50%;",
                }
            }
            for column in overlay.columns.iter() {
                div {
                    key: "kanban-column-{column.state}",
                    "data-kanban-column-header": "{column.state}",
                    "data-layout-anchor-x": "{layout_anchor_x(column.x)}",
                    "data-layout-anchor-y": "{layout_anchor_y(column.y)}",
                    "data-layout-anchor-z": "0",
                    "data-layout-anchor-origin": "center-bottom",
                    "data-layout-z-index": "18",
                    style: "position: absolute; display: none; min-width: 128px; max-width: 188px; padding: 7px 12px; border-radius: 999px; border: 1px solid rgba(122, 162, 255, 0.24); background: linear-gradient(180deg, rgba(20, 26, 38, 0.92) 0%, rgba(13, 17, 28, 0.84) 100%); box-shadow: 0 10px 22px rgba(0, 0, 0, 0.24); color: rgba(232, 240, 255, 0.92); font-family: sans-serif; font-size: 11px; font-weight: 700; letter-spacing: 0.08em; line-height: 1; text-align: center; text-transform: uppercase; white-space: nowrap; transform-origin: center bottom;",
                    "{state_label(&column.state)}"
                }
            }
            for row in overlay.row_labels.iter() {
                div {
                    key: "kanban-row-{row.label}",
                    "data-kanban-row-label": "{row.label}",
                    "data-layout-anchor-x": "{layout_anchor_x(row.x)}",
                    "data-layout-anchor-y": "{layout_anchor_y(row.y)}",
                    "data-layout-anchor-z": "0",
                    "data-layout-anchor-origin": "right-center",
                    "data-layout-z-index": "16",
                    style: "position: absolute; display: none; max-width: 196px; padding: 5px 8px; border-radius: 12px; border: 1px solid rgba(255, 255, 255, 0.10); background: rgba(14, 18, 28, 0.80); box-shadow: 0 8px 20px rgba(0, 0, 0, 0.20); color: rgba(218, 224, 236, 0.86); font-family: sans-serif; font-size: 9px; font-weight: 600; line-height: 1.02; letter-spacing: 0.02em; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; transform-origin: right center;",
                    "{row.label}"
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct Graph3DProps {
    pub workspace: String,
    pub root_id: String,
    pub on_select: EventHandler<TicketRef>,
    /// Optional callback invoked when the user hovers a graph node.
    #[props(optional)]
    pub on_hover: Option<EventHandler<Option<String>>>,
    /// Optional callback invoked when the user clicks empty graph space.
    #[props(optional)]
    pub on_deselect: Option<EventHandler<()>>,
    /// Optional graph-preview selection — highlights this node with an ember
    /// border effect without changing the primary ticket-list selection.
    #[props(optional)]
    pub selected_node_id: Option<String>,
    /// Currently hovered node id. Used for transient edge emphasis.
    #[props(optional)]
    pub hovered_node_id: Option<String>,
    /// Graph layout algorithm — drives `lift_2d` and the initial camera angle.
    #[props(default)]
    pub layout_mode: LayoutMode,
    /// Camera projection mode.
    #[props(default)]
    pub projection: Projection,
    /// Called by the built-in settings overlay when the user picks a new layout.
    #[props(default)]
    pub on_layout_mode_change: Option<EventHandler<LayoutMode>>,
    /// Called by the built-in settings overlay when the user picks a new projection.
    #[props(default)]
    pub on_projection_change: Option<EventHandler<Projection>>,
}

#[component]
pub fn Graph3D(props: Graph3DProps) -> Element {
    let workspace = props.workspace.clone();
    let root_id = props.root_id.clone();
    let on_select = props.on_select;
    let on_deselect = props.on_deselect.clone();
    let selected_node_id = props.selected_node_id.clone();
    let hovered_node_id = props.hovered_node_id.clone();
    let layout_mode = props.layout_mode;
    let projection = props.projection;
    let on_layout_mode_change = props.on_layout_mode_change.clone();
    let on_projection_change = props.on_projection_change.clone();

    // ── Fetch service + cache ─────────────────────────────────────────────
    // Graph3D never issues its own HTTP requests.  It reads the shared cache
    // (populated by GraphFetchService) and re-renders whenever the service
    // bumps its version counter after a fetch completes.
    let svc: crate::graph_fetch::GraphFetchService =
        use_context::<crate::graph_fetch::GraphFetchService>();
    let cache: crate::GraphCache = use_context::<crate::GraphCache>();
    let cache_key = crate::graph_fetch::workspace_cache_key(&workspace);

    // ── Lifecycle logging ─────────────────────────────────────────────────
    tracing::debug!(target: T, rid = %root_id, "mount");
    {
        let rid_drop = root_id.clone();
        use_drop(move || {
            tracing::debug!(target: T, rid = %rid_drop, "unmount");
        });
    }

    // Subscribe to the version signal so Dioxus re-renders this component
    // whenever a background fetch completes and writes a new layout to the
    // cache.  The actual value is not used — the read is purely for reactivity.
    let _ver = svc.version();
    let fetch_state = svc.state_for(&workspace, &root_id);
    #[cfg(target_arch = "wasm32")]
    web_sys::console::log_1(
        &format!("[graph3d] render rid={root_id} ver={_ver}").into(),
    );
    tracing::debug!(target: T, rid = %root_id, ver = _ver, state = ?fetch_state, "render");

    // Try to get the layout from the cache.  If absent, show the spinner.
    // GraphFetchService.ensure_fetched() was already called by TicketListPage
    // for the active workspace; we do NOT start a second fetch here.
    let (graph_layout, ticket_refs_by_id) = match cache.get(&cache_key) {
        Some(layout) => {
            let graph_layout = layout.with_mode(layout_mode);
            tracing::debug!(target: T, rid = %root_id, nodes = graph_layout.nodes.len(), "cache_hit");
            let ticket_refs_by_id = graph_layout
                .nodes
                .iter()
                .map(|node| (node.id.clone(), node.ticket_ref.clone()))
                .collect::<HashMap<_, _>>();
            (graph_layout, ticket_refs_by_id)
        },
        None => match fetch_state {
            crate::graph_fetch::GraphFetchState::Error {
                message,
                attempts,
            } => {
                tracing::warn!(target: T, rid = %root_id, attempts, error = %message, "graph_fetch_failed");
                let svc_retry = svc.clone();
                let ws_retry = workspace.clone();
                let rid_retry = root_id.clone();
                return rsx! {
                    div {
                        style: "position: absolute; top: 50%; left: 50%; transform: translate(-50%, -50%); max-width: 560px; color: #ddd; font-size: 13px; font-family: sans-serif; background: rgba(25, 28, 34, 0.92); border: 1px solid rgba(255, 120, 120, 0.35); border-radius: 8px; padding: 12px 14px; box-shadow: 0 8px 24px rgba(0,0,0,0.4);",
                        div { style: "font-weight: 600; color: #ff9d9d; margin-bottom: 6px;", "Failed to load graph" }
                        div { style: "color: #f0c7c7; margin-bottom: 4px;", "{message}" }
                        div { style: "color: #a8a8a8; font-size: 12px; margin-bottom: 10px;", "Attempts: {attempts}" }
                        button {
                            style: "padding: 6px 10px; border-radius: 6px; border: 1px solid rgba(255,255,255,0.2); background: rgba(255,255,255,0.08); color: #efefef; cursor: pointer;",
                            onclick: move |_| {
                                svc_retry.retry(&ws_retry, &rid_retry);
                            },
                            "Retry"
                        }
                    }
                };
            },
            _ => {
                tracing::debug!(target: T, rid = %root_id, state = ?fetch_state, "waiting_for_cache");
                return rsx! {
                    div {
                        style: "position: absolute; top: 50%; left: 50%; transform: translate(-50%, -50%); color: #aaa; font-size: 14px; font-family: sans-serif;",
                        "Loading graph\u{2026}"
                    }
                };
            },
        },
    };

    let kanban_overlay = graph_layout.kanban_overlay.clone();
    let layout = lift_2d(graph_layout, layout_mode);
    let nodes = layout.nodes.clone();
    let node_count = nodes.len();
    let focus_context = focus_context_ids(&layout, selected_node_id.as_deref());
    let initial_camera = initial_camera_for_layout(
        &layout,
        kanban_overlay.as_ref(),
        layout_mode,
    );

    rsx! {
        viewer_api_dioxus::Graph3D {
            layout: layout,
            initial_camera: Some(initial_camera),
            selected_node_id: selected_node_id.clone(),
            hovered_node_id: props.hovered_node_id.clone(),
            selection_auto_focus: true,
            projection: projection,
            layout_mode: layout_mode,
            on_layout_mode_change: on_layout_mode_change,
            on_projection_change: on_projection_change,
            on_deselect: move |_| {
                if let Some(ref handler) = on_deselect {
                    handler.call(());
                }
            },
            if layout_mode == LayoutMode::KanbanTable {
                if let Some(overlay) = kanban_overlay.as_ref() {
                    {render_kanban_overlay(overlay)}
                }
            }
            div {
                id: "graph3d-nodes",
                style: "position: absolute; inset: 0; pointer-events: none;",
                for (idx, node) in nodes.iter().enumerate() {
                    {
                        let node_id    = node.id.clone();
                        let title      = node.label.clone().unwrap_or_else(|| "Untitled".into());
                        let state_str  = node.state.clone().unwrap_or_else(|| "open".into());
                        let color      = ticket_card::state_color(Some(state_str.as_str()));
                        let short_id   = if node_id.len() > 8 { node_id[..8].to_string() } else { node_id.clone() };
                        let ticket_ref_click = ticket_refs_by_id
                            .get(&node_id)
                            .cloned()
                            .unwrap_or_else(|| TicketRef::new(workspace.clone(), node_id.clone()));
                        let is_selected = selected_node_id.as_deref() == Some(node_id.as_str());
                        let is_hovered = hovered_node_id.as_deref() == Some(node_id.as_str());
                        let is_focus_context = focus_context
                            .as_ref()
                            .map(|ids| ids.contains(&node_id))
                            .unwrap_or(false);
                        let card_class = if is_selected {
                            "graph-node-card content node-card-selected"
                        } else if is_hovered {
                            "graph-node-card content node-card-hovered"
                        } else if is_focus_context {
                            "graph-node-card content node-card-context"
                        } else {
                            "graph-node-card content"
                        };
                        let focus_style = if is_selected {
                            "opacity: 1.0; filter: saturate(1.0) brightness(1.0); transform: scale(1.08); box-shadow: 0 0 20px var(--graph-node-glow, rgba(122, 162, 255, 0.6));"
                        } else if is_focus_context {
                            "opacity: 0.95; filter: saturate(0.92) brightness(0.96);"
                        } else if focus_context.is_some() {
                            "opacity: 0.3; filter: saturate(0.4) brightness(0.78);"
                        } else {
                            "opacity: 1.0; filter: none;"
                        };
                        rsx! {
                            div {
                                key: "{node_id}",
                                class: "{card_class}",
                                "data-node-idx": "{idx}",
                                "data-node-id": "{node_id}",
                                "data-node-state": "{state_str}",
                                "data-layout-x": "{node.x}",
                                "data-layout-y": "{node.y}",
                                "data-layout-z": "{node.z}",
                                style: "position: absolute; top: 0; left: 0; pointer-events: auto; transform-origin: center center; display: none; width: 212px; height: 132px; box-sizing: border-box; border: 1px solid color-mix(in srgb, {color} 35%, var(--graph-node-border, rgba(200,200,200,0.35))); border-top: 3px solid {color}; border-radius: 18px; background: linear-gradient(180deg, color-mix(in srgb, {color} 10%, var(--graph-node-surface, rgba(30,30,40,0.92))) 0%, var(--graph-node-surface, rgba(30,30,40,0.94)) 100%); backdrop-filter: blur(6px); padding: 12px; cursor: pointer; overflow: hidden; font-family: sans-serif; color: var(--graph-node-text, #e8e8f0); box-shadow: var(--graph-node-shadow, 0 10px 24px rgba(0,0,0,0.42)); transition: opacity 160ms ease, filter 160ms ease, box-shadow 160ms ease, transform 160ms ease; {focus_style}",
                                onclick: move |evt: Event<MouseData>| {
                                    evt.stop_propagation();
                                    on_select.call(ticket_ref_click.clone());
                                },
                                onmouseenter: move |_| {
                                    if let Some(ref handler) = props.on_hover {
                                        handler.call(Some(node_id.clone()));
                                    }
                                },
                                onmouseleave: move |_| {
                                    if let Some(ref handler) = props.on_hover {
                                        handler.call(None);
                                    }
                                },
                                div {
                                    "data-node-detail-tier": "rich",
                                    "data-node-detail-display": "block",
                                    style: "display: flex; flex-direction: column; justify-content: space-between; gap: 10px; height: 100%;",
                                    div {
                                        span {
                                            style: "display: inline-flex; align-items: center; align-self: flex-start; padding: 4px 8px; border-radius: 999px; background: color-mix(in srgb, {color} 18%, rgba(255,255,255,0.06)); border: 1px solid color-mix(in srgb, {color} 32%, rgba(255,255,255,0.12)); font-size: 10px; color: {color}; font-weight: 700; letter-spacing: 0.05em; text-transform: uppercase;",
                                            "{state_str}"
                                        }
                                    }
                                    div {
                                        style: "font-size: 13px; line-height: 1.32; font-weight: 650; color: var(--graph-node-text, #e8e8f0); overflow: hidden; display: -webkit-box; -webkit-line-clamp: 3; -webkit-box-orient: vertical; word-break: break-word;",
                                        "{title}"
                                    }
                                    div {
                                        style: "display: flex; align-items: center; justify-content: space-between; gap: 8px; margin-top: auto;",
                                        div {
                                            style: "font-size: 10px; color: var(--graph-node-muted-text, #9aa0ac); letter-spacing: 0.05em; text-transform: uppercase;",
                                            "Ticket"
                                        }
                                        span {
                                            style: "display: inline-flex; align-items: center; padding: 3px 7px; border-radius: 999px; background: rgba(255,255,255,0.06); color: var(--graph-node-text, #e8e8f0); font-size: 10px; font-weight: 600; letter-spacing: 0.04em;",
                                            "{short_id}"
                                        }
                                    }
                                }
                                div {
                                    "data-node-detail-tier": "label",
                                    "data-node-detail-display": "flex",
                                    style: "display: none; flex-direction: column; justify-content: center; align-items: center; gap: 4px; width: 100%; height: 100%;",
                                    div {
                                        style: "font-size: 11px; font-weight: 700; color: var(--graph-node-text, #e8e8f0); text-align: center; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; width: 100%;",
                                        "{title}"
                                    }
                                    div {
                                        style: "font-size: 9px; color: {color}; text-transform: uppercase; font-weight: 600;",
                                        "{state_str}"
                                    }
                                }
                                div {
                                    "data-node-detail-tier": "icon",
                                    "data-node-detail-display": "flex",
                                    style: "display: none; align-items: center; justify-content: center; width: 100%; height: 100%;",
                                    span {
                                        style: "width: 24px; height: 24px; border-radius: 6px; background: {color}; display: flex; align-items: center; justify-content: center; color: white; font-size: 12px; font-weight: 800; box-shadow: 0 4px 10px rgba(0,0,0,0.3);",
                                        "{state_str.chars().next().unwrap_or('?').to_uppercase()}"
                                    }
                                }
                                div {
                                    "data-node-detail-tier": "minimal",
                                    "data-node-detail-display": "flex",
                                    style: "display: none; align-items: center; justify-content: center; gap: 6px; width: 100%; height: 100%;",
                                    span {
                                        style: "width: 12px; height: 12px; border-radius: 999px; background: {color}; flex-shrink: 0; box-shadow: 0 0 0 1px rgba(255,255,255,0.12);",
                                    }
                                    span {
                                        style: "font-size: 10px; font-weight: 700; color: var(--graph-node-text, #e8e8f0); letter-spacing: 0.08em; text-transform: uppercase;",
                                        "{short_id}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
            div {
                style: "position: absolute; bottom: 12px; left: 50%; transform: translateX(-50%); font-size: 11px; color: rgba(255,255,255,0.3); font-family: sans-serif; pointer-events: none; white-space: nowrap;",
                "Left-drag: orbit \u{00b7} Right-drag: pan \u{00b7} Scroll: zoom \u{00b7} Click card: open"
            }
            if node_count > 0 {
                div {
                    style: "position: absolute; top: 12px; right: 12px; font-size: 11px; color: rgba(255,255,255,0.35); font-family: sans-serif; pointer-events: none;",
                    "{node_count} nodes"
                }
            }
        }
    }
}
