use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
    time::{Duration, Instant},
};

use chrono::Utc;
use criterion::{Criterion, criterion_group, criterion_main};
use memory_fixtures::{
    TicketPerfFixtureOptions, append_fixture_ticket, materialize_fixture_with_ticket_perf_load,
    materialize_git_fixture_with_ticket_perf_load,
};
use memory_kernel::storage::move_kernel::MoveSetExecutionPhase;
use memory_kernel::testing::{
    MOVE_BENCH_SAMPLE_SIZE, MOVE_BENCH_WARM_UP, MoveBenchmarkWorkspace, drop_fixture_blockers,
    move_bench_criterion, move_bench_measurement_time,
};
use ticket_api::{
    health::collect_findings,
    model::edge::EdgeRecord,
    storage::{
        move_execution::MoveExecutionPhase, move_planner::MovePreflightBlocker, store::TicketStore,
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

fn add_perf_edges(store: &TicketStore, ids: &[Uuid]) {
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

fn append_incremental_fixture_tickets(root_store: &Path, batch: usize, count: usize) {
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

fn percentile_nearest_rank(sorted: &[u64], percentile: u32) -> u64 {
    assert!(!sorted.is_empty(), "percentile input must not be empty");
    assert!(percentile <= 100, "percentile must be <= 100");
    if sorted.len() == 1 {
        return sorted[0];
    }
    let rank = ((percentile as f64 / 100.0) * sorted.len() as f64).ceil() as usize;
    sorted[rank.saturating_sub(1).min(sorted.len() - 1)]
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

fn map_percentiles(values: &std::collections::BTreeMap<String, u64>) -> (u64, u64, u64) {
    percentile_summary(&values.values().copied().collect::<Vec<_>>())
}

mod move_health_health;
mod move_health_legacy;
mod move_health_scenarios;

criterion_group! {
    name = benches;
    config = move_bench_criterion();
    targets =
        move_health_health::bench_open_or_init_root_perf_fixture,
        move_health_health::bench_scan_reindex_root_perf_fixture,
        move_health_health::bench_scan_incremental_root_perf_fixture_1,
        move_health_health::bench_scan_incremental_root_perf_fixture_10,
        move_health_health::bench_scan_incremental_root_perf_fixture_100,
        move_health_legacy::bench_move_preflight_reference_heavy,
        move_health_legacy::bench_move_execute_reference_heavy,
        move_health_legacy::bench_move_rollback_reference_heavy,
        move_health_health::bench_health_workflow_build_large_fixture,
        move_health_health::bench_health_collect_large_fixture,
        move_health_health::bench_health_all_large_fixture,
        move_health_scenarios::bench_move_scenario_matrix
}
criterion_main!(benches);
