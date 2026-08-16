use dioxus::prelude::*;

use crate::{
    types::{
        TicketRef,
        TicketSummary,
    },
};

use super::{
    facets::ticket_type,
    recent::save_recent,
};

pub(super) fn activate_search_result(
    ticket: TicketSummary,
    workspace: String,
    query: String,
    mut open: Signal<bool>,
    on_ticket_open: EventHandler<TicketRef>,
) {
    let trimmed = query.trim().to_string();
    if !trimmed.is_empty() {
        save_recent(&workspace, &trimmed);
    }
    on_ticket_open.call(ticket.resolved_ticket_ref(&workspace));
    open.set(false);
}

pub(super) fn render_search_results(
    filtered: &[TicketSummary],
    workspace: String,
    query: String,
    open: Signal<bool>,
    on_ticket_open: EventHandler<TicketRef>,
    hovered_result: Signal<Option<usize>>,
) -> Element {
    rsx! {
        for (index, ticket) in filtered.iter().enumerate() {
            {
                render_result_row(
                    ticket.clone(),
                    index,
                    workspace.clone(),
                    query.clone(),
                    open,
                    on_ticket_open.clone(),
                    hovered_result,
                )
            }
        }
    }
}

pub(super) fn render_empty_state(query_display: &str) -> Element {
    rsx! {
        div {
            style: "padding: 1.5rem 1rem; color: #9999bb; font-size: 0.9rem; text-align: center;",
            "No tickets found for \"{query_display}\"."
        }
    }
}

fn render_result_row(
    ticket: TicketSummary,
    index: usize,
    workspace: String,
    query: String,
    open: Signal<bool>,
    on_ticket_open: EventHandler<TicketRef>,
    mut hovered_result: Signal<Option<usize>>,
) -> Element {
    let id = ticket.id.clone();
    let title = ticket.title.as_deref().unwrap_or("(untitled)").to_string();
    let state = ticket.state.as_deref().unwrap_or("").to_string();
    let ticket_type = ticket_type(&ticket).to_string();
    let (state_bg, state_fg) = crate::types::state_colors(&state);
    let id_short = shorten_id(&id);
    let is_hovered = *hovered_result.read() == Some(index);
    let row_bg = if is_hovered {
        "rgba(80,80,160,0.2)"
    } else {
        "transparent"
    };

    rsx! {
        button {
            "data-testid": "search-result-{id}",
            aria_selected: if is_hovered { "true" } else { "false" },
            style: "
                display: flex;
                flex-direction: column;
                gap: 0.2rem;
                width: 100%;
                background: {row_bg};
                border: none;
                border-bottom: 1px solid rgba(100,100,200,0.1);
                padding: 0.6rem 1rem;
                color: #e0e0e8;
                cursor: pointer;
                text-align: left;
            ",
            onmouseenter: move |_| hovered_result.set(Some(index)),
            onmouseleave: move |_| hovered_result.set(None),
            onclick: move |_| {
                activate_search_result(
                    ticket.clone(),
                    workspace.clone(),
                    query.clone(),
                    open,
                    on_ticket_open.clone(),
                );
            },
            div {
                style: "
                    display: flex;
                    align-items: center;
                    gap: 0.5rem;
                    font-size: 0.9rem;
                    font-weight: 600;
                ",
                if !state.is_empty() {
                    span {
                        style: "
                            background: {state_bg};
                            color: {state_fg};
                            border-radius: 10px;
                            padding: 1px 8px;
                            font-size: 10px;
                            font-weight: 700;
                            white-space: nowrap;
                        ",
                        "{state}"
                    }
                }
                span { "{title}" }
            }
            div {
                style: "
                    display: flex;
                    align-items: center;
                    gap: 0.75rem;
                    font-size: 11px;
                    color: #7777aa;
                ",
                span { "{id_short}" }
                if !ticket_type.is_empty() {
                    span { "· {ticket_type}" }
                }
            }
        }
    }
}

fn shorten_id(id: &str) -> String {
    id.get(..8).unwrap_or(id).to_string()
}
