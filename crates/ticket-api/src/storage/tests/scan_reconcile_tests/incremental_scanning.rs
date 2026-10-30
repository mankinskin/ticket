#[allow(unused_imports)]
use super::*;

#[test]
pub(super) fn scan_without_reindex_prunes_removed_scan_root_visibility() {
    let dir = tempdir().unwrap();
    let repo = dir.path().join("repo");
    let child_repo = repo.join("memory-viewers").join("viewer-api");
    fs::create_dir_all(&child_repo).unwrap();

    let root_store = TicketStore::init(&repo).unwrap();
    let ticket_id = {
        let child_store = TicketStore::init(&child_repo).unwrap();
        root_store
            .add_scan_root(ScanRoot {
                path: child_store.index_root.join("tickets"),
                label: "viewer-api".to_string(),
            })
            .unwrap();

        let ticket_id = child_store
            .create(
                None,
                "tracker-improvement",
                Some("Removed scan root ticket"),
                Some("planning"),
                Default::default(),
                None,
                None,
            )
            .unwrap();

        root_store.scan(true).unwrap();
        ticket_id
    };

    let manifest_path = root_store
        .get_indexed(&ticket_id)
        .unwrap()
        .unwrap()
        .path
        .join("ticket.toml");
    assert!(
        root_store
            .search_tickets("Removed scan root", 10)
            .unwrap()
            .iter()
            .any(|result| result.id == ticket_id)
    );

    fs::remove_dir_all(&child_repo).unwrap();

    let report = root_store.scan(false).unwrap();

    assert_eq!(report.pruned, 1);
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diag| { diag.path == manifest_path && diag.reason.contains("missing on disk") })
    );
    assert!(root_store.get_indexed(&ticket_id).unwrap().is_none());
    assert!(root_store.get(&ticket_id).is_err());
    assert!(
        !root_store
            .search_tickets("Removed scan root", 10)
            .unwrap()
            .iter()
            .any(|result| result.id == ticket_id)
    );
    assert!(
        !root_store
            .list(None, None, None)
            .unwrap()
            .iter()
            .any(|ticket| ticket.id == ticket_id)
    );
}

#[test]
pub(super) fn scan_report_includes_phase_timings_and_root_counts() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();
    store
        .create(
            None,
            "tracker-improvement",
            Some("Profile scan timings"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();

    let report = store.scan(true).unwrap();

    assert!(report.phase_timings_ms.contains_key("scan_total_ms"));
    assert!(report.phase_timings_ms.contains_key("list_scan_roots_ms"));
    assert!(
        report
            .phase_timings_ms
            .contains_key("rebuild_workflow_facts_ms")
    );
    assert!(
        report
            .phase_timings_ms
            .contains_key("integration.manifest_parse_ms")
    );
    assert!(
        report
            .phase_timings_ms
            .contains_key("integration.index_upsert_ms")
    );
    assert!(
        report
            .phase_timings_ms
            .contains_key("integration.edge_write_ms")
    );
    assert!(
        report
            .phase_timings_ms
            .contains_key("integration.description_read_ms")
    );
    assert!(
        report
            .phase_timings_ms
            .contains_key("integration.search_upsert_ms")
    );
    assert!(
        report
            .phase_timings_ms
            .contains_key("workflow.fetch_dependency_edges_ms")
    );
    assert!(
        report
            .phase_timings_ms
            .contains_key("workflow.fetch_dependency_tickets_ms")
    );
    assert!(
        report
            .phase_timings_ms
            .contains_key("workflow.compute_unresolved_ms")
    );
    assert!(
        report
            .phase_timings_ms
            .contains_key("workflow.write_facts_ms")
    );
    assert!(
        report
            .phase_timings_ms
            .keys()
            .any(|key| key.starts_with("scan_root_"))
    );
    assert!(!report.root_entry_counts.is_empty());
}

#[test]
pub(super) fn scan_without_reindex_skips_workflow_recompute_when_nothing_changed() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();
    let blocker = store
        .create(
            None,
            "tracker-improvement",
            Some("Stable blocker"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    let dependent = store
        .create(
            None,
            "tracker-improvement",
            Some("Stable dependent"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    store
        .add_edge(EdgeRecord {
            from: dependent,
            to: blocker,
            kind: "depends_on".to_string(),
            created_at: Utc::now(),
        })
        .unwrap();

    store.scan(true).unwrap();
    let report = store.scan(false).unwrap();

    assert_eq!(
        report
            .phase_timings_ms
            .get("workflow.incremental_root_count"),
        Some(&0)
    );
    assert_eq!(
        report
            .phase_timings_ms
            .get("workflow.incremental_affected_count"),
        Some(&0)
    );
    assert!(
        !report
            .phase_timings_ms
            .contains_key("workflow.fetch_dependency_edges_ms")
    );
}

#[test]
pub(super) fn scan_without_reindex_recomputes_workflow_facts_for_changed_ticket_slice() {
    let dir = tempdir().unwrap();
    let repo = dir.path().join("repo");
    let child_repo = repo.join("memory-viewers").join("memory-api");
    fs::create_dir_all(&child_repo).unwrap();

    let root_store = TicketStore::init(&repo).unwrap();
    let child_store = TicketStore::init(&child_repo).unwrap();
    root_store
        .add_scan_root(ScanRoot {
            path: child_store.index_root.join("tickets"),
            label: "memory-api".to_string(),
        })
        .unwrap();

    let blocker = child_store
        .create(
            None,
            "tracker-improvement",
            Some("Changed blocker"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    let dependent = child_store
        .create(
            None,
            "tracker-improvement",
            Some("Changed dependent"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    child_store
        .add_edge(EdgeRecord {
            from: dependent,
            to: blocker,
            kind: "depends_on".to_string(),
            created_at: Utc::now(),
        })
        .unwrap();

    root_store.scan(true).unwrap();
    let initial = root_store.get_workflow_facts(&dependent).unwrap().unwrap();
    assert_eq!(initial.unresolved_dependency_count, 1);

    child_store.close(&blocker, "done", None).unwrap();

    let report = root_store.scan(false).unwrap();
    assert_eq!(
        report
            .phase_timings_ms
            .get("workflow.incremental_root_count"),
        Some(&1)
    );
    assert_eq!(
        report
            .phase_timings_ms
            .get("workflow.incremental_affected_count"),
        Some(&2)
    );

    let updated = root_store.get_workflow_facts(&dependent).unwrap().unwrap();
    assert_eq!(updated.unresolved_dependency_count, 0);
    assert!(updated.became_actionable_at.is_some());
}
