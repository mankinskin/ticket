//! SearchBar — floating full-text search overlay for the ticket viewer.
//!
//! # Features
//!
//! * Triggered by **Ctrl+K** / **Cmd+K** / **/** at the document level.
//! * Free text searches ticket titles and description/body content.
//! * Input also accepts supported `field:value` predicates:
//!   `id:<value>`, `title:<value>`, `state:<value>` / `status:<value>`,
//!   and `type:<value>` / `ticket_type:<value>`.
//! * Delegates to `GET /api/tickets?workspace=<ws>&query=<q>` (existing FTS).
//! * Shows ranked results in a floating panel (max 8 items).
//! * State/type **filter-chip facets** narrow results client-side.
//! * Clicking a result navigates to [`Route::TicketDetailPage`].
//! * Last 5 searches persisted in `localStorage` under
//!   `ticket-viewer:{ws}:recent-searches`; surfaced when the bar opens empty.
//! * Dismissed by **Escape** or a click outside the panel.
//!
//! # Usage
//!
//! Mount `SearchBar` once per workspace page, passing the current workspace
//! name.  It renders nothing when closed and a floating overlay when open.
//!
//! ```rust,ignore
//! rsx! {
//!     SearchBar { workspace: workspace.clone() }
//!     // …rest of page
//! }
//! ```
//!
//! # Predicate syntax tooltip
//!
//! The input shows a helper hint that documents the supported predicate keys,
//! title/body free-text scope, AND semantics between terms, and quoted-phrase
//! behavior without advertising unsupported fields.

mod facets;
mod page;
mod recent;
mod results;

pub use page::SearchBar;
