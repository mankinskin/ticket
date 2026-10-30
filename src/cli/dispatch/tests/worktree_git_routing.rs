#[allow(unused_imports)]
use super::*;

#[test]
pub(super) fn dispatch_board_check_in_uses_main_store_for_managed_worktree() {
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
pub(super) fn dispatch_board_check_in_keeps_main_workspace_routing() {
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
pub(super) fn dispatch_board_check_in_keeps_submodule_store_without_commondir() {
    let (_dir, main, _worktree, ticket_id) = create_worktree_fixture();
    let git_dir = main.parent().unwrap().join("submodule-git-dir");
    std::fs::rename(main.join(".git"), &git_dir).unwrap();
    std::fs::write(main.join(".git"), "gitdir: ../submodule-git-dir\n").unwrap();

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
pub(super) fn dispatch_board_check_in_keeps_separate_git_dir_store_without_commondir() {
    let (_dir, main, _worktree, ticket_id) = create_worktree_fixture();
    let git_dir = main.parent().unwrap().join("separate-git-dir");
    std::fs::rename(main.join(".git"), &git_dir).unwrap();
    std::fs::write(
        main.join(".git"),
        format!("gitdir: {}\n", git_dir.display()),
    )
    .unwrap();

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
pub(super) fn dispatch_board_rejects_worktree_missing_commondir() {
    let (_dir, _main, worktree, ticket_id) = create_worktree_fixture();
    let git_dir = resolve_worktree_git_dir(&worktree, &worktree.join(".git")).unwrap();
    std::fs::remove_file(git_dir.join("commondir")).unwrap();

    let error = dispatch(
        board_check_in(ticket_id),
        None,
        Some(&worktree),
        None,
        true,
        false,
    )
    .unwrap_err();

    assert!(error.to_string().contains("Git worktree has no commondir"));
}

#[test]
pub(super) fn dispatch_board_rejects_empty_commondir() {
    let (_dir, _main, worktree, ticket_id) = create_worktree_fixture();
    let git_dir = resolve_worktree_git_dir(&worktree, &worktree.join(".git")).unwrap();
    std::fs::write(git_dir.join("commondir"), "").unwrap();

    let error = dispatch(
        board_check_in(ticket_id),
        None,
        Some(&worktree),
        None,
        true,
        false,
    )
    .unwrap_err();

    assert!(error.to_string().contains("empty Git commondir"));
}

#[test]
pub(super) fn dispatch_board_rejects_empty_git_dir_without_commondir() {
    let dir = tempdir().unwrap();
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("git-dir")).unwrap();
    std::fs::write(workspace.join(".git"), "gitdir: git-dir\n").unwrap();

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

#[test]
pub(super) fn dispatch_board_rejects_malformed_worktree_metadata() {
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
