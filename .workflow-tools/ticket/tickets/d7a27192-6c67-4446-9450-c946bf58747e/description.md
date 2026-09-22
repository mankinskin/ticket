# Problem

`ticket-viewer` currently resolves the local `.ticket` workspace and calls
`TicketStore::open(...)` during startup. In this checkout the repo already has
`.ticket/tickets/...` manifests, but the SQLite index artifacts (`tickets.db`,
`search_index/`) are missing. Starting the server therefore panics with
`WorkspaceNotFound` instead of rebuilding the local index and serving the app.

## Scope

- switch ticket-viewer startup to the store's idempotent bootstrap path so
  manifest-only local workspaces rebuild their SQLite index on launch
- replace the panicing startup path with a normal error return if store
  initialization still fails
- add a focused regression test for the manifest-only workspace case

## Acceptance criteria

- starting `ticket-viewer` from the repo root no longer panics when `.ticket/`
  contains ticket manifests but no `tickets.db`
- the startup path rebuilds the local SQLite index and preserves existing
  tickets on disk
- if initialization still fails, startup returns a contextual error instead of
  panicking via `expect(...)`

## Validation

- Passed: `cargo test -p ticket-viewer --bin ticket-viewer startup_bootstraps_manifest_only_ticket_store -- --nocapture`
- Passed: `./target/debug/ticket-viewer.exe --port 0` (server started successfully and printed `TICKET_VIEWER_PORT=59348` before shutdown)
- Blocked unrelated: `cargo test -p ticket-viewer` currently fails in `memory-viewers/ticket-viewer/tests/workspace_resolution.rs` because it imports removed `ticket_api::workspace::{find_local_workspace_file_from, LOCAL_WORKSPACE_FILE}` symbols.