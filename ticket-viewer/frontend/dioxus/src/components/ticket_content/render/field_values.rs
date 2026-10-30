#[allow(unused_imports)]
use super::*;

pub(super) fn additional_field_rows(fields: &serde_json::Value) -> Vec<DocumentFieldRow> {
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

pub(super) fn is_header_field(key: &str) -> bool {
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

pub(super) fn field_priority(key: &str) -> usize {
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

pub(super) fn field_label(key: &str) -> String {
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
                    }
                    None => String::new(),
                }
            })
            .collect::<Vec<String>>()
            .join(" "),
    }
}

pub(super) fn field_detail_value(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        serde_json::Value::Bool(flag) => Some(flag.to_string()),
        serde_json::Value::Number(number) => Some(number.to_string()),
        serde_json::Value::Array(items) => {
            if items.is_empty() {
                return None;
            }

            let simple_values: Vec<String> = items.iter().filter_map(value_to_string).collect();
            if simple_values.len() == items.len() {
                Some(simple_values.join("\n"))
            } else {
                serde_json::to_string_pretty(value).ok()
            }
        }
        serde_json::Value::Object(map) => {
            if map.is_empty() {
                None
            } else {
                serde_json::to_string_pretty(value).ok()
            }
        }
        serde_json::Value::Null => None,
    }
}

pub(super) fn field_value(fields: &serde_json::Value, key: &str) -> Option<String> {
    let value = fields.get(key)?;
    match value {
        serde_json::Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        serde_json::Value::Bool(flag) => Some(flag.to_string()),
        serde_json::Value::Number(number) => Some(number.to_string()),
        serde_json::Value::Array(items) => {
            let values: Vec<String> = items.iter().filter_map(value_to_string).collect();
            if values.is_empty() {
                None
            } else {
                Some(values.join(", "))
            }
        }
        _ => None,
    }
}

pub(super) fn value_to_string(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        serde_json::Value::Bool(flag) => Some(flag.to_string()),
        serde_json::Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

pub(super) fn compact_timestamp(value: Option<&str>) -> Option<String> {
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
