use super::*;

pub fn bench_move_preflight_reference_heavy(c: &mut Criterion) {
    init_perf_bench_tracing();
    c.bench_function("move_preflight_reference_heavy", |b| {
        b.iter_batched(
            || {
                let perf =
                    materialize_git_fixture_with_ticket_perf_load(TicketPerfFixtureOptions {
                        root_generated_ticket_count: 48,
                        submodule_generated_ticket_count: 24,
                        tracked_reference_file_count: 8,
                        references_per_file: 18,
                    })
                    .expect("perf fixture should materialize");
                let source_root = perf
                    .fixture
                    .store_root("ticket-submodule-a")
                    .expect("submodule store")
                    .to_path_buf();
                let target_workspace = perf.fixture.workspace_root.clone();
                let store = TicketStore::open_or_init(&source_root).expect("open source store");
                store.scan(true).expect("scan source store");
                let id: Uuid = perf.submodule_ticket_ids[0]
                    .parse()
                    .expect("fixture move id");
                (perf, store, target_workspace, id)
            },
            |(_perf, store, target_workspace, id)| {
                let started = Instant::now();
                let plan = store
                    .plan_move_preflight(&id, &target_workspace)
                    .expect("plan preflight");
                let elapsed = started.elapsed();
                criterion::black_box(elapsed);
                criterion::black_box(plan.path_reference_files.len());
                tracing::info!(
                    target: PERF_TRACE_TARGET,
                    benchmark = "move_preflight_reference_heavy",
                    run_id = %Uuid::new_v4(),
                    fixture_profile = "git_ref_heavy_48_24_8x18",
                    workspace_scope = "ticket-submodule-a",
                    change_count = 0,
                    reindex_mode = "true",
                    p50_ms = elapsed.as_millis() as u64,
                    p95_ms = elapsed.as_millis() as u64,
                    p99_ms = elapsed.as_millis() as u64,
                    tracked_reference_files = plan.path_reference_files.len(),
                    elapsed_ms = elapsed.as_millis() as u64,
                    "ticket_api_benchmark_iteration"
                );
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

pub fn bench_move_execute_reference_heavy(c: &mut Criterion) {
    init_perf_bench_tracing();
    c.bench_function("move_execute_reference_heavy", |b| {
        b.iter_batched(
            || {
                let perf =
                    materialize_git_fixture_with_ticket_perf_load(TicketPerfFixtureOptions {
                        root_generated_ticket_count: 48,
                        submodule_generated_ticket_count: 24,
                        tracked_reference_file_count: 8,
                        references_per_file: 18,
                    })
                    .expect("perf fixture should materialize");
                let source_root = perf
                    .fixture
                    .store_root("ticket-submodule-a")
                    .expect("submodule store")
                    .to_path_buf();
                let target_workspace = perf.fixture.workspace_root.clone();
                let store = TicketStore::open_or_init(&source_root).expect("open source store");
                store.scan(true).expect("scan source store");
                let target_store =
                    TicketStore::open_or_init(&target_workspace).expect("open target store");
                target_store.scan(true).expect("scan target store");
                let id: Uuid = perf.submodule_ticket_ids[0]
                    .parse()
                    .expect("fixture move id");
                let mut plan = store
                    .plan_move_preflight(&id, &target_workspace)
                    .expect("plan preflight");
                plan.blockers.retain(|blocker| {
                    !matches!(
                        blocker,
                        MovePreflightBlocker::PathReferenceScanUnavailable { .. }
                            | MovePreflightBlocker::DirtyTrackedFiles { .. }
                    )
                });
                (perf, store, plan)
            },
            |(_perf, store, plan)| {
                let started = Instant::now();
                let outcome = store
                    .execute_move_with_journal(&plan)
                    .expect("execute move");
                let elapsed = started.elapsed();
                criterion::black_box(elapsed);
                assert_eq!(outcome.journal.phase, MoveExecutionPhase::Validated);
                tracing::info!(
                    target: PERF_TRACE_TARGET,
                    benchmark = "move_execute_reference_heavy",
                    run_id = %Uuid::new_v4(),
                    fixture_profile = "git_ref_heavy_48_24_8x18",
                    workspace_scope = "ticket-submodule-a",
                    change_count = 0,
                    reindex_mode = "true",
                    p50_ms = elapsed.as_millis() as u64,
                    p95_ms = elapsed.as_millis() as u64,
                    p99_ms = elapsed.as_millis() as u64,
                    rewritten_files = outcome.journal.rewritten_path_files.len(),
                    elapsed_ms = elapsed.as_millis() as u64,
                    "ticket_api_benchmark_iteration"
                );
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

pub fn bench_move_rollback_reference_heavy(c: &mut Criterion) {
    c.bench_function("move_rollback_reference_heavy", |b| {
        b.iter_batched(
            || {
                let perf = materialize_git_fixture_with_ticket_perf_load(
                    TicketPerfFixtureOptions::heavy(),
                )
                .expect("perf fixture should materialize");
                let source_root = perf
                    .fixture
                    .store_root("ticket-submodule-a")
                    .expect("submodule store")
                    .to_path_buf();
                let target_workspace = perf.fixture.workspace_root.clone();
                let store = TicketStore::open_or_init(&source_root).expect("open source store");
                store.scan(true).expect("scan source store");
                let target_store =
                    TicketStore::open_or_init(&target_workspace).expect("open target store");
                target_store.scan(true).expect("scan target store");
                let id: Uuid = perf.submodule_ticket_ids[0]
                    .parse()
                    .expect("fixture move id");
                let mut plan = store
                    .plan_move_preflight(&id, &target_workspace)
                    .expect("plan preflight");
                plan.blockers.retain(|blocker| {
                    !matches!(
                        blocker,
                        MovePreflightBlocker::PathReferenceScanUnavailable { .. }
                            | MovePreflightBlocker::DirtyTrackedFiles { .. }
                    )
                });
                let outcome = store
                    .execute_move_with_journal(&plan)
                    .expect("execute move");
                (perf, store, outcome.journal.id)
            },
            |(_perf, store, journal_id)| {
                let started = Instant::now();
                let outcome = store
                    .rollback_move_with_journal(journal_id)
                    .expect("rollback move");
                criterion::black_box(started.elapsed());
                assert!(outcome.rolled_back);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}
