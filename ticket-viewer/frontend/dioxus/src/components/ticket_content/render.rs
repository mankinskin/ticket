use std::collections::HashSet;

use dioxus::prelude::*;
use viewer_api_dioxus::FileContentViewer;

use crate::{
    components::history::HistoryPanel,
    types::{HistoryEntry, ProjectedPart, ProjectedRef},
};

use super::{TAB_ACTIVE_STYLE, TAB_BASE_STYLE, Tab, parts::render_parts_panel};

#[derive(Clone, PartialEq)]
pub(super) struct TicketDocumentContext {
    pub workspace: String,
    pub ticket_id: String,
    pub title: Option<String>,
    pub ticket_state: Option<String>,
    pub ticket_type: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub fields: serde_json::Value,
}

#[derive(Clone, PartialEq)]
struct DocumentFieldRow {
    key: String,
    label: String,
    value: String,
    wide: bool,
}

#[path = "render/document_panels.rs"]
mod document_panels;
#[allow(unused_imports)]
pub use document_panels::*;

#[path = "render/field_values.rs"]
mod field_values;
#[allow(unused_imports)]
pub use field_values::*;
