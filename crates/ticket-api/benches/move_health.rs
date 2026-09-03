use std::{
    cell::Cell,
    path::{
        Path,
        PathBuf,
    },
    sync::OnceLock,
    time::{
        Duration,
        Instant,
    },
};

use chrono::Utc;
use criterion::{
    BenchmarkId,
    Criterion,
    Throughput,
    criterion_group,
    criterion_main,
};
use memory_fixtures::{
    TicketPerfFixtureOptions,
    append_fixture_ticket,
    materialize_fixture_with_ticket_perf_load,
    materialize_git_fixture_with_ticket_perf_load,
};
use tempfile::TempDir;
use ticket_api::{
    health::collect_findings,
    model::edge::EdgeRecord,
    storage::{
        move_execution::MoveExecutionPhase,
        move_planner::MovePreflightBlocker,
        store::TicketStore,
    },
    workflow::WorkflowModel,
};
use uuid::Uuid;

const PERF_TRACE_TARGET: &str = "ticket_api::perf";
static PERF_TRACING: OnceLock<()> = OnceLock::new();

fn init_perf_bench_tracing() {
    PERF_TRACING.get_or_init(|| {
        let filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("off"));
        let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
    });
}

fn parse_ids(ids: &[String]) -> Vec<Uuid> {
    ids.iter()
        .map(|id| id.parse().expect("valid fixture uuid"))
        .collect()
}

fn add_perf_edges(
    store: &TicketStore,
    ids: &[Uuid],
) {
    let now = Utc::now();
    for pair in ids.windows(2) {
        store
            .add_edge(EdgeRecord {
                from: pair[0],
                to: pair[1],
                kind: "depends_on".to_string(),
                created_at: now,
            })
            .expect("add chain edge");
    }

    let fanout = ids.len().min(24);
    if fanout > 1 {
        let root = ids[0];
        for id in &ids[1..fanout] {
            store
                .add_edge(EdgeRecord {
                    from: root,
                    to: *id,
                    kind: "linked".to_string(),
                    created_at: now,
                })
                .expect("add linked edge");
        }
    }
}

fn append_incremental_fixture_tickets(
    root_store: &Path,
    batch: usize,
    count: usize,
) {
    for offset in 0..count {
        let id = format!(
            "00000000-0000-5000-{batch:04x}-{value:012x}",
            value = offset + 1,
        );
        append_fixture_ticket(
            root_store,
            &id,
            &format!("bench incremental perf ticket {batch}-{offset}"),
            "planned",
            "perf",
        )
        .expect("append fixture ticket");
    }
}

fn percentile_nearest_rank(
    sorted: &[u64],
    percentile: u32,
) -> u64 {
    assert!(!sorted.is_empty(), "percentile input must not be empty");
    assert!(percentile <= 100, "percentile must be <= 100");

    if sorted.len() == 1 {
        return sorted[0];
    }

    let rank =
        ((percentile as f64 / 100.0) * sorted.len() as f64).ceil() as usize;
    let idx = rank.saturating_sub(1).min(sorted.len() - 1);
    sorted[idx]
}

fn percentile_summary(samples: &[u64]) -> (u64, u64, u64) {
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    (
        percentile_nearest_rank(&sorted, 50),
        percentile_nearest_rank(&sorted, 95),
        percentile_nearest_rank(&sorted, 99),
    )
}

fn map_percentiles(
    values: &std::collections::BTreeMap<String, u64>
) -> (u64, u64, u64) {
    let samples = values.values().copied().collect::<Vec<_>>();
    percentile_summary(&samples)
}

fn bench_move_preflight_reference_heavy(c: &mut Criterion) {
    init_perf_bench_tracing();
    c.bench_function("move_preflight_reference_heavy", |b| {
        b.iter_batched(
            || {
                let perf = materialize_git_fixture_with_ticket_perf_load(
                    TicketPerfFixtureOptions {
                        root_generated_ticket_count: 48,
                        submodule_generated_ticket_count: 24,
                        tracked_reference_file_count: 8,
                        references_per_file: 18,
                    },
                )
                .expect("perf fixture should materialize");
                let source_root = perf
                    .fixture
                    .store_root("ticket-submodule-a")
                    .expect("submodule store")
                    .to_path_buf();
                let target_workspace = perf.fixture.workspace_root.clone();
                let store = TicketStore::open_or_init(&source_root)
                    .expect("open source store");
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

fn bench_move_execute_reference_heavy(c: &mut Criterion) {
    init_perf_bench_tracing();
    c.bench_function("move_execute_reference_heavy", |b| {
        b.iter_batched(
            || {
                let perf = materialize_git_fixture_with_ticket_perf_load(TicketPerfFixtureOptions {
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
                let target_store = TicketStore::open_or_init(&target_workspace).expect("open target store");
                target_store.scan(true).expect("scan target store");
                let id: Uuid = perf.submodule_ticket_ids[0].parse().expect("fixture move id");
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

fn bench_move_rollback_reference_heavy(c: &mut Criterion) {
    c.bench_function("move_rollback_reference_heavy", |b| {
        b.iter_batched(
            || {
                let perf = materialize_git_fixture_with_ticket_perf_load(TicketPerfFixtureOptions::heavy())
                    .expect("perf fixture should materialize");
                let source_root = perf
                    .fixture
                    .store_root("ticket-submodule-a")
                    .expect("submodule store")
                    .to_path_buf();
                let target_workspace = perf.fixture.workspace_root.clone();
                let store = TicketStore::open_or_init(&source_root).expect("open source store");
                store.scan(true).expect("scan source store");
                let target_store = TicketStore::open_or_init(&target_workspace).expect("open target store");
                target_store.scan(true).expect("scan target store");
                let id: Uuid = perf.submodule_ticket_ids[0].parse().expect("fixture move id");
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
                let outcome = store.execute_move_with_journal(&plan).expect("execute move");
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

fn bench_open_or_init_root_perf_fixture(c: &mut Criterion) {
    init_perf_bench_tracing();
    c.bench_function("open_or_init_root_perf_fixture", |b| {
        b.iter_batched(
            || {
                let perf = materialize_fixture_with_ticket_perf_load(
                    TicketPerfFixtureOptions::heavy(),
                )
                .expect("perf fixture should materialize");
                let root_store = perf
                    .fixture
                    .store_root("ticket-root")
                    .expect("root store")
                    .to_path_buf();
                (perf, root_store)
            },
            |(_perf, root_store)| {
                let started = Instant::now();
                let (store, report) =
                    TicketStore::open_or_init_profiled(&root_store)
                        .expect("open store");
                let elapsed = started.elapsed();
                criterion::black_box(elapsed);
                let phase_count = report.phase_timings_ms.len();
                let (p50_ms, p95_ms, p99_ms) =
                    map_percentiles(&report.phase_timings_ms);
                criterion::black_box(report.phase_timings_ms);
                criterion::black_box(store);
                tracing::info!(
                    target: PERF_TRACE_TARGET,
                    benchmark = "open_or_init_root_perf_fixture",
                    run_id = %Uuid::new_v4(),
                    fixture_profile = "root_heavy",
                    workspace_scope = "ticket-root",
                    change_count = 0,
                    reindex_mode = "true",
                    p50_ms,
                    p95_ms,
                    p99_ms,
                    phase_count,
                    elapsed_ms = elapsed.as_millis() as u64,
                    "ticket_api_benchmark_iteration"
                );
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_scan_reindex_root_perf_fixture(c: &mut Criterion) {
    init_perf_bench_tracing();
    c.bench_function("scan_reindex_root_perf_fixture", |b| {
        b.iter_batched(
            || {
                let perf = materialize_fixture_with_ticket_perf_load(
                    TicketPerfFixtureOptions::heavy(),
                )
                .expect("perf fixture should materialize");
                let root_store = perf
                    .fixture
                    .store_root("ticket-root")
                    .expect("root store")
                    .to_path_buf();
                let store =
                    TicketStore::open_or_init(&root_store).expect("open store");
                (perf, store)
            },
            |(_perf, store)| {
                let started = Instant::now();
                let report = store.scan(true).expect("scan(true)");
                let elapsed = started.elapsed();
                let (p50_ms, p95_ms, p99_ms) =
                    map_percentiles(&report.phase_timings_ms);
                criterion::black_box(started.elapsed());
                criterion::black_box(report.phase_timings_ms);
                tracing::info!(
                    target: PERF_TRACE_TARGET,
                    benchmark = "scan_reindex_root_perf_fixture",
                    run_id = %Uuid::new_v4(),
                    fixture_profile = "root_heavy",
                    workspace_scope = "ticket-root",
                    change_count = 0,
                    reindex_mode = "true",
                    p50_ms,
                    p95_ms,
                    p99_ms,
                    elapsed_ms = elapsed.as_millis() as u64,
                    "ticket_api_benchmark_iteration"
                );
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_scan_incremental_root_perf_fixture(
    c: &mut Criterion,
    change_count: usize,
    label: &str,
) {
    init_perf_bench_tracing();
    c.bench_function(label, |b| {
        b.iter_batched(
            || {
                let perf = materialize_fixture_with_ticket_perf_load(
                    TicketPerfFixtureOptions::heavy(),
                )
                .expect("perf fixture should materialize");
                let root_store = perf
                    .fixture
                    .store_root("ticket-root")
                    .expect("root store")
                    .to_path_buf();
                let store =
                    TicketStore::open_or_init(&root_store).expect("open store");
                store.scan(true).expect("initial scan");
                append_incremental_fixture_tickets(
                    &root_store,
                    change_count,
                    change_count,
                );
                (perf, store)
            },
            |(_perf, store)| {
                let started = Instant::now();
                let report = store.scan(false).expect("scan(false)");
                let elapsed = started.elapsed();
                let (p50_ms, p95_ms, p99_ms) =
                    map_percentiles(&report.phase_timings_ms);
                criterion::black_box(started.elapsed());
                criterion::black_box(report.phase_timings_ms);
                criterion::black_box(report.root_entry_counts);
                tracing::info!(
                    target: PERF_TRACE_TARGET,
                    benchmark = label,
                    run_id = %Uuid::new_v4(),
                    fixture_profile = "root_heavy_incremental",
                    workspace_scope = "ticket-root",
                    change_count,
                    reindex_mode = "false",
                    p50_ms,
                    p95_ms,
                    p99_ms,
                    elapsed_ms = elapsed.as_millis() as u64,
                    "ticket_api_benchmark_iteration"
                );
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_scan_incremental_root_perf_fixture_1(c: &mut Criterion) {
    bench_scan_incremental_root_perf_fixture(
        c,
        1,
        "scan_incremental_root_perf_fixture_1_change",
    );
}

fn bench_scan_incremental_root_perf_fixture_10(c: &mut Criterion) {
    bench_scan_incremental_root_perf_fixture(
        c,
        10,
        "scan_incremental_root_perf_fixture_10_changes",
    );
}

fn bench_scan_incremental_root_perf_fixture_100(c: &mut Criterion) {
    bench_scan_incremental_root_perf_fixture(
        c,
        100,
        "scan_incremental_root_perf_fixture_100_changes",
    );
}

fn bench_health_workflow_build_large_fixture(c: &mut Criterion) {
    init_perf_bench_tracing();
    let options = TicketPerfFixtureOptions::heavy();
    c.bench_function("health_workflow_build_large_fixture", |b| {
        b.iter_batched(
            || {
                let perf = materialize_fixture_with_ticket_perf_load(options)
                    .expect("perf fixture should materialize");
                let root_store = perf
                    .fixture
                    .store_root("ticket-root")
                    .expect("root store")
                    .to_path_buf();
                let store =
                    TicketStore::open_or_init(&root_store).expect("open store");
                store.scan(true).expect("scan store");
                let ids = parse_ids(&perf.root_ticket_ids);
                add_perf_edges(&store, &ids);
                let tickets =
                    store.list(None, None, None).expect("list tickets");
                let all_edges = store.list_all_edges().expect("list edges");
                (perf, store, tickets, all_edges)
            },
            |(_perf, store, tickets, all_edges)| {
                let started = Instant::now();
                let workflow = WorkflowModel::build(&store, tickets, all_edges)
                    .expect("build workflow");
                let elapsed = started.elapsed();
                criterion::black_box(elapsed);
                criterion::black_box(workflow);
                tracing::info!(
                    target: PERF_TRACE_TARGET,
                    benchmark = "health_workflow_build_large_fixture",
                    run_id = %Uuid::new_v4(),
                    fixture_profile = "root_heavy",
                    workspace_scope = "ticket-root",
                    change_count = 0,
                    reindex_mode = "true",
                    p50_ms = elapsed.as_millis() as u64,
                    p95_ms = elapsed.as_millis() as u64,
                    p99_ms = elapsed.as_millis() as u64,
                    elapsed_ms = elapsed.as_millis() as u64,
                    "ticket_api_benchmark_iteration"
                );
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_health_collect_large_fixture(c: &mut Criterion) {
    init_perf_bench_tracing();
    let options = TicketPerfFixtureOptions::heavy();
    c.bench_function("health_collect_large_fixture", |b| {
        b.iter_batched(
            || {
                let perf = materialize_fixture_with_ticket_perf_load(options)
                    .expect("perf fixture should materialize");
                let root_store = perf
                    .fixture
                    .store_root("ticket-root")
                    .expect("root store")
                    .to_path_buf();
                let store =
                    TicketStore::open_or_init(&root_store).expect("open store");
                store.scan(true).expect("scan store");
                let ids = parse_ids(&perf.root_ticket_ids);
                add_perf_edges(&store, &ids);
                let tickets =
                    store.list(None, None, None).expect("list tickets");
                let all_edges = store.list_all_edges().expect("list edges");
                let workflow = WorkflowModel::build(
                    &store,
                    tickets.clone(),
                    all_edges.clone(),
                )
                .expect("build workflow");
                (perf, store, tickets, all_edges, workflow)
            },
            |(_perf, store, tickets, all_edges, workflow)| {
                let started = Instant::now();
                let report =
                    collect_findings(&store, &tickets, &all_edges, &workflow);
                let elapsed = started.elapsed();
                criterion::black_box(elapsed);
                criterion::black_box(report.findings.len());
                tracing::info!(
                    target: PERF_TRACE_TARGET,
                    benchmark = "health_collect_large_fixture",
                    run_id = %Uuid::new_v4(),
                    fixture_profile = "root_heavy",
                    workspace_scope = "ticket-root",
                    change_count = 0,
                    reindex_mode = "true",
                    p50_ms = elapsed.as_millis() as u64,
                    p95_ms = elapsed.as_millis() as u64,
                    p99_ms = elapsed.as_millis() as u64,
                    findings = report.findings.len(),
                    elapsed_ms = elapsed.as_millis() as u64,
                    "ticket_api_benchmark_iteration"
                );
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_health_all_large_fixture(c: &mut Criterion) {
    init_perf_bench_tracing();
    let options = TicketPerfFixtureOptions {
        root_generated_ticket_count: 240,
        submodule_generated_ticket_count: 64,
        tracked_reference_file_count: 4,
        references_per_file: 10,
    };
    c.bench_function("health_all_large_fixture", |b| {
        b.iter_batched(
            || {
                let perf = materialize_fixture_with_ticket_perf_load(options)
                    .expect("perf fixture should materialize");
                let root_store = perf
                    .fixture
                    .store_root("ticket-root")
                    .expect("root store")
                    .to_path_buf();
                let store =
                    TicketStore::open_or_init(&root_store).expect("open store");
                store.scan(true).expect("scan store");
                let ids = parse_ids(&perf.root_ticket_ids);
                add_perf_edges(&store, &ids);
                (perf, store)
            },
            |(_perf, store)| {
                let started = Instant::now();
                let tickets =
                    store.list(None, None, None).expect("list tickets");
                let all_edges = store.list_all_edges().expect("list edges");
                let workflow = WorkflowModel::build(
                    &store,
                    tickets.clone(),
                    all_edges.clone(),
                )
                .expect("build workflow");
                let report =
                    collect_findings(&store, &tickets, &all_edges, &workflow);
                let elapsed = started.elapsed();
                criterion::black_box(report.findings.len());
                tracing::info!(
                    target: PERF_TRACE_TARGET,
                    benchmark = "health_all_large_fixture",
                    run_id = %Uuid::new_v4(),
                    fixture_profile = "root240_sub64_refs4x10",
                    workspace_scope = "ticket-root",
                    change_count = 0,
                    reindex_mode = "true",
                    p50_ms = elapsed.as_millis() as u64,
                    p95_ms = elapsed.as_millis() as u64,
                    p99_ms = elapsed.as_millis() as u64,
                    findings = report.findings.len(),
                    elapsed_ms = elapsed.as_millis() as u64,
                    "ticket_api_benchmark_iteration"
                );
            },
            criterion::BatchSize::SmallInput,
        );
    });
    let mut group = c.benchmark_group("health_all_large_fixture_meta");
    group.throughput(Throughput::Elements(
        options.root_generated_ticket_count as u64,
    ));
    group.finish();
}

// --- Scenario matrix: entity count x link topology x link density x phase ---
//
// Real move batches are a sequence of single-entity `plan_move_preflight` /
// `execute_move_with_journal` / `rollback_move_with_journal` calls (the move
// kernel has no multi-entity API), so "entity count per move" below means the
// number of individual moves performed back-to-back within one measured
// iteration, mirroring how the Mandatory Batch Protocol drives up to 25 moves
// per batch in production.

/// Fixed pool size for "crossing" external targets: large enough to give every
/// density value (up to 20) a distinct target per moved ticket without
/// duplicate edges.
const CROSSING_EXTERNAL_POOL: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LinkTopology {
    None,
    Internal,
    Crossing,
}

impl LinkTopology {
    fn label(self) -> &'static str {
        match self {
            LinkTopology::None => "no_links",
            LinkTopology::Internal => "internal_links",
            LinkTopology::Crossing => "crossing_links",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MovePhase {
    Preflight,
    Apply,
    Rollback,
}

impl MovePhase {
    fn label(self) -> &'static str {
        match self {
            MovePhase::Preflight => "preflight",
            MovePhase::Apply => "apply",
            MovePhase::Rollback => "rollback",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct MoveScenario {
    entity_count: usize,
    topology: LinkTopology,
    links_per_entity: usize,
    phase: MovePhase,
}

/// Pre-built pool for one scenario: the fixture (git init + tickets + edges)
/// is built exactly once, not once per Criterion iteration/sample. Preflight
/// is read-only, so its single `ids` batch is reused unmodified across every
/// iteration. Apply/Rollback mutate state, so the pool holds
/// `SCENARIO_POOL_SIZE` independent batches (built from one bulk fixture
/// `entity_count * SCENARIO_POOL_SIZE` tickets wide) so each Criterion
/// iteration consumes the next untouched batch instead of the whole fixture
/// being torn down and rebuilt.
struct ScenarioPool {
    _workspace_dir: TempDir,
    store: TicketStore,
    target_root: PathBuf,
    phase: MovePhase,
    batches: Vec<Vec<Uuid>>,
}

/// Batches to pre-build per mutating scenario: must exceed the real number
/// of Criterion iterations (warm-up + measurement samples) or `next_batch`
/// panics with a message telling you to raise this rather than silently
/// wrapping around and re-measuring an already-mutated ticket.
const SCENARIO_POOL_SIZE: usize = 40;

impl ScenarioPool {
    /// Cheap: for `Preflight` this clones the one reusable read-only batch;
    /// for `Apply`/`Rollback` it hands out the next untouched pre-built
    /// batch. No fixture I/O happens here — that is the whole point.
    fn next_batch(
        &self,
        cursor: &Cell<usize>,
    ) -> Vec<Uuid> {
        match self.phase {
            MovePhase::Preflight => self.batches[0].clone(),
            MovePhase::Apply | MovePhase::Rollback => {
                let i = cursor.get();
                cursor.set(i + 1);
                self.batches.get(i).unwrap_or_else(|| {
                    panic!(
                        "scenario pool exhausted after {i} iterations; \
                         raise SCENARIO_POOL_SIZE"
                    )
                }).clone()
            },
        }
    }

    /// The real production call(s), one per entity in `ids` — identical to
    /// what `move_scenario_matrix` measured before, just no longer
    /// interleaved with fixture setup.
    fn run_batch(
        &self,
        ids: Vec<Uuid>,
    ) {
        match self.phase {
            MovePhase::Preflight => {
                for id in &ids {
                    let plan = self
                        .store
                        .plan_move_preflight(id, &self.target_root)
                        .expect("plan preflight");
                    criterion::black_box(plan);
                }
            },
            MovePhase::Apply => {
                for id in &ids {
                    let plan =
                        active_move_preflight(&self.store, &self.target_root, id);
                    let outcome = self
                        .store
                        .execute_move_with_journal(&plan)
                        .expect("execute move");
                    assert_eq!(
                        outcome.journal.phase,
                        MoveExecutionPhase::Validated
                    );
                    criterion::black_box(outcome);
                }
            },
            MovePhase::Rollback => {
                for journal_id in ids {
                    let outcome = self
                        .store
                        .rollback_move_with_journal(journal_id)
                        .expect("rollback move");
                    assert!(outcome.rolled_back);
                    criterion::black_box(outcome);
                }
            },
        }
    }
}

fn scenario_ticket_id(
    prefix: u8,
    offset: usize,
) -> String {
    format!("{prefix:08x}-0000-6000-8000-{offset:012x}")
}

/// Materialize an isolated store containing `entity_count` moved tickets and
/// (for `LinkTopology::Crossing`) a fixed pool of external tickets that stay
/// behind, then wire edges per `topology`/`links_per_entity`. The returned
/// `TempDir` must stay alive for as long as `source_root`/`target_root` are used.
fn build_move_scenario_fixture(
    scenario: MoveScenario
) -> (TempDir, TicketStore, PathBuf, Vec<Uuid>) {
    let workspace_dir = tempfile::tempdir().expect("tempdir");
    let source_root = workspace_dir.path().join("source-workspace");
    let target_root = workspace_dir.path().join("target-workspace");
    std::fs::create_dir_all(&source_root).expect("create source workspace");
    std::fs::create_dir_all(&target_root).expect("create target workspace");

    let git_init = std::process::Command::new("git")
        .current_dir(workspace_dir.path())
        .arg("init")
        .status()
        .expect("run git init");
    assert!(git_init.success(), "git init failed");

    let store = TicketStore::init(&source_root).expect("init source store");
    TicketStore::init(&target_root).expect("init target store");
    // `append_fixture_ticket` writes to `<store_root>/tickets/<id>`. With no
    // pre-existing `.ticket`/`.workflow-tools/ticket` marker anywhere in this
    // isolated tempdir's ancestor chain, `TicketStore::init` resolves its
    // index root to `source_root` itself (see
    // `workspace::resolve_store_root_from_with_diagnostics`'s no-marker-found
    // fallback), so fixture tickets must land there directly.
    let source_index_root = source_root.clone();

    let moved_ids: Vec<Uuid> = (0..scenario.entity_count)
        .map(|offset| {
            let id = scenario_ticket_id(0x10, offset);
            append_fixture_ticket(
                &source_index_root,
                &id,
                &format!("scenario moved ticket {offset}"),
                "planned",
                "perf-scenario",
            )
            .expect("append moved fixture ticket");
            id.parse().expect("valid moved ticket uuid")
        })
        .collect();

    let external_ids: Vec<Uuid> = if scenario.topology == LinkTopology::Crossing
    {
        (0..CROSSING_EXTERNAL_POOL)
            .map(|offset| {
                let id = scenario_ticket_id(0x20, offset);
                append_fixture_ticket(
                    &source_index_root,
                    &id,
                    &format!("scenario external ticket {offset}"),
                    "planned",
                    "perf-scenario-external",
                )
                .expect("append external fixture ticket");
                id.parse().expect("valid external ticket uuid")
            })
            .collect()
    } else {
        Vec::new()
    };

    store.scan(true).expect("scan scenario store");

    let now = Utc::now();
    match scenario.topology {
        LinkTopology::None => {},
        LinkTopology::Internal => {
            let available = moved_ids.len().saturating_sub(1);
            let density = scenario.links_per_entity.min(available);
            for (idx, id) in moved_ids.iter().enumerate() {
                for step in 0..density {
                    let target_idx = (idx + 1 + step) % moved_ids.len();
                    if target_idx == idx {
                        continue;
                    }
                    store
                        .add_edge(EdgeRecord {
                            from: *id,
                            to: moved_ids[target_idx],
                            kind: "linked".to_string(),
                            created_at: now,
                        })
                        .expect("add internal scenario edge");
                }
            }
        },
        LinkTopology::Crossing => {
            let density =
                scenario.links_per_entity.min(external_ids.len());
            for (idx, id) in moved_ids.iter().enumerate() {
                for step in 0..density {
                    let target_idx = (idx + step) % external_ids.len();
                    store
                        .add_edge(EdgeRecord {
                            from: *id,
                            to: external_ids[target_idx],
                            kind: "linked".to_string(),
                            created_at: now,
                        })
                        .expect("add crossing scenario edge");
                }
            }
        },
    }

    (workspace_dir, store, target_root, moved_ids)
}

fn active_move_preflight(
    store: &TicketStore,
    target_root: &Path,
    id: &Uuid,
) -> ticket_api::storage::move_planner::MovePreflightReport {
    let mut plan = store
        .plan_move_preflight(id, target_root)
        .expect("plan preflight");
    plan.blockers.retain(|blocker| {
        !matches!(
            blocker,
            MovePreflightBlocker::PathReferenceScanUnavailable { .. }
                | MovePreflightBlocker::DirtyTrackedFiles { .. }
        )
    });
    assert!(
        plan.blockers.is_empty(),
        "unexpected move blockers: {:?}",
        plan.blockers
    );
    plan
}

/// Build the pool for `scenario` once. Apply/Rollback need `pool_size`
/// independent batches, so the underlying fixture is built `pool_size`
/// entities wide and sliced into `entity_count`-sized chunks — one bulk
/// setup instead of `pool_size` separate fixture rebuilds. Rollback
/// additionally executes every move up front (unmeasured) so each batch is
/// a set of real, distinct journal ids ready to roll back.
///
/// Caveat: because Apply/Rollback share one bulk fixture across all
/// batches, the store holds `entity_count * pool_size` tickets throughout —
/// larger than the single-batch fixture Preflight uses. If a measured call
/// has a cost component proportional to *total store size* rather than just
/// the touched entities, this will read as somewhat pessimistic versus a
/// freshly-sized store; no evidence of such a component has been found so
/// far (Apply/Rollback costs track batch-local edge count, not pool size).
fn build_scenario_pool(
    scenario: MoveScenario,
    pool_size: usize,
) -> ScenarioPool {
    match scenario.phase {
        MovePhase::Preflight => {
            let (workspace_dir, store, target_root, ids) =
                build_move_scenario_fixture(scenario);
            ScenarioPool {
                _workspace_dir: workspace_dir,
                store,
                target_root,
                phase: scenario.phase,
                batches: vec![ids],
            }
        },
        MovePhase::Apply => {
            let bulk = MoveScenario {
                entity_count: scenario.entity_count * pool_size,
                ..scenario
            };
            let (workspace_dir, store, target_root, ids) =
                build_move_scenario_fixture(bulk);
            let batches = ids
                .chunks(scenario.entity_count)
                .take(pool_size)
                .map(<[Uuid]>::to_vec)
                .collect();
            ScenarioPool {
                _workspace_dir: workspace_dir,
                store,
                target_root,
                phase: scenario.phase,
                batches,
            }
        },
        MovePhase::Rollback => {
            let bulk = MoveScenario {
                entity_count: scenario.entity_count * pool_size,
                ..scenario
            };
            let (workspace_dir, store, target_root, ids) =
                build_move_scenario_fixture(bulk);
            let journal_ids: Vec<Uuid> = ids
                .iter()
                .map(|id| {
                    let plan =
                        active_move_preflight(&store, &target_root, id);
                    store
                        .execute_move_with_journal(&plan)
                        .expect("execute move for rollback setup")
                        .journal
                        .id
                })
                .collect();
            let batches = journal_ids
                .chunks(scenario.entity_count)
                .take(pool_size)
                .map(<[Uuid]>::to_vec)
                .collect();
            ScenarioPool {
                _workspace_dir: workspace_dir,
                store,
                target_root,
                phase: scenario.phase,
                batches,
            }
        },
    }
}

/// Sample size floor Criterion enforces (`Criterion` rejects `sample_size <
/// 10`); every scenario's `measurement_time` budget below is derived to fit
/// this many real iterations, not the other way around.
const SCENARIO_SAMPLE_SIZE: usize = 10;

/// Real single-iteration cost of *just* the production call this scenario
/// measures (fixture setup is built once via a throwaway `pool_size=1` pool
/// and excluded from the timed portion, mirroring exactly what Criterion
/// itself times below). This is the actual entity_count/linkage-driven cost,
/// not a guessed or extrapolated number, so the `measurement_time` budget
/// computed from it reflects reality per scenario instead of one blanket
/// duration applied to the whole matrix.
fn calibrate_scenario_cost(scenario: MoveScenario) -> Duration {
    let pool = build_scenario_pool(scenario, 1);
    let cursor = Cell::new(0usize);
    let batch = pool.next_batch(&cursor);
    let start = Instant::now();
    pool.run_batch(batch);
    start.elapsed()
}

/// First non-flag CLI argument, mirroring Criterion's own substring-filter
/// convention (`cargo bench -- <filter>`). Checked before calibrating each
/// scenario so a filtered manual run (e.g. one scenario id) does not pay the
/// real setup+measure cost of every other scenario in the matrix just to
/// have Criterion discard it afterward.
fn requested_scenario_filter() -> Option<String> {
    std::env::args().skip(1).find(|arg| !arg.starts_with('-'))
}

/// Every `(scenario, bench_id)` pair the matrix covers, shared between the
/// real Criterion registration loop and `estimate_scenario_matrix_budget` so
/// the two enumerations cannot drift apart.
fn scenario_matrix() -> Vec<(MoveScenario, String)> {
    let entity_counts = [1usize, 25, 100, 500];
    let topologies =
        [LinkTopology::None, LinkTopology::Internal, LinkTopology::Crossing];
    let phases =
        [MovePhase::Preflight, MovePhase::Apply, MovePhase::Rollback];

    let mut scenarios = Vec::new();
    for &entity_count in &entity_counts {
        for &topology in &topologies {
            // A batch of one has no other moved entity to link to internally.
            if topology == LinkTopology::Internal && entity_count < 2 {
                continue;
            }

            let densities: &[usize] = match topology {
                LinkTopology::None => &[0],
                LinkTopology::Internal | LinkTopology::Crossing => {
                    &[1, 5, 20]
                },
            };

            for &links_per_entity in densities {
                for &phase in &phases {
                    let scenario = MoveScenario {
                        entity_count,
                        topology,
                        links_per_entity,
                        phase,
                    };
                    let bench_id = format!(
                        "{entity_count}entities_{topo}_{links_per_entity}links_{phase}",
                        topo = topology.label(),
                        phase = phase.label(),
                    );
                    scenarios.push((scenario, bench_id));
                }
            }
        }
    }
    scenarios
}

/// `measurement_time` budget for a scenario given its real calibrated
/// single-iteration cost: `SCENARIO_SAMPLE_SIZE` real samples plus a 30%
/// margin absorbing the variance Criterion itself discovers across samples.
fn measurement_budget(calibrated: Duration) -> Duration {
    calibrated
        .saturating_mul(SCENARIO_SAMPLE_SIZE as u32)
        .mul_f64(1.3)
        .max(Duration::from_millis(200))
}

/// Real, measured (not guessed) total wall-time budget for every scenario in
/// the matrix: one calibration iteration per scenario (entity_count- and
/// linkage-driven, exactly as `calibrate_scenario_cost` measures it), summed
/// into the same `warm_up + measurement_time` budget the real Criterion run
/// below would use per scenario — at roughly 1/10th the cost, since it skips
/// the 10-sample collection. Enabled via `MOVE_BENCH_ESTIMATE_ONLY=1` so the
/// full-matrix total can be derived from real data before committing to the
/// full run.
fn estimate_scenario_matrix_budget() {
    let mut total = Duration::ZERO;
    for (scenario, bench_id) in scenario_matrix() {
        let calibrated = calibrate_scenario_cost(scenario);
        let budget = calibrated.max(Duration::from_millis(50))
            + measurement_budget(calibrated);
        total += budget;
        eprintln!(
            "{bench_id}: calibrated_iter={:.3}s budget={:.1}s running_total={:.1}s",
            calibrated.as_secs_f64(),
            budget.as_secs_f64(),
            total.as_secs_f64(),
        );
    }
    eprintln!(
        "move_scenario_matrix estimate: {} scenarios, total derived budget = {:.1}s ({:.1} min)",
        scenario_matrix().len(),
        total.as_secs_f64(),
        total.as_secs_f64() / 60.0,
    );
}

fn bench_move_scenario_matrix(c: &mut Criterion) {
    init_perf_bench_tracing();

    if std::env::var("MOVE_BENCH_ESTIMATE_ONLY").as_deref() == Ok("1") {
        estimate_scenario_matrix_budget();
        return;
    }

    let mut group = c.benchmark_group("move_scenario_matrix");
    // The fixture (git init + N tickets + edges) is built once per scenario
    // via `build_scenario_pool`, not once per Criterion iteration/sample, so
    // the measured time below is the real production call only. Scenario
    // cost still varies enormously (entity count, link density), so a single
    // blanket measurement_time would waste time on cheap scenarios or
    // overrun on expensive ones; each scenario's `measurement_time` is sized
    // from its own calibrated per-call cost instead.
    group.sample_size(SCENARIO_SAMPLE_SIZE);
    let filter = requested_scenario_filter();
    let mut total_expected = Duration::ZERO;
    let mut scenario_count = 0usize;

    for (scenario, bench_id) in scenario_matrix() {
        let full_bench_id = format!("move_scenario_matrix/move/{bench_id}");
        if let Some(filter) = &filter {
            if !full_bench_id.contains(filter.as_str()) {
                continue;
            }
        }

        let calibrated = calibrate_scenario_cost(scenario);
        let warm_up = calibrated.max(Duration::from_millis(50));
        let measurement = measurement_budget(calibrated);
        total_expected += warm_up + measurement;
        scenario_count += 1;
        group.warm_up_time(warm_up);
        group.measurement_time(measurement);

        // Built once per scenario, outside `bench_with_input`'s closure:
        // Criterion invokes that closure once per sample (not once total),
        // so a pool built *inside* it would still be rebuilt every sample.
        let pool = build_scenario_pool(scenario, SCENARIO_POOL_SIZE);
        let cursor = Cell::new(0usize);

        group.throughput(Throughput::Elements(scenario.entity_count as u64));
        group.bench_with_input(
            BenchmarkId::new("move", bench_id),
            &scenario,
            |b, _scenario| {
                b.iter_batched(
                    || pool.next_batch(&cursor),
                    |batch| pool.run_batch(batch),
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    eprintln!(
        "move_scenario_matrix: {scenario_count} scenarios, calibrated total budget = {:.1}s",
        total_expected.as_secs_f64(),
    );

    group.finish();
}

criterion_group!(
    benches,
    bench_open_or_init_root_perf_fixture,
    bench_scan_reindex_root_perf_fixture,
    bench_scan_incremental_root_perf_fixture_1,
    bench_scan_incremental_root_perf_fixture_10,
    bench_scan_incremental_root_perf_fixture_100,
    bench_move_preflight_reference_heavy,
    bench_move_execute_reference_heavy,
    bench_move_rollback_reference_heavy,
    bench_move_scenario_matrix,
    bench_health_workflow_build_large_fixture,
    bench_health_collect_large_fixture,
    bench_health_all_large_fixture,
);
criterion_main!(benches);
