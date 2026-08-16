use dioxus::prelude::*;
use dioxus_router::Navigator;

use crate::{
    layout::GraphLayout,
    routes::Route,
    types::TicketRef,
};

use super::state::{
    DragKind,
    DragState,
};

pub(super) fn select_node_or_navigate(
    on_select: Option<EventHandler<TicketRef>>,
    nav: Navigator,
    ticket_ref: TicketRef,
) {
    if let Some(ref callback) = on_select {
        callback.call(ticket_ref.clone());
    } else {
        nav.push(Route::TicketDetailPage {
            workspace: ticket_ref.workspace,
            id: ticket_ref.id,
        });
    }
}

pub(super) fn handle_wheel(
    mut zoom: Signal<f64>,
    event: Event<WheelData>,
) {
    event.prevent_default();
    let delta = event.delta().strip_units().y;
    let factor = if delta < 0.0 { 1.12 } else { 0.89 };
    zoom.with_mut(|value| *value = (*value * factor).clamp(0.1, 8.0));
}

pub(super) fn start_pan(
    mut drag: Signal<Option<DragState>>,
    pan_x: Signal<f64>,
    pan_y: Signal<f64>,
    event: Event<MouseData>,
) {
    let client = event.client_coordinates();
    drag.set(Some(DragState {
        kind: DragKind::Pan,
        start_client_x: client.x,
        start_client_y: client.y,
        start_pan_x: *pan_x.read(),
        start_pan_y: *pan_y.read(),
        start_node_x: 0.0,
        start_node_y: 0.0,
    }));
}

pub(super) fn start_node_drag(
    node_id: String,
    layout_x: f64,
    layout_y: f64,
    pan_x: Signal<f64>,
    pan_y: Signal<f64>,
    mut drag: Signal<Option<DragState>>,
    client_x: f64,
    client_y: f64,
) {
    drag.set(Some(DragState {
        kind: DragKind::Node(node_id),
        start_client_x: client_x,
        start_client_y: client_y,
        start_pan_x: *pan_x.read(),
        start_pan_y: *pan_y.read(),
        start_node_x: layout_x,
        start_node_y: layout_y,
    }));
}

pub(super) fn update_drag(
    drag: Signal<Option<DragState>>,
    mut pan_x: Signal<f64>,
    mut pan_y: Signal<f64>,
    zoom: Signal<f64>,
    mut layout: Signal<Option<GraphLayout>>,
    event: Event<MouseData>,
) {
    let Some(state) = drag.read().clone() else {
        return;
    };

    let client = event.client_coordinates();
    let zoom_value = *zoom.read();
    let delta_x = (client.x - state.start_client_x) / zoom_value;
    let delta_y = (client.y - state.start_client_y) / zoom_value;

    match &state.kind {
        DragKind::Pan => {
            pan_x.set(state.start_pan_x + delta_x);
            pan_y.set(state.start_pan_y + delta_y);
        },
        DragKind::Node(node_id) => {
            let node_id = node_id.clone();
            layout.with_mut(|maybe_layout| {
                if let Some(layout) = maybe_layout {
                    if let Some(node) =
                        layout.nodes.iter_mut().find(|node| node.id == node_id)
                    {
                        node.x = state.start_node_x + delta_x;
                        node.y = state.start_node_y + delta_y;
                    }
                }
            });
        },
    }
}

pub(super) fn end_drag(mut drag: Signal<Option<DragState>>) {
    drag.set(None);
}
