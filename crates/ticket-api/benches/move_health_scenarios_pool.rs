use std::cell::Cell;

use super::*;
use super::{MovePhase, MoveScenario, drop_fixture_blockers};

pub struct ScenarioPool {
    _workspace: MoveBenchmarkWorkspace,
    store: TicketStore,
    target_root: PathBuf,
    phase: MovePhase,
    batches: Vec<Vec<Uuid>>,
}

impl ScenarioPool {
    pub fn next_batch(&self, cursor: &Cell<usize>) -> Vec<Uuid> {
        match self.phase {
            MovePhase::Preflight => self.batches[0].clone(),
            MovePhase::Apply | MovePhase::Rollback => {
                let index = cursor.get();
                cursor.set(index + 1);
                self.batches
                    .get(index)
                    .unwrap_or_else(|| panic!("scenario pool exhausted after {index} iterations"))
                    .clone()
            }
        }
    }

    pub fn run_batch(&self, ids: Vec<Uuid>) {
        match self.phase {
            MovePhase::Preflight => {
                let plan = self
                    .store
                    .plan_move_set(&ids, &self.target_root)
                    .expect("plan move set");
                criterion::black_box(plan);
            }
            MovePhase::Apply => {
                let mut plan = self
                    .store
                    .plan_move_set(&ids, &self.target_root)
                    .expect("plan move set");
                for entity_plan in &mut plan.entity_plans {
                    drop_fixture_blockers(entity_plan);
                }
                let outcome = self
                    .store
                    .execute_move_set(&plan)
                    .expect("execute move set");
                assert_eq!(outcome.journal.phase, MoveSetExecutionPhase::Validated);
                criterion::black_box(outcome);
            }
            MovePhase::Rollback => {
                for journal_id in ids {
                    let outcome = self
                        .store
                        .rollback_move_set(journal_id)
                        .expect("rollback move");
                    assert_eq!(outcome.journal.phase, MoveSetExecutionPhase::RolledBack);
                    criterion::black_box(outcome);
                }
            }
        }
    }
}

pub fn build_scenario_pool(scenario: MoveScenario, pool_size: usize) -> ScenarioPool {
    let workspace = MoveBenchmarkWorkspace::new();
    let fixture_entity_count = match scenario.phase {
        MovePhase::Preflight => scenario.entity_count,
        MovePhase::Apply | MovePhase::Rollback => scenario.entity_count * pool_size,
    };
    let fixture_scenario = MoveScenario {
        entity_count: fixture_entity_count,
        ..scenario
    };
    let (store, target_root, ids) =
        super::build_move_scenario_fixture(&workspace, fixture_scenario);
    let batches = match scenario.phase {
        MovePhase::Preflight => vec![ids],
        MovePhase::Apply => ids
            .chunks(scenario.entity_count)
            .take(pool_size)
            .map(<[Uuid]>::to_vec)
            .collect(),
        MovePhase::Rollback => {
            let journals = ids
                .chunks(scenario.entity_count)
                .take(pool_size)
                .map(|batch| {
                    let mut plan = store
                        .plan_move_set(batch, &target_root)
                        .expect("plan move set for rollback setup");
                    for entity_plan in &mut plan.entity_plans {
                        drop_fixture_blockers(entity_plan);
                    }
                    store
                        .execute_move_set(&plan)
                        .expect("execute move for rollback setup")
                        .journal
                        .id
                })
                .collect::<Vec<_>>();
            journals
                .into_iter()
                .map(|journal_id| vec![journal_id])
                .collect()
        }
    };
    ScenarioPool {
        _workspace: workspace,
        store,
        target_root,
        phase: scenario.phase,
        batches,
    }
}
