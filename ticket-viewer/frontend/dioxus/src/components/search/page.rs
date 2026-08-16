use dioxus::prelude::*;
use gloo_events::EventListener;
use wasm_bindgen::JsCast as _;

use crate::{
    api::{
        HttpTicketBackend,
        TicketBackend,
    },
    search_syntax::{
        SEARCH_INPUT_PLACEHOLDER,
        SEARCH_SYNTAX_HINT,
    },
    types::{
        TicketRef,
        TicketSummary,
    },
};

use super::{
    facets::{
        render_facet_chips,
        ticket_state,
        ticket_type,
        unique_values,
    },
    recent::load_recent,
    results::{
        activate_search_result,
        render_empty_state,
        render_search_results,
    },
};

const MAX_RESULTS: usize = 20;

#[derive(Props, Clone, PartialEq)]
pub struct SearchBarProps {
    pub workspace: String,
    pub on_ticket_open: EventHandler<TicketRef>,
}

#[component]
pub fn SearchBar(props: SearchBarProps) -> Element {
    let workspace = props.workspace.clone();
    let on_ticket_open = props.on_ticket_open.clone();
    let mut open: Signal<bool> = use_signal(|| false);
    let mut query: Signal<String> = use_signal(String::new);
    let results: Signal<Vec<TicketSummary>> = use_signal(Vec::new);
    let loading: Signal<bool> = use_signal(|| false);
    let search_err: Signal<Option<String>> = use_signal(|| None);
    let mut state_filter: Signal<Option<String>> = use_signal(|| None);
    let mut type_filter: Signal<Option<String>> = use_signal(|| None);
    let recents: Signal<Vec<String>> = use_signal(|| load_recent(&workspace));
    let mut hovered_recent: Signal<Option<usize>> = use_signal(|| None);
    let mut hovered_result: Signal<Option<usize>> = use_signal(|| None);
    let keydown_listener: Signal<Option<EventListener>> = use_signal(|| None);

    use_open_shortcut_listener(
        workspace.clone(),
        open,
        query,
        results,
        recents,
        state_filter,
        type_filter,
        keydown_listener,
    );
    use_search_results(workspace.clone(), query, results, loading, search_err);

    if !*open.read() {
        return rsx! {};
    }

    let all_results = results.read().clone();
    let filtered = filtered_results(&all_results, state_filter, type_filter);
    let state_facets = unique_values(&all_results, ticket_state);
    let type_facets = unique_values(&all_results, ticket_type);
    let recent_items = recents.read().clone();
    let q_display = query.read().clone();
    let is_empty_query = q_display.trim().is_empty();
    let filtered_len = filtered.len();
    let recent_len = recent_items.len();

    use_effect(move || {
        if is_empty_query {
            let current_recent = *hovered_recent.read();
            let next_recent = normalize_hover_index(current_recent, recent_len);
            if current_recent != next_recent {
                hovered_recent.set(next_recent);
            }
            if hovered_result.read().is_some() {
                hovered_result.set(None);
            }
        } else {
            let current_result = *hovered_result.read();
            let next_result =
                normalize_hover_index(current_result, filtered_len);
            if current_result != next_result {
                hovered_result.set(next_result);
            }
            if hovered_recent.read().is_some() {
                hovered_recent.set(None);
            }
        }
    });

    rsx! {
        div {
            style: "
                position: fixed;
                inset: 0;
                z-index: 500;
                background: rgba(0, 0, 0, 0.55);
                display: flex;
                flex-direction: column;
                align-items: center;
                padding-top: 10vh;
            ",
            onclick: move |_| open.set(false),
            div {
                style: "
                    background: #1e1e2d;
                    border: 1px solid rgba(100, 100, 200, 0.35);
                    border-radius: 12px;
                    width: min(640px, 92vw);
                    box-shadow: 0 8px 40px rgba(0, 0, 0, 0.6);
                    overflow: hidden;
                    display: flex;
                    flex-direction: column;
                    font-family: sans-serif;
                    color: #e0e0e8;
                ",
                onclick: move |evt| evt.stop_propagation(),
                div {
                    style: "
                        display: flex;
                        align-items: center;
                        gap: 0.5rem;
                        padding: 0.75rem 1rem;
                    ",
                    span {
                        style: "color: #9999bb; font-size: 1rem; user-select: none;",
                        "🔍"
                    }
                    input {
                        "data-testid": "search-input",
                        r#type: "text",
                        autofocus: true,
                        placeholder: SEARCH_INPUT_PLACEHOLDER,
                        value: "{q_display}",
                        style: "
                            flex: 1;
                            background: transparent;
                            border: none;
                            outline: none;
                            color: #e0e0e8;
                            font-size: 1rem;
                            caret-color: #6666cc;
                        ",
                        oninput: move |evt| {
                            query.set(evt.value().to_string());
                            state_filter.set(None);
                            type_filter.set(None);
                        },
                        onkeydown: move |evt| {
                            match evt.key() {
                                Key::Escape => open.set(false),
                                Key::ArrowDown => {
                                    evt.prevent_default();
                                    if is_empty_query {
                                        let next_recent = move_hover_index(
                                            *hovered_recent.read(),
                                            recent_items.len(),
                                            1,
                                        );
                                        hovered_recent.set(next_recent);
                                    } else {
                                        let next_result = move_hover_index(
                                            *hovered_result.read(),
                                            filtered.len(),
                                            1,
                                        );
                                        hovered_result.set(next_result);
                                    }
                                },
                                Key::ArrowUp => {
                                    evt.prevent_default();
                                    if is_empty_query {
                                        let next_recent = move_hover_index(
                                            *hovered_recent.read(),
                                            recent_items.len(),
                                            -1,
                                        );
                                        hovered_recent.set(next_recent);
                                    } else {
                                        let next_result = move_hover_index(
                                            *hovered_result.read(),
                                            filtered.len(),
                                            -1,
                                        );
                                        hovered_result.set(next_result);
                                    }
                                },
                                Key::Enter => {
                                    evt.prevent_default();
                                    if is_empty_query {
                                        if let Some(index) =
                                            normalize_hover_index(
                                                *hovered_recent.read(),
                                                recent_items.len(),
                                            )
                                        {
                                            if let Some(recent) = recent_items.get(index) {
                                                state_filter.set(None);
                                                type_filter.set(None);
                                                query.set(recent.clone());
                                            }
                                        }
                                    } else if let Some(index) = normalize_hover_index(
                                        *hovered_result.read(),
                                        filtered.len(),
                                    ) {
                                        if let Some(ticket) = filtered.get(index) {
                                            activate_search_result(
                                                ticket.clone(),
                                                workspace.clone(),
                                                q_display.clone(),
                                                open,
                                                on_ticket_open.clone(),
                                            );
                                        }
                                    }
                                },
                                _ => {},
                            }
                        },
                    }
                    span {
                        style: "
                            font-size: 11px;
                            color: #6666aa;
                            white-space: nowrap;
                            user-select: none;
                        ",
                        "Esc to close"
                    }
                }
                div {
                    "data-testid": "search-syntax-hint",
                    style: "
                        padding: 0 1rem 0.6rem;
                        border-bottom: 1px solid rgba(100, 100, 200, 0.2);
                        color: #9999bb;
                        font-size: 11px;
                        line-height: 1.4;
                    ",
                    "{SEARCH_SYNTAX_HINT}"
                }
                if !state_facets.is_empty() || !type_facets.is_empty() {
                    {render_facet_chips(&state_facets, &type_facets, state_filter, type_filter)}
                }
                div {
                    style: "overflow-y: auto; max-height: 360px;",
                    if *loading.read() {
                        div {
                            style: "padding: 1rem; color: #9999bb; font-size: 0.9rem; text-align: center;",
                            "Searching…"
                        }
                    }
                    if let Some(err) = search_err.read().as_deref() {
                        div {
                            style: "
                                padding: 0.75rem 1rem;
                                color: #ff8080;
                                font-size: 0.85rem;
                            ",
                            "Error: {err}"
                        }
                    }
                    if is_empty_query && !recent_items.is_empty() {
                        {render_recent_searches(
                            &recent_items,
                            query,
                            state_filter,
                            type_filter,
                            hovered_recent,
                        )}
                    }
                    if !filtered.is_empty() {
                        {render_search_results(
                            &filtered,
                            workspace.clone(),
                            q_display.clone(),
                            open,
                            on_ticket_open.clone(),
                            hovered_result,
                        )}
                    }
                    if !is_empty_query && !*loading.read() && filtered.is_empty() && search_err.read().is_none() {
                        {render_empty_state(&q_display)}
                    }
                }
                div {
                    style: "
                        padding: 0.5rem 1rem;
                        border-top: 1px solid rgba(100,100,200,0.15);
                        font-size: 11px;
                        color: #55556a;
                        display: flex;
                        gap: 1rem;
                    ",
                    span { "↩ open ticket" }
                    span { "Esc close" }
                    span { "Ctrl+K toggle" }
                }
            }
        }
    }
}

fn use_open_shortcut_listener(
    workspace: String,
    mut open: Signal<bool>,
    mut query: Signal<String>,
    mut results: Signal<Vec<TicketSummary>>,
    mut recents: Signal<Vec<String>>,
    mut state_filter: Signal<Option<String>>,
    mut type_filter: Signal<Option<String>>,
    mut keydown_listener: Signal<Option<EventListener>>,
) {
    use_effect(move || {
        let Some(window) = web_sys::window() else {
            return;
        };

        let workspace = workspace.clone();
        let listener =
            gloo_events::EventListener::new(&window, "keydown", move |event| {
                let Some(keyboard_event) =
                    event.dyn_ref::<web_sys::KeyboardEvent>()
                else {
                    return;
                };

                if slash_inside_input(keyboard_event) {
                    return;
                }

                let ctrl_or_cmd =
                    keyboard_event.ctrl_key() || keyboard_event.meta_key();
                let key = keyboard_event.key();
                let should_open = (ctrl_or_cmd && key == "k") || key == "/";

                if should_open && !open() {
                    keyboard_event.prevent_default();
                    recents.set(load_recent(&workspace));
                    state_filter.set(None);
                    type_filter.set(None);
                    results.set(vec![]);
                    query.set(String::new());
                    open.set(true);
                } else if key == "Escape" && open() {
                    open.set(false);
                }
            });

        keydown_listener.set(Some(listener));
    });
}

fn use_search_results(
    workspace: String,
    query: Signal<String>,
    mut results: Signal<Vec<TicketSummary>>,
    mut loading: Signal<bool>,
    mut search_err: Signal<Option<String>>,
) {
    use_effect(move || {
        let query = query.read().clone();
        if query.trim().is_empty() {
            results.set(vec![]);
            loading.set(false);
            search_err.set(None);
            return;
        }

        loading.set(true);
        search_err.set(None);
        let workspace = workspace.clone();
        spawn(async move {
            let backend = HttpTicketBackend::new(None);
            match backend
                .list_tickets(
                    &workspace,
                    None,
                    Some(query.trim()),
                    Some(MAX_RESULTS as u32),
                )
                .await
            {
                Ok(response) => {
                    results.set(response.items);
                    loading.set(false);
                },
                Err(error) => {
                    search_err.set(Some(error));
                    loading.set(false);
                },
            }
        });
    });
}

fn filtered_results(
    all_results: &[TicketSummary],
    state_filter: Signal<Option<String>>,
    type_filter: Signal<Option<String>>,
) -> Vec<TicketSummary> {
    let active_state = state_filter.read().clone();
    let active_type = type_filter.read().clone();

    all_results
        .iter()
        .filter(|ticket| {
            matches_filter(ticket_state(ticket), active_state.as_deref())
                && matches_filter(ticket_type(ticket), active_type.as_deref())
        })
        .cloned()
        .collect()
}

fn matches_filter(
    value: &str,
    active: Option<&str>,
) -> bool {
    active.is_none_or(|active| value == active)
}

fn normalize_hover_index(
    current: Option<usize>,
    len: usize,
) -> Option<usize> {
    if len == 0 {
        None
    } else {
        Some(current.filter(|index| *index < len).unwrap_or(0))
    }
}

fn move_hover_index(
    current: Option<usize>,
    len: usize,
    delta: i32,
) -> Option<usize> {
    if len == 0 {
        return None;
    }

    let current = current.unwrap_or(0);
    let next = if delta.is_negative() {
        current.saturating_sub(1)
    } else {
        (current + 1).min(len.saturating_sub(1))
    };
    Some(next)
}

fn slash_inside_input(keyboard_event: &web_sys::KeyboardEvent) -> bool {
    let Some(target) = keyboard_event
        .target()
        .and_then(|target| target.dyn_into::<web_sys::HtmlElement>().ok())
    else {
        return false;
    };

    let tag = target.tag_name().to_lowercase();
    matches!(tag.as_str(), "input" | "textarea" | "select")
        && keyboard_event.key() == "/"
}

fn render_recent_searches(
    recents: &[String],
    mut query: Signal<String>,
    mut state_filter: Signal<Option<String>>,
    mut type_filter: Signal<Option<String>>,
    mut hovered_recent: Signal<Option<usize>>,
) -> Element {
    rsx! {
        div {
            style: "padding: 0.5rem 0;",
            div {
                style: "
                    padding: 0.25rem 1rem;
                    font-size: 11px;
                    color: #6666aa;
                    text-transform: uppercase;
                    letter-spacing: 0.08em;
                ",
                "Recent searches"
            }
            for (index, recent) in recents.iter().enumerate() {
                {
                    let recent = recent.clone();
                    let is_hovered = *hovered_recent.read() == Some(index);
                    let row_bg = if is_hovered {
                        "rgba(80,80,140,0.25)"
                    } else {
                        "transparent"
                    };
                    rsx! {
                        button {
                            "data-testid": "search-recent-{index}",
                            aria_selected: if is_hovered { "true" } else { "false" },
                            style: "
                                display: flex;
                                align-items: center;
                                gap: 0.5rem;
                                width: 100%;
                                background: {row_bg};
                                border: none;
                                padding: 0.5rem 1rem;
                                color: #c0c0e8;
                                font-size: 0.9rem;
                                cursor: pointer;
                                text-align: left;
                            ",
                            onmouseenter: move |_| hovered_recent.set(Some(index)),
                            onmouseleave: move |_| hovered_recent.set(None),
                            onclick: move |_| {
                                state_filter.set(None);
                                type_filter.set(None);
                                query.set(recent.clone());
                            },
                            span { style: "color: #6666aa; font-size: 0.85rem;", "↩" }
                            span { "{recent}" }
                        }
                    }
                }
            }
        }
    }
}
