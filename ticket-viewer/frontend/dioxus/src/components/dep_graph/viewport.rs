use dioxus::prelude::*;
use dioxus_router::Navigator;

use crate::{
    components::ticket_card::TicketCard,
    layout::GraphLayout,
    types::TicketRef,
};

use super::{
    interactions::{
        end_drag,
        handle_wheel,
        select_node_or_navigate,
        start_node_drag,
        start_pan,
        update_drag,
    },
    state::{
        canvas_size,
        DragState,
    },
};

#[component]
pub(super) fn GraphViewport(
    workspace: String,
    on_select: Option<EventHandler<TicketRef>>,
    layout: Signal<Option<GraphLayout>>,
    load_error: Signal<Option<String>>,
    mut fetch_trigger: Signal<u32>,
    mut pan_x: Signal<f64>,
    mut pan_y: Signal<f64>,
    mut zoom: Signal<f64>,
    mut drag: Signal<Option<DragState>>,
    mut picker_open: Signal<bool>,
    children: Element,
) -> Element {
    let nav = use_navigator();
    let (canvas_width, canvas_height) = canvas_size();
    let zoom_value = *zoom.read();
    let pan_x_value = *pan_x.read();
    let pan_y_value = *pan_y.read();
    let cursor = if drag.read().is_some() {
        "cursor: grabbing;"
    } else {
        "cursor: grab;"
    };

    rsx! {
        div {
            id: "dep-graph-viewport",
            style: "
                position: absolute;
                top: 0; left: 0;
                width: 100%; height: 100%;
                overflow: hidden;
                {cursor};
                user-select: none;
            ",
            onwheel: move |event| handle_wheel(zoom, event),
            onmousedown: move |event| start_pan(drag, pan_x, pan_y, event),
            onmousemove: move |event| update_drag(drag, pan_x, pan_y, zoom, layout, event),
            onmouseup: move |_| end_drag(drag),
            onmouseleave: move |_| end_drag(drag),
            {render_error_state(load_error, fetch_trigger)}
            {render_loading_state(layout, load_error)}
            {render_graph_nodes(
                workspace.clone(),
                on_select.clone(),
                nav,
                layout,
                pan_x,
                pan_y,
                zoom,
                drag,
                canvas_width,
                canvas_height,
                pan_x_value,
                pan_y_value,
                zoom_value,
            )}
            {render_hud_controls(pan_x, pan_y, zoom, picker_open)}
            {render_node_count_badge(layout)}
            {children}
        }
    }
}

fn render_error_state(
    load_error: Signal<Option<String>>,
    mut fetch_trigger: Signal<u32>,
) -> Element {
    let Some(error) = load_error.read().clone() else {
        return rsx! {};
    };

    rsx! {
        div {
            style: "
                position: absolute; top: 50%; left: 50%;
                transform: translate(-50%, -50%);
                color: #ef4444; font-family: sans-serif; font-size: 14px;
                text-align: center; pointer-events: auto;
            ",
            p { "Failed to load dependency graph:" }
            p { "{error}" }
            button {
                style: "margin-top:8px; padding:6px 14px; cursor:pointer; border-radius:4px;",
                onclick: move |_| fetch_trigger.with_mut(|value| *value += 1),
                "Retry"
            }
        }
    }
}

fn render_loading_state(
    layout: Signal<Option<GraphLayout>>,
    load_error: Signal<Option<String>>,
) -> Element {
    if layout.read().is_some() || load_error.read().is_some() {
        return rsx! {};
    }

    rsx! {
        div {
            style: "
                position: absolute; top: 50%; left: 50%;
                transform: translate(-50%, -50%);
                color: #9ca3af; font-family: sans-serif; font-size: 14px;
            ",
            "Loading dependency graph…"
        }
    }
}

fn render_graph_nodes(
    _workspace: String,
    on_select: Option<EventHandler<TicketRef>>,
    nav: Navigator,
    layout: Signal<Option<GraphLayout>>,
    pan_x: Signal<f64>,
    pan_y: Signal<f64>,
    _zoom: Signal<f64>,
    drag: Signal<Option<DragState>>,
    canvas_width: f64,
    canvas_height: f64,
    pan_x_value: f64,
    pan_y_value: f64,
    zoom_value: f64,
) -> Element {
    let layout_read = layout.read();
    let Some(layout) = layout_read.as_ref() else {
        return rsx! {};
    };

    rsx! {
        for node in layout.nodes.iter() {
            {
                let node_id = node.id.clone();
                let node_id_drag = node.id.clone();
                let ticket_ref = node.ticket_ref.clone();
                let node_title = node.title.clone();
                let node_state = node.state.clone();
                let node_depth = node.depth;
                let node_priority = node.priority.clone();
                let node_ticket_type = node.ticket_type.clone();
                let layout_x = node.x;
                let layout_y = node.y;
                let on_select = on_select.clone();
                let nav = nav.clone();
                rsx! {
                    TicketCard {
                        key: "{node.id}",
                        id: node_id.clone(),
                        title: node_title,
                        state: node_state,
                        depth: node_depth,
                        priority: node_priority,
                        ticket_type: node_ticket_type,
                        layout_x,
                        layout_y,
                        pan_x: pan_x_value,
                        pan_y: pan_y_value,
                        zoom: zoom_value,
                        canvas_w: canvas_width,
                        canvas_h: canvas_height,
                        on_click: move |_| {
                            select_node_or_navigate(
                                on_select.clone(),
                                nav.clone(),
                                ticket_ref.clone(),
                            )
                        },
                        on_drag_start: move |(_, client_x, client_y): (String, f64, f64)| {
                            start_node_drag(
                                node_id_drag.clone(),
                                layout_x,
                                layout_y,
                                pan_x,
                                pan_y,
                                drag,
                                client_x,
                                client_y,
                            )
                        },
                    }
                }
            }
        }
    }
}

fn render_hud_controls(
    mut pan_x: Signal<f64>,
    mut pan_y: Signal<f64>,
    mut zoom: Signal<f64>,
    mut picker_open: Signal<bool>,
) -> Element {
    rsx! {
        div {
            style: "
                position: absolute; bottom: 16px; right: 16px;
                display: flex; gap: 6px;
                pointer-events: auto;
            ",
            button {
                style: graph_btn_style(),
                title: "Zoom in",
                onclick: move |_| zoom.with_mut(|value| *value = (*value * 1.2).min(8.0)),
                "+"
            }
            button {
                style: graph_btn_style(),
                title: "Zoom out",
                onclick: move |_| zoom.with_mut(|value| *value = (*value / 1.2).max(0.1)),
                "−"
            }
            button {
                style: graph_btn_style(),
                title: "Reset view",
                onclick: move |_| {
                    pan_x.set(0.0);
                    pan_y.set(0.0);
                    zoom.set(1.0);
                },
                "⌂"
            }
            button {
                style: "
                    background: rgba(59,130,246,0.8);
                    color: #fff;
                    border: 1px solid rgba(147,197,253,0.4);
                    border-radius: 4px;
                    padding: 4px 10px;
                    font-size: 13px;
                    cursor: pointer;
                    line-height: 1;
                ",
                title: "Add dependency",
                onclick: move |_| picker_open.set(true),
                "+ Add dependency"
            }
        }
    }
}

fn render_node_count_badge(layout: Signal<Option<GraphLayout>>) -> Element {
    let layout_read = layout.read();
    let Some(layout) = layout_read.as_ref() else {
        return rsx! {};
    };

    rsx! {
        div {
            style: "
                position: absolute; top: 12px; left: 12px;
                color: rgba(200,200,210,0.7);
                font-family: monospace; font-size: 11px;
                pointer-events: none;
            ",
            "{layout.nodes.len()} nodes · {layout.edges.len()} edges"
        }
    }
}

fn graph_btn_style() -> &'static str {
    "
        background: rgba(40,40,55,0.85);
        color: #e0e0e8;
        border: 1px solid rgba(200,200,220,0.25);
        border-radius: 4px;
        padding: 4px 10px;
        font-size: 16px;
        cursor: pointer;
        line-height: 1;
    "
}
