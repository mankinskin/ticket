use dioxus::prelude::*;
#[cfg(target_arch = "wasm32")]
use dioxus_router::Navigator;

#[cfg(target_arch = "wasm32")]
use crate::types::TicketRef;
use crate::{
    api::{
        HttpTicketBackend,
        TicketBackend,
    },
    layout::GraphLayout,
};

#[cfg(target_arch = "wasm32")]
use super::interactions::select_node_or_navigate;
use super::{
    edge_list::{
        EdgeListSidebar,
        RemoveEdgeDialog,
    },
    picker::EdgePickerOverlay,
    state::{
        subscribe_sse,
        DepSseHandle,
        DragState,
        RemoveEdge,
    },
    viewport::GraphViewport,
    DepGraphProps,
};

#[component]
pub fn DepGraph(props: DepGraphProps) -> Element {
    let workspace = props.workspace.clone();
    let root_id = props.root_id.clone();
    let on_select = props.on_select.clone();
    let _on_hover = props.on_hover.clone();
    let _on_deselect = props.on_deselect.clone();
    let _selected_node_id = props.selected_node_id.clone();
    let _hovered_node_id = props.hovered_node_id.clone();
    let _nav = use_navigator();

    #[cfg(target_arch = "wasm32")]
    if crate::graph3d::can_use_webgpu() {
        return render_webgpu_graph(
            _nav,
            props,
            workspace,
            root_id,
            on_select,
            _on_hover,
            _on_deselect,
            _selected_node_id,
            _hovered_node_id,
        );
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut layout: Signal<Option<GraphLayout>> = use_signal(|| None);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut load_error: Signal<Option<String>> = use_signal(|| None);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut fetch_trigger: Signal<u32> = use_signal(|| 0_u32);
    let pan_x: Signal<f64> = use_signal(|| 0.0_f64);
    let pan_y: Signal<f64> = use_signal(|| 0.0_f64);
    let zoom: Signal<f64> = use_signal(|| 1.0_f64);
    let drag: Signal<Option<DragState>> = use_signal(|| None);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut sse_handle: Signal<Option<DepSseHandle>> = use_signal(|| None);
    let picker_open: Signal<bool> = use_signal(|| false);
    let remove_confirm: Signal<Option<RemoveEdge>> = use_signal(|| None);

    {
        let workspace_fetch = workspace.clone();
        use_effect(move || {
            let _trigger = fetch_trigger();
            let workspace = workspace_fetch.clone();
            let mut layout = layout;
            let mut load_error = load_error;
            spawn(async move {
                let backend = HttpTicketBackend::new(None);
                match backend.get_workspace_graph(&workspace).await {
                    Ok(response) => {
                        let active_workspace =
                            if response.active_workspace.is_empty() {
                                workspace.clone()
                            } else {
                                response.active_workspace.clone()
                            };
                        layout.set(Some(GraphLayout::build(
                            &active_workspace,
                            response.nodes,
                            response.edges,
                        )));
                        load_error.set(None);
                    },
                    Err(error) => load_error.set(Some(error)),
                }
            });
        });
    }

    {
        let workspace_sse = workspace.clone();
        use_effect(move || {
            sse_handle.set(subscribe_sse(&workspace_sse, fetch_trigger));
        });
    }

    rsx! {
        GraphViewport {
            workspace: workspace.clone(),
            on_select: on_select.clone(),
            layout,
            load_error,
            fetch_trigger,
            pan_x,
            pan_y,
            zoom,
            drag,
            picker_open,
            EdgeListSidebar {
                layout,
                remove_confirm,
            }
            EdgePickerOverlay {
                workspace: workspace.clone(),
                root_id: root_id.clone(),
                open: picker_open,
                fetch_trigger,
            }
            RemoveEdgeDialog {
                workspace,
                remove_confirm,
                fetch_trigger,
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn render_webgpu_graph(
    nav: Navigator,
    props: DepGraphProps,
    workspace: String,
    root_id: String,
    on_select: Option<EventHandler<TicketRef>>,
    on_hover: Option<EventHandler<Option<String>>>,
    on_deselect: Option<EventHandler<()>>,
    selected_node_id: Option<String>,
    hovered_node_id: Option<String>,
) -> Element {
    rsx! {
        crate::graph3d::Graph3D {
            key: "{root_id}",
            workspace: workspace.clone(),
            root_id: root_id.clone(),
            selected_node_id,
            hovered_node_id,
            layout_mode: props.layout_mode,
            projection: props.projection,
            on_layout_mode_change: props.on_layout_mode_change.clone(),
            on_projection_change: props.on_projection_change.clone(),
            on_deselect: move |_| {
                if let Some(ref handler) = on_deselect {
                    handler.call(());
                }
            },
            on_hover: move |id| {
                if let Some(ref handler) = on_hover {
                    handler.call(id);
                }
            },
            on_select: move |ticket_ref: TicketRef| {
                select_node_or_navigate(
                    on_select.clone(),
                    nav.clone(),
                    ticket_ref,
                )
            }
        }
    }
}
