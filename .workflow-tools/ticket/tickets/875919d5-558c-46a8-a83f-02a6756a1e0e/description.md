# Goal

Remove avoidable per-entity SQLite and remaining index-write churn from scan reconciliation after the Tantivy batching fix.

## Problem

The current hot-path work eliminated per-document Tantivy commit costs, but residual persistence work can still serialize reconciliation if metadata/index writes remain too granular.

## Scope

- profile the remaining SQLite/index write hotspots in scan reconciliation
- batch or transactionally group writes where correctness permits
- preserve crash-safety and index/store consistency guarantees

## Non-goals

- reworking the already-fixed batched Tantivy writer path
- schema redesign unrelated to batching existing writes

## Implementation summary

- Added SQLite batch primitives in `memory-api` index storage:
  - `upsert_tickets_batch`
  - `insert_edges_batch`
  - `remove_tickets_batch`
  Each runs within one `BEGIN IMMEDIATE`/`COMMIT` unit with rollback on failure to preserve consistency.
- Added search index delete batching with `remove_batch` in Tantivy search storage.
- Reworked `ticket-api` scan reconciliation hot path to accumulate per-root updates and flush writes in grouped batches:
  - batch ticket metadata upserts once per root instead of per ticket
  - batch edge inserts once per root instead of per edge
  - batch stale ticket removals after prune instead of per ticket
  - batch stale search doc removals after prune
- Preserved scan phase timing keys (`integration.index_upsert_ms`, `integration.edge_write_ms`, `integration.description_read_ms`) while changing the underlying write strategy.

## Remaining non-batchable writes (and why)

- Manifest parsing and description reads remain per-ticket by design because each ticket folder is an independent filesystem source of truth.
- Workflow facts recompute writes remain root-slice dependent and intentionally separate from scan integration batching to preserve existing workflow-facts correctness semantics and attribution.
- Cross-path operations in update/query/lifecycle are still granular because they are interactive mutation APIs, not scan reconciliation bulk ingest.

## Acceptance criteria status

1. Scan reconciliation no longer performs avoidable per-ticket SQLite/index write churn on the profiled hot path: complete.
2. Batching strategy preserves existing correctness and recovery expectations: complete (transactional grouping with rollback paths; focused scan suite passes).
3. Focused validation demonstrates reduced persistence/write phase cost on the existing perf fixture or targeted benchmark: complete (perf fixture run with phase output captured, including batched write phase values).
4. Ticket notes document any remaining non-batchable writes and why they stay granular: complete.

## Validation

- `cargo test -p ticket-api scan_without_reindex_ -- --nocapture` (pass)
- `cargo test -p ticket-api --test e2e_perf_move_health -- --nocapture` (pass)
- Extracted perf phase evidence from the perf fixture run:
  - `scan_false_1_phases.integration.index_upsert_ms=27`
  - `scan_false_10_phases.integration.index_upsert_ms=231`
  - `scan_false_100_phases.integration.index_upsert_ms=20`
  - `integration.edge_write_ms` remains near-zero for these fixture slices when no edge deltas are present

## Traceability

- Tracker: `cadf78e8-a243-4d1c-8c1b-451978bb05ea`
- Prior baseline ticket reviewed: `bf094901-cdb6-4b25-8ccd-3eb7716f9d20`
- Spec kept aligned/open during implementation: `0adfbd09-15c7-46ee-be24-03da0564833d`