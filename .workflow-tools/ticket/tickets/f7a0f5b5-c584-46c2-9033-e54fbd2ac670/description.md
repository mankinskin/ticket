## Objective
Ensure board state cannot persist active dependencies on another worktree's absolute path or display deleted worktrees as live claims.

## Required behavior
- Board entries do not persist cross-worktree absolute paths as required state.
- A deleted worktree's claim is reconciled to inactive/stale without preventing other board operations.
- Board snapshots never report a deleted worktree as an active owner.
- Legacy path-bearing entries are migrated or safely ignored.

## Owning paths
- `memory-api/crates/memory-api/src/storage/board.rs`
- `memory-api/crates/memory-api/src/storage/board/ops.rs`
- `memory-api/crates/memory-api/src/storage/board/ops/snapshot.rs`

## Validation
Add storage-level tests for foreign-path rejection or normalization, deleted-worktree reconciliation, and a non-phantom board snapshot.

## Parent
Implements one slice of ticket `3a624bf6`; align the final model with the session/MCP routing ticket `968e863b`.