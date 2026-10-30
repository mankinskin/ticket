use super::*;
use crate::{
    model::{edge::EdgeRecord, filesystem::ScanRoot},
    storage::index::RedbIndexStore,
};
use chrono::Utc;
use std::{fs, process::Command};
use tempfile::tempdir;

fn run_git(repo_root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(repo_root)
        .args(args)
        .status()
        .expect("git command");
    assert!(status.success(), "git {args:?} failed: {status}");
}

#[test]
fn preflight_reports_invisible_reference_visibility_and_path_refs() {
    let temp = tempdir().unwrap();
    let repo = temp.path().join("repo");
    fs::create_dir_all(&repo).unwrap();
    run_git(&repo, &["init"]);

    let source_workspace = repo.join("source-workspace");
    let target_workspace = repo.join("target-workspace");
    let docs_dir = repo.join("docs");
    fs::create_dir_all(&source_workspace).unwrap();
    fs::create_dir_all(&target_workspace).unwrap();
    fs::create_dir_all(&docs_dir).unwrap();

    let source_store = TicketStore::init(&source_workspace).unwrap();
    let target_store = TicketStore::init(&target_workspace).unwrap();

    let source_ticket = source_store
        .create(
            None,
            "tracker-improvement",
            Some("source ticket"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    let invisible_inbound = source_store
        .create(
            None,
            "tracker-improvement",
            Some("invisible inbound"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();

    let destination_visible = target_store
        .create(
            None,
            "tracker-improvement",
            Some("destination visible"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();

    source_store
        .add_edge(EdgeRecord {
            from: invisible_inbound,
            to: source_ticket,
            kind: "depends_on".to_string(),
            created_at: Utc::now(),
        })
        .unwrap();
    RedbIndexStore::open(&source_store.index_root.join("tickets.db"))
        .unwrap()
        .insert_edge(&EdgeRecord {
            from: source_ticket,
            to: destination_visible,
            kind: "depends_on".to_string(),
            created_at: Utc::now(),
        })
        .unwrap();

    let source_ticket_path = source_store
        .get_indexed(&source_ticket)
        .unwrap()
        .unwrap()
        .path;
    let relative_ticket_path = source_ticket_path
        .strip_prefix(&repo)
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/");
    let tracked_doc = docs_dir.join("move.md");
    fs::write(&tracked_doc, format!("See {relative_ticket_path}\n")).unwrap();
    run_git(&repo, &["add", "docs/move.md"]);

    let report = source_store
        .plan_move_preflight(&source_ticket, &target_workspace)
        .unwrap();

    assert!(report.supported());
    assert!(report.reference_visibility.iter().any(|entry| {
        entry.related_entity_id == invisible_inbound
            && entry.direction == MoveReferenceDirection::Inbound
            && !entry.visible_from_destination
    }));
    assert!(
        report
            .path_reference_files
            .iter()
            .any(|path| { path.ends_with("docs/move.md") })
    );
    assert!(
        !report
            .blockers
            .iter()
            .any(|blocker| matches!(blocker, MovePreflightBlocker::InvisibleReference { .. }))
    );
}

#[test]
fn preflight_allows_parent_submodule_worktree_topology() {
    let temp = tempdir().unwrap();
    let repo = temp.path().join("repo");
    fs::create_dir_all(&repo).unwrap();
    run_git(&repo, &["init"]);

    let nested_repo = repo.join("nested-repo");
    fs::create_dir_all(&nested_repo).unwrap();
    run_git(&nested_repo, &["init"]);

    let source_store = TicketStore::init(&repo).unwrap();
    let _target_store = TicketStore::init(&nested_repo).unwrap();

    let source_ticket = source_store
        .create(
            None,
            "tracker-improvement",
            Some("source ticket"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();

    let report = source_store
        .plan_move_preflight(&source_ticket, &nested_repo)
        .unwrap();

    assert_eq!(
        report.git_worktree_topology,
        GitWorktreeTopology::ParentToSubmodule
    );
    assert!(
        !report
            .blockers
            .iter()
            .any(|blocker| matches!(blocker, MovePreflightBlocker::DifferentGitWorktree { .. }))
    );
}

#[test]
fn preflight_blocks_unrelated_git_worktrees() {
    let temp = tempdir().unwrap();
    let source_repo = temp.path().join("source-repo");
    let target_repo = temp.path().join("target-repo");
    fs::create_dir_all(&source_repo).unwrap();
    fs::create_dir_all(&target_repo).unwrap();
    run_git(&source_repo, &["init"]);
    run_git(&target_repo, &["init"]);

    let source_store = TicketStore::init(&source_repo).unwrap();
    let _target_store = TicketStore::init(&target_repo).unwrap();

    let source_ticket = source_store
        .create(
            None,
            "tracker-improvement",
            Some("source ticket"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();

    let report = source_store
        .plan_move_preflight(&source_ticket, &target_repo)
        .unwrap();

    assert_eq!(report.git_worktree_topology, GitWorktreeTopology::Unrelated);
    assert!(
        report
            .blockers
            .iter()
            .any(|blocker| matches!(blocker, MovePreflightBlocker::DifferentGitWorktree { .. }))
    );
}

#[test]
fn source_entity_path_requires_path_ownership_not_aggregate_visibility() {
    let temp = tempdir().unwrap();
    let repo = temp.path().join("repo");
    let nested_repo = repo.join("nested-repo");
    std::fs::create_dir_all(&repo).unwrap();
    std::fs::create_dir_all(&nested_repo).unwrap();
    run_git(&repo, &["init"]);
    run_git(&nested_repo, &["init"]);

    let source_store = TicketStore::init(&repo).unwrap();
    let target_store = TicketStore::init(&nested_repo).unwrap();
    source_store
        .add_scan_root(ScanRoot {
            path: nested_repo
                .join(".workflow-tools")
                .join("ticket")
                .join("tickets"),
            label: "nested-tickets".to_string(),
        })
        .unwrap();

    let id = target_store
        .create(
            None,
            "tracker-improvement",
            Some("nested source path ticket"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();

    let target_indexed = target_store.get_indexed(&id).unwrap().unwrap();

    let poisoned_index = RedbIndexStore::open(&source_store.index_root.join("tickets.db")).unwrap();
    poisoned_index.insert_ticket(&target_indexed).unwrap();

    let source_domain = TicketMoveDomain::new(&source_store);
    let target_domain = TicketMoveDomain::new(&target_store);
    assert!(
        move_kernel::MoveDomain::source_entity_path(&source_domain, &id)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        move_kernel::MoveDomain::source_entity_path(&target_domain, &id)
            .unwrap()
            .as_deref(),
        Some(target_indexed.path.as_path())
    );
}
