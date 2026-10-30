#[allow(unused_imports)]
use super::*;

pub(super) fn sequential_move_can_execute_without_commit_between_moves() {
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

    let doc_path = repo.join("docs").join("shared-spec.md");
    std::fs::create_dir_all(doc_path.parent().unwrap()).unwrap();
    std::fs::write(
        &doc_path,
        format!(
            "first reference: {}\nsecond reference: {}\n",
            first_source_rel, second_source_rel
        ),
    )
    .unwrap();
    git_commit_path(&repo, "docs/shared-spec.md", "seed shared refs");

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

    let _first_outcome = source_store.execute_move_with_journal(&first_plan).unwrap();

    let second_after_first = source_store
        .plan_move_preflight(&second, &nested_repo)
        .unwrap();
    assert!(
        !second_after_first
            .blockers
            .iter()
            .any(|blocker| matches!(blocker, MovePreflightBlocker::DirtyTrackedFiles { .. }))
    );

    let second_destination_rel = second_after_first
        .destination_entity_path
        .strip_prefix(&repo)
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/");
    let second_outcome = source_store
        .execute_move_with_journal(&second_after_first)
        .unwrap();
    assert_eq!(second_outcome.journal.phase, MoveExecutionPhase::Validated);

    let updated_doc = std::fs::read_to_string(&doc_path).unwrap();
    assert!(updated_doc.contains(&first_destination_rel));
    assert!(updated_doc.contains(&second_destination_rel));
}

#[test]
pub(super) fn sequential_move_after_resumed_execution_can_continue_without_clean_commit() {
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

    let doc_path = repo.join("docs").join("shared-spec-resume.md");
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
        "docs/shared-spec-resume.md",
        "seed shared refs for resume",
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

    let journal_id = Uuid::new_v4();
    let journal = MoveJournal {
        id: journal_id,
        entity_id: first,
        source_store_root: first_plan.source_store_root.clone(),
        target_store_root: first_plan.target_store_root.clone(),
        source_entity_path: first_plan.source_entity_path.clone(),
        destination_entity_path: first_plan.destination_entity_path.clone(),
        phase: MoveExecutionPhase::Locked,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        steps: vec!["created move journal".to_string()],
        rollback_steps: vec!["rename destination entity folder back to source path".to_string()],
        lock_paths: move_kernel::collect_lock_paths(
            first,
            &first_plan.source_store_root,
            &first_plan.target_store_root,
        ),
        migrated_board_entries: Vec::new(),
        rewritten_path_files: Vec::new(),
        manual_followups: Vec::new(),
        phase_timings_ms: Default::default(),
        failure: None,
        next_recovery_step: None,
    };
    move_kernel::persist_journal(&first_plan.source_store_root, &journal).unwrap();

    let resumed = source_store.resume_move_with_journal(journal_id).unwrap();
    assert!(resumed.resumed);
    assert_eq!(resumed.journal.phase, MoveExecutionPhase::Validated);

    let second_after_resume = source_store
        .plan_move_preflight(&second, &nested_repo)
        .unwrap();
    assert!(
        !second_after_resume
            .blockers
            .iter()
            .any(|blocker| matches!(blocker, MovePreflightBlocker::DirtyTrackedFiles { .. }))
    );

    let continued = source_store
        .execute_move_with_journal(&second_after_resume)
        .unwrap();
    assert_eq!(continued.journal.phase, MoveExecutionPhase::Validated);
}
