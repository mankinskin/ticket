#[allow(unused_imports)]
use super::*;

#[test]
pub(super) fn reference_heavy_move_e2e_reports_timings() {
    init_perf_test_tracing();
    let Some(perf) = git_available_or_skip(materialize_git_fixture_with_ticket_perf_load(
        TicketPerfFixtureOptions {
            root_generated_ticket_count: 64,
            submodule_generated_ticket_count: 40,
            tracked_reference_file_count: 10,
            references_per_file: 18,
        },
    )) else {
        eprintln!("git not available; skipping perf move E2E");
        return;
    };

    let source_root = perf
        .fixture
        .store_root("ticket-submodule-a")
        .expect("submodule ticket store path")
        .to_path_buf();
    let target_workspace = perf.fixture.workspace_root.clone();
    let source_store = TicketStore::open_or_init(&source_root).expect("open source store");
    source_store.scan(true).expect("scan source");
    let target_store = TicketStore::open_or_init(&target_workspace).expect("open target store");
    target_store.scan(true).expect("scan target");

    let id: Uuid = perf.submodule_ticket_ids[0]
        .parse()
        .expect("fixture move id");

    let preflight_started = Instant::now();
    let mut plan = source_store
        .plan_move_preflight(&id, &target_workspace)
        .expect("plan move preflight");
    let preflight_elapsed = preflight_started.elapsed();
    plan.blockers.retain(|blocker| {
        !matches!(
            blocker,
            MovePreflightBlocker::PathReferenceScanUnavailable { .. }
                | MovePreflightBlocker::DirtyTrackedFiles { .. }
        )
    });
    assert!(plan.supported());
    assert!(!plan.path_reference_files.is_empty());

    let execute_started = Instant::now();
    let outcome = source_store
        .execute_move_with_journal(&plan)
        .expect("execute move");
    let execute_elapsed = execute_started.elapsed();
    assert_eq!(outcome.journal.phase, MoveExecutionPhase::Validated);
    assert!(!outcome.journal.rewritten_path_files.is_empty());
    assert!(
        outcome
            .journal
            .phase_timings_ms
            .contains_key("rewrite_path_refs_ms")
    );
    assert!(
        outcome
            .journal
            .phase_timings_ms
            .contains_key("scan_source_ms")
    );
    assert!(
        outcome
            .journal
            .phase_timings_ms
            .contains_key("scan_target_ms")
    );

    let rollback_started = Instant::now();
    let rolled_back = source_store
        .rollback_move_with_journal(outcome.journal.id)
        .expect("rollback move");
    let rollback_elapsed = rollback_started.elapsed();
    assert!(rolled_back.rolled_back);
    assert_eq!(rolled_back.journal.phase, MoveExecutionPhase::RolledBack);

    eprintln!(
        "move_perf preflight_ms={} execute_ms={} rollback_ms={} refs={} rewrites={} phases={:?}",
        preflight_elapsed.as_millis(),
        execute_elapsed.as_millis(),
        rollback_elapsed.as_millis(),
        plan.path_reference_files.len(),
        outcome.journal.rewritten_path_files.len(),
        outcome.journal.phase_timings_ms,
    );
    tracing::info!(
        target: PERF_TRACE_TARGET,
        run = "reference_heavy_move_e2e_reports_timings",
        tracked_reference_files = plan.path_reference_files.len(),
        rewritten_files = outcome.journal.rewritten_path_files.len(),
        preflight_ms = preflight_elapsed.as_millis() as u64,
        execute_ms = execute_elapsed.as_millis() as u64,
        rollback_ms = rollback_elapsed.as_millis() as u64,
        "ticket_api_perf_test_complete"
    );
}

#[test]
pub(super) fn reference_heavy_move_missing_tracked_file_records_followup_with_timing() {
    init_perf_test_tracing();
    let Some(perf) = git_available_or_skip(materialize_git_fixture_with_ticket_perf_load(
        TicketPerfFixtureOptions {
            root_generated_ticket_count: 48,
            submodule_generated_ticket_count: 24,
            tracked_reference_file_count: 8,
            references_per_file: 14,
        },
    )) else {
        eprintln!("git not available; skipping missing-file perf move E2E");
        return;
    };

    let source_root = perf
        .fixture
        .store_root("ticket-submodule-a")
        .expect("submodule ticket store path")
        .to_path_buf();
    let target_workspace = perf.fixture.workspace_root.clone();
    let source_store = TicketStore::open_or_init(&source_root).expect("open source store");
    source_store.scan(true).expect("scan source");
    let target_store = TicketStore::open_or_init(&target_workspace).expect("open target store");
    target_store.scan(true).expect("scan target");

    let id: Uuid = perf.submodule_ticket_ids[1]
        .parse()
        .expect("fixture move id");
    let mut plan = source_store
        .plan_move_preflight(&id, &target_workspace)
        .expect("plan move preflight");
    plan.blockers.retain(|blocker| {
        !matches!(
            blocker,
            MovePreflightBlocker::PathReferenceScanUnavailable { .. }
                | MovePreflightBlocker::DirtyTrackedFiles { .. }
        )
    });
    assert!(plan.supported());

    let missing_file = plan.path_reference_files[0].clone();
    std::fs::remove_file(&missing_file).expect("remove tracked reference file");

    let execute_started = Instant::now();
    let outcome = source_store
        .execute_move_with_journal(&plan)
        .expect("execute move with missing tracked reference file");
    let execute_elapsed = execute_started.elapsed();
    assert!(
        outcome
            .journal
            .manual_followups
            .iter()
            .any(|followup| followup.path == missing_file)
    );

    eprintln!(
        "move_missing_ref_perf execute_ms={} refs={} followups={}",
        execute_elapsed.as_millis(),
        plan.path_reference_files.len(),
        outcome.journal.manual_followups.len(),
    );
    tracing::info!(
        target: PERF_TRACE_TARGET,
        run = "reference_heavy_move_missing_tracked_file_records_followup_with_timing",
        tracked_reference_files = plan.path_reference_files.len(),
        manual_followups = outcome.journal.manual_followups.len(),
        execute_ms = execute_elapsed.as_millis() as u64,
        "ticket_api_perf_test_complete"
    );
}

#[test]
pub(super) fn stress_reference_heavy_sequential_moves_report_timings() {
    init_perf_test_tracing();
    let Some(perf) = git_available_or_skip(materialize_git_fixture_with_ticket_perf_load(
        TicketPerfFixtureOptions::heavy(),
    )) else {
        eprintln!("git not available; skipping sequential perf move E2E");
        return;
    };

    let source_root = perf
        .fixture
        .store_root("ticket-submodule-a")
        .expect("submodule ticket store path")
        .to_path_buf();
    let target_workspace = perf.fixture.workspace_root.clone();
    let source_store = TicketStore::open_or_init(&source_root).expect("open source store");
    source_store.scan(true).expect("scan source");
    let target_store = TicketStore::open_or_init(&target_workspace).expect("open target store");
    target_store.scan(true).expect("scan target");

    let first_id: Uuid = perf.submodule_ticket_ids[0].parse().expect("first move id");
    let second_id: Uuid = perf.submodule_ticket_ids[1]
        .parse()
        .expect("second move id");

    let mut first_plan = source_store
        .plan_move_preflight(&first_id, &target_workspace)
        .expect("plan first move");
    first_plan.blockers.retain(|blocker| {
        !matches!(
            blocker,
            MovePreflightBlocker::PathReferenceScanUnavailable { .. }
                | MovePreflightBlocker::DirtyTrackedFiles { .. }
        )
    });
    let first_started = Instant::now();
    let first = source_store
        .execute_move_with_journal(&first_plan)
        .expect("execute first move");
    let first_elapsed = first_started.elapsed();

    let mut second_plan = source_store
        .plan_move_preflight(&second_id, &target_workspace)
        .expect("plan second move");
    second_plan.blockers.retain(|blocker| {
        !matches!(
            blocker,
            MovePreflightBlocker::PathReferenceScanUnavailable { .. }
                | MovePreflightBlocker::DirtyTrackedFiles { .. }
        )
    });
    let second_started = Instant::now();
    let second = source_store
        .execute_move_with_journal(&second_plan)
        .expect("execute second move");
    let second_elapsed = second_started.elapsed();

    eprintln!(
        "move_seq_perf first_ms={} second_ms={} first_phases={:?} second_phases={:?}",
        first_elapsed.as_millis(),
        second_elapsed.as_millis(),
        first.journal.phase_timings_ms,
        second.journal.phase_timings_ms,
    );
    tracing::info!(
        target: PERF_TRACE_TARGET,
        run = "stress_reference_heavy_sequential_moves_report_timings",
        first_ms = first_elapsed.as_millis() as u64,
        second_ms = second_elapsed.as_millis() as u64,
        first_rewrites = first.journal.rewritten_path_files.len(),
        second_rewrites = second.journal.rewritten_path_files.len(),
        "ticket_api_perf_test_complete"
    );
}
