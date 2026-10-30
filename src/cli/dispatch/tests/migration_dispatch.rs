#[allow(unused_imports)]
use super::*;

#[test]
pub(super) fn migrate_part_layout_dispatch_routes_to_runtime() {
    let directory = tempdir().unwrap();
    let repo = directory.path().join("repo");
    let child = repo.join("memory-api");
    std::fs::create_dir_all(&child).unwrap();

    let root_store = TicketStore::init(&repo).unwrap();
    let root_ticket = root_store
        .create(
            None,
            "tracker-improvement",
            Some("Root migration dispatch fixture"),
            Some("planning"),
            BTreeMap::new(),
            None,
            Some("Root ticket part content"),
        )
        .unwrap();
    root_store
        .write_part(
            &root_ticket,
            Uuid::new_v4(),
            "review",
            "Root ticket part content",
            None,
        )
        .unwrap();
    let root_store_root = root_store.index_root.clone();
    drop(root_store);

    let child_store = TicketStore::init(&child).unwrap();
    let child_ticket = child_store
        .create(
            None,
            "tracker-improvement",
            Some("Nested migration dispatch fixture"),
            Some("planning"),
            BTreeMap::new(),
            None,
            Some("Nested ticket part content"),
        )
        .unwrap();
    child_store
        .write_part(
            &child_ticket,
            Uuid::new_v4(),
            "review",
            "Nested ticket part content",
            None,
        )
        .unwrap();
    let child_store_root = child_store.index_root.clone();
    drop(child_store);

    let root_before = snapshot_migration_ticket(&root_store_root, &root_ticket.to_string());
    let child_before = snapshot_migration_ticket(&child_store_root, &child_ticket.to_string());
    let root_store_before = snapshot_directory_files(&root_store_root);
    let child_store_before = snapshot_directory_files(&child_store_root);
    assert!(root_before.keys().any(|path| path.starts_with("parts")));
    assert!(child_before.keys().any(|path| path.starts_with("parts")));

    for (workspace, store_root, ticket_id, report_name, before, store_before) in [
        (
            repo.as_path(),
            root_store_root.as_path(),
            root_ticket.to_string(),
            "root-dry-run.json",
            &root_before,
            &root_store_before,
        ),
        (
            child.as_path(),
            child_store_root.as_path(),
            child_ticket.to_string(),
            "nested-dry-run.json",
            &child_before,
            &child_store_before,
        ),
    ] {
        let report_file = directory.path().join(report_name);
        let payload = dispatch(
            TicketCommandCli::MigratePartLayout(MigratePartLayoutArgs {
                apply: false,
                verify: false,
                undo: false,
                plan_file: None,
                apply_report: None,
                report_file: report_file.clone(),
            }),
            None,
            Some(workspace),
            None,
            true,
            true,
        )
        .unwrap();

        assert_eq!(payload["status"], json!("planned"));
        let expected_store_root = std::fs::canonicalize(store_root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        assert_eq!(payload["report"]["store_root"], json!(expected_store_root));
        let saved_report: serde_json::Value =
            serde_json::from_slice(&std::fs::read(report_file).unwrap()).unwrap();
        assert_eq!(saved_report, payload["report"]);
        assert_eq!(snapshot_migration_ticket(store_root, &ticket_id), *before);
        assert_eq!(snapshot_directory_files(store_root), *store_before);
    }
}

#[test]
pub(super) fn migrate_part_layout_dry_run_does_not_initialize_a_missing_index() {
    let directory = tempdir().unwrap();
    let repo = directory.path().join("repo");
    let store = TicketStore::init(&repo).unwrap();
    let ticket_id = store
        .create(
            None,
            "tracker-improvement",
            Some("Filesystem-only migration fixture"),
            Some("planning"),
            BTreeMap::new(),
            None,
            Some("Ticket content"),
        )
        .unwrap();
    store
        .write_part(
            &ticket_id,
            Uuid::new_v4(),
            "review",
            "Ticket review content",
            None,
        )
        .unwrap();
    let store_root = store.index_root.clone();
    drop(store);

    std::fs::remove_file(store_root.join("tickets.db")).unwrap();
    std::fs::remove_dir_all(store_root.join("search_index")).unwrap();
    let ticket_before = snapshot_migration_ticket(&store_root, &ticket_id.to_string());

    let report_file = directory.path().join("dry-run.json");
    let payload = dispatch(
        TicketCommandCli::MigratePartLayout(MigratePartLayoutArgs {
            apply: false,
            verify: false,
            undo: false,
            plan_file: None,
            apply_report: None,
            report_file: report_file.clone(),
        }),
        None,
        Some(&repo),
        None,
        true,
        true,
    )
    .unwrap();

    assert_eq!(payload["status"], json!("planned"));
    assert_eq!(payload["report"]["scanned_tickets"], json!(1));
    assert!(!store_root.join("tickets.db").exists());
    assert!(!store_root.join("search_index").exists());
    assert_eq!(
        snapshot_migration_ticket(&store_root, &ticket_id.to_string()),
        ticket_before
    );
    assert!(report_file.is_file());
}
