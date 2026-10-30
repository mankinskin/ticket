use std::collections::BTreeMap;

use super::cmd_get;
use crate::cli::IdArgs;
use ticket_api::storage::store::TicketStore;

#[test]
fn cmd_get_includes_authoritative_ticket_folder_path() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = TicketStore::init(dir.path()).expect("open store");
    let id = store
        .create(
            None,
            "tracker-improvement",
            Some("path output regression"),
            Some("planning"),
            BTreeMap::new(),
            None,
            None,
        )
        .expect("create ticket");
    let indexed = store
        .get_indexed(&id)
        .expect("indexed get")
        .expect("indexed ticket");

    let payload = cmd_get(
        IdArgs {
            id: id.to_string(),
            view: None,
            parts: None,
        },
        &store,
    )
    .expect("cmd_get succeeds");

    assert_eq!(
        payload["ticket"]["path"].as_str(),
        Some(indexed.path.display().to_string().as_str())
    );
}

/// End-to-end CLI proof (ticket 4c7b884e, AC7): `ticket get --view
/// summary` reaches the projection helper and returns exactly the
/// `objective` part, not the raw manifest fields path.
#[test]
fn cmd_get_with_view_summary_projects_to_objective_only() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = TicketStore::init(dir.path()).expect("open store");
    let id = store
        .create(
            None,
            "tracker-improvement",
            Some("cli projection"),
            Some("planning"),
            BTreeMap::new(),
            None,
            Some("objective body"),
        )
        .expect("create ticket");

    let payload = cmd_get(
        IdArgs {
            id: id.to_string(),
            view: Some("summary".to_string()),
            parts: None,
        },
        &store,
    )
    .expect("cmd_get with --view summary succeeds");

    let parts = payload["ticket"]["parts"].as_array().expect("parts array");
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0]["kind"], "objective");
}

/// CLI proof (ticket 4c7b884e, AC3): `--view` and `--parts` together is
/// rejected, not silently favoring one.
#[test]
fn cmd_get_rejects_both_view_and_parts() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = TicketStore::init(dir.path()).expect("open store");
    let id = store
        .create(
            None,
            "tracker-improvement",
            Some("cli projection conflict"),
            Some("planning"),
            BTreeMap::new(),
            None,
            Some("objective body"),
        )
        .expect("create ticket");

    let error = cmd_get(
        IdArgs {
            id: id.to_string(),
            view: Some("summary".to_string()),
            parts: Some("objective".to_string()),
        },
        &store,
    )
    .expect_err("both view and parts must be rejected");

    assert!(error.to_string().contains("both"));
}
