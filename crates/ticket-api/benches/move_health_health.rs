use super::*;
use criterion::Throughput;

pub fn bench_open_or_init_root_perf_fixture(c: &mut Criterion) {
    init_perf_bench_tracing();
    c.bench_function("open_or_init_root_perf_fixture", |b| {
        b.iter_batched(
            || {
                let perf =
                    materialize_fixture_with_ticket_perf_load(TicketPerfFixtureOptions::heavy())
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
                    TicketStore::open_or_init_profiled(&root_store).expect("open store");
                let elapsed = started.elapsed();
                criterion::black_box(elapsed);
                let phase_count = report.phase_timings_ms.len();
                let (p50_ms, p95_ms, p99_ms) = map_percentiles(&report.phase_timings_ms);
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

pub fn bench_scan_reindex_root_perf_fixture(c: &mut Criterion) {
    init_perf_bench_tracing();
    c.bench_function("scan_reindex_root_perf_fixture", |b| {
        b.iter_batched(
            || {
                let perf =
                    materialize_fixture_with_ticket_perf_load(TicketPerfFixtureOptions::heavy())
                        .expect("perf fixture should materialize");
                let root_store = perf
                    .fixture
                    .store_root("ticket-root")
                    .expect("root store")
                    .to_path_buf();
                let store = TicketStore::open_or_init(&root_store).expect("open store");
                (perf, store)
            },
            |(_perf, store)| {
                let started = Instant::now();
                let report = store.scan(true).expect("scan(true)");
                let elapsed = started.elapsed();
                let (p50_ms, p95_ms, p99_ms) = map_percentiles(&report.phase_timings_ms);
                criterion::black_box(elapsed);
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

pub fn bench_scan_incremental_root_perf_fixture(
    c: &mut Criterion,
    change_count: usize,
    label: &str,
) {
    init_perf_bench_tracing();
    c.bench_function(label, |b| {
        b.iter_batched(
            || {
                let perf =
                    materialize_fixture_with_ticket_perf_load(TicketPerfFixtureOptions::heavy())
                        .expect("perf fixture should materialize");
                let root_store = perf
                    .fixture
                    .store_root("ticket-root")
                    .expect("root store")
                    .to_path_buf();
                let store = TicketStore::open_or_init(&root_store).expect("open store");
                store.scan(true).expect("initial scan");
                append_incremental_fixture_tickets(&root_store, change_count, change_count);
                (perf, store)
            },
            |(_perf, store)| {
                let started = Instant::now();
                let report = store.scan(false).expect("scan(false)");
                let elapsed = started.elapsed();
                let (p50_ms, p95_ms, p99_ms) = map_percentiles(&report.phase_timings_ms);
                criterion::black_box(elapsed);
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

pub fn bench_scan_incremental_root_perf_fixture_1(c: &mut Criterion) {
    bench_scan_incremental_root_perf_fixture(c, 1, "scan_incremental_root_perf_fixture_1_change");
}

pub fn bench_scan_incremental_root_perf_fixture_10(c: &mut Criterion) {
    bench_scan_incremental_root_perf_fixture(
        c,
        10,
        "scan_incremental_root_perf_fixture_10_changes",
    );
}

pub fn bench_scan_incremental_root_perf_fixture_100(c: &mut Criterion) {
    bench_scan_incremental_root_perf_fixture(
        c,
        100,
        "scan_incremental_root_perf_fixture_100_changes",
    );
}

pub fn bench_health_workflow_build_large_fixture(c: &mut Criterion) {
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
                let store = TicketStore::open_or_init(&root_store).expect("open store");
                store.scan(true).expect("scan store");
                let ids = parse_ids(&perf.root_ticket_ids);
                add_perf_edges(&store, &ids);
                let tickets = store.list(None, None, None).expect("list tickets");
                let all_edges = store.list_all_edges().expect("list edges");
                (perf, store, tickets, all_edges)
            },
            |(_perf, store, tickets, all_edges)| {
                let started = Instant::now();
                let workflow =
                    WorkflowModel::build(&store, tickets, all_edges).expect("build workflow");
                let elapsed = started.elapsed();
                criterion::black_box(elapsed);
                criterion::black_box(workflow);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

pub fn bench_health_collect_large_fixture(c: &mut Criterion) {
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
                let store = TicketStore::open_or_init(&root_store).expect("open store");
                store.scan(true).expect("scan store");
                let ids = parse_ids(&perf.root_ticket_ids);
                add_perf_edges(&store, &ids);
                let tickets = store.list(None, None, None).expect("list tickets");
                let all_edges = store.list_all_edges().expect("list edges");
                let workflow = WorkflowModel::build(&store, tickets.clone(), all_edges.clone())
                    .expect("build workflow");
                (perf, store, tickets, all_edges, workflow)
            },
            |(_perf, store, tickets, all_edges, workflow)| {
                let started = Instant::now();
                let report = collect_findings(&store, &tickets, &all_edges, &workflow);
                let elapsed = started.elapsed();
                criterion::black_box(elapsed);
                criterion::black_box(report.findings.len());
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

pub fn bench_health_all_large_fixture(c: &mut Criterion) {
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
                let store = TicketStore::open_or_init(&root_store).expect("open store");
                store.scan(true).expect("scan store");
                let ids = parse_ids(&perf.root_ticket_ids);
                add_perf_edges(&store, &ids);
                (perf, store)
            },
            |(_perf, store)| {
                let started = Instant::now();
                let tickets = store.list(None, None, None).expect("list tickets");
                let all_edges = store.list_all_edges().expect("list edges");
                let workflow = WorkflowModel::build(&store, tickets.clone(), all_edges.clone())
                    .expect("build workflow");
                let report = collect_findings(&store, &tickets, &all_edges, &workflow);
                let elapsed = started.elapsed();
                criterion::black_box(elapsed);
                criterion::black_box(report.findings.len());
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
