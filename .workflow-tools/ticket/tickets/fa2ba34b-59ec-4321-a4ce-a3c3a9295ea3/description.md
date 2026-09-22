## Objective

Eliminate silent ticket-store divergence between a worktree agent's file/CLI edits and ticket-mcp MCP writes.

## Reproduction

ticket-mcp server launched from main checkout via VS Code; agent operates in .worktrees/<name>; use MCP with workspace default to create/write ticket while reading/writing .ticket via CLI/files inside worktree; writes land in different stores. Actual incident caused 17-file / 82,453-byte manual recovery harvest.

## Root Cause

- memory-api/tools/mcp/ticket-mcp/src/main.rs lines 14-20: `let index_root = std::env::var("TICKET_INDEX_ROOT")\n    .map(PathBuf::from)\n    .unwrap_or_else(|_| {\n        let (path, _source) = ticket_api::workspace::resolve_workspace();\n        path\n    });`
- memory-api/crates/memory-api/src/workspace.rs lines 108-114: `pub fn working_dir() -> Option<PathBuf> {\n    resolve_working_dir(\n        std::env::current_dir().ok().as_deref(),\n        std::env::var_os("PWD").as_deref().map(Path::new),\n    )\n}`
- memory-api/tools/mcp/ticket-mcp/src/server.rs lines 100-106: `if workspace.is_empty() || workspace == "default" {\n    return Ok(self.index_root.clone());\n}`
- .vscode/mcp.json lines 16-22: ticket-mcp runs `mcp-toolmon -- ticket-mcp` with no `cwd` and no `TICKET_INDEX_ROOT`, inheriting VS Code's main checkout workspace folder for the session.

## Impact

.ticket is version-controlled; worktrees have independent copies; main-store MCP writes and worktree CLI/file writes silently fork authority.

## Acceptance Criteria

1. With caller cwd inside a worktree, MCP ticket writes either resolve to the worktree ticket store OR produce an explicit error/warning naming both candidate absolute store roots; silent divergence eliminated.
2. A tool call or startup log exposes the absolute store root to which default resolved.
3. Regression test covers worktree-cwd-vs-server-cwd divergence.
4. Record follow-up that .agents/instructions/commit/branch-worktree.instructions.md likely needs a companion rule: ticket-store edits must NOT be performed from a worktree; do not edit that guidance in this ticket.


## Widened Scope (2026-08-06): session-anchored resolution across every proxied server

The original scope above treats this as a ticket-mcp defect. It is not. `default` resolving to the server process cwd is a property of every MCP server fronted by `mcp-toolmon`, so the fix belongs in the proxy, not in one server. The anchor is the **session id**: a session already knows its active worktree, so resolving session -> worktree -> workspace makes cwd irrelevant.

### Why the proxy is the right layer

`mcp-toolmon` already does exactly this shape of work for a different field:

- `memory-api/tools/mcp/mcp-toolmon/src/proxy.rs#L203` — reads and validates a required `caller_model` argument on EVERY proxied call.
- `memory-api/tools/mcp/mcp-toolmon/src/proxy.rs#L284` — strips `caller_model` before forwarding to the wrapped server.
- The same code injects the required property into `tools/list` responses, so every tool advertises it.

A `session_id` anchor is the same mechanism with a different field. No per-server change is required to make the anchor *mandatory*.

### Session already carries the worktree

- `memory-api/crates/session-api/src/model.rs#L316` — session metadata carries a `worktree` field.
- `memory-api/crates/session-api/src/model.rs#L336` — `SessionWorktreeAssignment { path, branch, allocation_mode: New|Reused|Rotated, status: Active|Superseded|Invalidated, predecessor_session_id, predecessor_path }`.
- `memory-api/crates/session-api/src/store.rs#L171-L199` — `check_in_worktree` mutates the assignment mid-session, so the binding is authoritative and current rather than fixed at session start.

## Additional Acceptance Criteria

5. Every proxied MCP tool call carries a required `session_id` argument, injected into `tools/list` schemas, validated on `tools/call`, and stripped before forwarding — mirroring the existing `caller_model` handling at `proxy.rs#L203` and `proxy.rs#L284`. A call without a resolvable `session_id` is rejected with an error naming the missing anchor.
6. The proxy resolves `session_id` -> active `SessionWorktreeAssignment` -> workspace root, and applies that resolution to ALL proxied servers (ticket, spec, session, test, feedback, audit, doc, log), not to ticket-mcp alone. A server-specific fix is not an acceptable implementation of this criterion.
7. A workspace selection that would be derived from the server process cwd is REJECTED rather than silently resolved. The bare `default` selector is only honoured when it can be resolved through a session anchor; unanchored, it is an error whose message names the candidate store roots it refused to choose between.
8. The resolved scope is a typed value that distinguishes at least: the repository root, a worktree inside that repository, and the main (non-worktree) checkout. The type must be sufficient to express and enforce a policy that BLOCKS mutating work resolved to the main checkout.
9. An OPTIONAL relative workspace path parameter addresses a workspace nested deeper inside the session's worktree. It is interpreted relative to the resolved worktree root, never to the server cwd, and it cannot escape that root.
10. A workspace not bound to any git repository remains supported — Memory API permits stores outside a checkout — but is explicitly opt-in and never the default resolution. Selecting a non-Git workspace requires an explicit selector rather than falling through from a failed git resolution.
11. Regression tests cover, at proxy level: a call with no `session_id` is rejected; a call whose session resolves to a worktree writes to that worktree's store while the server process cwd is the main checkout; an unanchored `default` is rejected; and a main-checkout-scoped mutation is blocked under the enforcing policy.

## Design source

`transcripts/06-08-2026_worktree-session-proxy/merged.clean.md`

## Open question (not resolved by this ticket)

Whether the Copilot hook API exposes a `SessionStart` event at all is UNVERIFIED. `.github/hooks/hooks.json` currently maps only `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `Stop`, and `SessionEnd`. Prompt-time bootstrap is tracked separately in `3d535b2c`; do not assume a `SessionStart` hook exists when implementing this ticket.

## Correction (2026-08-06): AC4's proposed companion rule is inverted

AC4 above records a follow-up asserting that `.agents/instructions/commit/branch-worktree.instructions.md` needs a companion rule reading "ticket-store edits must NOT be performed from a worktree". That direction is WRONG under the decided model and must not be written as stated.

### Decided model

All active stores are worktree-local. `.session`, `.ticket`, and `.spec` live inside the session's worktree. The main checkout holds no active store; its copies are a merge target, current only once a branch merges. A chat lifecycle hook initializes the worktree before any other tool runs, so there is no bootstrap window without a worktree and no main-checkout fallback is specified anywhere.

### Corrected AC4

- AC4-C. Add a companion rule to `.agents/instructions/commit/branch-worktree.instructions.md` stating the inverse of the original wording: entity-store edits belong to the session's worktree, and no session may write an active store in the main checkout. Main-checkout store copies change only through a branch merge.

This correction resolves the conflict between the original AC4 and AC1 above, which already required MCP writes to resolve to the worktree store.