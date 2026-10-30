#[path = "ui/render.rs"]
mod render;
#[path = "ui/widgets.rs"]
mod widgets;

pub(super) use render::{TicketDetailHandlers, TicketDetailViewState, render_ticket_detail};
