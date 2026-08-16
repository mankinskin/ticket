use dioxus::prelude::*;

use super::widgets::{
    ConflictDialog,
    FieldRow,
    StateBadge,
};
use crate::components::ticket_detail::model::{
    field_bool,
    field_str,
    static_options,
    ConflictState,
    STRING_FIELDS,
};

#[derive(Clone)]
pub(crate) struct TicketDetailViewState {
    pub fields: serde_json::Value,
    pub load_error: Option<String>,
    pub save_error: Option<String>,
    pub current_editing: Option<String>,
    pub current_draft: String,
    pub save_pending: bool,
    pub current_state: String,
    pub valid_next_states: Vec<String>,
    pub missing_required_states: Vec<String>,
    pub terminal_states: Vec<String>,
    pub transition_pending: bool,
    pub transition_error: Option<String>,
    pub conflict: Option<ConflictState>,
    pub feedback_pending: bool,
    pub feedback_error: Option<String>,
}

#[derive(Clone, Copy)]
pub(crate) struct TicketDetailHandlers {
    pub on_discard: EventHandler<()>,
    pub on_keep: EventHandler<()>,
    pub on_start_edit: EventHandler<String>,
    pub on_draft_change: EventHandler<String>,
    pub on_save_field: EventHandler<(String, String)>,
    pub on_save_bool: EventHandler<(String, bool)>,
    pub on_cancel_edit: EventHandler<()>,
    pub on_transition: EventHandler<String>,
    pub on_undo: EventHandler<()>,
    pub on_feedback: EventHandler<String>,
}

pub(crate) fn render_ticket_detail(
    view: TicketDetailViewState,
    handlers: TicketDetailHandlers,
) -> Element {
    rsx! {
        {render_conflict_dialog(view.conflict.clone(), handlers.on_discard, handlers.on_keep)}
        div {
            "data-testid": "ticket-detail-panel",
            style: "
                width: 100%; min-width: 0; height: 100%;
                overflow-y: auto; overflow-x: hidden;
                background: var(--panel-bg-strong);
                backdrop-filter: blur(var(--panel-blur)) saturate(var(--panel-saturate));
                -webkit-backdrop-filter: blur(var(--panel-blur)) saturate(var(--panel-saturate));
                border-left: 1px solid var(--border-color);
                color: var(--text-primary);
                padding: 1rem;
                font-family: var(--font-sans);
                box-sizing: border-box;
                flex-shrink: 0;
            ",
            {render_error_banner(view.load_error.as_ref(), "Error")}
            {render_save_error(view.save_error.as_ref())}
            {render_transition_section(&view, handlers.on_transition, handlers.on_undo)}
            {render_feedback_section(&view, handlers.on_feedback)}
            {render_string_fields(&view, handlers)}
            {render_bootstrap_blocker(&view, handlers.on_save_bool)}
        }
    }
}

fn render_conflict_dialog(
    conflict: Option<ConflictState>,
    on_discard: EventHandler<()>,
    on_keep: EventHandler<()>,
) -> Element {
    let Some((field_key, server_value, my_draft)) = conflict else {
        return rsx! { Fragment {} };
    };

    rsx! {
        ConflictDialog {
            field_label: field_key,
            server_value,
            my_draft,
            on_discard,
            on_keep,
        }
    }
}

fn render_error_banner(
    error: Option<&String>,
    prefix: &str,
) -> Element {
    let Some(error) = error else {
        return rsx! { Fragment {} };
    };

    rsx! {
        p {
            style: "color: var(--accent-red); font-size: 12px;",
            "{prefix}: {error}"
        }
    }
}

fn render_save_error(error: Option<&String>) -> Element {
    let Some(error) = error else {
        return rsx! { Fragment {} };
    };

    rsx! {
        div {
            style: "
                background: color-mix(in srgb, var(--accent-red) 15%, transparent);
                border: 1px solid var(--accent-red);
                border-radius: 4px; padding: 0.5rem; margin-bottom: 0.75rem;
                font-size: 12px; color: var(--accent-red);
            ",
            "Save failed: {error}"
        }
    }
}

fn render_transition_section(
    view: &TicketDetailViewState,
    on_transition: EventHandler<String>,
    on_undo: EventHandler<()>,
) -> Element {
    let transition_pending = view.transition_pending;

    rsx! {
        div {
            style: "
                margin-bottom: 1rem;
                padding-bottom: 0.75rem;
                border-bottom: 1px solid var(--border-color);
            ",
            div {
                style: "margin-bottom: 0.75rem;",
                div {
                    style: "
                        font-size: 10px; color: var(--text-muted);
                        text-transform: uppercase; letter-spacing: 0.05em;
                        margin-bottom: 4px;
                    ",
                    "State"
                }
                StateBadge { state: view.current_state.clone() }
            }
            if !view.valid_next_states.is_empty() {
                div {
                    style: "margin-bottom: 0.5rem;",
                    div {
                        style: "
                            font-size: 10px; color: var(--text-muted);
                            text-transform: uppercase; letter-spacing: 0.05em;
                            margin-bottom: 4px;
                        ",
                        "Advance to"
                    }
                    div {
                        style: "display: flex; flex-wrap: wrap; gap: 4px;",
                        for next_state in view.valid_next_states.iter() {
                            {
                                let next = next_state.clone();
                                let is_terminal = view.terminal_states.contains(&next);
                                let is_blocked =
                                    is_terminal && !view.missing_required_states.is_empty();
                                let tooltip = if is_blocked {
                                    format!(
                                        "Must visit first: {}",
                                        view.missing_required_states.join(", ")
                                    )
                                } else {
                                    String::new()
                                };
                                let button_class = if is_blocked {
                                    "chip chip--neutral"
                                } else {
                                    "chip chip--feature chip-button"
                                };
                                let extra_style = if is_blocked {
                                    "opacity: 0.5; cursor: not-allowed;"
                                } else {
                                    "cursor: pointer;"
                                };
                                let on_transition = on_transition;
                                rsx! {
                                    button {
                                        key: "{next}",
                                        class: "{button_class}",
                                        style: "{extra_style}",
                                        title: "{tooltip}",
                                        disabled: is_blocked || transition_pending,
                                        onclick: move |_| {
                                            if !is_blocked && !transition_pending {
                                                on_transition.call(next.clone());
                                            }
                                        },
                                        "{next_state}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
            div {
                style: "margin-bottom: 0.25rem;",
                button {
                    class: "btn btn-ghost btn-sm",
                    disabled: transition_pending,
                    onclick: move |_| on_undo.call(()),
                    "↩ Undo"
                }
            }
            if let Some(error) = view.transition_error.as_ref() {
                div {
                    style: "
                        background: color-mix(in srgb, var(--accent-red) 15%, transparent);
                        border: 1px solid var(--accent-red);
                        border-radius: 4px; padding: 0.5rem;
                        margin-top: 0.4rem;
                        font-size: 12px; color: var(--accent-red);
                    ",
                    "Transition failed: {error}"
                }
            }
        }
    }
}

fn render_string_fields(
    view: &TicketDetailViewState,
    handlers: TicketDetailHandlers,
) -> Element {
    rsx! {
        for (field_key, label) in STRING_FIELDS.iter() {
            {
                let field_key = field_key.to_string();
                let label = label.to_string();
                let current_value = field_str(&view.fields, &field_key);
                let is_editing = view.current_editing.as_deref() == Some(&field_key);
                let options = static_options(&field_key)
                    .iter()
                    .map(|value| value.to_string())
                    .collect();
                let on_save_field = handlers.on_save_field;
                rsx! {
                    FieldRow {
                        key: "{field_key}",
                        field_key: field_key.clone(),
                        label,
                        current_value: current_value.clone(),
                        options,
                        is_editing,
                        draft: if is_editing {
                            view.current_draft.clone()
                        } else {
                            current_value.clone()
                        },
                        is_pending: view.save_pending,
                        on_start_edit: handlers.on_start_edit,
                        on_draft_change: handlers.on_draft_change,
                        on_save: EventHandler::new(move |value: String| {
                            on_save_field.call((field_key.clone(), value));
                        }),
                        on_cancel: handlers.on_cancel_edit,
                    }
                }
            }
        }
    }
}

fn render_feedback_section(
    view: &TicketDetailViewState,
    on_feedback: EventHandler<String>,
) -> Element {
    rsx! {
        div {
            style: "
                margin-bottom: 1rem;
                padding-bottom: 0.75rem;
                border-bottom: 1px solid var(--border-color);
            ",
            div {
                style: "
                    font-size: 10px; color: var(--text-muted);
                    text-transform: uppercase; letter-spacing: 0.05em;
                    margin-bottom: 4px;
                ",
                "Feedback"
            }
            div {
                style: "display: flex; flex-wrap: wrap; gap: 6px; margin-bottom: 0.4rem;",
                button {
                    class: "chip chip--feature chip-button",
                    disabled: view.feedback_pending,
                    onclick: move |_| on_feedback.call("helpful".to_string()),
                    "Mark helpful"
                }
                button {
                    class: "chip chip--neutral chip-button",
                    disabled: view.feedback_pending,
                    onclick: move |_| on_feedback.call("mixed".to_string()),
                    "Needs follow-up"
                }
            }
            if let Some(error) = view.feedback_error.as_ref() {
                div {
                    style: "
                        background: color-mix(in srgb, var(--accent-red) 15%, transparent);
                        border: 1px solid var(--accent-red);
                        border-radius: 4px; padding: 0.5rem;
                        font-size: 12px; color: var(--accent-red);
                    ",
                    "Feedback failed: {error}"
                }
            }
        }
    }
}

fn render_bootstrap_blocker(
    view: &TicketDetailViewState,
    on_save_bool: EventHandler<(String, bool)>,
) -> Element {
    let blocker = field_bool(&view.fields, "bootstrap_blocker");

    rsx! {
        div {
            style: "margin-top: 0.5rem;",
            label {
                style: "
                    display: flex; align-items: center; gap: 0.5rem;
                    cursor: pointer; font-size: 12px; color: var(--text-primary);
                ",
                input {
                    r#type: "checkbox",
                    checked: blocker,
                    disabled: view.save_pending,
                    onchange: move |event: Event<FormData>| {
                        on_save_bool.call((
                            "bootstrap_blocker".to_string(),
                            event.value() == "true",
                        ));
                    },
                }
                "Bootstrap Blocker"
            }
        }
    }
}
