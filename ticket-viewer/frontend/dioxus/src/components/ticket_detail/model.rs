use crate::types::{
    HistoryEntry,
    TypeSchema,
};

pub(super) type ConflictState = (String, String, String);

pub(super) const PRIORITY_OPTIONS: &[&str] =
    &["", "none", "low", "medium", "high", "critical"];
pub(super) const RISK_LEVEL_OPTIONS: &[&str] =
    &["", "none", "low", "medium", "high", "critical"];

pub(super) const STRING_FIELDS: &[(&str, &str)] = &[
    ("title", "Title"),
    ("priority", "Priority"),
    ("component", "Component"),
    ("effort", "Effort"),
    ("risk_level", "Risk Level"),
    ("tags", "Tags"),
    ("workflow_stage", "Workflow Stage"),
    ("phase", "Phase"),
    ("doc_category", "Doc Category"),
    ("release_target", "Release Target"),
    ("release_version", "Release Version"),
    ("rollout_stage", "Rollout Stage"),
];

pub(super) fn static_options(key: &str) -> &'static [&'static str] {
    match key {
        "priority" => PRIORITY_OPTIONS,
        "risk_level" => RISK_LEVEL_OPTIONS,
        _ => &[],
    }
}

pub(super) fn field_str(
    fields: &serde_json::Value,
    key: &str,
) -> String {
    fields
        .get(key)
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string()
}

pub(super) fn field_bool(
    fields: &serde_json::Value,
    key: &str,
) -> bool {
    fields
        .get(key)
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

pub(super) fn collect_visited_states(entries: &[HistoryEntry]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    entries
        .iter()
        .filter_map(|entry| {
            entry
                .fields
                .get("state")
                .and_then(|value| value.as_str())
                .map(str::to_string)
        })
        .filter(|state| seen.insert(state.clone()))
        .collect()
}

pub(super) fn valid_next_states(
    schema: Option<&TypeSchema>,
    current_state: &str,
) -> Vec<String> {
    schema
        .map(|schema| {
            schema
                .transitions
                .iter()
                .filter(|transition| transition.from == current_state)
                .map(|transition| transition.to.clone())
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn missing_required_states(
    schema: Option<&TypeSchema>,
    visited_states: &[String],
) -> Vec<String> {
    schema
        .map(|schema| {
            schema
                .required_states
                .iter()
                .filter(|state| !visited_states.contains(state))
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn terminal_states(schema: Option<&TypeSchema>) -> Vec<String> {
    schema
        .map(|schema| schema.terminal_states.clone())
        .unwrap_or_default()
}
