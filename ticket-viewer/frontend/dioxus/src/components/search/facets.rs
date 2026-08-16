use std::collections::HashSet;

use dioxus::prelude::*;

use crate::types::TicketSummary;

pub(super) fn ticket_state(ticket: &TicketSummary) -> &str {
    ticket.state.as_deref().unwrap_or("")
}

pub(super) fn ticket_type(ticket: &TicketSummary) -> &str {
    ticket.ticket_type.as_deref().unwrap_or("")
}

pub(super) fn unique_values<F: Fn(&TicketSummary) -> &str>(
    items: &[TicketSummary],
    key_fn: F,
) -> Vec<String> {
    let mut seen = HashSet::new();
    items
        .iter()
        .map(key_fn)
        .filter(|value| !value.is_empty())
        .filter(|value| seen.insert(value.to_string()))
        .map(str::to_string)
        .collect()
}

pub(super) fn render_facet_chips(
    state_facets: &[String],
    type_facets: &[String],
    state_filter: Signal<Option<String>>,
    type_filter: Signal<Option<String>>,
) -> Element {
    rsx! {
        div {
            style: "
                display: flex;
                flex-wrap: wrap;
                gap: 0.4rem;
                padding: 0.5rem 1rem;
                border-bottom: 1px solid rgba(100, 100, 200, 0.15);
            ",
            for state in state_facets.iter() {
                {render_state_chip(state.clone(), state_filter)}
            }
            for ticket_type in type_facets.iter() {
                {render_type_chip(ticket_type.clone(), type_filter)}
            }
        }
    }
}

fn render_state_chip(
    state: String,
    mut state_filter: Signal<Option<String>>,
) -> Element {
    let active = *state_filter.read() == Some(state.clone());
    let (bg, fg) = crate::types::state_colors(&state);
    let chip_border = if active {
        "2px solid #8080ff"
    } else {
        "1px solid rgba(180,180,255,0.2)"
    };

    rsx! {
        button {
            style: "
                background: {bg};
                color: {fg};
                border: {chip_border};
                border-radius: 20px;
                padding: 2px 10px;
                font-size: 11px;
                font-weight: 600;
                cursor: pointer;
                letter-spacing: 0.03em;
            ",
            onclick: move |_| {
                if *state_filter.read() == Some(state.clone()) {
                    state_filter.set(None);
                } else {
                    state_filter.set(Some(state.clone()));
                }
            },
            "{state}"
        }
    }
}

fn render_type_chip(
    ticket_type: String,
    mut type_filter: Signal<Option<String>>,
) -> Element {
    let active = *type_filter.read() == Some(ticket_type.clone());
    let chip_bg = if active { "#2d2d5a" } else { "#252540" };
    let chip_border = if active {
        "2px solid #8080ff"
    } else {
        "1px solid rgba(180,180,255,0.2)"
    };

    rsx! {
        button {
            style: "
                background: {chip_bg};
                color: #b0b0e8;
                border: {chip_border};
                border-radius: 20px;
                padding: 2px 10px;
                font-size: 11px;
                font-weight: 600;
                cursor: pointer;
                letter-spacing: 0.03em;
            ",
            onclick: move |_| {
                if *type_filter.read() == Some(ticket_type.clone()) {
                    type_filter.set(None);
                } else {
                    type_filter.set(Some(ticket_type.clone()));
                }
            },
            "{ticket_type}"
        }
    }
}
