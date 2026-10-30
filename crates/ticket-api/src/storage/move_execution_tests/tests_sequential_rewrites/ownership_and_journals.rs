#[allow(unused_imports)]
use super::*;

#[test]
pub(super) fn entity_indexed_in_requires_path_ownership_not_aggregate_visibility() {
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
            Some("nested visibility ticket"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();

    let target_indexed = target_store.get_indexed(&id).unwrap().unwrap();

    let poisoned_index = RedbIndexStore::open(&source_store.index_root.join("tickets.db")).unwrap();
    poisoned_index.insert_ticket(&target_indexed).unwrap();

    let domain = TicketMoveDomain::new(&source_store);
    assert!(!move_kernel::MoveDomain::entity_indexed_in(&domain, &repo, &id).unwrap());
    assert!(move_kernel::MoveDomain::entity_indexed_in(&domain, &nested_repo, &id).unwrap());
}

#[test]
pub(super) fn resume_move_normalizes_journal_paths_before_validation() {
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

    let id = source_store
        .create(
            None,
            "tracker-improvement",
            Some("resume normalized paths"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();

    let plan = source_store.plan_move_preflight(&id, &nested_repo).unwrap();
    std::fs::create_dir_all(plan.destination_entity_path.parent().unwrap()).unwrap();
    std::fs::rename(&plan.source_entity_path, &plan.destination_entity_path).unwrap();
    let source_domain = TicketMoveDomain::new(&source_store);
    let target_domain = TicketMoveDomain::new(&target_store);
    move_kernel::MoveDomain::reconcile_store_touched(
        &source_domain,
        &plan.source_store_root,
        &[id],
    )
    .unwrap();
    move_kernel::MoveDomain::reconcile_store_touched(
        &target_domain,
        &plan.target_store_root,
        &[id],
    )
    .unwrap();

    let journal_id = Uuid::new_v4();
    let journal = MoveJournal {
        id: journal_id,
        entity_id: id,
        source_store_root: plan.source_store_root.clone(),
        target_store_root: plan.target_store_root.clone(),
        source_entity_path: plan.destination_entity_path.clone(),
        destination_entity_path: plan.destination_entity_path.clone(),
        phase: MoveExecutionPhase::TargetScanned,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        steps: vec![
            "created move journal".to_string(),
            "acquired source/target store locks and move entity lock".to_string(),
            "moved entity folder".to_string(),
            "scanned source store".to_string(),
            "scanned target store".to_string(),
        ],
        rollback_steps: vec!["rename destination entity folder back to source path".to_string()],
        lock_paths: move_kernel::collect_lock_paths(
            id,
            &plan.source_store_root,
            &plan.target_store_root,
        ),
        migrated_board_entries: Vec::new(),
        rewritten_path_files: Vec::new(),
        manual_followups: Vec::new(),
        phase_timings_ms: Default::default(),
        failure: Some("injected stale source path".to_string()),
        next_recovery_step: Some(
            "run rollback_move for safety, or resume_move to retry".to_string(),
        ),
    };
    move_kernel::persist_journal(&plan.source_store_root, &journal).unwrap();

    let resumed = source_store.resume_move_with_journal(journal_id).unwrap();
    assert!(resumed.resumed);
    assert_eq!(resumed.journal.phase, MoveExecutionPhase::Validated);
    assert_eq!(resumed.journal.source_entity_path, plan.source_entity_path);
    assert_eq!(
        resumed.journal.destination_entity_path,
        plan.destination_entity_path
    );
}
