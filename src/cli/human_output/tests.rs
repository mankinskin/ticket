use super::*;
use serde_json::json;

#[test]
fn flat_create_payload() {
    let payload = json!({
        "command": "create",
        "status": "ok",
        "id": "abc-123",
        "type": "tracker-improvement"
    });
    let out = render_human_readable(&payload);
    assert!(out.starts_with("create ok\n"));
    assert!(out.contains("id: abc-123"));
    assert!(out.contains("type: tracker-improvement"));
}

#[test]
fn nested_object_section() {
    let payload = json!({
        "command": "get",
        "status": "ok",
        "ticket": {
            "id": "abc-123",
            "created_at": "2026-01-01"
        }
    });
    let out = render_human_readable(&payload);
    assert!(out.contains("[ticket]"));
    assert!(out.contains("  id: abc-123"));
    assert!(out.contains("  created_at: 2026-01-01"));
}

#[test]
fn array_of_objects() {
    let payload = json!({
        "command": "list",
        "status": "ok",
        "count": 2,
        "items": [
            { "id": "aaa", "title": "First" },
            { "id": "bbb", "title": "Second" }
        ]
    });
    let out = render_human_readable(&payload);
    assert!(out.contains("count: 2"));
    assert!(out.contains("[items] (2)"));
    assert!(out.contains("  id: aaa"));
    assert!(out.contains("  title: First"));
    assert!(out.contains("  id: bbb"));
    assert!(out.contains("  title: Second"));
}

#[test]
fn scalar_array_inline() {
    let payload = json!({
        "command": "assets",
        "status": "ok",
        "assets": ["a.png", "b.txt"]
    });
    let out = render_human_readable(&payload);
    assert!(out.contains("assets: a.png, b.txt"));
}

#[test]
fn null_rendered_as_dash() {
    let payload = json!({
        "command": "get",
        "status": "ok",
        "title": null
    });
    let out = render_human_readable(&payload);
    assert!(out.contains("title: -"));
}

#[test]
fn deeply_nested_status_payload() {
    let payload = json!({
        "command": "status",
        "status": "ok",
        "summary": {
            "total": 10,
            "done": 3,
            "active": 2,
            "planned": 3
        },
        "active": [
            { "id": "aaa", "title": "Bug fix", "state": "in-implementation", "component": "core" }
        ],
        "planned": [],
        "parallel_groups": []
    });
    let out = render_human_readable(&payload);
    assert!(out.contains("[summary]"));
    assert!(out.contains("  total: 10"));
    assert!(out.contains("[active] (1)"));
    assert!(out.contains("  title: Bug fix"));
    // Empty arrays render inline (no items to section)
    assert!(out.contains("planned:"));
}

#[test]
fn nested_fields_in_get() {
    let payload = json!({
        "command": "get",
        "status": "ok",
        "ticket": {
            "id": "abc",
            "fields": {
                "priority": "high",
                "tags": ["rust", "cli"]
            }
        }
    });
    let out = render_human_readable(&payload);
    assert!(out.contains("[ticket]"));
    assert!(out.contains("  id: abc"));
    assert!(out.contains("  [fields]"));
    assert!(out.contains("    priority: high"));
    assert!(out.contains("    tags: rust, cli"));
}

#[test]
fn non_object_payload_fallback() {
    let payload = json!("just a string");
    let out = render_human_readable(&payload);
    assert_eq!(out.trim(), "just a string");
}

#[test]
fn dry_run_payload() {
    let payload = json!({
        "command": "create",
        "dry_run": true,
        "status": "ok",
        "would_execute": "create ticket"
    });
    let out = render_human_readable(&payload);
    assert!(out.starts_with("create ok\n"));
    assert!(out.contains("dry_run: true"));
    assert!(out.contains("would_execute: create ticket"));
}

#[test]
fn audit_with_map_sections() {
    let payload = json!({
        "command": "audit",
        "status": "ok",
        "total": 10,
        "active": 8,
        "deleted": 2,
        "by_state": {
            "open": 3,
            "in-implementation": 2,
            "done": 3
        },
        "by_type": {
            "tracker-improvement": 10
        }
    });
    let out = render_human_readable(&payload);
    assert!(out.contains("total: 10"));
    assert!(out.contains("[by_state]"));
    assert!(out.contains("  open: 3"));
    assert!(out.contains("[by_type]"));
    assert!(out.contains("  tracker-improvement: 10"));
}
