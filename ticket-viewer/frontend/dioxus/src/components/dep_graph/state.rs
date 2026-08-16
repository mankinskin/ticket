use dioxus::prelude::*;
use wasm_bindgen::JsCast;

#[derive(Debug, Clone, PartialEq)]
pub(super) enum DragKind {
    Pan,
    Node(String),
}

#[derive(Debug, Clone)]
pub(super) struct DragState {
    pub kind: DragKind,
    pub start_client_x: f64,
    pub start_client_y: f64,
    pub start_pan_x: f64,
    pub start_pan_y: f64,
    pub start_node_x: f64,
    pub start_node_y: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct RemoveEdge {
    pub from_id: String,
    pub to_id: String,
    pub kind: String,
    pub from_title: String,
    pub to_title: String,
}

pub(super) fn canvas_size() -> (f64, f64) {
    if let Some((width, height)) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id("dep-graph-viewport"))
        .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
        .map(|element| {
            (
                element.client_width() as f64,
                element.client_height() as f64,
            )
        })
        .filter(|&(width, height)| width > 0.0 && height > 0.0)
    {
        return (width, height);
    }

    web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id("webgpu-canvas"))
        .and_then(|element| {
            element.dyn_into::<web_sys::HtmlCanvasElement>().ok()
        })
        .map(|canvas| {
            (canvas.client_width() as f64, canvas.client_height() as f64)
        })
        .unwrap_or((800.0, 600.0))
}

pub(super) struct DepSseHandle {
    es: web_sys::EventSource,
    _listeners: [gloo_events::EventListener; 3],
}

impl Drop for DepSseHandle {
    fn drop(&mut self) {
        self.es.close();
    }
}

pub(super) fn subscribe_sse(
    workspace: &str,
    fetch_trigger: Signal<u32>,
) -> Option<DepSseHandle> {
    let url = format!("/api/stream?workspace={workspace}");
    let event_source = web_sys::EventSource::new(&url).ok()?;
    let mut trigger_upsert = fetch_trigger;
    let mut trigger_delete = fetch_trigger;
    let mut trigger_ticket = fetch_trigger;
    let upsert_listener = gloo_events::EventListener::new(
        &event_source,
        "edge.upsert",
        move |_| {
            trigger_upsert.with_mut(|value| *value += 1);
        },
    );
    let delete_listener = gloo_events::EventListener::new(
        &event_source,
        "edge.delete",
        move |_| {
            trigger_delete.with_mut(|value| *value += 1);
        },
    );
    let ticket_listener = gloo_events::EventListener::new(
        &event_source,
        "ticket.upsert",
        move |_| {
            trigger_ticket.with_mut(|value| *value += 1);
        },
    );

    Some(DepSseHandle {
        es: event_source,
        _listeners: [upsert_listener, delete_listener, ticket_listener],
    })
}
