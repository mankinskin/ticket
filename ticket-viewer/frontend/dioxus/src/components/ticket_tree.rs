//! TicketTree — sidebar widget that lists tickets with filter controls.
//!
//! Renders a filter input, state-chip bar, and a scrollable sorted list of
//! ticket buttons.  Clicking a ticket calls `on_select`.
//!
//! Each ticket row has an expand/collapse arrow.  When expanded the row shows
//! the ticket's files (description.md + assets/*.md) fetched lazily from the
//! server.  Clicking a file row calls `on_select_file((ticket_ref, path))`.

use dioxus::prelude::*;

use crate::types::{
    TicketRef,
    TicketSummary,
};

// ── Props ──────────────────────────────────────────────────────────────────

#[derive(Props, Clone, PartialEq)]
pub struct TicketTreeProps {
    /// Workspace name — needed to fetch per-ticket file lists.
    pub workspace: String,

    /// Full (unfiltered) ticket list — filtering and sorting happen server-side
    /// or in the parent; this component just renders.
    pub tickets: Vec<TicketSummary>,
    pub loading: bool,
    pub error: Option<String>,

    /// Current filter text (two-way binding via signals in the parent).
    pub filter: String,
    pub on_filter_change: EventHandler<String>,

    /// Active state-chip value (empty string = "All").
    pub state_filter: String,
    pub on_state_filter_change: EventHandler<String>,

    /// Current sort key used to order the list.
    pub sort_key: String,

    /// ID of the selected ticket (highlighted row).
    pub selected_id: Option<String>,
    /// Called when a ticket row is clicked.
    pub on_select: EventHandler<TicketRef>,

    /// Called when a file sub-row is clicked: `(ticket_ref, relative_path)`.
    #[props(default)]
    pub on_select_file: Option<EventHandler<(TicketRef, String)>>,

    /// Currently selected file: `(ticket_ref, relative_path)` — used to
    /// highlight the active file row.
    #[props(default)]
    pub selected_file: Option<(TicketRef, String)>,

    /// When true, each row shows a checkbox for batch selection.
    #[props(default = false)]
    pub show_checkboxes: bool,

    /// IDs currently selected via checkboxes.
    #[props(default)]
    pub selected_ids: Vec<String>,

    /// Called when a checkbox is toggled (ticket id passed).
    pub on_toggle_select: Option<EventHandler<String>>,

    /// Called when the "Select all" header checkbox is toggled.
    pub on_select_all: Option<EventHandler<bool>>,

    /// Called when the user clicks the "+ New" button in the list header.
    #[props(default)]
    pub on_new_ticket: Option<EventHandler<()>>,

    /// Called when the user clicks the "☑ Batch" toggle button in the list header.
    #[props(default)]
    pub on_toggle_batch: Option<EventHandler<()>>,
}

// ── State filter chip definitions ──────────────────────────────────────────

const STATE_CHIPS: &[(&str, &str)] = &[
    ("All", ""),
    ("open", "open"),
    ("planned", "planned"),
    ("impl", "in-implementation"),
    ("review", "in-review"),
    ("done", "done"),
    ("cancelled", "cancelled"),
];

mod header;
mod page;
mod rows;

pub use page::TicketTree;
