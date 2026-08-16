use dioxus::prelude::*;

use crate::{
    api::{
        HttpTicketBackend,
        TicketBackend,
    },
    layout::GraphLayout,
    types::EdgeMutationBody,
};

use super::state::RemoveEdge;

#[component]
pub(super) fn EdgeListSidebar(
    layout: Signal<Option<GraphLayout>>,
    mut remove_confirm: Signal<Option<RemoveEdge>>,
) -> Element {
    let layout_read = layout.read();
    let Some(layout) = layout_read.as_ref() else {
        return rsx! {};
    };
    if layout.edges.is_empty() {
        return rsx! {};
    }

    rsx! {
        div {
            style: "
                position: absolute; top: 12px; right: 12px;
                max-width: 280px; max-height: 220px;
                background: rgba(18,18,30,0.92);
                border: 1px solid rgba(200,200,220,0.18);
                border-radius: 6px;
                overflow-y: auto;
                pointer-events: auto;
                font-family: sans-serif;
            ",
            div {
                style: "
                    padding: 5px 8px;
                    font-size: 10px; font-weight: 700;
                    color: rgba(180,180,210,0.65);
                    text-transform: uppercase; letter-spacing: 0.6px;
                    border-bottom: 1px solid rgba(200,200,220,0.12);
                    background: rgba(28,28,46,0.6);
                ",
                "Dependencies"
            }
            for edge in layout.edges.iter() {
                {
                    let from = edge.from.clone();
                    let to = edge.to.clone();
                    let kind = edge.kind.clone();
                    let from_title = layout
                        .nodes
                        .iter()
                        .find(|node| node.id == from)
                        .and_then(|node| node.title.as_deref())
                        .unwrap_or(&from)
                        .to_string();
                    let to_title = layout
                        .nodes
                        .iter()
                        .find(|node| node.id == to)
                        .and_then(|node| node.title.as_deref())
                        .unwrap_or(&to)
                        .to_string();
                    let remove_edge = RemoveEdge {
                        from_id: from.clone(),
                        to_id: to.clone(),
                        kind: kind.clone(),
                        from_title: from_title.clone(),
                        to_title: to_title.clone(),
                    };
                    rsx! {
                        div {
                            key: "{from}-{to}-{kind}",
                            style: "
                                display: flex; align-items: center; gap: 6px;
                                padding: 4px 8px;
                                border-bottom: 1px solid rgba(200,200,220,0.08);
                                font-size: 11px; color: #b0b0c8;
                            ",
                            span {
                                style: "
                                    flex: 1;
                                    overflow: hidden;
                                    text-overflow: ellipsis;
                                    white-space: nowrap;
                                ",
                                title: "{from_title} → {to_title}",
                                "{from_title} → {to_title}"
                            }
                            span {
                                style: "
                                    background: rgba(100,100,200,0.25);
                                    border-radius: 3px; padding: 1px 4px;
                                    font-size: 9px; color: #8080b8;
                                    flex-shrink: 0;
                                ",
                                "{kind}"
                            }
                            button {
                                style: "
                                    background: none; border: none;
                                    color: #ef4444; cursor: pointer;
                                    font-size: 14px; padding: 0 2px;
                                    line-height: 1; flex-shrink: 0;
                                ",
                                title: "Remove this edge",
                                onclick: move |_| remove_confirm.set(Some(remove_edge.clone())),
                                "×"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub(super) fn RemoveEdgeDialog(
    workspace: String,
    mut remove_confirm: Signal<Option<RemoveEdge>>,
    mut fetch_trigger: Signal<u32>,
) -> Element {
    let Some(edge) = remove_confirm.read().clone() else {
        return rsx! {};
    };

    rsx! {
        div {
            style: "
                position: absolute; inset: 0;
                background: rgba(0,0,0,0.70);
                display: flex; align-items: center; justify-content: center;
                pointer-events: auto;
                z-index: 60;
            ",
            div {
                style: "
                    background: #15152a;
                    border: 1px solid rgba(239,68,68,0.3);
                    border-radius: 10px; padding: 20px;
                    width: 360px; max-width: 92%;
                    font-family: sans-serif;
                    display: flex; flex-direction: column; gap: 12px;
                ",
                h3 {
                    style: "margin: 0; color: #e0e0f0; font-size: 15px;",
                    "Remove dependency?"
                }
                p {
                    style: "margin: 0; color: #9ca3af; font-size: 13px; line-height: 1.5;",
                    "Remove the \""
                    span { style: "color: #c0c0e0; font-weight: 600;", "{edge.kind}" }
                    "\" edge from \""
                    span { style: "color: #c0c0e0;", "{edge.from_title}" }
                    "\" to \""
                    span { style: "color: #c0c0e0;", "{edge.to_title}" }
                    "\"?"
                }
                div {
                    style: "display: flex; gap: 8px; justify-content: flex-end;",
                    button {
                        style: "
                            padding: 7px 16px; border-radius: 5px;
                            border: 1px solid rgba(200,200,220,0.2);
                            background: rgba(40,40,60,0.8);
                            color: #c0c0d8; font-size: 13px; cursor: pointer;
                        ",
                        onclick: move |_| remove_confirm.set(None),
                        "Cancel"
                    }
                    button {
                        style: "
                            padding: 7px 16px; border-radius: 5px; border: none;
                            background: rgba(239,68,68,0.8);
                            color: #fff; font-size: 13px; cursor: pointer;
                        ",
                        onclick: {
                            let workspace = workspace.clone();
                            let edge = edge.clone();
                            move |_| confirm_remove_edge(
                                workspace.clone(),
                                edge.clone(),
                                remove_confirm,
                                fetch_trigger,
                            )
                        },
                        "Remove"
                    }
                }
            }
        }
    }
}

fn confirm_remove_edge(
    workspace: String,
    edge: RemoveEdge,
    mut remove_confirm: Signal<Option<RemoveEdge>>,
    mut fetch_trigger: Signal<u32>,
) {
    spawn(async move {
        let backend = HttpTicketBackend::new(None);
        let body = EdgeMutationBody {
            from_id: edge.from_id,
            to_id: edge.to_id,
            kind: edge.kind,
            reason: None,
        };
        let _ = backend.delete_edge(&workspace, &body).await;
        remove_confirm.set(None);
        fetch_trigger.with_mut(|value| *value += 1);
    });
}
