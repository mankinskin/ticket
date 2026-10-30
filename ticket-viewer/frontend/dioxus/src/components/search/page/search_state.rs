#[allow(unused_imports)]
use super::*;

pub(super) fn use_open_shortcut_listener(
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
        let listener = gloo_events::EventListener::new(&window, "keydown", move |event| {
            let Some(keyboard_event) = event.dyn_ref::<web_sys::KeyboardEvent>() else {
                return;
            };

            if slash_inside_input(keyboard_event) {
                return;
            }

            let ctrl_or_cmd = keyboard_event.ctrl_key() || keyboard_event.meta_key();
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

pub(super) fn use_search_results(
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
                }
                Err(error) => {
                    search_err.set(Some(error));
                    loading.set(false);
                }
            }
        });
    });
}

pub(super) fn filtered_results(
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

pub(super) fn matches_filter(value: &str, active: Option<&str>) -> bool {
    active.is_none_or(|active| value == active)
}

pub(super) fn normalize_hover_index(current: Option<usize>, len: usize) -> Option<usize> {
    if len == 0 {
        None
    } else {
        Some(current.filter(|index| *index < len).unwrap_or(0))
    }
}

pub(super) fn move_hover_index(current: Option<usize>, len: usize, delta: i32) -> Option<usize> {
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

pub(super) fn slash_inside_input(keyboard_event: &web_sys::KeyboardEvent) -> bool {
    let Some(target) = keyboard_event
        .target()
        .and_then(|target| target.dyn_into::<web_sys::HtmlElement>().ok())
    else {
        return false;
    };

    let tag = target.tag_name().to_lowercase();
    matches!(tag.as_str(), "input" | "textarea" | "select") && keyboard_event.key() == "/"
}
