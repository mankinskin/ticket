use super::*;
fn run_scan_reconciliation_visibility_agreement(reindex: bool) {
    let dir = tempdir().unwrap();
    let repo = dir.path().join("repo");
    let child_repo_a = repo.join("memory-viewers").join("memory-api");
    let child_repo_b = repo.join("memory-viewers").join("viewer-api");
    fs::create_dir_all(&child_repo_a).unwrap();
    fs::create_dir_all(&child_repo_b).unwrap();

    let root_store = TicketStore::init(&repo).unwrap();
    let child_store_a = TicketStore::init(&child_repo_a).unwrap();
    let child_store_b = TicketStore::init(&child_repo_b).unwrap();
    root_store
        .add_scan_root(ScanRoot {
            path: child_store_a.index_root.join("tickets"),
            label: "memory-api".to_string(),
        })
        .unwrap();
    root_store
        .add_scan_root(ScanRoot {
            path: child_store_b.index_root.join("tickets"),
            label: "viewer-api".to_string(),
        })
        .unwrap();

    let stable_id = child_store_a
        .create(
            None,
            "tracker-improvement",
            Some("VisibilityFixture stable"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    let delete_id = child_store_a
        .create(
            None,
            "tracker-improvement",
            Some("VisibilityFixture delete"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    let move_id = child_store_b
        .create(
            None,
            "tracker-improvement",
            Some("VisibilityFixture move"),
            Some("in-review"),
            Default::default(),
            None,
            None,
        )
        .unwrap();

    root_store.scan(reindex).unwrap();

    let add_id = child_store_b
        .create(
            None,
            "tracker-improvement",
            Some("VisibilityFixture add"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    let known_ids = vec![stable_id, delete_id, move_id, add_id];

    assert_visibility_surfaces_agree(
        &root_store,
        "VisibilityFixture",
        &known_ids,
        &[stable_id, delete_id, move_id],
    );

    root_store.scan(reindex).unwrap();
    assert_visibility_surfaces_agree(
        &root_store,
        "VisibilityFixture",
        &known_ids,
        &[stable_id, delete_id, move_id, add_id],
    );

    let mut stable_patch = BTreeMap::new();
    stable_patch.insert(
        "title".to_string(),
        Value::String("VisibilityFixture stable updated".to_string()),
    );
    child_store_a
        .update(
            &stable_id,
            stable_patch,
            Some(&[]),
            Some("in-implementation"),
            None,
            None,
        )
        .unwrap();

    root_store.scan(reindex).unwrap();
    assert_visibility_surfaces_agree(
        &root_store,
        "VisibilityFixture",
        &known_ids,
        &[stable_id, delete_id, move_id, add_id],
    );
    assert_ticket_title_and_state(
        &root_store,
        "VisibilityFixture",
        stable_id,
        "VisibilityFixture stable updated",
        "in-implementation",
    );

    let mut move_patch = BTreeMap::new();
    move_patch.insert(
        "title".to_string(),
        Value::String("VisibilityFixture move repaired".to_string()),
    );
    child_store_b
        .update(
            &move_id,
            move_patch,
            Some(&[]),
            Some("in-implementation"),
            None,
            None,
        )
        .unwrap();
    let expected_move = child_store_b.get_indexed(&move_id).unwrap().unwrap();

    let poisoned_index = RedbIndexStore::open(&root_store.index_root.join("tickets.db")).unwrap();
    let mut poisoned = root_store.get_indexed(&move_id).unwrap().unwrap();
    poisoned.path = root_store
        .index_root
        .join("tickets")
        .join(move_id.to_string());
    poisoned.type_id = "wrong-type".to_string();
    poisoned.title = Some("Wrong title".to_string());
    poisoned.state = Some("planning".to_string());
    poisoned.created_at = expected_move.created_at - Duration::days(1);
    poisoned_index.insert_ticket(&poisoned).unwrap();

    root_store.scan(reindex).unwrap();
    assert_visibility_surfaces_agree(
        &root_store,
        "VisibilityFixture",
        &known_ids,
        &[stable_id, delete_id, move_id, add_id],
    );
    let repaired_move = root_store.get_indexed(&move_id).unwrap().unwrap();
    assert_eq!(repaired_move.path, expected_move.path);
    assert_eq!(repaired_move.title, expected_move.title);
    assert_eq!(repaired_move.state, expected_move.state);
    assert_ticket_title_and_state(
        &root_store,
        "VisibilityFixture",
        move_id,
        "VisibilityFixture move repaired",
        "in-implementation",
    );

    let delete_path = child_store_a.get_indexed(&delete_id).unwrap().unwrap().path;
    fs::remove_dir_all(&delete_path).unwrap();

    root_store.scan(reindex).unwrap();
    assert_visibility_surfaces_agree(
        &root_store,
        "VisibilityFixture",
        &known_ids,
        &[stable_id, move_id, add_id],
    );

    fs::remove_dir_all(&child_repo_b).unwrap();

    root_store.scan(reindex).unwrap();
    assert_visibility_surfaces_agree(&root_store, "VisibilityFixture", &known_ids, &[stable_id]);
    assert_ticket_title_and_state(
        &root_store,
        "VisibilityFixture",
        stable_id,
        "VisibilityFixture stable updated",
        "in-implementation",
    );
}

#[path = "scan_visibility_tests/nested_workspace_visibility.rs"]
mod nested_workspace_visibility;
#[allow(unused_imports)]
pub use nested_workspace_visibility::*;
