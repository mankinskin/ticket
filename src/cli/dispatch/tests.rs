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

fn snapshot_directory_files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, current: &Path, snapshot: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in std::fs::read_dir(current).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                visit(root, &path, snapshot);
            } else {
                snapshot.insert(
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    std::fs::read(path).unwrap(),
                );
            }
        }
    }

    let mut snapshot = BTreeMap::new();
    visit(root, root, &mut snapshot);
    snapshot
}

#[path = "tests/worktree_git_routing.rs"]
mod worktree_git_routing;
#[allow(unused_imports)]
pub use worktree_git_routing::*;

#[path = "tests/migration_dispatch.rs"]
mod migration_dispatch;
#[allow(unused_imports)]
pub use migration_dispatch::*;

#[path = "tests/command_outputs.rs"]
mod command_outputs;
#[allow(unused_imports)]
pub use command_outputs::*;

#[path = "tests/workspace_root_resolution.rs"]
mod workspace_root_resolution;
#[allow(unused_imports)]
pub use workspace_root_resolution::*;

#[path = "tests/explicit_workspace_reads.rs"]
mod explicit_workspace_reads;
#[allow(unused_imports)]
pub use explicit_workspace_reads::*;
