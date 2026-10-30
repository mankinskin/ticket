#[allow(unused_imports)]
use super::*;

#[test]
pub(super) fn reconcile_known_tickets_is_noop_for_unchanged_ticket_and_unaffected_rows() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();

    let touched = store
        .create(
            None,
            "tracker-improvement",
            Some("Known reconcile touched"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    let unaffected = store
        .create(
            None,
            "tracker-improvement",
            Some("Known reconcile unaffected"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();

    store.scan(true).unwrap();
    let before_touched = store.get_indexed(&touched).unwrap().unwrap().updated_at;
    let before_unaffected = store.get_indexed(&unaffected).unwrap().unwrap().updated_at;

    let report = store.reconcile_known_tickets(&[touched]).unwrap();

    assert_eq!(report.integrated, 1);
    assert_eq!(report.pruned, 0);
    assert_eq!(
        report
            .phase_timings_ms
            .get("targeted_reconcile_known_count"),
        Some(&1)
    );
    assert_eq!(
        store.get_indexed(&touched).unwrap().unwrap().updated_at,
        before_touched
    );
    assert_eq!(
        store.get_indexed(&unaffected).unwrap().unwrap().updated_at,
        before_unaffected
    );
}

#[test]
pub(super) fn reconcile_known_tickets_handles_move_and_updates_affected_dependents() {
    let dir = tempdir().unwrap();
    let source_workspace = dir.path().join("source");
    let target_workspace = dir.path().join("target");
    fs::create_dir_all(&source_workspace).unwrap();
    fs::create_dir_all(&target_workspace).unwrap();

    let source_store = TicketStore::init(&source_workspace).unwrap();
    let target_store = TicketStore::init(&target_workspace).unwrap();

    let blocker = source_store
        .create(
            None,
            "tracker-improvement",
            Some("Moved blocker"),
            Some("done"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    let dependent = source_store
        .create(
            None,
            "tracker-improvement",
            Some("Dependent in source"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    source_store
        .add_edge(EdgeRecord {
            from: dependent,
            to: blocker,
            kind: "depends_on".to_string(),
            created_at: Utc::now(),
        })
        .unwrap();

    source_store.scan(true).unwrap();
    let initial = source_store
        .get_workflow_facts(&dependent)
        .unwrap()
        .unwrap();
    assert_eq!(initial.unresolved_dependency_count, 0);

    let source_path = source_store.get_indexed(&blocker).unwrap().unwrap().path;
    fs::create_dir_all(target_store.index_root.join("tickets")).unwrap();
    let target_path = target_store
        .index_root
        .join("tickets")
        .join(blocker.to_string());
    fs::rename(&source_path, &target_path).unwrap();

    let source_report = source_store.reconcile_known_tickets(&[blocker]).unwrap();
    let target_report = target_store.reconcile_known_tickets(&[blocker]).unwrap();

    assert_eq!(source_report.pruned, 1);
    assert_eq!(source_report.integrated, 0);
    assert_eq!(target_report.integrated, 1);
    assert!(source_store.get_indexed(&blocker).unwrap().is_none());
    assert!(target_store.get_indexed(&blocker).unwrap().is_some());

    let updated = source_store
        .get_workflow_facts(&dependent)
        .unwrap()
        .unwrap();
    assert_eq!(updated.unresolved_dependency_count, 1);
}

#[test]
pub(super) fn scan_force_skips_stale_db_edges_for_missing_ticket_folders() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();
    let source_id = store
        .create(
            None,
            "tracker-improvement",
            Some("Missing source ticket"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    let target_id = store
        .create(
            None,
            "tracker-improvement",
            Some("Remaining target ticket"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();

    let legacy_index = RedbIndexStore::open(&store.index_root.join("tickets.db")).unwrap();
    legacy_index
        .insert_edge(&EdgeRecord {
            from: source_id,
            to: target_id,
            kind: "depends_on".to_string(),
            created_at: Utc::now(),
        })
        .unwrap();

    let source_path = store.get_indexed(&source_id).unwrap().unwrap().path;
    fs::remove_dir_all(&source_path).unwrap();

    store.scan(true).unwrap();

    assert!(store.get_indexed(&source_id).unwrap().is_none());
    assert!(store.edges_from(&source_id).unwrap().is_empty());
    assert!(store.get_indexed(&target_id).unwrap().is_some());
}

#[test]
pub(super) fn scan_force_rebuilds_dependency_edges_from_ticket_manifests() {
    let dir = tempdir().unwrap();
    let index_root;
    let source_id;
    let target_id;

    {
        let store = TicketStore::init(dir.path()).unwrap();
        index_root = store.index_root.clone();
        source_id = store
            .create(
                None,
                "tracker-improvement",
                Some("Source ticket"),
                Some("planning"),
                Default::default(),
                None,
                None,
            )
            .unwrap();
        target_id = store
            .create(
                None,
                "tracker-improvement",
                Some("Target ticket"),
                Some("planning"),
                Default::default(),
                None,
                None,
            )
            .unwrap();

        store
            .add_edge(EdgeRecord {
                from: source_id,
                to: target_id,
                kind: "depends_on".to_string(),
                created_at: Utc::now(),
            })
            .unwrap();

        let manifest = store.get(&source_id).unwrap();
        let targets = manifest
            .extra
            .get("depends_on")
            .and_then(|value| value.as_array())
            .unwrap();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].as_str(), Some(target_id.to_string().as_str()));
    }

    fs::remove_file(index_root.join("tickets.db")).unwrap();
    let _ = fs::remove_file(index_root.join("tickets.db-shm"));
    let _ = fs::remove_file(index_root.join("tickets.db-wal"));
    let _ = fs::remove_dir_all(index_root.join("search_index"));

    let rebuilt = TicketStore::init(&index_root).unwrap();
    rebuilt.scan(true).unwrap();

    let edges = rebuilt.edges_from(&source_id).unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].to, target_id);
    assert_eq!(edges[0].kind, "depends_on");
}

#[test]
pub(super) fn open_or_init_bootstraps_manifest_only_workspace() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();
    let ticket_id = store
        .create(
            None,
            "tracker-improvement",
            Some("Bootstrap ticket store"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();

    let index_root = store.index_root.clone();
    drop(store);

    fs::remove_file(index_root.join("tickets.db")).unwrap();
    let _ = fs::remove_file(index_root.join("tickets.db-shm"));
    let _ = fs::remove_file(index_root.join("tickets.db-wal"));
    let _ = fs::remove_dir_all(index_root.join("search_index"));

    let rebuilt = TicketStore::open_or_init(dir.path()).unwrap();
    let manifest = rebuilt.get(&ticket_id).unwrap();

    assert_eq!(manifest.id, ticket_id);
}
