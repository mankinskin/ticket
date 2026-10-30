#[allow(unused_imports)]
use super::*;

#[test]
pub(super) fn dispatch_create_writes_only_to_the_explicit_child_workspace() {
    let dir = tempdir().unwrap();
    let repo = dir.path().join("repo");
    let child = repo.join("selected");
    let sibling = repo.join("sibling");
    std::fs::create_dir_all(&child).unwrap();
    std::fs::create_dir_all(&sibling).unwrap();

    let parent_store = TicketStore::init_in_workspace(&repo).unwrap();
    let ticket_id = Uuid::new_v4();
    let payload = dispatch(
        TicketCommandCli::Create(crate::cli::CreateArgs {
            id: Some(ticket_id),
            ticket_type: Some("tracker-improvement".to_string()),
            title: Some("Child workspace only".to_string()),
            state: None,
            fields: vec![],
            body_file: None,
        }),
        None,
        Some(&child),
        None,
        false,
        false,
    )
    .unwrap();

    assert_eq!(payload["id"], ticket_id.to_string());
    assert!(
        TicketStore::open_in_workspace(&child)
            .unwrap()
            .get(&ticket_id)
            .is_ok()
    );
    assert!(matches!(
        parent_store.get(&ticket_id),
        Err(ticket_api::error::StorageError::NotFound(_))
    ));
    assert!(!ticket_api::workspace::canonical_store_root(&sibling, ".ticket").exists());
    assert!(!ticket_api::workspace::canonical_store_root(dir.path(), ".ticket").exists());
}

#[test]
pub(super) fn dispatch_get_reads_child_ticket_from_explicit_workspace_root() {
    let (_dir, _repo, child, ticket_id) = create_nested_ticket_fixture();

    let payload = dispatch(
        TicketCommandCli::Get(IdArgs {
            id: ticket_id.clone(),
            view: None,
            parts: None,
        }),
        None,
        Some(&child),
        None,
        true,
        false,
    )
    .unwrap();

    assert_eq!(payload["command"], "get");
    assert_eq!(payload["ticket"]["id"], ticket_id);
    assert_eq!(
        payload["ticket"]["fields"]["title"],
        "Nested workspace ticket"
    );
}

#[test]
pub(super) fn dispatch_search_reads_child_ticket_from_explicit_workspace_root() {
    let (_dir, _repo, child, ticket_id) = create_nested_ticket_fixture();

    let payload = dispatch(
        TicketCommandCli::Search(TextArgs {
            expression: "Nested workspace ticket".to_string(),
            limit: 10,
        }),
        None,
        Some(&child),
        None,
        true,
        false,
    )
    .unwrap();

    assert_eq!(payload["command"], "search");
    assert_eq!(payload["count"], 1);
    assert_eq!(payload["results"][0]["id"], ticket_id);
}

#[test]
pub(super) fn dispatch_list_reads_child_ticket_from_explicit_workspace_root() {
    let (_dir, _repo, child, ticket_id) = create_nested_ticket_fixture();

    let payload = dispatch(
        TicketCommandCli::List(ListArgs {
            state: None,
            ticket_type: None,
            limit: Some(10),
            with_repro: false,
            where_clauses: Vec::new(),
        }),
        None,
        Some(&child),
        None,
        true,
        false,
    )
    .unwrap();

    assert_eq!(payload["command"], "list");
    assert_eq!(payload["count"], 1);
    assert_eq!(payload["items"][0]["id"], ticket_id);
}

#[test]
pub(super) fn dispatch_list_repairs_existing_empty_root_index() {
    let dir = tempdir().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();

    let store = TicketStore::init(&repo).unwrap();
    let ticket_id = store
        .create(
            None,
            "tracker-improvement",
            Some("Root workspace ticket"),
            None,
            BTreeMap::<String, serde_json::Value>::new(),
            None,
            Some("Root workspace ticket body"),
        )
        .unwrap();

    let index_root = store.index_root.clone();
    drop(store);

    std::fs::remove_file(index_root.join("tickets.db")).unwrap();
    let _ = std::fs::remove_file(index_root.join("tickets.db-shm"));
    let _ = std::fs::remove_file(index_root.join("tickets.db-wal"));
    let _ = std::fs::remove_dir_all(index_root.join("search_index"));
    RedbIndexStore::open(&index_root.join("tickets.db")).unwrap();

    let payload = dispatch(
        TicketCommandCli::List(ListArgs {
            state: None,
            ticket_type: None,
            limit: Some(10),
            with_repro: false,
            where_clauses: Vec::new(),
        }),
        None,
        Some(&repo),
        None,
        true,
        false,
    )
    .unwrap();

    assert_eq!(payload["command"], "list");
    assert_eq!(payload["count"], 1);
    assert_eq!(payload["items"][0]["id"], ticket_id.to_string());
}

#[test]
pub(super) fn dispatch_scan_registers_child_ticket_from_explicit_workspace_root() {
    let (_dir, repo, _child, ticket_id) = create_nested_ticket_fixture();

    let payload = dispatch(
        TicketCommandCli::Scan(ScanArgs {
            reindex: false,
            force: false,
        }),
        None,
        Some(&repo),
        None,
        true,
        false,
    )
    .unwrap();

    assert_eq!(payload["command"], "scan");

    let root_store = TicketStore::open(&repo.join(".workflow-tools").join("ticket")).unwrap();
    let search_payload = dispatch_store_command(
        TicketCommandCli::Search(TextArgs {
            expression: "Nested workspace ticket".to_string(),
            limit: 10,
        }),
        root_store,
        false,
    )
    .unwrap();

    assert_eq!(search_payload["command"], "search");
    assert_eq!(search_payload["count"], 1);
    assert_eq!(search_payload["results"][0]["id"], ticket_id);
}

#[test]
pub(super) fn dispatch_get_reads_child_ticket_after_scan_root_augmentation() {
    let (_dir, repo, _child, ticket_id) = create_nested_ticket_fixture();
    let root_store = TicketStore::open(&repo.join(".workflow-tools").join("ticket")).unwrap();

    let reindex = register_descendant_scan_roots(&root_store, &repo).unwrap();
    assert!(reindex);
    root_store.scan(true).unwrap();

    let payload = dispatch_store_command(
        TicketCommandCli::Get(IdArgs {
            id: ticket_id.clone(),
            view: None,
            parts: None,
        }),
        root_store,
        false,
    )
    .unwrap();

    assert_eq!(payload["command"], "get");
    assert_eq!(payload["ticket"]["id"], ticket_id);
}

#[test]
pub(super) fn dispatch_search_reads_child_ticket_after_scan_root_augmentation() {
    let (_dir, repo, _child, ticket_id) = create_nested_ticket_fixture();
    let root_store = TicketStore::open(&repo.join(".workflow-tools").join("ticket")).unwrap();

    let reindex = register_descendant_scan_roots(&root_store, &repo).unwrap();
    assert!(reindex);
    root_store.scan(true).unwrap();

    let payload = dispatch_store_command(
        TicketCommandCli::Search(TextArgs {
            expression: "Nested workspace ticket".to_string(),
            limit: 10,
        }),
        root_store,
        false,
    )
    .unwrap();

    assert_eq!(payload["command"], "search");
    assert_eq!(payload["count"], 1);
    assert_eq!(payload["results"][0]["id"], ticket_id);
}
