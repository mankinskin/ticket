//! TicketContent — tabbed panel showing description, TOML, history, and an
//! optional Markdown editor for the currently selected ticket.

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tab {
    Description,
    Toml,
    Edit,
    History,
}

pub(crate) const TAB_BASE_STYLE: &str = "
    padding: 8px 16px;
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    color: #9ca3af;
    font-size: 12px;
    font-family: sans-serif;
    cursor: pointer;
    transition: color 0.1s, border-color 0.1s;
";

pub(crate) const TAB_ACTIVE_STYLE: &str = "
    padding: 8px 16px;
    background: transparent;
    border: none;
    border-bottom: 2px solid #6366f1;
    color: #a5b4fc;
    font-size: 12px;
    font-family: sans-serif;
    cursor: pointer;
";

mod edit;
mod helpers;
mod page;
mod parts;
mod render;

pub use page::TicketContent;
