#[allow(unused_imports)]
use super::*;

pub(super) fn render_next_report(obj: &serde_json::Map<String, Value>) -> String {
    let mut out = String::new();

    let command = obj.get("command").and_then(Value::as_str).unwrap_or("?");
    let status = obj.get("status").and_then(Value::as_str).unwrap_or("?");
    let _ = writeln!(out, "{command} {status}");

    let mut scalars = Vec::new();
    let mut sections = Vec::new();

    for (key, val) in obj {
        if key == "command" || key == "status" || key == "items" || key == "blocker_tree" {
            continue;
        }
        if is_section(val) {
            sections.push((key.as_str(), val));
        } else {
            scalars.push((key.as_str(), val));
        }
    }

    for (key, val) in &scalars {
        let _ = writeln!(out, "{key}: {}", format_scalar(val));
    }

    for (key, val) in &sections {
        write_section(&mut out, key, val, 0);
    }

    if let Some(tree) = obj.get("blocker_tree") {
        let _ = writeln!(out);
        let _ = writeln!(out, "Blocker Tree:");
        write_workflow_tree_node(&mut out, tree, "", true, true);
    }

    let recommendations = obj
        .get("items")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(parse_board_recommendation)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    write_next_up(&mut out, &recommendations);

    let trimmed = out.trim_end();
    let mut result = trimmed.to_string();
    result.push('\n');
    result
}

pub(super) fn render_workflow_tree_report(
    obj: &serde_json::Map<String, Value>,
    tree_heading: &str,
) -> String {
    let mut out = String::new();

    let command = obj.get("command").and_then(Value::as_str).unwrap_or("?");
    let status = obj.get("status").and_then(Value::as_str).unwrap_or("?");
    let _ = writeln!(out, "{command} {status}");

    let mut scalars = Vec::new();
    let mut sections = Vec::new();

    for (key, val) in obj {
        if key == "command" || key == "status" || key == "root" || key == "frontier_items" {
            continue;
        }
        if is_section(val) {
            sections.push((key.as_str(), val));
        } else {
            scalars.push((key.as_str(), val));
        }
    }

    for (key, val) in &scalars {
        let _ = writeln!(out, "{key}: {}", format_scalar(val));
    }

    for (key, val) in &sections {
        write_section(&mut out, key, val, 0);
    }

    if let Some(root) = obj.get("root") {
        let _ = writeln!(out);
        let _ = writeln!(out, "{tree_heading}:");
        write_workflow_tree_node(&mut out, root, "", true, true);
    }

    let frontier_items = obj
        .get("frontier_items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    write_frontier_items(&mut out, &frontier_items);

    let trimmed = out.trim_end();
    let mut result = trimmed.to_string();
    result.push('\n');
    result
}

pub(super) fn write_frontier_items(out: &mut String, items: &[Value]) {
    let _ = writeln!(out);
    let _ = writeln!(out, "Frontier Leaves:");

    if items.is_empty() {
        let _ = writeln!(out, "  (no frontier leaves)");
        return;
    }

    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            let _ = writeln!(out);
        } else {
            let _ = writeln!(out);
        }

        let rank = item.get("rank").and_then(Value::as_u64).unwrap_or(0);
        let ticket_id = item.get("id").and_then(Value::as_str).unwrap_or("-");
        let title = item
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("(untitled ticket)");
        let state = item.get("state").and_then(Value::as_str).unwrap_or("-");
        let priority = item
            .get("priority")
            .and_then(Value::as_str)
            .unwrap_or("none");
        let dependee_count = item
            .get("dependee_count")
            .or_else(|| item.get("dependees"))
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let dependency_count = item
            .get("dependency_count")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let remaining_blocker_count = item
            .get("remaining_blocker_count")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let remaining_blockers = item
            .get("remaining_blockers")
            .and_then(Value::as_array)
            .map(|blockers| {
                blockers
                    .iter()
                    .filter_map(Value::as_str)
                    .map(short_ticket_value)
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .filter(|joined| !joined.is_empty())
            .unwrap_or_else(|| "-".to_string());
        let created_at = item
            .get("created_at")
            .and_then(Value::as_str)
            .unwrap_or("-");

        let _ = writeln!(
            out,
            "  #{}  {}  {}",
            rank,
            short_ticket_value(ticket_id),
            title,
        );
        let _ = writeln!(
            out,
            "  state: {}  priority: {}  dependee_count: {}  dependency_count: {}",
            state, priority, dependee_count, dependency_count,
        );
        let _ = writeln!(
            out,
            "  remaining_blockers: {}  blocker_ids: {}",
            remaining_blocker_count, remaining_blockers,
        );
        let _ = writeln!(out, "  created_at: {}", created_at);
        let _ = writeln!(out, "  ticket_id: {}", ticket_id);
    }
}

pub(super) fn write_workflow_tree_node(
    out: &mut String,
    node: &Value,
    prefix: &str,
    is_last: bool,
    is_root: bool,
) {
    let ticket_id = node.get("id").and_then(Value::as_str).unwrap_or("-");
    let title = node
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("(untitled ticket)");
    let state = node.get("state").and_then(Value::as_str).unwrap_or("-");
    let priority = node
        .get("priority")
        .and_then(Value::as_str)
        .unwrap_or("none");
    let remaining_blocker_count = node
        .get("remaining_blocker_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let frontier_leaf_count = node
        .get("unresolved_frontier_leaf_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let blocker_distance = node
        .get("blocker_distance")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let is_frontier = node
        .get("is_frontier")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let branch = if is_root {
        ""
    } else if is_last {
        "\\- "
    } else {
        "|- "
    };
    let marker = if is_frontier { "*" } else { "o" };
    let detail_indent = if is_root {
        "  ".to_string()
    } else if is_last {
        format!("{prefix}   ")
    } else {
        format!("{prefix}|  ")
    };

    let _ = writeln!(
        out,
        "{prefix}{branch}{marker} {}  {}",
        short_ticket_value(ticket_id),
        title,
    );
    let _ = writeln!(
        out,
        "{detail_indent}state: {}  priority: {}  remaining_blockers: {}  frontier_leaves: {}  blocker_distance: {}",
        state, priority, remaining_blocker_count, frontier_leaf_count, blocker_distance,
    );

    let children = node
        .get("children")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let child_prefix = if is_root {
        "  ".to_string()
    } else if is_last {
        format!("{prefix}   ")
    } else {
        format!("{prefix}|  ")
    };

    for (index, child) in children.iter().enumerate() {
        write_workflow_tree_node(
            out,
            child,
            &child_prefix,
            index + 1 == children.len(),
            false,
        );
    }
}
