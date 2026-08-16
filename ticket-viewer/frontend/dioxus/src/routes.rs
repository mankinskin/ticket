//! Dioxus Router route definitions for the ticket viewer SPA.

use dioxus::prelude::*;

use crate::{
    api::{
        HttpTicketBackend,
        TicketBackend,
    },
    types::WorkspacesResponse,
};

mod detail;
mod list;
mod new_ticket;

pub use detail::TicketDetailPage;
pub use list::TicketListPage;
pub use new_ticket::NewTicketPage;

#[component]
pub fn TicketListRootPage() -> Element {
    let backend = HttpTicketBackend::new(None);
    let mut workspace: Signal<Option<String>> = use_signal(|| None);
    let mut load_error: Signal<Option<String>> = use_signal(|| None);

    use_effect(move || {
        let backend = backend.clone();
        spawn(async move {
            match backend.list_workspaces().await {
                Ok(response) =>
                    workspace.set(resolve_active_workspace(&response)),
                Err(error) => load_error.set(Some(error)),
            }
        });
    });

    if let Some(workspace) = workspace.read().clone() {
        return rsx! { TicketListPage { workspace } };
    }

    let message = load_error
        .read()
        .clone()
        .unwrap_or_else(|| "Loading workspace…".to_string());

    rsx! {
        div {
            style: "display: flex; min-height: 100vh; align-items: center; justify-content: center; color: var(--text-muted); font-size: 14px;",
            "{message}"
        }
    }
}

fn resolve_active_workspace(response: &WorkspacesResponse) -> Option<String> {
    if !response.active_workspace.is_empty() {
        return Some(response.active_workspace.clone());
    }

    response
        .workspaces
        .first()
        .map(|workspace| workspace.name.clone())
}

// ── Route enum ────────────────────────────────────────────────────────────────

#[derive(Clone, Routable, Debug, PartialEq)]
pub enum Route {
    #[route("/")]
    TicketListRootPage {},

    #[route("/workspace/:workspace")]
    TicketListPage { workspace: String },

    #[route("/workspace/:workspace/new")]
    NewTicketPage { workspace: String },

    #[route("/workspace/:workspace/ticket/:id")]
    TicketDetailPage { workspace: String, id: String },
}
