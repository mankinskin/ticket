## Problem

The orchestration protocol in `.agents/instructions/commit/branch-worktree.instructions.md` requires each implementation unit to run in its own freshly provisioned worktree on its own branch. Claims are bound to the session's assigned worktree, so a delegated sub-agent working in an orchestrator-provisioned worktree cannot legitimately claim the assigned worktree. A dispatched Implement Agent refused to make any edit, reporting: "the active session is assigned to `.worktrees/d5d9a736-session`, while the requested worktree is `.worktrees/e70471d4-toolmon-paths`." The implementation unit proceeded only because the orchestrator explicitly waived the claim.

## Established Facts

- `board_check_in` rejects only an exact active worktree-path collision in `memory-api/crates/memory-api/src/storage/board/ops.rs` (line 231); the error type is in `memory-api/crates/memory-api/src/storage/board.rs` (line 194).
- `session_check_in` reuses an assignment only when `worktree_path` and `branch` both match; otherwise, the operation rotates the assignment when owner and ticket match. The behavior is in `memory-api/crates/session-api/src/store/helpers/storage.rs` (line 289).
- Neither check-in surface has an override or force parameter: board claim parameters are in `memory-api/crates/ticket-api/src/storage/store/board.rs` (line 92), and session claim parameters are in `memory-api/crates/session-api/src/store.rs` (line 171).
- The intended rehome path is the session move API: `plan_move_preflight` -> `execute_move_with_journal` -> `resume_move_with_journal` / `rollback_move_with_journal` in `memory-api/crates/session-api/src/move_domain.rs` (line 136). Reassigning a live session away from the worktree is disruptive because the authoritative session store is worktree-local.
- Claim enforcement is convention-only at write time: `tools/agent-hooks/preflight-write.sh` runs syntax and lint checks without inspecting ownership, so an unclaimed edit is not technically blocked.
- `.agents/instructions/commit/branch-worktree.instructions.md` (approximately lines 56 and 76) and `.agents/instructions/ticket/board.instructions.md` (approximately line 45) prescribe bootstrap order but do not resolve this delegated-worktree claim case.

## Scope

Decide and implement a safe, auditable mechanism for delegated agents to claim orchestrator-provisioned worktrees without disrupting the live session store. Candidate directions include a constrained claim parameter, a per-worktree claim identity decoupled from session identity, or an explicit documented waiver. The selected direction must define authorization, collision behavior, and audit data.

## Acceptance Criteria

- [ ] A documented design decision identifies one legitimate mechanism for a delegated agent to claim an orchestrator-provisioned worktree, including authorization, collision handling, and audit behavior.
- [ ] The selected mechanism is implemented and a focused automated test proves that a delegated agent can make a legitimate claim for an orchestrator-provisioned worktree without moving or corrupting the live session's worktree-local store.
- [ ] A focused automated test proves that an unauthorized or conflicting claim remains rejected with an actionable error.
- [ ] `.agents/instructions/commit/branch-worktree.instructions.md` is updated with the selected delegated-worktree claim procedure.
- [ ] `.agents/instructions/ticket/board.instructions.md` is updated with the same procedure and no longer leaves the session/worktree contradiction unaddressed.
- [ ] Focused validation commands for the affected board/session APIs and instruction validation pass, with results recorded in the ticket review evidence.