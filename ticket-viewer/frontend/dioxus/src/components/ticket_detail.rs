//! Ticket detail sidebar panel with inline click-to-edit fields.
//!
//! Renders as a fixed left panel showing all editable fields from the
//! tracker-improvement schema, with optimistic updates and SSE-based conflict
//! detection for concurrent edits.

#[path = "ticket_detail/actions.rs"]
mod actions;
#[path = "ticket_detail/model.rs"]
mod model;
#[path = "ticket_detail/page.rs"]
mod page;
#[path = "ticket_detail/ui.rs"]
mod ui;

pub use page::TicketDetail;
