use std::{
    path::PathBuf,
    sync::OnceLock,
    time::{Duration, Instant},
};

use chrono::Utc;
use criterion::{Criterion, criterion_group, criterion_main};
use memory_fixtures::{
    TicketPerfFixtureOptions, append_fixture_ticket,
    materialize_git_fixture_with_ticket_perf_load,
};
use memory_kernel::storage::move_kernel::MoveSetExecutionPhase;
use memory_kernel::testing::{
    MOVE_BENCH_SAMPLE_SIZE, MOVE_BENCH_WARM_UP, MoveBenchmarkWorkspace,
    drop_fixture_blockers, move_bench_criterion, move_bench_measurement_time,
};
use ticket_api::{
    model::edge::EdgeRecord,
    storage::{
        move_execution::MoveExecutionPhase, move_planner::MovePreflightBlocker, store::TicketStore,
    },
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

mod move_health_legacy;
mod move_health_scenarios;

criterion_group! {
    name = benches;
    config = move_bench_criterion();
    targets =
        move_health_legacy::bench_move_preflight_reference_heavy,
        move_health_legacy::bench_move_execute_reference_heavy,
        move_health_legacy::bench_move_rollback_reference_heavy,
        move_health_scenarios::bench_move_scenario_matrix
}
criterion_main!(benches);
