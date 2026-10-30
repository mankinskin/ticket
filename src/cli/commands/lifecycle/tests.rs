use super::*;
use std::process::Command;
use tempfile::tempdir;

fn run_git(repo_root: &std::path::Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(repo_root)
        .args(args)
        .status()
        .expect("git command");
    assert!(status.success(), "git {args:?} failed: {status}");
}

#[test]
fn cmd_move_dry_run_returns_preflight_plan() {
    let temp = tempdir().unwrap();
    let repo = temp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    run_git(&repo, &["init"]);

    let source_workspace = repo.join("source");
    let target_workspace = repo.join("target");
    std::fs::create_dir_all(&source_workspace).unwrap();
    std::fs::create_dir_all(&target_workspace).unwrap();

    let source_store = TicketStore::init(&source_workspace).unwrap();
    let _target_store = TicketStore::init(&target_workspace).unwrap();

    let id = source_store
        .create(
            None,
            "tracker-improvement",
            Some("move me"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();

    let payload = cmd_move(
        MoveArgs {
            id: Some(id.to_string()),
            to_workspace_root: Some(target_workspace),
            resume: None,
            rollback: None,
            dry_run: true,
        },
        &source_store,
        false,
    )
    .unwrap();

    assert_eq!(payload["command"], "move");
    assert_eq!(payload["mode"], "plan");
    assert_eq!(payload["dry_run"], true);
    assert!(payload["plan"]["source_ticket_path"].is_string());
    assert!(payload["plan"]["destination_ticket_path"].is_string());
}

#[test]
fn cmd_move_rejects_ambient_workspace_aliases() {
    let temp = tempdir().unwrap();
    let repo = temp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    run_git(&repo, &["init"]);
    let source_store = TicketStore::init(&repo).unwrap();
    source_store
        .create(
            None,
            "tracker-improvement",
            Some("Reject invalid move target"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();

    for selector in ["", "  ", "default", ".."] {
        let result = cmd_move(
            MoveArgs {
                id: Some("11111111-1111-4111-8111-111111111111".to_string()),
                to_workspace_root: Some(std::path::PathBuf::from(selector)),
                resume: None,
                rollback: None,
                dry_run: true,
            },
            &source_store,
            false,
        );
        assert!(result.is_err(), "selector {selector:?} must be rejected");
    }
}

#[test]
fn cmd_move_dry_run_returns_preflight_plan_for_ticket_set() {
    let temp = tempdir().unwrap();
    let repo = temp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    run_git(&repo, &["init"]);

    let source_workspace = repo.join("source");
    let target_workspace = repo.join("target");
    std::fs::create_dir_all(&source_workspace).unwrap();
    std::fs::create_dir_all(&target_workspace).unwrap();

    let source_store = TicketStore::init(&source_workspace).unwrap();
    let _target_store = TicketStore::init(&target_workspace).unwrap();
    let first_id = source_store
        .create(
            None,
            "tracker-improvement",
            Some("first ticket"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    let second_id = source_store
        .create(
            None,
            "tracker-improvement",
            Some("second ticket"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();

    let payload = cmd_move(
        MoveArgs {
            id: Some(format!("{first_id},{second_id}")),
            to_workspace_root: Some(target_workspace),
            resume: None,
            rollback: None,
            dry_run: true,
        },
        &source_store,
        false,
    )
    .unwrap();

    assert_eq!(payload["command"], "move");
    assert_eq!(payload["mode"], "plan");
    assert_eq!(payload["ticket_ids"].as_array().unwrap().len(), 2);
    assert_eq!(payload["plan"]["entity_plans"].as_array().unwrap().len(), 2);
}
