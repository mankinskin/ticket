# Goal

Remove the major performance regressions surfaced by the move-domain E2E and perf scan tests in the ticket store hot path.

## Problem summary

Focused validation with `cargo test -p ticket-api --test e2e_perf_move_health -- --nocapture | rg "move_perf|move_missing_ref_perf|health_perf|move_seq_perf"` originally showed four concrete issues:

1. `scan(false)` behaved like a full rescan of every ticket root instead of a cheap incremental reconciliation.
2. search indexing dominated scan time because the Tantivy path committed and waited for merge threads per document.
3. move execution forced expensive rescans of both source and target stores through `scan(true)` after each move.
4. `search_rebuild_check_ms` measured the whole scan window instead of only the rebuild probe.

## Implementation summary

Implemented hot-path fixes in:

- `memory-api/crates/memory-api/src/storage/search.rs`
  - added batched search upserts so scan integration reuses one Tantivy writer/commit per batch instead of per ticket.
- `memory-api/crates/ticket-api/src/storage/store/scan.rs`
  - switched scan integration to use batched search writes.
  - added a conservative unchanged-entry fast path for `scan(false)` using indexed metadata equality plus manifest/description mtime checks against `updated_at`.
  - fixed `search_rebuild_check_ms` so it measures only the rebuild probe.
- `memory-api/crates/ticket-api/src/storage/move_planner.rs`
  - changed ticket move-domain rescans from `scan(true)` to `scan(false)`.

## Validation

Command run:

- `cargo test -p ticket-api --test e2e_perf_move_health -- --nocapture | rg "move_perf|move_missing_ref_perf|health_perf|move_seq_perf|error|FAILED|panicked"`

Representative before/after deltas on the same perf fixture:

- `move_perf execute_ms`: `17746` -> `5209`
- `move_perf scan_target_ms`: `12121` -> `2790`
- `move_seq_perf first_ms`: `36952` -> `11392`
- `move_seq_perf second_ms`: `40013` -> `10075`
- `scan_true integration.search_upsert_ms`: `12340` -> `187`
- `scan_false_1 integration.index_upsert_ms`: `2192` -> `33`
- `scan_false_1 integration.search_upsert_ms`: `14003` -> `126`
- `search_rebuild_check_ms`: previously aliased `scan_total_ms`; now reports probe-only values (`0`, `6`, `14`, `8` ms in the perf run)

## Remaining behavior to watch

- `rebuild_workflow_facts_ms` is now the dominant remaining scan cost on the perf fixture. That is outside the scan/search write hot path fixed here and now has dedicated follow-up planning tickets.

## Acceptance criteria status

- `scan(false)` no longer redoes full search/index integration work for unchanged entries on the perf fixture: complete.
- scan/search write cost no longer performs one commit+merge wait per entity during bulk scan integration: complete.
- move execution no longer pays two forced full-store rescans on the ticket perf fixture: complete.
- perf E2E remains passing and shows materially reduced `scan_target_ms` / `integration.search_upsert_ms` on the same fixture: complete.
- `search_rebuild_check_ms` reports only the rebuild probe window, not total scan duration: complete.

## Traceability

- Spec: `memory-api/store-scan-move-hot-path-performance` (`0adfbd09-15c7-46ee-be24-03da0564833d`)
- Remaining-work tracker: `C:/Users/linus/git/graph_app/context-engine/.ticket/tickets/cadf78e8-a243-4d1c-8c1b-451978bb05ea/ticket.toml`
- Follow-up child tickets:
  - `C:/Users/linus/git/graph_app/workflow-tools/ticket/.workflow-tools/ticket/tickets/3e4718af-3fd3-40a4-ac89-d298c99c806a/ticket.toml`
  - `C:/Users/linus/git/graph_app/workflow-tools/ticket/.workflow-tools/ticket/tickets/875919d5-558c-46a8-a83f-02a6756a1e0e/ticket.toml`
  - `C:/Users/linus/git/graph_app/workflow-tools/ticket/.workflow-tools/ticket/tickets/013b57bd-2e8c-4d4d-87c8-6f8687a195c8/ticket.toml`
