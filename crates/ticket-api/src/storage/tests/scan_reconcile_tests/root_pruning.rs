#[allow(unused_imports)]
use super::*;

#[test]
pub(super) fn add_scan_root_rejects_sibling_worktree_root() {
    let dir = tempdir().unwrap();
    let repo = dir.path().join("repo");
    let store = TicketStore::init(&repo).unwrap();
    let sibling_root = repo
        .join(".worktrees")
        .join("sibling")
        .join(".workflow-tools")
        .join("ticket")
        .join("tickets");

    let error = store
        .add_scan_root(ScanRoot {
            path: sibling_root,
            label: "sibling".to_string(),
        })
        .unwrap_err();

    assert!(error.to_string().contains(".worktrees outside store root"));
}

#[test]
pub(super) fn add_scan_root_allows_own_worktree_root() {
    let dir = tempdir().unwrap();
    let worktree_store = dir
        .path()
        .join("repo")
        .join(".worktrees")
        .join("own")
        .join(".workflow-tools")
        .join("ticket");
    let store = TicketStore::init(&worktree_store).unwrap();
    let root = worktree_store.join("additional-tickets");

    store
        .add_scan_root(ScanRoot {
            path: root.clone(),
            label: "own".to_string(),
        })
        .unwrap();

    assert!(
        store
            .list_scan_roots()
            .unwrap()
            .iter()
            .any(|scan_root| scan_root.path == root)
    );
}

#[test]
pub(super) fn open_reconciles_deleted_worktree_indexed_ticket_to_main_store() {
    use memory_kernel::storage::index::RedbIndexStore;

    let dir = tempdir().unwrap();
    let repo = dir.path().join("repo");
    let store = TicketStore::init(&repo).unwrap();
    let ticket_id = store
        .create(
            None,
            "tracker-improvement",
            Some("Main store ticket"),
            Some("in-review"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    let index_root = store.index_root.clone();
    let mut stale_indexed = store.get_indexed(&ticket_id).unwrap().unwrap();
    let stale_root = repo
        .join(".worktrees")
        .join("deleted-worktree")
        .join(".workflow-tools")
        .join("ticket")
        .join("tickets");
    stale_indexed.path = stale_root.join(ticket_id.to_string());
    drop(store);

    let index = RedbIndexStore::open(&index_root.join("tickets.db")).unwrap();
    index
        .add_scan_root(&ScanRoot {
            path: stale_root.clone(),
            label: "deleted-worktree".to_string(),
        })
        .unwrap();
    index.upsert_tickets_batch(&[stale_indexed]).unwrap();
    drop(index);

    let reopened = TicketStore::open(&index_root).unwrap();
    assert!(
        !reopened
            .list_scan_roots()
            .unwrap()
            .iter()
            .any(|root| root.path == stale_root)
    );
    assert_eq!(
        reopened
            .get(&ticket_id)
            .unwrap()
            .extra
            .get("state")
            .and_then(|value| value.as_str()),
        Some("in-review")
    );
}

#[test]
pub(super) fn scan_force_prunes_row_for_physically_removed_ticket() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();
    let ticket_id = store
        .create(
            None,
            "tracker-improvement",
            Some("Removed from disk"),
            Some("in-review"),
            Default::default(),
            None,
            None,
        )
        .unwrap();

    let ticket_path = store.get_indexed(&ticket_id).unwrap().unwrap().path;
    fs::remove_dir_all(&ticket_path).unwrap();

    store.scan(true).unwrap();

    assert!(store.get_indexed(&ticket_id).unwrap().is_none());
}

#[test]
pub(super) fn scan_force_prunes_empty_uuid_artifact_folder_without_manifest() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();
    let artifact_id = Uuid::parse_str("4ea42273-a134-4342-b601-1759df6d562f").unwrap();
    let artifact_dir = store
        .index_root
        .join("tickets")
        .join(artifact_id.to_string());
    fs::create_dir_all(&artifact_dir).unwrap();
    assert!(artifact_dir.exists());
    assert!(!artifact_dir.join("ticket.toml").exists());

    let report = store.scan(true).unwrap();

    assert!(
        !artifact_dir.exists(),
        "scan should prune empty artifact dirs"
    );
    assert!(
        !report
            .diagnostics
            .iter()
            .any(|diag| diag.path == artifact_dir.join("ticket.toml")
                && diag.reason.contains("missing ticket.toml"))
    );
}

#[test]
pub(super) fn scan_without_reindex_prunes_deleted_nested_ticket_from_search_and_index() {
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

    let ticket_id = child_store
        .create(
            None,
            "tracker-improvement",
            Some("Deleted nested visibility ticket"),
            Some("in-review"),
            Default::default(),
            None,
            None,
        )
        .unwrap();

    root_store.scan(true).unwrap();
    assert!(
        root_store
            .search_tickets("Deleted nested visibility", 10)
            .unwrap()
            .iter()
            .any(|result| result.id == ticket_id)
    );

    let ticket_path = child_store.get_indexed(&ticket_id).unwrap().unwrap().path;
    let parent_path = ticket_path.parent().unwrap().to_path_buf();
    fs::remove_dir_all(&ticket_path).unwrap();

    let report = root_store.scan(false).unwrap();

    assert_eq!(report.pruned, 1);
    assert!(report.diagnostics.iter().any(|diag| {
        diag.path.starts_with(&parent_path) && diag.reason.contains("missing on disk")
    }));
    assert!(root_store.get_indexed(&ticket_id).unwrap().is_none());
    assert!(root_store.get(&ticket_id).is_err());
    assert!(
        !root_store
            .search_tickets("Deleted nested visibility", 10)
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
