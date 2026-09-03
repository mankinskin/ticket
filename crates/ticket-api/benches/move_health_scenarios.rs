use std::cell::Cell;

use super::*;
use criterion::{BenchmarkId, Throughput};

#[path = "move_health_scenarios_pool.rs"]
mod move_health_scenarios_pool;
use move_health_scenarios_pool::build_scenario_pool;

const CROSSING_EXTERNAL_POOL: usize = 20;
const SCENARIO_POOL_SIZE: usize = MOVE_BENCH_SAMPLE_SIZE * 3;
const SCENARIO_SAMPLE_SIZE: usize = MOVE_BENCH_SAMPLE_SIZE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LinkTopology {
    None,
    Crossing,
}

impl LinkTopology {
    fn label(self) -> &'static str {
        match self {
            Self::None => "no_links",
            Self::Crossing => "crossing_links",
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
            Self::Preflight => "preflight",
            Self::Apply => "apply",
            Self::Rollback => "rollback",
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

fn scenario_ticket_id(prefix: u8, offset: usize) -> String {
    format!("{prefix:08x}-0000-6000-8000-{offset:012x}")
}

fn build_move_scenario_fixture(
    workspace: &MoveBenchmarkWorkspace,
    scenario: MoveScenario,
) -> (TicketStore, PathBuf, Vec<Uuid>) {
    workspace.reset();
    let source_root = workspace.source_root().to_path_buf();
    let target_root = workspace.target_root().to_path_buf();
    let store = TicketStore::init(&source_root).expect("init source store");
    TicketStore::init(&target_root).expect("init target store");

    let moved_ids = (0..scenario.entity_count)
        .map(|offset| {
            let id = scenario_ticket_id(0x10, offset);
            append_fixture_ticket(
                &source_root,
                &id,
                &format!("scenario moved ticket {offset}"),
                "planned",
                "perf-scenario",
            )
            .expect("append moved fixture ticket");
            id.parse().expect("valid moved ticket uuid")
        })
        .collect::<Vec<_>>();
    let external_ids = if scenario.topology == LinkTopology::Crossing {
        (0..CROSSING_EXTERNAL_POOL)
            .map(|offset| {
                let id = scenario_ticket_id(0x20, offset);
                append_fixture_ticket(
                    &source_root,
                    &id,
                    &format!("scenario external ticket {offset}"),
                    "planned",
                    "perf-scenario-external",
                )
                .expect("append external fixture ticket");
                id.parse().expect("valid external ticket uuid")
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    store.scan(true).expect("scan scenario store");

    let now = Utc::now();
    match scenario.topology {
        LinkTopology::None => {}
        LinkTopology::Crossing => {
            let density = scenario.links_per_entity.min(external_ids.len());
            for (index, id) in moved_ids.iter().enumerate() {
                for step in 0..density {
                    store
                        .add_edge(EdgeRecord {
                            from: *id,
                            to: external_ids[(index + step) % external_ids.len()],
                            kind: "linked".to_string(),
                            created_at: now,
                        })
                        .expect("add crossing scenario edge");
                }
            }
        }
    }
    (store, target_root, moved_ids)
}

fn scenario_matrix() -> Vec<(MoveScenario, String)> {
    let mut scenarios = Vec::new();
    for &entity_count in &[1usize, 25, 100, 500] {
        for &topology in &[LinkTopology::None, LinkTopology::Crossing] {
            let densities = match topology {
                LinkTopology::None => &[0][..],
                LinkTopology::Crossing => &[5][..],
            };
            for &links_per_entity in densities {
                for &phase in &[MovePhase::Preflight, MovePhase::Apply, MovePhase::Rollback] {
                    if entity_count == 500 && phase != MovePhase::Preflight {
                        continue;
                    }
                    let scenario = MoveScenario {
                        entity_count,
                        topology,
                        links_per_entity,
                        phase,
                    };
                    scenarios.push((
                        scenario,
                        format!(
                            "{entity_count}entities_{topo}_{links_per_entity}links_{phase}",
                            topo = topology.label(),
                            phase = phase.label(),
                        ),
                    ));
                }
            }
        }
    }
    scenarios
}

fn calibrate_scenario_cost(scenario: MoveScenario) -> Duration {
    let pool = build_scenario_pool(scenario, 1);
    let batch = pool.next_batch(&Cell::new(0));
    let started = Instant::now();
    pool.run_batch(batch);
    started.elapsed()
}

fn requested_scenario_filter() -> Option<String> {
    std::env::args().skip(1).find(|arg| !arg.starts_with('-'))
}

fn measurement_budget(calibrated: Duration) -> Duration {
    move_bench_measurement_time(calibrated)
}

fn estimate_scenario_matrix_budget() {
    let mut total = Duration::ZERO;
    let scenarios = scenario_matrix();
    let filter = std::env::var("MOVE_BENCH_ESTIMATE_FILTER").ok();
    let selected = scenarios.iter().filter(|(_, bench_id)| {
        let full_id = format!("move_scenario_matrix/move/{bench_id}");
        filter.as_ref().is_none_or(|value| full_id.contains(value))
    });
    let mut scenario_count = 0usize;
    for (scenario, bench_id) in selected {
        let calibrated = calibrate_scenario_cost(*scenario);
        let budget = calibrated.max(MOVE_BENCH_WARM_UP) + measurement_budget(calibrated);
        total += budget;
        scenario_count += 1;
        eprintln!(
            "{bench_id}: calibrated_iter={:.3}s budget={:.1}s",
            calibrated.as_secs_f64(),
            budget.as_secs_f64()
        );
    }
    eprintln!(
        "move_scenario_matrix estimate: {scenario_count} scenarios, {:.1} min",
        total.as_secs_f64() / 60.0
    );
}

pub fn bench_move_scenario_matrix(c: &mut Criterion) {
    init_perf_bench_tracing();
    if std::env::var("MOVE_BENCH_ESTIMATE_ONLY").as_deref() == Ok("1") {
        estimate_scenario_matrix_budget();
        return;
    }
    let mut group = c.benchmark_group("move_scenario_matrix");
    group.sample_size(SCENARIO_SAMPLE_SIZE);
    let filter = requested_scenario_filter();
    for (scenario, bench_id) in scenario_matrix() {
        let full_id = format!("move_scenario_matrix/move/{bench_id}");
        if filter
            .as_ref()
            .is_some_and(|value| !full_id.contains(value))
        {
            continue;
        }
        let calibrated = calibrate_scenario_cost(scenario);
        group.warm_up_time(calibrated.max(MOVE_BENCH_WARM_UP));
        group.measurement_time(measurement_budget(calibrated));
        let pool = build_scenario_pool(scenario, SCENARIO_POOL_SIZE);
        let cursor = Cell::new(0usize);
        group.throughput(Throughput::Elements(scenario.entity_count as u64));
        group.bench_with_input(BenchmarkId::new("move", bench_id), &scenario, |b, _| {
            b.iter_batched(
                || pool.next_batch(&cursor),
                |batch| pool.run_batch(batch),
                criterion::BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}
