use std::{path::Path, sync::OnceLock, time::Instant};

use chrono::Utc;
use memory_fixtures::{
    FixtureError, TicketPerfFixtureOptions, append_fixture_ticket,
    materialize_fixture_with_ticket_perf_load, materialize_git_fixture_with_ticket_perf_load,
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

fn init_perf_test_tracing() {
    PERF_TRACING.get_or_init(|| {
        let filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("off"));
        let _ = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_test_writer()
            .try_init();
    });
}

fn git_available_or_skip(
    result: Result<memory_fixtures::TicketPerfFixture, FixtureError>,
) -> Option<memory_fixtures::TicketPerfFixture> {
    match result {
        Ok(fixture) => Some(fixture),
        Err(FixtureError::Git { detail, .. }) if detail.contains("os error 2") => None,
        Err(err) => panic!("git perf fixture should materialize: {err}"),
    }
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
            &format!("incremental perf fixture ticket {batch}-{offset}"),
            "planning",
            "perf",
        )
        .expect("append incremental fixture ticket");
    }
}

fn percentile_nearest_rank(sorted: &[u64], percentile: u32) -> u64 {
    assert!(!sorted.is_empty(), "percentile input must not be empty");
    assert!(percentile <= 100, "percentile must be <= 100");

    if sorted.len() == 1 {
        return sorted[0];
    }

    // Nearest-rank method with 1-based rank, then mapped back to 0-based index.
    let rank = ((percentile as f64 / 100.0) * sorted.len() as f64).ceil() as usize;
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

#[path = "e2e_perf_move_health/move_performance.rs"]
mod move_performance;
#[allow(unused_imports)]
pub use move_performance::*;

#[path = "e2e_perf_move_health/health_performance.rs"]
mod health_performance;
#[allow(unused_imports)]
pub use health_performance::*;
