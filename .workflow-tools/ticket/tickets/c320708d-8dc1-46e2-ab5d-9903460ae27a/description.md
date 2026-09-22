Two copies of the same board module exist, confirmed by skeleton inspection:

- LIVE copy: memory-api/crates/memory-api/src/storage/board.rs — 425 lines, defines BoardEntry, BoardEntryStatus, BoardConfig, BoardSnapshot, ActiveWorktree, BoardHistorySnapshot, BoardCleanPreview, BoardCleanResult, ReconcileAction, BoardReconcileResult, BoardError.
- STALE copy: memory-kernel/src/storage/board.rs — 295 lines, defines the same set of types MINUS `ActiveWorktree`.

The two copies have now diverged: ticket c060bf94-2435-4cc5-8016-ca1d2c8264f5 ("Bind board entries to sessions and worktrees; add active-worktree discovery") added `session_id`/`worktree_path`/`branch` fields to `BoardEntry`, the `ActiveWorktree` type, a `WorktreeConflict` error, and `board_worktrees` surfaces — all applied only to the memory-api copy. The memory-kernel copy is off the live call path and was never updated. The user has explicitly confirmed this duplication is NOT intentional.

Acceptance criteria:
1. Establish whether memory-kernel/src/storage/board.rs has ANY live callers anywhere in the workspace before making any change.
2. If it has no live callers, delete memory-kernel/src/storage/board.rs (and any now-dead re-exports/module declarations pointing at it). If it does have live callers, replace its body with a re-export of the memory-api definitions instead of deleting it outright.
3. `cargo check` passes across the whole workspace after the change.

Reference: ticket c060bf94-2435-4cc5-8016-ca1d2c8264f5 is the ticket whose changes caused this divergence.
## Handoff package (from the worktree-helper session)

**Next objective:** confirm there are no live callers of `memory-kernel/src/storage/board.rs`, then either delete it or make it re-export the memory-api definitions, and prove `cargo check --workspace` passes.

**Repository state at handoff:** `main` is at `b84506a3`, fully pushed to `origin/main`. Submodule `memory-api` commit `c6eb0a3a` was pushed to its own origin *before* the superproject, repairing a pointer that previously resolved only on the original machine. All five submodules are healthy and in sync.

**Commits from that session:**
- `bb0b0ca3` feat(tooling): add git-worktree helper script for agent isolation protocol
- `a415b25c` chore(tickets): add worktree helper script and board dedup tickets
- `4909db52` fix(tooling): drop harmful submodule deinit from worktree teardown
- `b84506a3` chore(tickets): record worktree helper protocol fix evidence

**The duplication to resolve:**
- LIVE: `memory-api/crates/memory-api/src/storage/board.rs` — 425 lines; defines `BoardEntry`, `BoardEntryStatus`, `BoardConfig`, `BoardSnapshot`, `ActiveWorktree`, `BoardHistorySnapshot`, `BoardCleanPreview`, `BoardCleanResult`, `ReconcileAction`, `BoardReconcileResult`, `BoardError`.
- STALE: `memory-kernel/src/storage/board.rs` — 295 lines; the same set **minus** `ActiveWorktree`. Off the live call path.
- Cause of divergence: ticket `c060bf94` added `session_id`/`worktree_path`/`branch` to `BoardEntry`, plus the `ActiveWorktree` type, a `WorktreeConflict` error, and `board_worktrees` MCP/CLI surfaces — all applied to the memory-api copy only.
- The user explicitly confirmed this duplication is **not** intentional.

**Decisions carried forward:**
- Worktree teardown is now `git worktree remove --force <path>` then `git worktree prune` then `git branch -d <branch>`. The former `submodule deinit` and `submodule init` repair steps were removed.
- `git submodule deinit` must NEVER run inside a linked worktree: it rewrites `submodule.*` in the SHARED `.git/config` and silently deinitializes the main checkout's submodules. `git worktree remove --force` handles submodules directly, per git's own documentation.
- `extensions.worktreeConfig` does not help — submodule init/deinit always read and write the shared `.git/config` regardless.
- The "each submodule needs its own worktree" hypothesis is DISPROVEN: the superproject still detects submodules structurally so the removal refusal still triggers; `git submodule status` reports the submodule uninitialized because it was never registered; and every submodule worktree needs its own separate teardown.
- Submodules must always be pushed before the superproject, or the superproject publishes pointers to unreachable commits.

**Tooling now available:** `tools/worktree/worktree.sh` — subcommands `new`, `list`, `rebase`, `merge`, `remove`, `doctor`; `--dry-run` on every mutating subcommand; refuses destructive operations from inside a linked worktree. Known quirk it self-repairs: a new worktree's submodule init can fail to find a memory-api commit present only in the main checkout's clone.

**Validation commands:** `bash -n tools/worktree/worktree.sh`; `bash tools/worktree/worktree.sh doctor`; `cargo check --workspace`.

**Non-goals:** wiring `worktree.sh` into `install-tools.sh`; triaging the in-review ticket backlog; pushing the `backup/pre-lockfile-rebase` branch.

**Risks / open items:** `shellcheck` is NOT installed in this environment, so `worktree.sh` has no lint coverage beyond `bash -n`. The parent repository at `c:/Users/linus/git/graph_app` tracks context-engine as a submodule and its pointer was NOT bumped. The local branch `backup/pre-lockfile-rebase-fda1a6e39adac9da80496fc053d15995f18a7439` is 2109 commits ahead of origin and was deliberately not pushed. Roughly 37 tickets sit in `in-review` and were not triaged.