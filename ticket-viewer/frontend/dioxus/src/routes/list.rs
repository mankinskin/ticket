pub(super) const DETAIL_COLLAPSE_PX: u32 = 1160;
pub(super) const GRAPH_COLLAPSE_PX: u32 = 880;

#[path = "list/helpers.rs"]
mod helpers;
#[path = "list/page.rs"]
mod page;
#[path = "list/panels.rs"]
mod panels;

pub use page::TicketListPage;
