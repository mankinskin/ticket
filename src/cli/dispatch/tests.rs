use std::collections::BTreeMap;

use super::*;
use crate::cli::{
    BoardArgs, BoardCommand, IdArgs, ListArgs, MigratePartLayoutArgs, ScanArgs, TextArgs,
    WorkspaceArgs, WorkspaceCommand,
};
use tempfile::tempdir;
use ticket_api::storage::index::RedbIndexStore;
use uuid::Uuid;

fn run_git(directory: &Path, arguments: &[&str]) {
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

    let store = TicketStore::init(&main.join(".workflow-tools").join("ticket")).unwrap();
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
    std::fs::create_dir_all(
        worktree
            .join(".workflow-tools")
            .join("ticket")
            .join("tickets"),
    )
    .unwrap();
    std::fs::write(
        worktree
            .join(".workflow-tools")
            .join("ticket")
            .join("tickets")
            .join("copied.txt"),
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
    assert!(
        !worktree
            .join(".workflow-tools")
            .join("ticket")
            .join("tickets.db")
            .exists()
    );
    assert!(
        main.join(".workflow-tools")
            .join("ticket")
            .join("tickets.db")
            .is_file()
    );
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

fn create_nested_ticket_fixture() -> (tempfile::TempDir, PathBuf, PathBuf, String) {
    let dir = tempdir().unwrap();
    let repo = dir.path().join("repo");
    let child = repo.join("memory-api");
    std::fs::create_dir_all(&child).unwrap();

    let _root_store = TicketStore::init(&repo).unwrap();
    let child_store = TicketStore::init(&child).unwrap();
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

fn snapshot_migration_ticket(store_root: &Path, ticket_id: &str) -> BTreeMap<PathBuf, Vec<u8>> {
    let ticket_dir = store_root.join("tickets").join(ticket_id);
    let mut snapshot = BTreeMap::new();
    let manifest = ticket_dir.join("ticket.toml");
    snapshot.insert(
        PathBuf::from("ticket.toml"),
        std::fs::read(manifest).unwrap(),
    );

    let parts_dir = ticket_dir.join("parts");
    for entry in std::fs::read_dir(parts_dir).unwrap() {
        let entry = entry.unwrap();
        let relative_path = PathBuf::from("parts").join(entry.file_name());
        snapshot.insert(relative_path, std::fs::read(entry.path()).unwrap());
    }

    snapshot
}

#[test]
fn migrate_part_layout_dispatch_routes_to_runtime() {
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
    assert!(root_before.keys().any(|path| path.starts_with("parts")));
    assert!(child_before.keys().any(|path| path.starts_with("parts")));

    for (workspace, store_root, ticket_id, report_name, before) in [
        (
            repo.as_path(),
            root_store_root.as_path(),
            root_ticket.to_string(),
            "root-dry-run.json",
            &root_before,
        ),
        (
            child.as_path(),
            child_store_root.as_path(),
            child_ticket.to_string(),
            "nested-dry-run.json",
            &child_before,
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
    }
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
    std::fs::create_dir_all(repo.join(".workflow-tools").join("ticket")).unwrap();
    std::fs::create_dir_all(child.join(".workflow-tools").join("ticket")).unwrap();

    let resolved = resolve_index_root_from(None, Some(&child), None, Some(&repo)).unwrap();

    assert_eq!(resolved, child.join(".workflow-tools").join("ticket"));
}

#[test]
fn create_rejects_ambient_workspace_aliases_before_store_open() {
    for selector in ["", "  ", "default", ".."] {
        let workspace = PathBuf::from(selector);
        let result = dispatch(
            TicketCommandCli::Create(crate::cli::CreateArgs {
                id: None,
                ticket_type: Some("task".to_string()),
                title: Some("Selector validation".to_string()),
                state: None,
                fields: vec![],
                body_file: None,
            }),
            None,
            Some(&workspace),
            None,
            false,
            false,
        );

        let error = result.expect_err("ambient selector must fail");
        assert!(error.to_string().contains("invalid workspace selector"));
    }
}

#[test]
fn init_requires_explicit_workspace_or_store_root() {
    let error = dispatch(TicketCommandCli::Init, None, None, None, false, false)
        .expect_err("init must not select an ambient workspace");

    assert!(
        error
            .to_string()
            .contains("store initialization requires explicit")
    );
}

#[test]
fn resolve_workspace_root_keeps_explicit_nested_workspace_bounded() {
    let dir = tempdir().unwrap();
    let repo = dir.path().join("repo");
    let child = repo.join("file-combiner");
    std::fs::create_dir_all(repo.join(".workflow-tools").join("ticket")).unwrap();
    std::fs::create_dir_all(&child).unwrap();

    let resolved =
        resolve_workspace_root(&repo.join(".workflow-tools").join("ticket"), Some(&child));

    assert_eq!(resolved, child);
}

#[test]
fn dispatch_create_writes_only_to_the_explicit_child_workspace() {
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
fn resolve_index_root_prefers_explicit_index_root_over_workspace_root() {
    let dir = tempdir().unwrap();
    let repo = dir.path().join("repo");
    let child = repo.join("memory-api");
    std::fs::create_dir_all(repo.join(".workflow-tools").join("ticket")).unwrap();
    std::fs::create_dir_all(child.join(".workflow-tools").join("ticket")).unwrap();

    let resolved = resolve_index_root_from(
        Some(&repo.join(".workflow-tools").join("ticket")),
        Some(&child),
        None,
        Some(&repo),
    )
    .unwrap();

    assert_eq!(resolved, repo.join(".workflow-tools").join("ticket"));
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
    std::fs::create_dir_all(repo.join(".workflow-tools").join("ticket")).unwrap();
    std::fs::create_dir_all(child.join(".workflow-tools").join("ticket")).unwrap();

    let resolved = resolve_index_root_from(
        Some(Path::new(".workflow-tools/ticket")),
        Some(&child),
        None,
        Some(&repo),
    )
    .unwrap();

    assert_eq!(resolved, repo.join(".workflow-tools").join("ticket"));
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
fn dispatch_get_reads_child_ticket_after_scan_root_augmentation() {
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
fn dispatch_search_reads_child_ticket_after_scan_root_augmentation() {
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
