use dioxus::prelude::*;
use viewer_api_dioxus::FilterToggleButton;

use crate::types::{
    TicketRef,
    TicketSummary,
};

use super::{
    TicketTreeProps,
    STATE_CHIPS,
};

pub(super) fn render_filter_controls(
    props: TicketTreeProps,
    state_filter_val: String,
    all_checked: bool,
) -> Element {
    rsx! {
        div {
            style: "
                padding: 0 12px 8px;
                border-bottom: 1px solid var(--border-subtle);
                display: flex;
                align-items: center;
                flex-wrap: wrap;
                gap: 4px;
            ",
            for &(label, value) in STATE_CHIPS.iter() {
                {
                    let is_active = state_filter_val.as_str() == value;
                    let value = value.to_string();
                    rsx! {
                        FilterToggleButton {
                            key: "{value}",
                            test_id: if value.is_empty() {
                                "ticket-tree-state-chip-all".to_string()
                            } else {
                                format!("ticket-tree-state-chip-{value}")
                            },
                            class: "chip".to_string(),
                            active_class: "chip--active".to_string(),
                            inactive_class: "chip--neutral".to_string(),
                            active: is_active,
                            onclick: move |_| props.on_state_filter_change.call(value.clone()),
                            "{label}"
                        }
                    }
                }
            }
            div { style: "flex: 1;" }
            if props.show_checkboxes {
                input {
                    r#type: "checkbox",
                    checked: all_checked,
                    style: "width: 14px; height: 14px; cursor: pointer; accent-color: var(--accent-blue); flex-shrink: 0;",
                    aria_label: "Select all tickets",
                    onchange: move |event| {
                        if let Some(ref handler) = props.on_select_all {
                            handler.call(event.value() == "true");
                        }
                    },
                }
            }
            if let Some(ref on_batch) = props.on_toggle_batch {
                button {
                    class: if props.show_checkboxes { "btn btn-secondary btn-sm btn-active" } else { "btn btn-secondary btn-sm" },
                    style: "font-size: 11px; padding: 3px 8px; min-height: 24px;",
                    aria_label: "Toggle batch selection",
                    aria_pressed: if props.show_checkboxes { "true" } else { "false" },
                    onclick: {
                        let on_batch = on_batch.clone();
                        move |_| on_batch.call(())
                    },
                    "☑"
                }
            }
            if let Some(ref on_new) = props.on_new_ticket {
                button {
                    class: "btn btn-primary btn-sm",
                    style: "font-size: 11px; padding: 3px 8px; min-height: 24px;",
                    aria_label: "Create new ticket",
                    onclick: {
                        let on_new = on_new.clone();
                        move |_| on_new.call(())
                    },
                    "+ New"
                }
            }
        }
    }
}

pub(super) fn active_ticket_id(
    displayed_ticket_ids: &[String],
    focused_ticket_id: Signal<Option<String>>,
    selected_id: Option<String>,
) -> Option<String> {
    let focused = focused_ticket_id.read().clone();
    focused
        .filter(|id| {
            displayed_ticket_ids.iter().any(|candidate| candidate == id)
        })
        .or_else(|| {
            selected_id.filter(|id| {
                displayed_ticket_ids.iter().any(|candidate| candidate == id)
            })
        })
        .or_else(|| displayed_ticket_ids.first().cloned())
}

pub(super) fn move_ticket_focus(
    displayed_ticket_ids: &[String],
    mut focused_ticket_id: Signal<Option<String>>,
    selected_id: Option<String>,
    delta: i32,
) {
    if displayed_ticket_ids.is_empty() {
        focused_ticket_id.set(None);
        return;
    }

    let current_id =
        active_ticket_id(displayed_ticket_ids, focused_ticket_id, selected_id);
    let current_index = current_id
        .as_ref()
        .and_then(|id| {
            displayed_ticket_ids
                .iter()
                .position(|candidate| candidate == id)
        })
        .unwrap_or(0);
    let next_index = if delta.is_negative() {
        current_index.saturating_sub(1)
    } else {
        (current_index + 1).min(displayed_ticket_ids.len().saturating_sub(1))
    };

    focused_ticket_id.set(Some(displayed_ticket_ids[next_index].clone()));
}

pub(super) fn resolve_ticket_ref(
    tickets: &[TicketSummary],
    workspace: &str,
    ticket_id: &str,
) -> TicketRef {
    tickets
        .iter()
        .find(|ticket| ticket.id == ticket_id)
        .map(|ticket| ticket.resolved_ticket_ref(workspace))
        .unwrap_or_else(|| TicketRef::new(workspace, ticket_id.to_string()))
}
