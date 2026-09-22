## Problem

Many tickets have `type = "task"`, but no schema was registered for the `task` type. Transitions/close operations failed with `no schema for type 'task'` (see the schema lookup in `memory-api/crates/ticket-api/src/storage/store/lifecycle.rs`), and prior sessions worked around it by renaming the type to `tracker-improvement`.

Root cause: `TicketStore::create` only validates a manifest when a schema is known (`if let Some(schema) = self.schema_registry.get(type_id)`), so unknown types were silently accepted, then broke later at transition time.

## Implementation

1. Added built-in `task` schema (clone of `tracker-improvement`, dedicated `type_id`):
   - `TASK_TYPE_ID`, `task_schema()` in `memory-api/crates/ticket-api/src/model/default_schema.rs`.
   - Registered in `SchemaRegistry::with_builtins`, `is_builtin_type`, and `schema_for_type`.
2. Added ticket-api health criterion `unknown_type` (severity `error`) in `memory-api/crates/ticket-api/src/health.rs`, run for every ticket in `collect_findings` (before the done/cancelled skip), so any ticket whose `type_id` has no registered schema is surfaced by `ticket health` / MCP `health_check` going forward.

## Validation

- `cargo build -p ticket-api` — ok.
- `cargo test -p ticket-api --lib` — 123 passed, 1 pre-existing failure (`storage::move_planner::tests::preflight_reports_invisible_reference_visibility_and_path_refs`, confirmed failing on baseline via `git stash`; unrelated git/move test).
- New tests: `unknown_type_produces_error_finding`, `task_type_has_registered_schema_and_no_unknown_type_finding`, `task_schema_uses_task_type_id`, `builtin_type_checks_include_task` — all pass.
- `cargo build -p ticket-cli -p ticket-http -p ticket-mcp` — ok.

## Acceptance Criteria

- AC-1 (met): `task` type resolves to a schema; creation and transitions work like `tracker-improvement`.
- AC-2 (met): `collect_findings` emits an `unknown_type` error finding for tickets whose type has no schema.
- AC-3 (met): ticket-api lib tests pass except the pre-existing unrelated move-planner failure.

## Notes

- `memory-api/ticket-vscode/src/extensionSupport.ts` `TICKET_TYPES` is a stale UI hint (already omits `bug`) and is not the validation authority; left unchanged (out of scope).