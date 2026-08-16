//! CreateTicketModal — dialog for composing and submitting a new ticket.
//!
//! # Features
//!
//! * Type selector populated from `GET /api/schema`.
//! * Dynamic field section driven by the selected type's schema definition:
//!   required fields show a red asterisk; optional fields are collapsible.
//! * Static fields always visible: title (text), description (textarea),
//!   priority (select), component (text).
//! * Auto-saves draft to `localStorage["draft_new_ticket"]` on every
//!   keystroke; clears on successful submit or cancel.
//! * Submits via `POST /api/tickets` (`HttpTicketBackend::create_ticket`).
//! * Shows inline server-validation errors.
//! * Redirects to the new ticket's detail page on success.
//! * `prefill: Option<TicketSummary>` prop pre-populates all fields (template
//!   mode).

mod actions;
mod draft;
mod fields;
mod page;

pub use page::CreateTicketModal;
