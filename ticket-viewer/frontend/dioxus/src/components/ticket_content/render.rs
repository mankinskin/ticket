use std::collections::HashSet;

use dioxus::prelude::*;
use viewer_api_dioxus::FileContentViewer;

use crate::{
    components::history::HistoryPanel,
    types::{
        HistoryEntry,
        ProjectedPart,
        ProjectedRef,
    },
};

use super::{
    parts::render_parts_panel,
    Tab,
    TAB_ACTIVE_STYLE,
    TAB_BASE_STYLE,
};

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

pub(super) fn render_tab_bar(
    mut active_tab: Signal<Tab>,
    content_tab_label: String,
    show_edit_tab: bool,
    history_count: usize,
) -> Element {
    rsx! {
        div {
            role: "tablist",
            "data-testid": "ticket-content-tabs",
            style: "
                display: flex;
                border-bottom: 1px solid #3a3a4a;
                flex-shrink: 0;
            ",
            button {
                role: "tab",
                "aria-selected": if active_tab() == Tab::Description { "true" } else { "false" },
                "data-testid": "tab-description",
                style: if active_tab() == Tab::Description { TAB_ACTIVE_STYLE } else { TAB_BASE_STYLE },
                onclick: move |_| active_tab.set(Tab::Description),
                "{content_tab_label}"
            }
            button {
                role: "tab",
                "aria-selected": if active_tab() == Tab::Toml { "true" } else { "false" },
                "data-testid": "tab-toml",
                style: if active_tab() == Tab::Toml { TAB_ACTIVE_STYLE } else { TAB_BASE_STYLE },
                onclick: move |_| active_tab.set(Tab::Toml),
                "TOML"
            }
            if show_edit_tab {
                button {
                    role: "tab",
                    "aria-selected": if active_tab() == Tab::Edit { "true" } else { "false" },
                    "data-testid": "tab-edit",
                    style: if active_tab() == Tab::Edit { TAB_ACTIVE_STYLE } else { TAB_BASE_STYLE },
                    onclick: move |_| active_tab.set(Tab::Edit),
                    "Edit"
                }
            }
            if history_count > 1 {
                button {
                    role: "tab",
                    "aria-selected": if active_tab() == Tab::History { "true" } else { "false" },
                    "data-testid": "tab-history",
                    style: if active_tab() == Tab::History { TAB_ACTIVE_STYLE } else { TAB_BASE_STYLE },
                    onclick: move |_| active_tab.set(Tab::History),
                    "History"
                }
            }
        }
    }
}

pub(super) fn render_document_panel(
    document: TicketDocumentContext,
    desc_loading: bool,
    desc_error: Option<String>,
    desc_text: Option<String>,
    asset_path: Option<String>,
) -> Element {
    let ticket_title = document
        .title
        .clone()
        .or_else(|| field_value(&document.fields, "title"))
        .unwrap_or_else(|| document.ticket_id.clone());
    let state = document
        .ticket_state
        .clone()
        .or_else(|| field_value(&document.fields, "state"));
    let ticket_type = document
        .ticket_type
        .clone()
        .or_else(|| field_value(&document.fields, "type"));
    let priority = field_value(&document.fields, "priority");
    let component = field_value(&document.fields, "component");
    let risk_level = field_value(&document.fields, "risk_level");
    let tags = field_value(&document.fields, "tags");
    let spec_refs = field_value(&document.fields, "spec_refs");
    let dependency_ids = field_value(&document.fields, "dependencies")
        .or_else(|| field_value(&document.fields, "depends_on"));
    let created_at = document
        .created_at
        .clone()
        .or_else(|| field_value(&document.fields, "created_at"));
    let updated_at = document
        .updated_at
        .clone()
        .or_else(|| field_value(&document.fields, "updated_at"));
    let created_at = compact_timestamp(created_at.as_deref());
    let updated_at = compact_timestamp(updated_at.as_deref());
    let extra_fields = additional_field_rows(&document.fields);

    let mut chips: Vec<(String, String)> = Vec::new();
    if let Some(value) = state.clone() {
        chips.push(("State".to_string(), value));
    }
    if let Some(value) = ticket_type {
        chips.push(("Type".to_string(), value));
    }
    if let Some(value) = priority {
        chips.push(("Priority".to_string(), value));
    }
    if let Some(value) = component {
        chips.push(("Component".to_string(), value));
    }
    if let Some(value) = risk_level {
        chips.push(("Risk".to_string(), value));
    }

    let mut metadata_rows: Vec<(String, String)> = vec![
        ("Workspace".to_string(), document.workspace.clone()),
        ("Ticket".to_string(), document.ticket_id.clone()),
    ];
    if let Some(value) = created_at {
        metadata_rows.push(("Created".to_string(), value));
    }
    if let Some(value) = updated_at {
        metadata_rows.push(("Updated".to_string(), value));
    }
    if let Some(value) = tags {
        metadata_rows.push(("Tags".to_string(), value));
    }
    if let Some(value) = spec_refs {
        metadata_rows.push(("Specs".to_string(), value));
    }
    if let Some(value) = dependency_ids {
        metadata_rows.push(("Dependencies".to_string(), value));
    }

    if desc_loading {
        return rsx! {
            div {
                "data-testid": "ticket-document",
                style: document_panel_style(),
                {render_document_header(ticket_title, chips, metadata_rows, state, asset_path.as_deref())}
                {render_document_fields(extra_fields)}
                div {
                    "data-testid": "desc-loading",
                    style: document_body_style(),
                    "Loading…"
                }
            }
        };
    }

    if let Some(error) = desc_error {
        return rsx! {
            div {
                "data-testid": "ticket-document",
                style: document_panel_style(),
                {render_document_header(ticket_title, chips, metadata_rows, state, asset_path.as_deref())}
                {render_document_fields(extra_fields)}
                div {
                    "data-testid": "desc-error",
                    style: "{document_body_style()} color: #f87171; font-size: 13px;",
                    strong { "Error: " }
                    "{error}"
                }
            }
        };
    }

    if let Some(content) = desc_text {
        let display_filename = asset_path
            .as_deref()
            .unwrap_or("description.md")
            .to_string();
        return rsx! {
            div {
                "data-testid": "ticket-document",
                style: document_panel_style(),
                {render_document_header(ticket_title, chips, metadata_rows, state, asset_path.as_deref())}
                {render_document_fields(extra_fields)}
                if let Some(path) = asset_path.clone().filter(|path| path != "description.md") {
                    div {
                        "data-testid": "ticket-document-asset-banner",
                        style: "padding: 0 24px;",
                        div {
                            style: "padding: 10px 12px; border: 1px solid var(--border-subtle); border-radius: 10px; background: color-mix(in srgb, var(--bg-secondary) 84%, transparent); color: var(--text-secondary); font-size: 12px;",
                            strong { "Viewing asset" }
                            " {path}"
                        }
                    }
                }
                div {
                    "data-testid": "desc-markdown",
                    style: document_body_style(),
                    FileContentViewer {
                        content,
                        filename: display_filename,
                    }
                }
            }
        };
    }

    rsx! {
        div {
            "data-testid": "ticket-document",
            style: document_panel_style(),
            {render_document_header(ticket_title, chips, metadata_rows, state, asset_path.as_deref())}
            {render_document_fields(extra_fields)}
            div {
                style: document_body_style(),
                em {
                    "data-testid": "desc-empty",
                    style: "color: #6b7280; font-size: 13px;",
                    "No content found."
                }
            }
        }
    }
}

/// Renders the ticket document header plus its parts as independently
/// collapsible sections (the `view=full` structured render), replacing the
/// single-blob description body. Legacy tickets still render correctly: the
/// backend synthesizes a sole `objective` part when `[[parts]]` is absent.
pub(super) fn render_full_document_panel(
    document: TicketDocumentContext,
    full_loading: bool,
    full_error: Option<String>,
    parts: Vec<ProjectedPart>,
    refs: Option<Vec<ProjectedRef>>,
    collapsed: Signal<HashSet<String>>,
) -> Element {
    let ticket_title = document
        .title
        .clone()
        .or_else(|| field_value(&document.fields, "title"))
        .unwrap_or_else(|| document.ticket_id.clone());
    let state = document
        .ticket_state
        .clone()
        .or_else(|| field_value(&document.fields, "state"));
    let ticket_type = document
        .ticket_type
        .clone()
        .or_else(|| field_value(&document.fields, "type"));
    let priority = field_value(&document.fields, "priority");
    let component = field_value(&document.fields, "component");
    let risk_level = field_value(&document.fields, "risk_level");
    let tags = field_value(&document.fields, "tags");
    let spec_refs = field_value(&document.fields, "spec_refs");
    let dependency_ids = field_value(&document.fields, "dependencies")
        .or_else(|| field_value(&document.fields, "depends_on"));
    let created_at = document
        .created_at
        .clone()
        .or_else(|| field_value(&document.fields, "created_at"));
    let updated_at = document
        .updated_at
        .clone()
        .or_else(|| field_value(&document.fields, "updated_at"));
    let created_at = compact_timestamp(created_at.as_deref());
    let updated_at = compact_timestamp(updated_at.as_deref());
    let extra_fields = additional_field_rows(&document.fields);

    let mut chips: Vec<(String, String)> = Vec::new();
    if let Some(value) = state.clone() {
        chips.push(("State".to_string(), value));
    }
    if let Some(value) = ticket_type {
        chips.push(("Type".to_string(), value));
    }
    if let Some(value) = priority {
        chips.push(("Priority".to_string(), value));
    }
    if let Some(value) = component {
        chips.push(("Component".to_string(), value));
    }
    if let Some(value) = risk_level {
        chips.push(("Risk".to_string(), value));
    }

    let mut metadata_rows: Vec<(String, String)> = vec![
        ("Workspace".to_string(), document.workspace.clone()),
        ("Ticket".to_string(), document.ticket_id.clone()),
    ];
    if let Some(value) = created_at {
        metadata_rows.push(("Created".to_string(), value));
    }
    if let Some(value) = updated_at {
        metadata_rows.push(("Updated".to_string(), value));
    }
    if let Some(value) = tags {
        metadata_rows.push(("Tags".to_string(), value));
    }
    if let Some(value) = spec_refs {
        metadata_rows.push(("Specs".to_string(), value));
    }
    if let Some(value) = dependency_ids {
        metadata_rows.push(("Dependencies".to_string(), value));
    }

    if full_loading {
        return rsx! {
            div {
                "data-testid": "ticket-document",
                style: document_panel_style(),
                {render_document_header(ticket_title, chips, metadata_rows, state, None)}
                {render_document_fields(extra_fields)}
                div {
                    "data-testid": "desc-loading",
                    style: document_body_style(),
                    "Loading…"
                }
            }
        };
    }

    if let Some(error) = full_error {
        return rsx! {
            div {
                "data-testid": "ticket-document",
                style: document_panel_style(),
                {render_document_header(ticket_title, chips, metadata_rows, state, None)}
                {render_document_fields(extra_fields)}
                div {
                    "data-testid": "desc-error",
                    style: "{document_body_style()} color: #f87171; font-size: 13px;",
                    strong { "Error: " }
                    "{error}"
                }
            }
        };
    }

    rsx! {
        div {
            "data-testid": "ticket-document",
            style: document_panel_style(),
            {render_document_header(ticket_title, chips, metadata_rows, state, None)}
            {render_document_fields(extra_fields)}
            {render_parts_panel(parts, refs, collapsed)}
        }
    }
}

fn render_document_fields(fields: Vec<DocumentFieldRow>) -> Element {
    if fields.is_empty() {
        return rsx! { Fragment {} };
    }

    rsx! {
        div {
            "data-testid": "ticket-document-fields",
            style: "margin: 0 24px; display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 10px;",
            for field in fields {
                div {
                    key: "{field.key}",
                    style: if field.wide {
                        "grid-column: 1 / -1; padding: 14px 16px; border-radius: 14px; border: 1px solid var(--border-subtle); background: color-mix(in srgb, var(--bg-secondary) 74%, transparent); min-width: 0;"
                    } else {
                        "padding: 14px 16px; border-radius: 14px; border: 1px solid var(--border-subtle); background: color-mix(in srgb, var(--bg-secondary) 74%, transparent); min-width: 0;"
                    },
                    div {
                        style: "font-size: 10px; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.08em; margin-bottom: 6px;",
                        "{field.label}"
                    }
                    div {
                        style: "font-size: 13px; color: var(--text-primary); line-height: 1.5; white-space: pre-wrap; overflow-wrap: anywhere;",
                        "{field.value}"
                    }
                }
            }
        }
    }
}

fn render_document_header(
    ticket_title: String,
    chips: Vec<(String, String)>,
    metadata_rows: Vec<(String, String)>,
    state: Option<String>,
    asset_path: Option<&str>,
) -> Element {
    rsx! {
        div {
            "data-testid": "ticket-document-header",
            style: "padding: 20px 24px 0; display: flex; flex-direction: column; gap: 14px;",
            div {
                style: "display: flex; align-items: flex-start; justify-content: space-between; gap: 12px; flex-wrap: wrap;",
                div {
                    style: "display: flex; flex-direction: column; gap: 8px; min-width: 0;",
                    if let Some(path) = asset_path.filter(|path| *path != "description.md") {
                        div {
                            style: "font-size: 11px; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.08em;",
                            "Asset context"
                        }
                        div {
                            style: "font-size: 12px; color: var(--text-secondary);",
                            "{path}"
                        }
                    } else {
                        div {
                            style: "font-size: 11px; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.08em;",
                            "Ticket document"
                        }
                    }
                    h1 {
                        style: "margin: 0; font-size: 24px; line-height: 1.2; color: var(--text-primary);",
                        "{ticket_title}"
                    }
                }
                if let Some(current_state) = state {
                    div {
                        style: "padding: 6px 10px; border-radius: 999px; border: 1px solid var(--border-subtle); background: color-mix(in srgb, var(--accent-blue) 16%, transparent); color: var(--text-primary); font-size: 11px; text-transform: uppercase; letter-spacing: 0.08em; white-space: nowrap;",
                        "{current_state}"
                    }
                }
            }
            if !chips.is_empty() {
                div {
                    style: "display: flex; flex-wrap: wrap; gap: 8px;",
                    for (label, value) in chips {
                        div {
                            key: "{label}-{value}",
                            style: "display: inline-flex; align-items: center; gap: 6px; padding: 6px 10px; border-radius: 999px; border: 1px solid var(--border-subtle); background: color-mix(in srgb, var(--bg-secondary) 80%, transparent); font-size: 11px; color: var(--text-secondary);",
                            span { style: "color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.08em;", "{label}" }
                            span { style: "color: var(--text-primary);", "{value}" }
                        }
                    }
                }
            }
            div {
                "data-testid": "ticket-document-metadata",
                style: "display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 10px;",
                for (label, value) in metadata_rows {
                    div {
                        key: "meta-{label}",
                        style: "padding: 12px 14px; border-radius: 12px; border: 1px solid var(--border-subtle); background: color-mix(in srgb, var(--bg-secondary) 74%, transparent); min-width: 0;",
                        div {
                            style: "font-size: 10px; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.08em; margin-bottom: 4px;",
                            "{label}"
                        }
                        div {
                            style: "font-size: 13px; color: var(--text-primary); line-height: 1.4; overflow-wrap: anywhere;",
                            "{value}"
                        }
                    }
                }
            }
        }
    }
}

fn additional_field_rows(fields: &serde_json::Value) -> Vec<DocumentFieldRow> {
    let Some(map) = fields.as_object() else {
        return Vec::new();
    };

    let mut entries: Vec<(&String, &serde_json::Value)> = map
        .iter()
        .filter(|(key, _)| !is_header_field(key.as_str()))
        .collect();
    entries.sort_by(|(left, _), (right, _)| {
        field_priority(left.as_str())
            .cmp(&field_priority(right.as_str()))
            .then_with(|| left.cmp(right))
    });

    entries
        .into_iter()
        .filter_map(|(key, value)| {
            let formatted = field_detail_value(value)?;
            let wide = formatted.contains('\n') || formatted.len() > 140;
            Some(DocumentFieldRow {
                key: key.clone(),
                label: field_label(key),
                value: formatted,
                wide,
            })
        })
        .collect()
}

fn is_header_field(key: &str) -> bool {
    matches!(
        key,
        "title"
            | "state"
            | "type"
            | "priority"
            | "component"
            | "risk_level"
            | "created_at"
            | "updated_at"
            | "tags"
            | "spec_refs"
            | "dependencies"
            | "depends_on"
            // Structural manifest tables rendered by the dedicated parts/refs
            // UI, not the generic field-row grid (spec 24b3d22b).
            | "parts"
            | "refs"
            | "plan_revision"
    )
}

fn field_priority(key: &str) -> usize {
    match key {
        "acceptance_criteria" => 0,
        "implementation_summary" => 1,
        "validation_summary" => 2,
        "workflow_stage" => 3,
        "phase" => 4,
        "doc_category" => 5,
        "release_target" => 6,
        "release_version" => 7,
        "rollout_stage" => 8,
        "execution_wave" => 9,
        "parallel_track" => 10,
        "impl_status" => 11,
        _ => 100,
    }
}

fn field_label(key: &str) -> String {
    match key {
        "acceptance_criteria" => "Acceptance Criteria".to_string(),
        "implementation_summary" => "Implementation Summary".to_string(),
        "validation_summary" => "Validation Summary".to_string(),
        "workflow_stage" => "Workflow Stage".to_string(),
        "doc_category" => "Doc Category".to_string(),
        "release_target" => "Release Target".to_string(),
        "release_version" => "Release Version".to_string(),
        "rollout_stage" => "Rollout Stage".to_string(),
        "execution_wave" => "Execution Wave".to_string(),
        "parallel_track" => "Parallel Track".to_string(),
        "impl_status" => "Implementation Status".to_string(),
        "dependencies" | "depends_on" => "Dependencies".to_string(),
        _ => key
            .split('_')
            .filter(|segment| !segment.is_empty())
            .map(|segment| {
                let mut chars = segment.chars();
                match chars.next() {
                    Some(first) => {
                        let mut word = first.to_uppercase().collect::<String>();
                        word.push_str(chars.as_str());
                        word
                    },
                    None => String::new(),
                }
            })
            .collect::<Vec<String>>()
            .join(" "),
    }
}

fn field_detail_value(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        },
        serde_json::Value::Bool(flag) => Some(flag.to_string()),
        serde_json::Value::Number(number) => Some(number.to_string()),
        serde_json::Value::Array(items) => {
            if items.is_empty() {
                return None;
            }

            let simple_values: Vec<String> =
                items.iter().filter_map(value_to_string).collect();
            if simple_values.len() == items.len() {
                Some(simple_values.join("\n"))
            } else {
                serde_json::to_string_pretty(value).ok()
            }
        },
        serde_json::Value::Object(map) =>
            if map.is_empty() {
                None
            } else {
                serde_json::to_string_pretty(value).ok()
            },
        serde_json::Value::Null => None,
    }
}

fn field_value(
    fields: &serde_json::Value,
    key: &str,
) -> Option<String> {
    let value = fields.get(key)?;
    match value {
        serde_json::Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        },
        serde_json::Value::Bool(flag) => Some(flag.to_string()),
        serde_json::Value::Number(number) => Some(number.to_string()),
        serde_json::Value::Array(items) => {
            let values: Vec<String> =
                items.iter().filter_map(value_to_string).collect();
            if values.is_empty() {
                None
            } else {
                Some(values.join(", "))
            }
        },
        _ => None,
    }
}

fn value_to_string(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        },
        serde_json::Value::Bool(flag) => Some(flag.to_string()),
        serde_json::Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

fn compact_timestamp(value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    if value.is_empty() {
        return None;
    }
    Some(
        value
            .trim_end_matches('Z')
            .replace('T', " ")
            .replace("+00:00", " UTC"),
    )
}

fn document_panel_style() -> &'static str {
    "display: flex; flex-direction: column; gap: 14px; min-height: 100%; padding-bottom: 24px;"
}

fn document_body_style() -> &'static str {
    "margin: 0 24px; padding: 20px 22px; border-radius: 16px; border: 1px solid var(--border-subtle); background: color-mix(in srgb, var(--bg-secondary) 72%, transparent); color: var(--text-primary);"
}

pub(super) fn render_toml_panel(toml_text: String) -> Element {
    rsx! {
        pre {
            "data-testid": "toml-content",
            style: "
                font-family: 'Cascadia Code', 'Fira Code', monospace;
                font-size: 12.5px;
                line-height: 1.6;
                color: #e0e0e8;
                margin: 0;
                white-space: pre;
                overflow-x: auto;
            ",
            "{toml_text}"
        }
    }
}

pub(super) fn render_history_panel(
    workspace: String,
    ticket_id: String,
    entries: Vec<HistoryEntry>,
    mut history_refresh_key: Signal<u32>,
) -> Element {
    rsx! {
        HistoryPanel {
            workspace,
            ticket_id,
            entries,
            on_refresh: move |_| {
                history_refresh_key.set(history_refresh_key() + 1);
            },
        }
    }
}
