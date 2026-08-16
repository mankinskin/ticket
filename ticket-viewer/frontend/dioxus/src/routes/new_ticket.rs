use dioxus::prelude::*;

use crate::{
    components::create_ticket::CreateTicketModal,
    routes::Route,
};

#[component]
pub fn NewTicketPage(workspace: String) -> Element {
    let nav = use_navigator();
    let workspace_back = workspace.clone();

    rsx! {
        CreateTicketModal {
            workspace,
            prefill: None,
            on_cancel: move |_| {
                nav.push(Route::TicketListPage {
                    workspace: workspace_back.clone(),
                });
            },
        }
    }
}
