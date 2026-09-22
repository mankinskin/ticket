# Goal

Provide targeted reconcile/scan modes so move flows and internal tooling can update only the touched ticket set instead of walking unrelated roots.

## Problem

`scan(false)` is a useful improvement over forced rebuild scans, but move flows and store tooling still need a narrower reconcile mode when the touched entity set is already known.

## Scope

- define internal reconcile entry points or modes for a known set of touched tickets/roots
- wire move execution to use the targeted mode where safe
- identify related internal tooling paths that should use the same narrowed reconcile behavior

## Non-goals

- public API redesign for ticket CLI, HTTP, or MCP
- speculative optimization of unrelated store operations

## Implementation summary

- Added an internal targeted reconcile API in ticket store scan logic:
  - `TicketStore::reconcile_known_tickets(&[Uuid]) -> ScanReport`
  - reconciles only known IDs (integrate/prune/index/search/workflow refresh) instead of walking all scan roots.
  - keeps existing phase telemetry and adds targeted-mode counters (`targeted_reconcile_known_count`, `targeted_reconcile_known_ms`).
- Added move-kernel extension point for touched-set reconciliation:
  - new optional `MoveDomain::reconcile_store_touched(store_root, touched_ids)` defaulting to existing `scan_store` behavior.
  - move execute/resume/rollback scan phases now call the touched-set hook with moved entity id.
- Implemented ticket-domain override in `TicketMoveDomain` to route move scan phases to `reconcile_known_tickets`.
- Added and validated targeted tests:
  - no-op unchanged touched ticket + unaffected ticket remains unchanged
  - moved ticket reconciliation across source/target stores updates source prune + target integration and refreshes dependent workflow facts in source

## Acceptance criteria status

1. Move execution can reconcile a known touched set without walking unrelated ticket roots: complete (move kernel now passes touched id set to ticket domain targeted reconcile).
2. Internal tooling paths that already know the touched set can opt into the targeted reconcile mode: complete (new internal `reconcile_known_tickets` entry point).
3. Tests cover correctness for moved tickets, affected references, and no-op unaffected tickets: complete.
4. Validation evidence shows reduced scan/reconcile work relative to the current `scan(false)` fallback in the targeted scenarios: complete (move perf phase timings show scan phase collapse to targeted work).

## Validation

- `cargo test -p ticket-api reconcile_known_tickets_ -- --nocapture` (pass)
- `cargo test -p ticket-api move_with_journal_ -- --nocapture` (pass)
- `cargo test -p ticket-api --test e2e_perf_move_health -- --nocapture` (pass)
- perf evidence (same fixture) after targeted move reconcile:
  - `move_perf scan_source_ms=131, scan_target_ms=198` (previous broad fallback had materially higher scan phases)
  - `move_seq_perf first_phases.scan_source_ms=87, scan_target_ms=157`
  - `move_seq_perf second_phases.scan_source_ms=102, scan_target_ms=153`

## Traceability

- Tracker: `cadf78e8-a243-4d1c-8c1b-451978bb05ea`
- Baseline: `bf094901-cdb6-4b25-8ccd-3eb7716f9d20`
- Sibling child slices completed/in-review in this session set:
  - `3e4718af-3fd3-40a4-ac89-d298c99c806a`
  - `875919d5-558c-46a8-a83f-02a6756a1e0e`
- Spec kept aligned/open while implementing: `0adfbd09-15c7-46ee-be24-03da0564833d`