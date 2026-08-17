use std::collections::BTreeMap;

use super::*;
use crate::cli::{
    BoardArgs,
    BoardCommand,
    IdArgs,
    ListArgs,
    ScanArgs,
    TextArgs,
    WorkspaceArgs,
    WorkspaceCommand,
};
use tempfile::tempdir;
use ticket_api::storage::index::RedbIndexStore;
use uuid::Uuid;

fn run_git(
    directory: &Path,
    arguments: &[&str],
) {
    let output = std::process::Command::new("git")
        .args(arguments)
        .current_dir(directory)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {} failed: {}",
        arguments.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn create_worktree_fixture() -> (tempfile::TempDir, PathBuf, PathBuf, String) {
    let dir = tempdir().unwrap();
    let main = dir.path().join("main");
    std::fs::create_dir_all(&main).unwrap();
    run_git(dir.path(), &["init", "main"]);
    run_git(&main, &["config", "user.email", "ticket@example.test"]);
    run_git(&main, &["config", "user.name", "Ticket Test"]);
    std::fs::write(main.join("README.md"), "fixture\n").unwrap();
    run_git(&main, &["add", "README.md"]);
    run_git(&main, &["commit", "-m", "fixture"]);

    let store = TicketStore::init(&main.join(".ticket")).unwrap();
    let ticket_id = store
        .create(
            None,
            "task",
            Some("Canonical board fixture"),
            None,
            BTreeMap::new(),
            None,
            None,
        )
        .unwrap();
    drop(store);

    let worktree = main.join(".worktrees").join("session").join("fixture");
    std::fs::create_dir_all(worktree.parent().unwrap()).unwrap();
    run_git(
        &main,
        &[
            "worktree",
            "add",
            "--detach",
            worktree.to_str().unwrap(),
            "HEAD",
        ],
    );
    std::fs::create_dir_all(worktree.join(".ticket").join("tickets")).unwrap();
    std::fs::write(
        worktree.join(".ticket").join("tickets").join("copied.txt"),
        "populated but indexless\n",
    )
    .unwrap();

    (dir, main, worktree, ticket_id.to_string())
}

fn board_check_in(ticket_id: String) -> TicketCommandCli {
    TicketCommandCli::Board(BoardArgs {
        command: BoardCommand::CheckIn {
            id: ticket_id,
            agent: "test-agent".to_string(),
            intent: None,
            files: Vec::new(),
            ttl_secs: None,
            session_id: None,
            worktree_path: None,
            branch: None,
        },
    })
}

#[test]
fn dispatch_board_check_in_uses_main_store_for_managed_worktree() {
    let (_dir, main, worktree, ticket_id) = create_worktree_fixture();

    let payload = dispatch(
        board_check_in(ticket_id.clone()),
        None,
        Some(&worktree),
        None,
        true,
        false,
    )
    .unwrap();

    assert_eq!(payload["ticket_id"], ticket_id);
    assert!(!worktree.join(".ticket").join("tickets.db").exists());
    assert!(main.join(".ticket").join("tickets.db").is_file());
    let store = TicketStore::open(&main).unwrap();
    assert_eq!(store.board_show(None).unwrap().entries.len(), 1);
}

#[test]
fn dispatch_board_check_in_keeps_main_workspace_routing() {
    let (_dir, main, _worktree, ticket_id) = create_worktree_fixture();

    dispatch(
        board_check_in(ticket_id),
        None,
        Some(&main),
        None,
        true,
        false,
    )
    .unwrap();

    let store = TicketStore::open(&main).unwrap();
    assert_eq!(store.board_show(None).unwrap().entries.len(), 1);
}

#[test]
fn dispatch_board_rejects_malformed_worktree_metadata() {
    let dir = tempdir().unwrap();
    let workspace = dir.path().join("worktree");
    std::fs::create_dir_all(&workspace).unwrap();
    std::fs::write(workspace.join(".git"), "gitdir: missing\n").unwrap();

    let error = dispatch(
        board_check_in(Uuid::new_v4().to_string()),
        None,
        Some(&workspace),
        None,
        true,
        false,
    )
    .unwrap_err();

    assert!(error.to_string().contains("invalid board workspace"));
}

fn create_nested_ticket_fixture()
-> (tempfile::TempDir, PathBuf, PathBuf, String) {
    let dir = tempdir().unwrap();
    let repo = dir.path().join("repo");
    let child = repo.join("memory-api");
    std::fs::create_dir_all(&child).unwrap();

    let _root_store = TicketStore::init(&repo.join(".ticket")).unwrap();
    let child_store = TicketStore::init(&child.join(".ticket")).unwrap();
    let ticket_id = child_store
        .create(
            None,
            "tracker-improvement",
            Some("Nested workspace ticket"),
            None,
            BTreeMap::<String, serde_json::Value>::new(),
            None,
            Some("Nested workspace ticket body"),
        )
        .unwrap();

    (dir, repo, child, ticket_id.to_string())
}

#[test]
fn dry_run_payload_is_returned_for_mutating_command() {
    let payload = dry_run_command_payload(&TicketCommandCli::Delete(IdArgs {
        id: Uuid::new_v4().to_string(),
        view: None,
        parts: None,
    }))
    .expect("delete should be dry-runnable");
    assert_eq!(payload["dry_run"], json!(true));
    assert_eq!(payload["command"], json!("delete"));
}

#[test]
fn dry_run_payload_is_none_for_read_only_command() {
    let payload = dry_run_command_payload(&TicketCommandCli::Leases);
    assert!(payload.is_none());
}

#[test]
fn resolve_index_root_prefers_explicit_workspace_root() {
    let dir = tempdir().unwrap();
    let repo = dir.path().join("repo");
    let child = repo.join("memory-api");
    std::fs::create_dir_all(repo.join(".ticket")).unwrap();
    std::fs::create_dir_all(child.join(".ticket")).unwrap();

    let resolved =
        resolve_index_root_from(None, Some(&child), None, Some(&repo));

    assert_eq!(resolved, child.join(".ticket"));
}

#[test]
fn resolve_index_root_prefers_explicit_index_root_over_workspace_root() {
    let dir = tempdir().unwrap();
    let repo = dir.path().join("repo");
    let child = repo.join("memory-api");
    std::fs::create_dir_all(repo.join(".ticket")).unwrap();
    std::fs::create_dir_all(child.join(".ticket")).unwrap();

    let resolved = resolve_index_root_from(
        Some(&repo.join(".ticket")),
        Some(&child),
        None,
        Some(&repo),
    );

    assert_eq!(resolved, repo.join(".ticket"));
}

#[test]
fn dispatch_explicit_index_root_overrides_workspace_root() {
    let dir = tempdir().unwrap();
    let repo = dir.path().join("repo");
    let child = repo.join("memory-api");
    std::fs::create_dir_all(&child).unwrap();
    let root_store = TicketStore::init(&repo).unwrap();
    let child_store = TicketStore::init(&child).unwrap();
    let ticket_id = Uuid::new_v4();
    root_store
        .create(
            Some(ticket_id),
            "tracker-improvement",
            Some("Root ticket"),
            None,
            BTreeMap::new(),
            None,
            None,
        )
        .unwrap();
    child_store
        .create(
            Some(ticket_id),
            "tracker-improvement",
            Some("Child ticket"),
            None,
            BTreeMap::new(),
            None,
            None,
        )
        .unwrap();

    let payload = dispatch(
        TicketCommandCli::Get(IdArgs {
            id: ticket_id.to_string(),
            view: None,
            parts: None,
        }),
        Some(&root_store.index_root),
        Some(&child),
        None,
        true,
        false,
    )
    .unwrap();

    assert_eq!(payload["ticket"]["fields"]["title"], "Root ticket");
}

#[test]
fn dispatch_workspace_roots_and_prune_roots_return_machine_output() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();

    let roots = dispatch_store_command(
        TicketCommandCli::Workspace(WorkspaceArgs {
            command: WorkspaceCommand::Roots,
        }),
        store,
        false,
    )
    .unwrap();
    assert_eq!(roots["command"], "workspace_roots");
    assert_eq!(roots["status"], "ok");
    assert!(!roots["roots"].as_array().unwrap().is_empty());

    let store = TicketStore::open(dir.path()).unwrap();
    let pruned = dispatch_store_command(
        TicketCommandCli::Workspace(WorkspaceArgs {
            command: WorkspaceCommand::PruneRoots,
        }),
        store,
        false,
    )
    .unwrap();
    assert_eq!(pruned["command"], "workspace_prune_roots");
    assert_eq!(pruned["pruned"], 0);
}

#[test]
fn resolve_index_root_preserves_relative_explicit_index_root() {
    let dir = tempdir().unwrap();
    let repo = dir.path().join("repo");
    let child = repo.join("memory-api");
    std::fs::create_dir_all(repo.join(".ticket")).unwrap();
    std::fs::create_dir_all(child.join(".ticket")).unwrap();

    let resolved = resolve_index_root_from(
        Some(Path::new(".ticket")),
        Some(&child),
        None,
        Some(&repo),
    );

    assert_eq!(resolved, repo.join(".ticket"));
}

#[test]
fn dispatch_get_reads_child_ticket_from_explicit_workspace_root() {
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
fn dispatch_search_reads_child_ticket_from_explicit_workspace_root() {
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
fn dispatch_list_reads_child_ticket_from_explicit_workspace_root() {
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
fn dispatch_list_repairs_existing_empty_root_index() {
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
fn dispatch_scan_registers_child_ticket_from_explicit_workspace_root() {
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

    let root_store = TicketStore::open(&repo.join(".ticket")).unwrap();
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
fn dispatch_get_reads_child_ticket_after_scan_root_augmentation() {
    let (_dir, repo, _child, ticket_id) = create_nested_ticket_fixture();
    let root_store = TicketStore::open(&repo.join(".ticket")).unwrap();

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
fn dispatch_search_reads_child_ticket_after_scan_root_augmentation() {
    let (_dir, repo, _child, ticket_id) = create_nested_ticket_fixture();
    let root_store = TicketStore::open(&repo.join(".ticket")).unwrap();

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
