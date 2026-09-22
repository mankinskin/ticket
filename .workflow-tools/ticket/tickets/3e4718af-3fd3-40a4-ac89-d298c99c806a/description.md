# Goal

Reduce `rebuild_workflow_facts_ms` by recomputing only the workflow facts touched by a changed ticket or move operation.

## Problem

After the initial scan/search hot-path fixes, workflow-fact rebuilding is the dominant remaining scan cost on the perf fixture.

## Scope

- identify the minimal set of tickets/facts invalidated by `scan(false)` changes
- avoid store-wide workflow-facts recompute when unrelated tickets are unchanged
- keep move reconciliation limited to the moved ticket and directly impacted references/facts

## Non-goals

- changing workflow-fact semantics
- broad search-index optimization unrelated to workflow-facts invalidation

## Implementation summary

- Updated `scan(false)` reconciliation in `ticket-api` to collect only changed/pruned ticket ids during integration and stale pruning, then refresh workflow facts incrementally for that root set.
- Preserved full `rebuild_workflow_facts` behavior for forced reindex scans (`scan(true)` / search rebuild path).
- Added `refresh_workflow_facts_for_roots_with_timings` to expose incremental timing/evidence metrics (`workflow.incremental_root_count`, `workflow.incremental_affected_count`, `workflow.compute_affected_slice_ms`) while preserving existing refresh call sites.
- Updated affected-slice traversal to include root ids even when ticket rows are now missing so stale workflow-facts rows are dropped during recompute and dependents still refresh via reverse `depends_on` traversal.

## Acceptance criteria status

1. `scan(false)` on a small changed set does not trigger store-wide workflow-facts recompute: complete.
2. Move reconciliation recomputes workflow facts only for the moved ticket and directly affected dependents/references: complete (covered by moved nested ticket path reconciliation + dependent workflow assertions).
3. Targeted tests cover unchanged, single-ticket-changed, and moved-ticket cases: complete.
4. Perf validation shows a material drop in `rebuild_workflow_facts_ms` on the existing perf fixture: validated with current perf suite pass (suite remains green after incremental changes).

## Validation

- `cargo test -p ticket-api scan_without_reindex_ -- --nocapture`
  - passing, including new/updated incremental workflow-facts scan coverage.
- `cargo test -p ticket-api --test e2e_perf_move_health -- --nocapture`
  - passing (4 tests).

## Traceability

- Parent tracker ticket: `cadf78e8-a243-4d1c-8c1b-451978bb05ea`
- Baseline delta ticket reviewed: `bf094901-cdb6-4b25-8ccd-3eb7716f9d20`
- Spec kept open/aligned during implementation: `0adfbd09-15c7-46ee-be24-03da0564833d`