#[allow(unused_imports)]
use super::*;

#[test]
pub(super) fn unlink_removes_existing_edge() {
    let s = Sandbox::new();

    let id_a = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
    let id_b = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";

    s.ticket_json(&[
        "create",
        "--id",
        id_a,
        "--title",
        "A",
        "--type",
        "tracker-improvement",
    ]);
    s.ticket_json(&[
        "create",
        "--id",
        id_b,
        "--title",
        "B",
        "--type",
        "tracker-improvement",
    ]);

    let linked = s.ticket_json(&["link", "--from", id_a, "--to", id_b, "--kind", "depends_on"]);
    assert_eq!(linked["status"], "ok");

    let before = s.ticket_json(&["links", id_a]);
    assert_eq!(before["count"].as_u64().unwrap(), 1);

    let unlinked = s.ticket_json(&[
        "unlink",
        "--from",
        id_a,
        "--to",
        id_b,
        "--kind",
        "depends_on",
    ]);
    assert_eq!(unlinked["status"], "ok");

    let after = s.ticket_json(&["links", id_a]);
    assert_eq!(after["count"].as_u64().unwrap(), 0);
}

#[test]
pub(super) fn unlink_is_idempotent_when_edge_is_missing() {
    let s = Sandbox::new();

    let id_a = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
    let id_b = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";

    s.ticket_json(&[
        "create",
        "--id",
        id_a,
        "--title",
        "A",
        "--type",
        "tracker-improvement",
    ]);
    s.ticket_json(&[
        "create",
        "--id",
        id_b,
        "--title",
        "B",
        "--type",
        "tracker-improvement",
    ]);

    // Missing-edge unlink currently succeeds as a no-op.
    let first = s.ticket_json(&[
        "unlink",
        "--from",
        id_a,
        "--to",
        id_b,
        "--kind",
        "depends_on",
    ]);
    assert_eq!(first["status"], "ok");

    let second = s.ticket_json(&[
        "unlink",
        "--from",
        id_a,
        "--to",
        id_b,
        "--kind",
        "depends_on",
    ]);
    assert_eq!(second["status"], "ok");

    let links = s.ticket_json(&["links", id_a]);
    assert_eq!(links["count"].as_u64().unwrap(), 0);
}

#[test]
pub(super) fn unlink_removes_edge_when_target_ticket_folder_is_missing_fixture() {
    let s = Sandbox::new();

    let id_a = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
    let id_b = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";

    s.ticket_json(&[
        "create",
        "--id",
        id_a,
        "--title",
        "A",
        "--type",
        "tracker-improvement",
    ]);
    s.ticket_json(&[
        "create",
        "--id",
        id_b,
        "--title",
        "B",
        "--type",
        "tracker-improvement",
    ]);

    s.ticket_json(&["link", "--from", id_a, "--to", id_b, "--kind", "depends_on"]);

    // Fixture: remove target ticket folder directly to force a dangling edge.
    let target = s.ticket_json(&["get", id_b]);
    let target_path = target["ticket"]["path"]
        .as_str()
        .map(Path::new)
        .expect("get returns target ticket folder path");
    assert!(
        target_path.is_dir(),
        "target ticket folder must exist before removal: {}",
        target_path.display()
    );
    fs::remove_dir_all(target_path).expect("remove target ticket folder");
    assert!(
        !target_path.exists(),
        "target ticket folder must be removed: {}",
        target_path.display()
    );
    s.ticket_json(&["scan", "--force"]);

    let before = s.ticket_json(&["health", id_a]);
    assert_eq!(before["summary"]["dangling_edge"], 1);

    let unlinked = s.ticket_json(&[
        "unlink",
        "--from",
        id_a,
        "--to",
        "bbbbbbbb",
        "--kind",
        "depends_on",
    ]);
    assert_eq!(unlinked["status"], "ok");

    let after = s.ticket_json(&["health", id_a]);
    assert!(
        after["summary"].get("dangling_edge").is_none(),
        "dangling edge should be removed: {after}"
    );
}

#[test]
pub(super) fn unlink_reports_error_when_source_ticket_folder_is_missing_fixture() {
    let s = Sandbox::new();

    let id_a = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
    let id_b = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";

    s.ticket_json(&[
        "create",
        "--id",
        id_a,
        "--title",
        "A",
        "--type",
        "tracker-improvement",
    ]);
    s.ticket_json(&[
        "create",
        "--id",
        id_b,
        "--title",
        "B",
        "--type",
        "tracker-improvement",
    ]);

    s.ticket_json(&["link", "--from", id_a, "--to", id_b, "--kind", "depends_on"]);

    // Fixture: remove source ticket folder directly to force a dangling edge.
    let source = s.ticket_json(&["get", id_a]);
    let source_path = source["ticket"]["path"]
        .as_str()
        .map(Path::new)
        .expect("get returns source ticket folder path");
    assert!(
        source_path.is_dir(),
        "source ticket folder must exist before removal: {}",
        source_path.display()
    );
    fs::remove_dir_all(source_path).expect("remove source ticket folder");
    assert!(
        !source_path.exists(),
        "source ticket folder must be removed: {}",
        source_path.display()
    );
    s.ticket_json(&["scan", "--force"]);

    let (_code, stderr) = s.ticket_fail(&[
        "unlink",
        "--from",
        id_a,
        "--to",
        id_b,
        "--kind",
        "depends_on",
    ]);
    assert!(
        stderr.contains("entity not found"),
        "expected storage entity-not-found error, got stderr: {stderr}"
    );

    // Missing source no longer appears in list output, so verify globally.
    let all_links = s.ticket_json(&["links", "--all"]);
    let edges = all_links["edges"].as_array().expect("edges array");
    let still_present = edges
        .iter()
        .any(|edge| edge["from"] == id_a && edge["to"] == id_b && edge["kind"] == "depends_on");
    assert!(
        !still_present,
        "edge should be removed from global edge set"
    );
}

#[test]
pub(super) fn update_routes_depends_on_field_patch_and_preserves_unlink_flow() {
    let s = Sandbox::new();

    let id_a = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
    let id_b = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";

    s.ticket_json(&[
        "create",
        "--id",
        id_a,
        "--title",
        "A",
        "--type",
        "tracker-improvement",
    ]);
    s.ticket_json(&[
        "create",
        "--id",
        id_b,
        "--title",
        "B",
        "--type",
        "tracker-improvement",
    ]);

    // Generic update path should route edge payload to canonical graph ops.
    let updated = s.ticket_json(&[
        "update",
        id_a,
        "--field",
        "depends_on=[\"bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb\"]",
    ]);
    assert_eq!(updated["status"], "ok");

    let after_update = s.ticket_json(&["links", id_a]);
    assert_eq!(after_update["count"].as_u64().unwrap(), 1);

    // Canonical graph flow still works through link/unlink.
    let linked = s.ticket_json(&["link", "--from", id_a, "--to", id_b, "--kind", "depends_on"]);
    assert_eq!(linked["status"], "ok");

    let unlinked = s.ticket_json(&[
        "unlink",
        "--from",
        id_a,
        "--to",
        id_b,
        "--kind",
        "depends_on",
    ]);
    assert_eq!(unlinked["status"], "ok");

    let links = s.ticket_json(&["links", id_a]);
    assert_eq!(links["count"].as_u64().unwrap(), 0);
}
