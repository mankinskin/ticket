use dioxus::prelude::*;

use viewer_api_dioxus::is_mobile_sidebar_viewport;

use crate::types::{
    TicketRef,
    TicketSummary,
};

pub(super) fn initial_window_width() -> u32 {
    #[cfg(target_arch = "wasm32")]
    {
        return web_sys::window()
            .and_then(|window| window.inner_width().ok())
            .and_then(|value| value.as_f64())
            .map(|width| width as u32)
            .unwrap_or(1200);
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        1200
    }
}

pub(super) fn sidebar_button_state(
    sidebar_is_mobile: bool,
    mobile_sidebar_open: bool,
    sidebar_collapsed: bool,
) -> (bool, &'static str) {
    if sidebar_is_mobile {
        let label = if mobile_sidebar_open {
            "Close tickets sidebar"
        } else {
            "Open tickets sidebar"
        };
        return (mobile_sidebar_open, label);
    }

    if sidebar_collapsed {
        (false, "Open tickets sidebar")
    } else {
        (true, "Collapse tickets sidebar")
    }
}

pub(super) fn toggle_sidebar_button(
    mut mobile_sidebar_open: Signal<bool>,
    mut sidebar_collapsed: Signal<bool>,
) {
    if is_mobile_sidebar_viewport() {
        let next = !*mobile_sidebar_open.read();
        mobile_sidebar_open.set(next);
    } else {
        sidebar_collapsed.toggle();
    }
}

pub(super) fn close_mobile_or_toggle_sidebar(
    mut mobile_sidebar_open: Signal<bool>,
    mut sidebar_collapsed: Signal<bool>,
) {
    if is_mobile_sidebar_viewport() {
        mobile_sidebar_open.set(false);
    } else {
        sidebar_collapsed.toggle();
    }
}

pub(super) fn handle_file_selection(
    ticket_ref: TicketRef,
    path: String,
    mut selected_ticket: Signal<Option<TicketRef>>,
    mut selected_file: Signal<Option<(TicketRef, String)>>,
    mut mobile_sidebar_open: Signal<bool>,
    mut view_mode: Signal<String>,
) {
    selected_ticket.set(Some(ticket_ref.clone()));
    selected_file.set(Some((ticket_ref, path)));
    mobile_sidebar_open.set(false);
    if view_mode.read().as_str() == "graph" {
        view_mode.set("content".to_string());
    }
}

pub(super) fn toggle_ticket_selection(
    ticket_id: String,
    mut selected_ids: Signal<Vec<String>>,
) {
    let mut ids = selected_ids.write();
    if let Some(position) = ids.iter().position(|value| value == &ticket_id) {
        ids.remove(position);
    } else {
        ids.push(ticket_id);
    }
}

pub(super) fn apply_select_all(
    checked: bool,
    tickets: Signal<Vec<TicketSummary>>,
    mut selected_ids: Signal<Vec<String>>,
) {
    if checked {
        let ids: Vec<String> = tickets
            .read()
            .iter()
            .map(|ticket| ticket.id.clone())
            .collect();
        selected_ids.set(ids);
    } else {
        selected_ids.set(Vec::new());
    }
}

pub(super) fn toggle_batch_selection(
    mut show_checkboxes: Signal<bool>,
    mut selected_ids: Signal<Vec<String>>,
) {
    show_checkboxes.toggle();
    if !*show_checkboxes.read() {
        selected_ids.set(Vec::new());
    }
}
