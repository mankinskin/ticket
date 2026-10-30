#[allow(unused_imports)]
use super::*;

// ---------------------------------------------------------------------------
// CRUD
// ---------------------------------------------------------------------------

#[test]
pub(super) fn create_and_get_roundtrip() {
    let s = Sandbox::new();

    let created = s.ticket_json(&[
        "create",
        "--title",
        "Fix login bug",
        "--type",
        "tracker-improvement",
    ]);
    assert_eq!(created["status"], "ok");
    // ticket-api makes no assumption of a default type, so the type is retained.
    assert_eq!(created["type"], "tracker-improvement");

    let id = created["id"].as_str().expect("id must be present");

    let got = s.ticket_json(&["get", id]);
    assert_eq!(got["status"], "ok");
    assert_eq!(got["ticket"]["id"], id);
    assert_eq!(got["ticket"]["fields"]["title"], "Fix login bug");
    assert_eq!(got["ticket"]["fields"]["state"], "planning");
    assert_eq!(got["ticket"]["fields"]["type"], "tracker-improvement");
    // Interview metadata is schema-supported but optional, so it should not be
    // auto-initialized for tickets without an active interview.
    assert!(got["ticket"]["fields"]["interview_file_type"].is_null());
    assert!(got["ticket"]["fields"]["interview_files"].is_null());
}

#[test]
pub(super) fn create_multiple_and_list_all() {
    let s = Sandbox::new();

    for title in &["Alpha feature", "Beta fix", "Gamma refactor"] {
        let r = s.ticket_json(&["create", "--title", title, "--type", "tracker-improvement"]);
        assert_eq!(r["status"], "ok");
    }

    let list = s.ticket_json(&["list"]);
    assert_eq!(list["status"], "ok");
    assert_eq!(list["count"].as_u64().unwrap(), 3);

    let titles: Vec<&str> = list["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t["title"].as_str())
        .collect();

    assert!(titles.contains(&"Alpha feature"));
    assert!(titles.contains(&"Beta fix"));
    assert!(titles.contains(&"Gamma refactor"));
}

#[test]
pub(super) fn list_filters_by_state() {
    let s = Sandbox::new();

    create_ticket(&s, "Stays planning");
    let id2 = create_ticket(&s, "Goes ready");
    s.ticket_json(&["update", &id2, "--to-state", "ready"]);

    let new_tickets = s.ticket_json(&["list", "--state", "planning"]);
    assert_eq!(new_tickets["count"].as_u64().unwrap(), 1);
    assert_eq!(new_tickets["items"][0]["title"], "Stays planning");

    let ready = s.ticket_json(&["list", "--state", "ready"]);
    assert_eq!(ready["count"].as_u64().unwrap(), 1);
    assert_eq!(ready["items"][0]["id"], id2.as_str());
}

#[test]
pub(super) fn list_with_repro_includes_reproduction_status() {
    let s = Sandbox::new();
    let id = create_ticket(&s, "Repro status ticket");

    let _ = s.ticket_json(&[
        "repro",
        &id,
        "--outcome",
        "reproduced",
        "--command",
        "cargo test -p context-read validate_triple_repeat -- --nocapture",
    ]);

    let listed = s.ticket_json(&["list", "--with-repro"]);
    assert_eq!(listed["status"].as_str().unwrap(), "ok");
    assert!(listed["with_repro"].as_bool().unwrap());

    let item = listed["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["id"] == id)
        .expect("ticket should be present in list output");

    assert_eq!(item["repro"]["count"].as_u64().unwrap(), 1);
    assert_eq!(item["repro"]["last_outcome"], "reproduced");
    assert!(item["repro"]["last_commit"].as_str().is_some());
}

#[test]
pub(super) fn update_fields_and_state_transition() {
    let s = Sandbox::new();
    let id = create_ticket(&s, "Needs work");

    let updated = s.ticket_json(&[
        "update",
        &id,
        "--field",
        "title=Updated title",
        "--to-state",
        "ready",
    ]);
    assert_eq!(updated["status"], "ok");

    let got = s.ticket_json(&["get", &id]);
    assert_eq!(got["ticket"]["fields"]["title"], "Updated title");
    assert_eq!(got["ticket"]["fields"]["state"], "ready");
}

#[test]
pub(super) fn update_description_requires_an_explicit_mode() {
    let s = Sandbox::new();
    let id = create_ticket(&s, "Needs description mode");

    let (code, stderr) = s.ticket_fail(&["update", &id, "--description", "Summary text"]);

    assert_eq!(code, 1);
    assert!(
        stderr.contains(REQUIRED_DESCRIPTION_MODE_ERROR),
        "expected store-level description-mode error, got: {stderr}"
    );
}

#[test]
pub(super) fn update_help_documents_description_mode() {
    let help = Command::new(env!("CARGO_BIN_EXE_ticket"))
        .args(["update", "--help"])
        .output()
        .expect("update help should run");

    assert!(help.status.success());
    let stdout = String::from_utf8_lossy(&help.stdout);
    assert!(stdout.contains("--description-mode <DESCRIPTION_MODE>"));
    assert!(stdout.contains("Required when setting a description"));
    assert!(stdout.contains("append"));
}

#[test]
pub(super) fn delete_removes_ticket_from_list() {
    let s = Sandbox::new();
    let del_id = create_ticket(&s, "Will be deleted");
    let keep_id = create_ticket(&s, "Will survive");

    let del = s.ticket_json(&["delete", &del_id]);
    assert_eq!(del["status"], "ok");

    let list = s.ticket_json(&["list"]);
    assert_eq!(list["count"].as_u64().unwrap(), 1);
    assert_eq!(list["items"][0]["id"], keep_id.as_str());
    assert_eq!(list["items"][0]["title"], "Will survive");
}

#[test]
pub(super) fn get_after_delete_exits_nonzero() {
    let s = Sandbox::new();
    let id = create_ticket(&s, "Temporary ticket");
    s.ticket_json(&["delete", &id]);

    let (exit_code, _stderr) = s.ticket_fail(&["get", &id]);
    assert_eq!(exit_code, 1);
}

// ---------------------------------------------------------------------------
// Full-text search
// ---------------------------------------------------------------------------

#[test]
pub(super) fn search_returns_matching_titles() {
    let s = Sandbox::new();
    create_ticket(&s, "Fix the database connection pool");
    create_ticket(&s, "Improve UI rendering performance");
    create_ticket(&s, "Refactor database migration scripts");

    let results = s.ticket_json(&["search", "database"]);
    assert_eq!(results["status"], "ok");

    let count = results["count"].as_u64().unwrap();
    assert!(
        count >= 2,
        "expected >= 2 results for 'database', got {count}"
    );

    // Both matching titles must appear somewhere in the result set.
    let titles: Vec<&str> = results["results"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|r| r["title"].as_str())
        .collect();
    assert!(
        titles.iter().any(|t| t.contains("database")),
        "at least one result should contain 'database': {titles:?}"
    );
}

#[test]
pub(super) fn search_returns_empty_for_unknown_query() {
    let s = Sandbox::new();
    create_ticket(&s, "Fix the login page");
    create_ticket(&s, "Improve the dashboard");

    let results = s.ticket_json(&["search", "zxqwerty_nonexistent_phrase"]);
    assert_eq!(results["status"], "ok");
    assert_eq!(results["count"].as_u64().unwrap(), 0);
}

// ---------------------------------------------------------------------------
// Scan / reindex
// ---------------------------------------------------------------------------

#[test]
pub(super) fn scan_reindex_preserves_searchability() {
    let s = Sandbox::new();
    create_ticket(&s, "Audit log improvements");
    create_ticket(&s, "Performance benchmark suite");

    // Run a full reindex — rebuilds the Tantivy search index from the
    // filesystem source of truth.
    let scan = s.ticket_json(&["scan", "--reindex"]);
    assert_eq!(scan["status"], "ok");
    assert_eq!(
        scan["integrated"].as_u64().unwrap(),
        2,
        "reindex should re-integrate both tickets"
    );

    // Search must still return the correct result.
    let results = s.ticket_json(&["search", "benchmark"]);
    assert_eq!(results["count"].as_u64().unwrap(), 1);
    assert_eq!(
        results["results"][0]["title"],
        "Performance benchmark suite"
    );
}
