use dioxus::prelude::*;

#[component]
pub(super) fn StateBadge(state: String) -> Element {
    let (background, foreground) = crate::types::state_colors(&state);
    rsx! {
        span {
            style: "
                display: inline-block;
                padding: 3px 10px;
                border-radius: 12px;
                font-size: 11px;
                font-weight: 600;
                letter-spacing: 0.04em;
                background: {background};
                color: {foreground};
                white-space: nowrap;
            ",
            "{state}"
        }
    }
}

#[component]
pub(super) fn ConflictDialog(
    field_label: String,
    server_value: String,
    my_draft: String,
    on_discard: EventHandler<()>,
    on_keep: EventHandler<()>,
) -> Element {
    rsx! {
        div {
            style: "
                position: fixed; inset: 0; z-index: 1000;
                background: rgba(0,0,0,0.6);
                display: flex; align-items: center; justify-content: center;
            ",
            div {
                style: "
                    background: var(--bg-secondary); border: 1px solid var(--border-color);
                    border-radius: 8px; padding: 1.5rem; width: 360px;
                    color: var(--text-primary); font-family: var(--font-sans);
                ",
                h3 {
                    style: "margin: 0 0 0.75rem; font-size: 14px; color: var(--accent-yellow);",
                    "Concurrent edit conflict"
                }
                p {
                    style: "font-size: 13px; margin: 0 0 0.75rem;",
                    "\"",
                    "{field_label}",
                    "\" was updated on the server while you were editing."
                }
                div {
                    style: "
                        font-size: 12px; background: var(--bg-primary);
                        border-radius: 4px; padding: 0.5rem; margin-bottom: 1rem;
                    ",
                    p { style: "margin: 0 0 0.25rem; color: var(--text-secondary);", "Server value:" }
                    p { style: "margin: 0 0 0.5rem; font-weight: 600;", "{server_value}" }
                    p { style: "margin: 0 0 0.25rem; color: var(--text-secondary);", "Your draft:" }
                    p { style: "margin: 0; font-weight: 600;", "{my_draft}" }
                }
                div {
                    style: "display: flex; gap: 0.5rem; justify-content: flex-end;",
                    button {
                        class: "btn btn-secondary",
                        onclick: move |_| on_keep.call(()),
                        "Keep editing"
                    }
                    button {
                        style: "
                            padding: 6px 14px; border-radius: 4px; border: none;
                            background: var(--accent-red); color: white; cursor: pointer;
                            font-size: 12px;
                        ",
                        onclick: move |_| on_discard.call(()),
                        "Discard my changes"
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub(super) struct FieldRowProps {
    pub field_key: String,
    pub label: String,
    pub current_value: String,
    pub options: Vec<String>,
    pub is_editing: bool,
    pub draft: String,
    pub is_pending: bool,
    pub on_start_edit: EventHandler<String>,
    pub on_draft_change: EventHandler<String>,
    pub on_save: EventHandler<String>,
    pub on_cancel: EventHandler<()>,
}

#[component]
pub(super) fn FieldRow(props: FieldRowProps) -> Element {
    let FieldRowProps {
        field_key,
        label,
        current_value,
        options,
        is_editing,
        draft,
        is_pending,
        on_start_edit,
        on_draft_change,
        on_save,
        on_cancel,
    } = props;
    let display_opacity = if is_pending { "0.5" } else { "1" };

    rsx! {
        div {
            "data-testid": "ticket-detail-field-{field_key}",
            style: "margin-bottom: 0.5rem;",
            div {
                style: "
                    font-size: 10px; color: var(--text-muted); text-transform: uppercase;
                    letter-spacing: 0.05em; margin-bottom: 2px;
                ",
                "{label}"
            }
            if is_editing {
                if !options.is_empty() {
                    select {
                        style: "
                            width: 100%; background: var(--bg-primary); color: var(--text-primary);
                            border: 1px solid var(--border-color); border-radius: 4px;
                            padding: 4px 6px; font-size: 13px; cursor: pointer;
                        ",
                        disabled: is_pending,
                        onchange: move |event: Event<FormData>| {
                            on_save.call(event.value().to_string());
                        },
                        for option_value in options.iter() {
                            option {
                                key: "{option_value}",
                                value: "{option_value}",
                                selected: *option_value == current_value,
                                "{option_value}"
                            }
                        }
                    }
                } else {
                    div {
                        style: "display: flex; gap: 4px;",
                        input {
                            r#type: "text",
                            value: "{draft}",
                            autofocus: true,
                            style: "
                                flex: 1; background: var(--bg-primary); color: var(--text-primary);
                                border: 1px solid var(--border-color); border-radius: 4px;
                                padding: 4px 6px; font-size: 13px; min-width: 0;
                            ",
                            disabled: is_pending,
                            oninput: move |event| on_draft_change.call(event.value().to_string()),
                            onkeydown: {
                                let draft = draft.clone();
                                move |event: Event<KeyboardData>| match event.key() {
                                    Key::Enter => on_save.call(draft.clone()),
                                    Key::Escape => on_cancel.call(()),
                                    _ => {}
                                }
                            },
                        }
                        button {
                            class: "btn btn-primary btn-sm",
                            disabled: is_pending,
                            onclick: {
                                let draft = draft.clone();
                                move |_| on_save.call(draft.clone())
                            },
                            "✓"
                        }
                        button {
                            class: "btn btn-secondary btn-sm",
                            onclick: move |_| on_cancel.call(()),
                            "✕"
                        }
                    }
                }
            } else {
                div {
                    style: "
                        font-size: 13px; color: var(--text-primary);
                        padding: 4px 6px; border-radius: 4px;
                        border: 1px solid transparent; min-height: 26px;
                        cursor: pointer; opacity: {display_opacity};
                    ",
                    onclick: {
                        let current_value = current_value.clone();
                        let field_key = field_key.clone();
                        move |_| {
                            if !is_pending {
                                on_start_edit.call(field_key.clone());
                                on_draft_change.call(current_value.clone());
                            }
                        }
                    },
                    if current_value.is_empty() {
                        span { style: "color: var(--text-muted); font-style: italic;", "—" }
                    } else {
                        "{current_value}"
                    }
                }
            }
        }
    }
}
