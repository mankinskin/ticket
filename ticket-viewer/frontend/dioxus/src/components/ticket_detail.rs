//! Ticket detail sidebar panel with inline click-to-edit fields.
//!
//! Renders as a fixed left panel showing all editable fields from the
//! tracker-improvement schema, with optimistic updates and SSE-based conflict
//! detection for concurrent edits.

mod actions;
mod model;
mod page;
mod ui;

pub use page::TicketDetail;
