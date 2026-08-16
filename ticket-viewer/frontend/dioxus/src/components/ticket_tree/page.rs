use std::collections::{
    HashMap,
    HashSet,
};

use dioxus::prelude::*;
use viewer_api_dioxus::{
    ExplorerShell,
    SidebarSearch,
};

use crate::{
    search_syntax::{
        SEARCH_INPUT_PLACEHOLDER,
        SEARCH_SYNTAX_HINT,
    },
    types::TicketFileEntry,
};

use super::{
    header::render_filter_controls,
    rows::render_ticket_rows,
    TicketTreeProps,
};

#[component]
pub fn TicketTree(props: TicketTreeProps) -> Element {
    let filter = props.filter.clone();
    let state_filter_val = props.state_filter.clone();
    let expanded_ids: Signal<HashSet<String>> = use_signal(HashSet::new);
    let file_cache: Signal<HashMap<String, Vec<TicketFileEntry>>> =
        use_signal(HashMap::new);
    let loading_files: Signal<HashSet<String>> = use_signal(HashSet::new);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut focused_ticket_id: Signal<Option<String>> = use_signal(|| None);
    // Only set by keyboard navigation; the scroll effect subscribes to this
    // rather than to `focused_ticket_id` so that data refreshes and focus
    // initialisation never trigger a scroll jump.
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut keyboard_scroll_request: Signal<Option<String>> =
        use_signal(|| None);
    let sorted = sorted_tickets(props.tickets.clone(), &props.sort_key);
    let displayed_ticket_ids = sorted
        .iter()
        .map(|ticket| ticket.id.clone())
        .collect::<Vec<_>>();
    {
        let displayed_ticket_ids = displayed_ticket_ids.clone();
        let selected_id = props.selected_id.clone();
        use_effect(move || {
            let current_focus = focused_ticket_id.read().clone();
            let next_focus = resolve_focused_ticket(
                &displayed_ticket_ids,
                current_focus.as_deref(),
                selected_id.as_deref(),
            );
            if current_focus != next_focus {
                focused_ticket_id.set(next_focus);
            }
        });
    }
    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        let Some(ticket_id) = keyboard_scroll_request.read().clone() else {
            return;
        };

        let Some(document) =
            web_sys::window().and_then(|window| window.document())
        else {
            return;
        };

        let selector =
            format!(r#"button[data-testid="ticket-tree-ticket-{ticket_id}"]"#);
        if let Ok(Some(element)) = document.query_selector(&selector) {
            let mut opts = web_sys::ScrollIntoViewOptions::new();
            opts.block(web_sys::ScrollLogicalPosition::Nearest);
            element.scroll_into_view_with_scroll_into_view_options(&opts);
        }
    });
    let all_checked = props.show_checkboxes
        && !sorted.is_empty()
        && sorted
            .iter()
            .all(|ticket| props.selected_ids.contains(&ticket.id));
    let is_empty = props.tickets.is_empty();
    let displayed_ticket_ids_for_focus = displayed_ticket_ids.clone();
    let displayed_ticket_ids_for_keydown = displayed_ticket_ids.clone();
    let selected_id_for_focus = props.selected_id.clone();
    let selected_id_for_keydown = props.selected_id.clone();
    let mut keyboard_scroll_request_for_keydown = keyboard_scroll_request;
    let on_filter_change = props.on_filter_change.clone();
    let on_select_for_keydown = props.on_select.clone();
    let search_tickets = props.tickets.clone();
    let search_workspace = props.workspace.clone();
    let status = if props.loading {
        Some(rsx! {
            div {
                class: "sidebar-loading",
                "Loading tickets…"
            }
        })
    } else if let Some(ref error) = props.error {
        Some(rsx! {
            div {
                style: "padding: 12px; color: var(--error); font-size: 12px;",
                "Failed to load: {error}"
            }
        })
    } else if is_empty {
        Some(rsx! {
            div {
                class: "sidebar-empty",
                "No tickets in this workspace."
            }
        })
    } else {
        None
    };

    rsx! {
        ExplorerShell {
            search: Some(rsx! {
                SidebarSearch {
                    value: filter,
                    on_input: EventHandler::new(move |value: String| on_filter_change.call(value)),
                    placeholder: SEARCH_INPUT_PLACEHOLDER.to_string(),
                    hint: Some(SEARCH_SYNTAX_HINT.to_string()),
                    input_testid: Some("ticket-tree-filter".to_string()),
                    hint_testid: Some("ticket-tree-filter-hint".to_string()),
                    on_focus: Some(EventHandler::new(move |_| {
                        if let Some(ticket_id) = super::header::active_ticket_id(
                            &displayed_ticket_ids_for_focus,
                            focused_ticket_id,
                            selected_id_for_focus.clone(),
                        ) {
                            focused_ticket_id.set(Some(ticket_id));
                        }
                    })),
                    on_keydown: Some(EventHandler::new(move |event: KeyboardEvent| match event.key() {
                        Key::ArrowDown => {
                            event.prevent_default();
                            super::header::move_ticket_focus(
                                &displayed_ticket_ids_for_keydown,
                                focused_ticket_id,
                                selected_id_for_keydown.clone(),
                                1,
                            );
                            keyboard_scroll_request_for_keydown
                                .set(focused_ticket_id.read().clone());
                        },
                        Key::ArrowUp => {
                            event.prevent_default();
                            super::header::move_ticket_focus(
                                &displayed_ticket_ids_for_keydown,
                                focused_ticket_id,
                                selected_id_for_keydown.clone(),
                                -1,
                            );
                            keyboard_scroll_request_for_keydown
                                .set(focused_ticket_id.read().clone());
                        },
                        Key::Enter => {
                            if let Some(ticket_id) = super::header::active_ticket_id(
                                &displayed_ticket_ids_for_keydown,
                                focused_ticket_id,
                                selected_id_for_keydown.clone(),
                            ) {
                                event.prevent_default();
                                on_select_for_keydown.call(super::header::resolve_ticket_ref(
                                    &search_tickets,
                                    &search_workspace,
                                    &ticket_id,
                                ));
                            }
                        },
                        _ => {},
                    })),
                }
            }),
            controls: Some(render_filter_controls(
                props.clone(),
                state_filter_val,
                all_checked,
            )),
            status,
            body: if !props.loading && !is_empty {
                Some(rsx! {
                    div {
                        class: "file-tree ticket-tree-file-tree",
                        div {
                            class: "tree-view ticket-tree-scroll-region",
                            "data-testid": "ticket-tree-scroll-region",
                            {render_ticket_rows(
                                props,
                                sorted,
                                expanded_ids,
                                file_cache,
                                loading_files,
                                focused_ticket_id,
                            )}
                        }
                    }
                })
            } else {
                None
            },
        }
    }
}

fn sorted_tickets(
    mut tickets: Vec<crate::types::TicketSummary>,
    sort_key: &str,
) -> Vec<crate::types::TicketSummary> {
    match sort_key {
        "title" => tickets.sort_by(|a, b| a.title.cmp(&b.title)),
        "state" => tickets.sort_by(|a, b| a.state.cmp(&b.state)),
        "created_at" => tickets.sort_by(|a, b| b.created_at.cmp(&a.created_at)),
        _ => tickets.sort_by(|a, b| b.updated_at.cmp(&a.updated_at)),
    }
    tickets
}

fn resolve_focused_ticket(
    displayed_ticket_ids: &[String],
    current_focus: Option<&str>,
    selected_id: Option<&str>,
) -> Option<String> {
    if displayed_ticket_ids.is_empty() {
        return None;
    }

    current_focus
        .filter(|id| {
            displayed_ticket_ids.iter().any(|candidate| candidate == id)
        })
        .map(ToString::to_string)
        .or_else(|| {
            selected_id
                .filter(|id| {
                    displayed_ticket_ids.iter().any(|candidate| candidate == id)
                })
                .map(ToString::to_string)
        })
        .or_else(|| displayed_ticket_ids.first().cloned())
}
