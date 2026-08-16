use dioxus::prelude::*;

use viewer_api_dioxus::{
    FilterToggleButton,
    HamburgerIcon,
    Layout,
    LayoutMode,
    Overlay,
    PageHeader,
    Projection,
    Sidebar,
    ThemeSettings,
    SIDEBAR_MOBILE_BREAKPOINT_PX,
};

use crate::{
    api::{
        HttpTicketBackend,
        TicketBackend,
    },
    components::{
        batch_panel::BatchPanel,
        search::SearchBar,
        ticket_tree::TicketTree,
        workflow_explorer::{
            WorkflowExplorer,
            WorkflowSidebarMode,
        },
    },
    routes::Route,
    sse::use_sse,
    types::{
        TicketRef,
        TicketSummary,
    },
};

use super::{
    helpers::{
        apply_select_all,
        close_mobile_or_toggle_sidebar,
        handle_file_selection,
        initial_window_width,
        sidebar_button_state,
        toggle_batch_selection,
        toggle_sidebar_button,
        toggle_ticket_selection,
    },
    panels::{
        render_empty_main_panel,
        render_selected_main_panel,
    },
    DETAIL_COLLAPSE_PX,
    GRAPH_COLLAPSE_PX,
};

#[component]
pub fn TicketListPage(workspace: String) -> Element {
    let nav = use_navigator();
    let store = crate::store::TicketListStore::use_store(&workspace);

    let workspace_persist = workspace.clone();
    use_effect(move || {
        store.persist(&workspace_persist);
    });

    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut sidebar_collapsed = use_signal(|| false);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut mobile_sidebar_open = use_signal(|| false);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut show_theme_settings = use_signal(|| false);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut workflow_mode: Signal<Option<WorkflowSidebarMode>> =
        use_signal(|| None);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut workflow_root_ticket: Signal<Option<TicketRef>> =
        use_signal(|| None);

    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut tickets: Signal<Vec<TicketSummary>> = use_signal(Vec::new);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut loading: Signal<bool> = use_signal(|| true);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut list_error: Signal<Option<String>> = use_signal(|| None);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut workspace_label: Signal<String> = use_signal(|| workspace.clone());
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut selected_ticket = store.open_ticket;
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut graph_root_ticket: Signal<Option<TicketRef>> = use_signal(|| None);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut graph_content_ticket: Signal<Option<TicketRef>> =
        use_signal(|| None);
    use_effect(move || {
        let next_selected = selected_ticket.read().clone();
        match next_selected {
            Some(ticket_ref) => {
                let root_needs_reset = graph_root_ticket
                    .peek()
                    .as_ref()
                    .map(|root| root.workspace != ticket_ref.workspace)
                    .unwrap_or(true);
                if root_needs_reset {
                    graph_root_ticket.set(Some(ticket_ref));
                }
            },
            None => graph_root_ticket.set(None),
        }
        graph_content_ticket.set(None);
    });
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut filter = store.filter;
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut state_filter = store.state_filter;
    let sort_key = store.sort_key;

    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut selected_ids: Signal<Vec<String>> = use_signal(Vec::new);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut show_checkboxes: Signal<bool> = use_signal(|| false);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut refresh_counter: Signal<u32> = use_signal(|| 0);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut silent_refresh: Signal<bool> = use_signal(|| false);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut list_request_seq: Signal<u64> = use_signal(|| 0);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut workspace_label_request_seq: Signal<u64> = use_signal(|| 0);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut view_mode: Signal<String> = use_signal(|| "split".to_string());
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut graph_layout_mode: Signal<LayoutMode> =
        use_signal(|| LayoutMode::Hierarchical3D);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut graph_projection: Signal<Projection> =
        use_signal(|| Projection::Orthographic);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut graph_panel_width: Signal<f64> = use_signal(|| 320.0_f64);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut detail_panel_width: Signal<f64> = use_signal(|| 240.0_f64);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut selected_file: Signal<Option<(TicketRef, String)>> =
        use_signal(|| None);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut window_width: Signal<u32> = use_signal(initial_window_width);
    #[cfg(target_arch = "wasm32")]
    let mut resize_guard: Signal<Option<gloo_events::EventListener>> =
        use_signal(|| None);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut detail_panel_override: Signal<Option<bool>> = use_signal(|| None);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut graph_panel_override: Signal<Option<bool>> = use_signal(|| None);

    {
        let workspace = workspace.clone();
        use_effect(move || {
            let workspace = workspace.clone();
            workspace_label.set(workspace.clone());
            let request_seq = workspace_label_request_seq.with_mut(|value| {
                *value += 1;
                *value
            });
            spawn(async move {
                let workspace_label_request_seq = workspace_label_request_seq;
                let backend = HttpTicketBackend::new(None);
                if let Ok(response) = backend.list_workspaces().await {
                    if *workspace_label_request_seq.peek() != request_seq {
                        return;
                    }
                    workspace_label.set(
                        response
                            .label_for_name(&workspace)
                            .unwrap_or(workspace),
                    );
                }
            });
        });
    }

    use_sse(workspace.clone(), tickets, refresh_counter, silent_refresh);

    {
        let workspace = workspace.clone();
        use_effect(move || {
            let workspace = workspace.clone();
            let query = filter.read().clone();
            let state = state_filter.read().clone();
            let _refresh = *refresh_counter.read();
            let request_seq = list_request_seq.with_mut(|value| {
                *value += 1;
                *value
            });
            let is_silent_refresh = *silent_refresh.peek();
            if !is_silent_refresh {
                loading.set(true);
            }
            list_error.set(None);
            spawn(async move {
                let mut silent_refresh = silent_refresh;
                let list_request_seq = list_request_seq;
                let backend = HttpTicketBackend::new(None);
                let query = if query.trim().is_empty() {
                    None
                } else {
                    Some(query.trim().to_string())
                };
                let state = if state.is_empty() { None } else { Some(state) };
                match backend
                    .list_tickets(
                        &workspace,
                        state.as_deref(),
                        query.as_deref(),
                        Some(200),
                    )
                    .await
                {
                    Ok(response) => {
                        if *list_request_seq.peek() != request_seq {
                            return;
                        }
                        tickets.set(response.items);
                        loading.set(false);
                        silent_refresh.set(false);
                    },
                    Err(error) => {
                        if *list_request_seq.peek() != request_seq {
                            return;
                        }
                        list_error.set(Some(error));
                        loading.set(false);
                        silent_refresh.set(false);
                    },
                }
            });
        });
    }

    #[cfg(target_arch = "wasm32")]
    {
        let service = use_context::<crate::graph_fetch::GraphFetchService>();
        use_effect(move || {
            if let Some(ticket_ref) = selected_ticket.read().clone() {
                service.ensure_fetched(&ticket_ref.workspace, &ticket_ref.id);
            }
        });
    }

    #[cfg(target_arch = "wasm32")]
    {
        const PREFETCH_N: usize = 8;
        let service = use_context::<crate::graph_fetch::GraphFetchService>();
        let workspace_prefetch = workspace.clone();
        use_effect(move || {
            let tickets = tickets.read();
            for ticket in tickets.iter().take(PREFETCH_N) {
                let ticket_ref =
                    ticket.resolved_ticket_ref(&workspace_prefetch);
                service.ensure_fetched(&ticket_ref.workspace, &ticket_ref.id);
            }
        });
    }

    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        if let Some(window) = web_sys::window() {
            let listener =
                gloo_events::EventListener::new(&window, "resize", move |_| {
                    if let Some(window) = web_sys::window() {
                        if let Ok(inner_width) = window.inner_width() {
                            if let Some(value) = inner_width.as_f64() {
                                window_width.set(value as u32);
                            }
                        }
                    }
                });
            resize_guard.set(Some(listener));
        }
    });

    let ticket_count = tickets.read().len();
    let window_width_value = *window_width.read();
    let workspace_for_new = workspace.clone();
    let workspace_for_search = workspace.clone();
    let selected_ticket_snapshot = selected_ticket.read().clone();
    let workflow_mode_value = *workflow_mode.read();
    let detail_is_collapsed = detail_panel_override
        .read()
        .unwrap_or(window_width_value < DETAIL_COLLAPSE_PX);
    let graph_panel_collapsed = graph_panel_override
        .read()
        .unwrap_or(window_width_value < GRAPH_COLLAPSE_PX);
    let sidebar_is_mobile =
        (window_width_value as f64) <= SIDEBAR_MOBILE_BREAKPOINT_PX;
    let (sidebar_button_active, sidebar_button_label) = sidebar_button_state(
        sidebar_is_mobile,
        *mobile_sidebar_open.read(),
        *sidebar_collapsed.read(),
    );
    let selected_ticket_ref = selected_ticket_snapshot.clone();
    let workspace_title = workspace_label.read().clone();
    let workspace_subtitle =
        (workspace_title != workspace).then(|| workspace.clone());

    rsx! {
        SearchBar {
            workspace: workspace.clone(),
            on_ticket_open: move |ticket_ref: TicketRef| {
                selected_ticket.set(Some(ticket_ref));
                store.persist(&workspace_for_search);
                mobile_sidebar_open.set(false);
            },
        }
        Layout {
            header: rsx! {
                PageHeader {
                    lead: Some(rsx! {
                        button {
                            class: if sidebar_button_active { "btn btn-icon btn-active" } else { "btn btn-icon" },
                            aria_label: sidebar_button_label,
                            title: sidebar_button_label,
                            onclick: move |_| toggle_sidebar_button(mobile_sidebar_open, sidebar_collapsed),
                            HamburgerIcon {}
                        }
                    }),
                    icon: Some(rsx! { "🎫" }),
                    title: Some(workspace_title),
                    subtitle: workspace_subtitle,
                    on_theme_toggle: Some(EventHandler::new(move |_| {
                        let next = !*show_theme_settings.read();
                        show_theme_settings.set(next);
                    })),
                }
            },
            Sidebar {
                title: if workflow_mode_value.is_some() { "Workflow" } else { "Tickets" },
                badge: if workflow_mode_value.is_none() && ticket_count > 0 { Some(ticket_count.to_string()) } else { None },
                collapsed: *sidebar_collapsed.read(),
                on_toggle: move |_| close_mobile_or_toggle_sidebar(mobile_sidebar_open, sidebar_collapsed),
                mobile_open: Some(*mobile_sidebar_open.read()),
                on_mobile_open_change: move |open| mobile_sidebar_open.set(open),
                div {
                    style: "display: flex; flex-direction: column; height: 100%; min-height: 0;",
                    div {
                        style: "
                            padding: 0 12px 8px;
                            border-bottom: 1px solid var(--border-subtle);
                            display: flex;
                            align-items: center;
                            flex-wrap: wrap;
                            gap: 4px;
                        ",
                        FilterToggleButton {
                            test_id: "sidebar-mode-browse".to_string(),
                            class: "chip".to_string(),
                            active_class: "chip--active".to_string(),
                            inactive_class: "chip--neutral".to_string(),
                            active: workflow_mode_value.is_none(),
                            onclick: move |_| workflow_mode.set(None),
                            "Browse"
                        }
                        FilterToggleButton {
                            test_id: "sidebar-mode-next".to_string(),
                            class: "chip".to_string(),
                            active_class: "chip--active".to_string(),
                            inactive_class: "chip--neutral".to_string(),
                            active: workflow_mode_value == Some(WorkflowSidebarMode::Next),
                            onclick: move |_| workflow_mode.set(Some(WorkflowSidebarMode::Next)),
                            "Next"
                        }
                        FilterToggleButton {
                            test_id: "sidebar-mode-blockers".to_string(),
                            class: "chip".to_string(),
                            active_class: "chip--active".to_string(),
                            inactive_class: "chip--neutral".to_string(),
                            active: workflow_mode_value == Some(WorkflowSidebarMode::Blockers),
                            onclick: move |_| {
                                if let Some(ticket_ref) = selected_ticket.read().clone() {
                                    workflow_root_ticket.set(Some(ticket_ref));
                                }
                                workflow_mode.set(Some(WorkflowSidebarMode::Blockers));
                            },
                            "Blockers"
                        }
                        FilterToggleButton {
                            test_id: "sidebar-mode-unblocked-by".to_string(),
                            class: "chip".to_string(),
                            active_class: "chip--active".to_string(),
                            inactive_class: "chip--neutral".to_string(),
                            active: workflow_mode_value == Some(WorkflowSidebarMode::UnblockedBy),
                            onclick: move |_| {
                                if let Some(ticket_ref) = selected_ticket.read().clone() {
                                    workflow_root_ticket.set(Some(ticket_ref));
                                }
                                workflow_mode.set(Some(WorkflowSidebarMode::UnblockedBy));
                            },
                            "Unblocked"
                        }
                    }
                    if workflow_mode_value.is_none() {
                        TicketTree {
                            workspace: workspace.clone(),
                            tickets: tickets.read().clone(),
                            loading: *loading.read(),
                            error: list_error.read().clone(),
                            filter: filter.read().clone(),
                            on_filter_change: move |value: String| filter.set(value),
                            state_filter: state_filter.read().clone(),
                            on_state_filter_change: move |value: String| state_filter.set(value),
                            sort_key: sort_key.read().clone(),
                            selected_id: selected_ticket.read().as_ref().map(|ticket| ticket.id.clone()),
                            on_select: move |ticket_ref: TicketRef| {
                                selected_ticket.set(Some(ticket_ref));
                                mobile_sidebar_open.set(false);
                            },
                            on_select_file: move |(ticket_ref, path): (TicketRef, String)| handle_file_selection(
                                ticket_ref,
                                path,
                                selected_ticket,
                                selected_file,
                                mobile_sidebar_open,
                                view_mode,
                            ),
                            selected_file: selected_file.read().clone(),
                            show_checkboxes: *show_checkboxes.read(),
                            selected_ids: selected_ids.read().clone(),
                            on_toggle_select: move |ticket_id: String| toggle_ticket_selection(ticket_id, selected_ids),
                            on_select_all: move |checked: bool| apply_select_all(checked, tickets, selected_ids),
                            on_new_ticket: move |_| {
                                nav.push(Route::NewTicketPage {
                                    workspace: workspace_for_new.clone(),
                                });
                            },
                            on_toggle_batch: move |_| toggle_batch_selection(show_checkboxes, selected_ids),
                        }
                    } else if let Some(mode) = workflow_mode_value {
                        WorkflowExplorer {
                            workspace: workspace.clone(),
                            mode,
                            selected_ticket: selected_ticket_snapshot.clone(),
                            root_ticket: workflow_root_ticket.read().clone(),
                            on_select: move |ticket_ref: TicketRef| {
                                selected_ticket.set(Some(ticket_ref));
                                mobile_sidebar_open.set(false);
                            },
                            on_use_selected_root: move |_| {
                                if let Some(ticket_ref) = selected_ticket.read().clone() {
                                    workflow_root_ticket.set(Some(ticket_ref));
                                }
                            },
                        }
                    }
                }
            }
            div {
                class: "content",
                style: "display: flex; flex-direction: column; overflow: hidden;",
                if let Some(selected_ticket_ref) = selected_ticket_ref {
                    {render_selected_main_panel(
                        workspace.clone(),
                        selected_ticket_ref,
                        graph_root_ticket,
                        tickets,
                        graph_content_ticket,
                        view_mode,
                        graph_panel_collapsed,
                        detail_is_collapsed,
                        graph_layout_mode,
                        graph_projection,
                        graph_panel_width,
                        detail_panel_width,
                        selected_file,
                        graph_panel_override,
                        detail_panel_override,
                        window_width,
                    )}
                } else {
                    {render_empty_main_panel()}
                }
            }
        }
        Overlay {
            open: *show_theme_settings.read(),
            on_close: move |_| show_theme_settings.set(false),
            panel_class: "theme-settings-modal".to_string(),
            aria_label: "Theme settings".to_string(),
            ThemeSettings {
                on_close: move |_| show_theme_settings.set(false),
            }
        }
        if !selected_ids.read().is_empty() {
            BatchPanel {
                workspace: workspace.clone(),
                selected_ids: selected_ids.read().clone(),
                on_done: move |_| {
                    selected_ids.set(Vec::new());
                    show_checkboxes.set(false);
                    *refresh_counter.write() += 1;
                },
            }
        }
    }
}
