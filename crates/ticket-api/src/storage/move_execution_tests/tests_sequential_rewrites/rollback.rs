#[allow(unused_imports)]
use super::*;

#[test]
pub(super) fn rollback_clears_rewrites_and_unblocks_next_sequential_move() {
    let temp = tempdir().unwrap();
    let repo = temp.path().join("repo");
    let nested_repo = repo.join("nested-repo");
    std::fs::create_dir_all(&repo).unwrap();
    std::fs::create_dir_all(&nested_repo).unwrap();
    run_git(&repo, &["init"]);
    run_git(&nested_repo, &["init"]);

    let source_store = TicketStore::init(&repo).unwrap();
    let _target_store = TicketStore::init(&nested_repo).unwrap();

    let first = source_store
        .create(
            None,
            "tracker-improvement",
            Some("first move"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    let second = source_store
        .create(
            None,
            "tracker-improvement",
            Some("second move"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();

    let mut first_plan = source_store
        .plan_move_preflight(&first, &nested_repo)
        .unwrap();
    let second_plan = source_store
        .plan_move_preflight(&second, &nested_repo)
        .unwrap();

    let first_source_rel = first_plan
        .source_entity_path
        .strip_prefix(&repo)
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/");
    let second_source_rel = second_plan
        .source_entity_path
        .strip_prefix(&repo)
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/");

    let doc_path = repo.join("docs").join("shared-spec-rollback.md");
    std::fs::create_dir_all(doc_path.parent().unwrap()).unwrap();
    std::fs::write(
        &doc_path,
        format!(
            "first reference: {}\nsecond reference: {}\n",
            first_source_rel, second_source_rel
        ),
    )
    .unwrap();
    git_commit_path(
        &repo,
        "docs/shared-spec-rollback.md",
        "seed shared refs for rollback",
    );

    first_plan = source_store
        .plan_move_preflight(&first, &nested_repo)
        .unwrap();
    first_plan.blockers.retain(|blocker| {
        !matches!(
            blocker,
            MovePreflightBlocker::PathReferenceScanUnavailable { .. }
                | MovePreflightBlocker::DirtyTrackedFiles { .. }
        )
    });

    let first_outcome = source_store.execute_move_with_journal(&first_plan).unwrap();
    let _rolled_back = source_store
        .rollback_move_with_journal(first_outcome.journal.id)
        .unwrap();

    let second_after_rollback = source_store
        .plan_move_preflight(&second, &nested_repo)
        .unwrap();
    assert!(
        !second_after_rollback
            .blockers
            .iter()
            .any(|blocker| matches!(blocker, MovePreflightBlocker::DirtyTrackedFiles { .. }))
    );
}

#[test]
pub(super) fn rollback_of_second_move_preserves_first_move_rewrites() {
    let temp = tempdir().unwrap();
    let repo = temp.path().join("repo");
    let nested_repo = repo.join("nested-repo");
    std::fs::create_dir_all(&repo).unwrap();
    std::fs::create_dir_all(&nested_repo).unwrap();
    run_git(&repo, &["init"]);
    run_git(&nested_repo, &["init"]);

    let source_store = TicketStore::init(&repo).unwrap();
    let _target_store = TicketStore::init(&nested_repo).unwrap();

    let first = source_store
        .create(
            None,
            "tracker-improvement",
            Some("first move"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    let second = source_store
        .create(
            None,
            "tracker-improvement",
            Some("second move"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();

    let mut first_plan = source_store
        .plan_move_preflight(&first, &nested_repo)
        .unwrap();
    let second_plan = source_store
        .plan_move_preflight(&second, &nested_repo)
        .unwrap();

    let first_source_rel = first_plan
        .source_entity_path
        .strip_prefix(&repo)
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/");
    let second_source_rel = second_plan
        .source_entity_path
        .strip_prefix(&repo)
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/");
    let doc_path = repo.join("docs").join("shared-spec-sequential-rollback.md");
    std::fs::create_dir_all(doc_path.parent().unwrap()).unwrap();
    std::fs::write(
        &doc_path,
        format!(
            "first reference: {}\nsecond reference: {}\n",
            first_source_rel, second_source_rel
        ),
    )
    .unwrap();
    git_commit_path(
        &repo,
        "docs/shared-spec-sequential-rollback.md",
        "seed shared refs for sequential rollback",
    );

    first_plan = source_store
        .plan_move_preflight(&first, &nested_repo)
        .unwrap();
    first_plan.blockers.retain(|blocker| {
        !matches!(
            blocker,
            MovePreflightBlocker::PathReferenceScanUnavailable { .. }
                | MovePreflightBlocker::DirtyTrackedFiles { .. }
        )
    });
    let first_destination_rel = first_plan
        .destination_entity_path
        .strip_prefix(&repo)
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/");
    let first_outcome = source_store.execute_move_with_journal(&first_plan).unwrap();

    let second_after_first = source_store
        .plan_move_preflight(&second, &nested_repo)
        .unwrap();
    let second_destination_rel = second_after_first
        .destination_entity_path
        .strip_prefix(&repo)
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/");
    let second_outcome = source_store
        .execute_move_with_journal(&second_after_first)
        .unwrap();

    let rewritten_doc = std::fs::read_to_string(&doc_path).unwrap();
    assert!(rewritten_doc.contains(&first_destination_rel));
    assert!(rewritten_doc.contains(&second_destination_rel));

    let _rolled_back = source_store
        .rollback_move_with_journal(second_outcome.journal.id)
        .unwrap();
    let restored_doc = std::fs::read_to_string(&doc_path).unwrap();
    assert!(restored_doc.contains(&first_destination_rel));
    assert!(restored_doc.contains(&second_source_rel));
    assert!(!restored_doc.contains(&second_destination_rel));

    let first_ticket = TicketStore::open(&nested_repo)
        .unwrap()
        .get_indexed(&first)
        .unwrap();
    assert!(first_ticket.is_some());
    let second_ticket = TicketStore::open(&repo)
        .unwrap()
        .get_indexed(&second)
        .unwrap();
    assert!(second_ticket.is_some());
    let _ = first_outcome;
}
