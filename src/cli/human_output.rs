use std::fmt::Write;

use serde_json::Value;
use uuid::Uuid;

use crate::cli::commands::{parse_board_recommendation, write_next_up};

/// Render a command payload as human-readable key-value text.
///
/// Layout:
///   - Header line: `command status`
///   - Scalar fields: `key: value`
///   - Nested objects: section `[key]` with indented children
///   - Arrays of objects: section `[key] (N items)` with blank-line-separated entries
///   - Arrays of scalars: inline `key: a, b, c`
///   - Null values: shown as `-`
pub(crate) fn render_human_readable(payload: &Value) -> String {
    let Some(obj) = payload.as_object() else {
        return format_scalar(payload);
    };

    if let Some(rendered) = render_special_command(obj) {
        return rendered;
    }

    let mut out = String::new();

    // Header: "command status"
    let command = obj.get("command").and_then(Value::as_str).unwrap_or("?");
    let status = obj.get("status").and_then(Value::as_str).unwrap_or("?");
    let _ = writeln!(out, "{command} {status}");

    let (scalars, sections) = partition_object_fields(obj, &["command", "status"]);

    for (key, val) in &scalars {
        let _ = writeln!(out, "{key}: {}", format_scalar(val));
    }

    for (key, val) in &sections {
        write_section(&mut out, key, val, 0);
    }

    // Trim trailing whitespace but keep one final newline
    let trimmed = out.trim_end();
    let mut result = trimmed.to_string();
    result.push('\n');
    result
}

fn render_special_command(obj: &serde_json::Map<String, Value>) -> Option<String> {
    let command = obj.get("command").and_then(Value::as_str)?;

    if matches!(command, "board_show" | "board_history") {
        return obj.get("human").and_then(Value::as_str).map(str::to_string);
    }

    match command {
        "next" => Some(render_next_report(obj)),
        "blockers" => Some(render_workflow_tree_report(obj, "Blocker Tree")),
        "unblocked_by" => Some(render_workflow_tree_report(obj, "Unlock Tree")),
        "subgraph" | "topgraph" => obj.get("tree").and_then(Value::as_str).map(str::to_string),
        "describe" => Some(
            obj.get("description")
                .and_then(Value::as_str)
                .unwrap_or("(no description)")
                .to_string(),
        ),
        "health" => Some(render_health_report(obj)),
        _ => None,
    }
}

fn partition_object_fields<'a>(
    obj: &'a serde_json::Map<String, Value>,
    excluded_keys: &[&str],
) -> (Vec<(&'a str, &'a Value)>, Vec<(&'a str, &'a Value)>) {
    let mut scalars = Vec::new();
    let mut sections = Vec::new();

    for (key, val) in obj {
        if excluded_keys.iter().any(|excluded| key == excluded) {
            continue;
        }
        if is_section(val) {
            sections.push((key.as_str(), val));
        } else {
            scalars.push((key.as_str(), val));
        }
    }

    (scalars, sections)
}

fn short_ticket_value(raw: &str) -> String {
    raw.parse::<Uuid>()
        .map(|id| id.simple().to_string()[..8].to_string())
        .unwrap_or_else(|_| raw.chars().take(8).collect())
}

// ── predicates ─────────────────────────────────────────────────────────────────

/// Returns true when a value should be rendered as its own `[section]`.
fn is_section(val: &Value) -> bool {
    match val {
        Value::Object(_) => true,
        Value::Array(arr) => arr.iter().any(|v| v.is_object() || v.is_array()),
        _ => false,
    }
}

// ── scalar formatting ──────────────────────────────────────────────────────────

fn format_scalar(val: &Value) -> String {
    match val {
        Value::Null => "-".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.to_string(),
        Value::Array(arr) => arr.iter().map(format_scalar).collect::<Vec<_>>().join(", "),
        // Fallback for inline objects
        Value::Object(_) => serde_json::to_string(val).unwrap_or_default(),
    }
}

// ── section rendering ──────────────────────────────────────────────────────────

fn write_section(out: &mut String, key: &str, val: &Value, depth: usize) {
    let indent = "  ".repeat(depth);

    match val {
        Value::Object(map) => {
            let _ = write!(out, "\n{indent}[{key}]\n");
            write_object_fields(out, map, depth + 1);
        }
        Value::Array(arr) => {
            let count = arr.len();
            let _ = write!(out, "\n{indent}[{key}] ({count})\n");
            write_array_items(out, arr, depth + 1);
        }
        _ => {
            let _ = writeln!(out, "{indent}{key}: {}", format_scalar(val));
        }
    }
}

fn write_object_fields(out: &mut String, map: &serde_json::Map<String, Value>, depth: usize) {
    let indent = "  ".repeat(depth);
    let mut child_sections = Vec::new();

    for (k, v) in map {
        if is_section(v) {
            child_sections.push((k.as_str(), v));
        } else {
            let _ = writeln!(out, "{indent}{k}: {}", format_scalar(v));
        }
    }

    for (k, v) in child_sections {
        write_section(out, k, v, depth);
    }
}

fn write_array_items(out: &mut String, arr: &[Value], depth: usize) {
    let indent = "  ".repeat(depth);

    for (i, item) in arr.iter().enumerate() {
        if i > 0 {
            let _ = writeln!(out);
        }
        match item {
            Value::Object(map) => {
                write_object_fields(out, map, depth);
            }
            _ => {
                let _ = writeln!(out, "{indent}{}", format_scalar(item));
            }
        }
    }
}

// ── tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "human_output/tests.rs"]
mod tests;

#[path = "human_output/workflow_reports.rs"]
mod workflow_reports;
#[allow(unused_imports)]
pub use workflow_reports::*;

#[path = "human_output/health_report.rs"]
mod health_report;
#[allow(unused_imports)]
pub use health_report::*;
