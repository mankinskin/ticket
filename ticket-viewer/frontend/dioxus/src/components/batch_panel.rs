//! `BatchPanel` — floating bottom bar for bulk ticket operations.
//!
//! Shown when one or more tickets are selected via the TicketTree checkboxes.
//! Provides buttons for state transitions, priority updates, close, and cancel.
//! On confirm it POSTs a `BatchRequest` to `/api/batch`, shows per-command
//! results, then calls `on_done` to clear the selection and refresh the list.

#[path = "batch_panel/actions.rs"]
mod actions;
#[path = "batch_panel/model.rs"]
mod model;
#[path = "batch_panel/page.rs"]
mod page;
#[path = "batch_panel/results.rs"]
mod results;

pub use page::BatchPanel;
