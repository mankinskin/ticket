use dioxus::prelude::*;

use crate::{
    api::{
        HttpTicketBackend,
        TicketBackend,
    },
    types::{
        EdgeMutationBody,
        TicketSummary,
    },
};

#[component]
pub(super) fn EdgePickerOverlay(
    workspace: String,
    root_id: String,
    mut open: Signal<bool>,
    mut fetch_trigger: Signal<u32>,
) -> Element {
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut picker_query: Signal<String> = use_signal(String::new);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut picker_results: Signal<Vec<TicketSummary>> = use_signal(Vec::new);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut picker_selected_id: Signal<Option<String>> = use_signal(|| None);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut picker_kind: Signal<String> =
        use_signal(|| "depends_on".to_string());
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut picker_reason: Signal<String> = use_signal(String::new);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut picker_error: Signal<Option<String>> = use_signal(|| None);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut picker_searching: Signal<bool> = use_signal(|| false);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut debounce_gen: Signal<u32> = use_signal(|| 0u32);

    use_effect(move || {
        let is_open = open();
        if !is_open {
            return;
        }
        reset_picker_state(
            picker_query,
            picker_results,
            picker_selected_id,
            picker_kind,
            picker_reason,
            picker_error,
            picker_searching,
            debounce_gen,
        );
    });

    if !open() {
        return rsx! {};
    }

    rsx! {
        div {
            style: "
                position: absolute; inset: 0;
                background: rgba(0,0,0,0.65);
                display: flex; align-items: center; justify-content: center;
                pointer-events: auto;
                z-index: 50;
            ",
            onclick: move |_| open.set(false),
            div {
                style: "
                    background: #15152a;
                    border: 1px solid rgba(200,200,220,0.25);
                    border-radius: 10px;
                    padding: 20px;
                    width: 420px; max-width: 92%;
                    display: flex; flex-direction: column;
                    gap: 12px;
                    font-family: sans-serif;
                ",
                onclick: move |event| event.stop_propagation(),
                h3 {
                    style: "margin: 0; color: #e0e0f0; font-size: 15px;",
                    "Add dependency"
                }
                input {
                    r#type: "text",
                    placeholder: "Search tickets…",
                    value: "{picker_query}",
                    oninput: {
                        let workspace = workspace.clone();
                        move |event: Event<FormData>| {
                            handle_picker_input(
                                event.value(),
                                workspace.clone(),
                                picker_query,
                                picker_results,
                                picker_selected_id,
                                picker_error,
                                picker_searching,
                                debounce_gen,
                            )
                        }
                    },
                    autofocus: true,
                    style: "
                        width: 100%; box-sizing: border-box;
                        background: #0e0e1e;
                        border: 1px solid rgba(200,200,220,0.25);
                        border-radius: 5px;
                        padding: 7px 10px;
                        color: #e0e0f0; font-size: 13px;
                        outline: none;
                    ",
                }
                div {
                    style: "
                        max-height: 180px; overflow-y: auto;
                        border: 1px solid rgba(200,200,220,0.12);
                        border-radius: 5px;
                        background: #0e0e1e;
                    ",
                    if *picker_searching.read() {
                        div {
                            style: "padding: 10px; color: #9ca3af; font-size: 13px;",
                            "Searching…"
                        }
                    }
                    if !*picker_searching.read() && picker_results.read().is_empty() && !picker_query.read().is_empty() {
                        div {
                            style: "padding: 10px; color: #9ca3af; font-size: 13px;",
                            "No tickets found."
                        }
                    }
                    for result in picker_results.read().clone().into_iter() {
                        {
                            let id = result.id.clone();
                            let title = result.title.clone().unwrap_or_else(|| id.clone());
                            let state_label = result.state.clone().unwrap_or_default();
                            let is_selected = picker_selected_id.read().as_deref() == Some(id.as_str());
                            rsx! {
                                button {
                                    key: "{id}",
                                    style: if is_selected {
                                        "
                                            display: flex; align-items: center; gap: 8px;
                                            width: 100%; text-align: left;
                                            padding: 7px 10px; border: none; cursor: pointer;
                                            background: rgba(59,130,246,0.25);
                                            color: #dde8ff; font-size: 12px;
                                        "
                                    } else {
                                        "
                                            display: flex; align-items: center; gap: 8px;
                                            width: 100%; text-align: left;
                                            padding: 7px 10px; border: none; cursor: pointer;
                                            background: transparent;
                                            color: #c8c8e0; font-size: 12px;
                                        "
                                    },
                                    onclick: move |_| picker_selected_id.set(Some(id.clone())),
                                    span {
                                        style: "flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                                        "{title}"
                                    }
                                    if !state_label.is_empty() {
                                        span {
                                            style: "font-size: 10px; color: #7878a0; flex-shrink: 0;",
                                            "{state_label}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                div {
                    style: "display: flex; flex-direction: column; gap: 4px;",
                    label {
                        style: "font-size: 11px; color: #9090b8; text-transform: uppercase; letter-spacing: 0.5px;",
                        "Edge type"
                    }
                    select {
                        style: "
                            background: #0e0e1e;
                            border: 1px solid rgba(200,200,220,0.25);
                            border-radius: 5px; padding: 6px 8px;
                            color: #e0e0f0; font-size: 13px;
                        ",
                        onchange: move |event| picker_kind.set(event.value()),
                        option { value: "depends_on", "depends_on" }
                        option { value: "linked", "linked" }
                    }
                }
                div {
                    style: "display: flex; flex-direction: column; gap: 4px;",
                    label {
                        style: "font-size: 11px; color: #9090b8; text-transform: uppercase; letter-spacing: 0.5px;",
                        "Reason (optional)"
                    }
                    textarea {
                        rows: "2",
                        placeholder: "Why does this dependency exist?",
                        value: "{picker_reason}",
                        oninput: move |event| picker_reason.set(event.value()),
                        style: "
                            background: #0e0e1e;
                            border: 1px solid rgba(200,200,220,0.25);
                            border-radius: 5px; padding: 6px 8px;
                            color: #e0e0f0; font-size: 13px;
                            resize: vertical; min-height: 48px;
                            font-family: sans-serif;
                        ",
                    }
                }
                if let Some(error) = picker_error.read().clone() {
                    div {
                        style: "
                            background: rgba(239,68,68,0.12);
                            border: 1px solid rgba(239,68,68,0.45);
                            border-radius: 5px; padding: 8px 12px;
                            color: #ef4444; font-size: 13px;
                        ",
                        "{error}"
                    }
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
                        onclick: move |_| open.set(false),
                        "Cancel"
                    }
                    button {
                        style: if picker_selected_id.read().is_some() {
                            "
                                padding: 7px 16px; border-radius: 5px; border: none;
                                background: rgba(59,130,246,0.85);
                                color: #fff; font-size: 13px; cursor: pointer;
                            "
                        } else {
                            "
                                padding: 7px 16px; border-radius: 5px; border: none;
                                background: rgba(59,130,246,0.3);
                                color: rgba(255,255,255,0.4); font-size: 13px;
                                cursor: not-allowed;
                            "
                        },
                        disabled: picker_selected_id.read().is_none(),
                        onclick: {
                            let workspace = workspace.clone();
                            let root_id = root_id.clone();
                            move |_| confirm_add_edge(
                                workspace.clone(),
                                root_id.clone(),
                                picker_selected_id.read().clone(),
                                picker_kind.read().clone(),
                                picker_reason.read().clone(),
                                picker_error,
                                open,
                                fetch_trigger,
                            )
                        },
                        "Add"
                    }
                }
            }
        }
    }
}

fn reset_picker_state(
    mut picker_query: Signal<String>,
    mut picker_results: Signal<Vec<TicketSummary>>,
    mut picker_selected_id: Signal<Option<String>>,
    mut picker_kind: Signal<String>,
    mut picker_reason: Signal<String>,
    mut picker_error: Signal<Option<String>>,
    mut picker_searching: Signal<bool>,
    mut debounce_gen: Signal<u32>,
) {
    picker_query.set(String::new());
    picker_results.set(Vec::new());
    picker_selected_id.set(None);
    picker_kind.set("depends_on".to_string());
    picker_reason.set(String::new());
    picker_error.set(None);
    picker_searching.set(false);
    debounce_gen.set(0);
}

fn handle_picker_input(
    query: String,
    workspace: String,
    mut picker_query: Signal<String>,
    mut picker_results: Signal<Vec<TicketSummary>>,
    mut picker_selected_id: Signal<Option<String>>,
    mut picker_error: Signal<Option<String>>,
    mut picker_searching: Signal<bool>,
    mut debounce_gen: Signal<u32>,
) {
    picker_query.set(query.clone());
    picker_selected_id.set(None);
    picker_error.set(None);

    let generation = *debounce_gen.read() + 1;
    debounce_gen.set(generation);

    if query.trim().is_empty() {
        picker_results.set(Vec::new());
        picker_searching.set(false);
        return;
    }

    spawn(async move {
        let promise = js_sys::Promise::new(&mut |resolve, _| {
            web_sys::window()
                .unwrap()
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    &resolve, 300,
                )
                .unwrap();
        });
        let _ = wasm_bindgen_futures::JsFuture::from(promise).await;

        if *debounce_gen.read() != generation {
            return;
        }

        picker_searching.set(true);
        let backend = HttpTicketBackend::new(None);
        if let Ok(response) = backend
            .list_tickets(&workspace, None, Some(&query), Some(20))
            .await
        {
            picker_results.set(response.items);
        }
        picker_searching.set(false);
    });
}

fn confirm_add_edge(
    workspace: String,
    root_id: String,
    selected_id: Option<String>,
    kind: String,
    reason_text: String,
    mut picker_error: Signal<Option<String>>,
    mut open: Signal<bool>,
    mut fetch_trigger: Signal<u32>,
) {
    let Some(to_id) = selected_id else {
        return;
    };
    let reason = if reason_text.trim().is_empty() {
        None
    } else {
        Some(reason_text)
    };

    spawn(async move {
        let backend = HttpTicketBackend::new(None);
        let body = EdgeMutationBody {
            from_id: root_id,
            to_id,
            kind,
            reason,
        };
        match backend.create_edge(&workspace, &body).await {
            Ok(()) => {
                open.set(false);
                fetch_trigger.with_mut(|value| *value += 1);
            },
            Err(error) => {
                picker_error.set(Some(if error.starts_with("cycle_detected:") {
                    "Adding this edge would create a dependency cycle. Choose a different target."
                        .to_string()
                } else {
                    error
                }));
            },
        }
    });
}
