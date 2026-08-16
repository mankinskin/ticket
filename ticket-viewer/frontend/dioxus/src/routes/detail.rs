use dioxus::prelude::*;
use viewer_api_dioxus::update_hash_params;

use crate::routes::Route;

#[component]
pub fn TicketDetailPage(
    workspace: String,
    id: String,
) -> Element {
    let nav = use_navigator();
    let workspace = workspace.clone();
    let ticket_id = id.clone();

    use_effect(move || {
        update_hash_params(
            &[("ticket-workspace", &workspace), ("ticket-id", &ticket_id)],
            &["id"],
        );
        nav.replace(Route::TicketListRootPage {});
    });

    rsx! {}
}
