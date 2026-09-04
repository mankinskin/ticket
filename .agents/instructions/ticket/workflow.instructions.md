---
description: "Use in ticket workflow operations that apply across sessions: orientation, discovery before creating, picking next work, dependency semantics, health checks, command chaining, and transactional batch."
---

## Ticket Workflow Operations

These operations apply during **every session**, not only when working on
ticket-system code. Session-wide principles (create tickets before code, keep
state current, review before close) live in [AGENTS.md](../../../AGENTS.md); the
operational detail lives here.

### Orientation (start of every session)

Before writing any code, run a quick orientation to understand the current
ticket landscape:

```bash
# Check the draftboard (active agents, WIP limit, stale warnings)
ticket board show --toon

# Check for stale in-implementation tickets that may conflict with your work
./target/debug/ticket.exe list --where state=in-implementation --toon

# Survey all open tickets
./target/debug/ticket.exe list --where state=open --toon

# Check overall graph health
./target/debug/ticket.exe health --all --toon
```

Alternatively, use the MCP ticket tools (`mcp_ticket-mcp_next_tickets`,
`mcp_ticket-mcp_list_tickets`, `mcp_ticket-mcp_health`,
`mcp_ticket-mcp_board_show`) when the MCP server is running.

The handoff package's `entity_store_root` selects the ticket store for the
entire unit. Pass the root explicitly to every ticket read and write. After a
mutation, fetch the same ticket id from the same root and verify the new state
or part before reporting success. Discovery from another root, an aggregated
index, or a worktree-local shadow store does not validate the intended write.

### Workspace Discovery vs Enumeration (note to self)

Do not assume a nested `.ticket` store is "excluded" just because it is absent
from `list_workspaces`.

- **Discovery/reads are aggregated.** When the root
  `.ticket/workspace-policy.toml` sets `include_descendants = true`, every
  descendant store (for example `memory-api`, `memory-viewers`, `viewer-api`,
  `context-stack`) is recursively discovered and folded into the aggregated
  `default` workspace index. `get`, `list`, `next`, `search`, and graph/health
  queries against `default` therefore already include descendant tickets, and
  `get` returns each ticket's real owning-store path.
- **`list_workspaces` only enumerates the aggregated root.** It reports
  `default` plus the root store path and does **not** list each descendant store
  as a separately selectable workspace. A store missing from `list_workspaces`
  is an enumeration/presentation limitation, not a discovery exclusion.
- **Writes target the addressed store.** `create` and `update` land in the store
  that owns the `workspace` you pass. To co-locate a new ticket with a
  descendant subtree (for example ticket-viewer / viewer-api work under
  `memory-viewers`), pass that store's absolute path as `workspace` instead of
  `default`.
- **Before concluding a store is unreachable:** confirm with `get`/`list`
  against `default` and inspect `.ticket/index.toon` `source_path` prefixes;
  only treat it as a real gap if the descendant tickets are genuinely absent
  from the aggregated index.

### Reading Tickets: View Profiles and Parts

A ticket is not one description file — it is a set of typed `parts/<uuid>.md`
files (kinds: `objective`, `requirements`, `design`, `examples`,
`acceptance_criteria`, `review`, `validation`, `notes`, `amendment`, plus any
free-form attachment kind), indexed by `[[parts]]` in `ticket.toml`. Legacy
tickets with no `[[parts]]` synthesize an `objective` part automatically.

`get_ticket` / `ticket get` / `ticket describe` project reads through a named
`--view` profile instead of returning everything. **Default to the narrowest
profile that answers the question** — this is also a token-cost win:

| Profile | Contains | Use for |
|---|---|---|
| `summary` (default) | metadata + `objective` | Orienting on a ticket you have not started. |
| `plan` | `objective` + `requirements` + `design` + `examples` + `acceptance_criteria` + refs | Implementing — everything needed to execute. |
| `review` | `acceptance_criteria` + `review` + `validation` | Verifying — the criteria plus what was recorded against them. |
| `full` | every part present (core and free-form) + refs | Auditing or migrating; rarely needed for routine work. |

Use `--parts <kind,kind,...>` to pull specific kinds outside a named profile.
An unknown profile or unknown kind is rejected, not silently returned empty.

Write to a specific part with `write-part`/`write_part` (`--kind <KIND>`), not
by replacing the whole description. See
[lifecycle.instructions.md](lifecycle.instructions.md) for the `planned`-state
freeze contract that governs which kinds are writable when.

### Typed References (`[[refs]]`)

Attach external context — specs, test executions, logs, rules, files, commits
— through the `[[refs]]` table in `ticket.toml` rather than inlining links or
prose pointers inside a part. Each entry has a `kind` in
`{spec, test_execution, log, rule, file, commit}`, a `ce://...` `urn`, and an
optional `note`. Unknown kinds and malformed URNs are rejected at write time.
The `plan` and `full` view profiles include `[[refs]]`; `summary` and `review`
do not.

### Discovery Before Creating

Always search for existing tickets before creating new ones, using `ticket search`,
`list_tickets`, `get_ticket_description`, or `ticket list`. Duplicate tickets degrade
store quality. When a matching ticket already exists, report its id and evidence
(or reuse/update it) instead of creating another; when only some of the needed
tickets already exist, reuse those and create only the missing ones.

```bash
./target/debug/ticket.exe search "<keywords>" --toon
```

Or via MCP: `mcp_ticket-mcp_list_tickets` with a `where` filter, or
`mcp_ticket-mcp_get_ticket_description`.

### Picking Next Work

Use `ticket next` to find the highest-priority unblocked tickets:

- Use `ticket next` for the global queue of unblocked work.
- Use `ticket next <ticket-id>` when a larger ticket is blocked and you need the
  immediate leaf blockers that can start now.
- `ticket next <ticket-id>` also returns a blocker tree so intermediate blocked
  dependencies stay visible while you execute the frontier leaves.
- Prefer this root-scoped form when unblocking tracker or epic tickets so agents
  pick work that directly reduces the root blocker set.

```bash
# Find unblocked planned tickets you can work on now (priority-ordered)
ticket next --toon

# For a blocked tracker/epic, find immediate actionable leaf blockers under it
ticket next <ticket-id> --toon

# With a title prefix filter for a specific track
./target/debug/ticket.exe next --filter "[bootstrap]" --toon

# Limit results
./target/debug/ticket.exe next --limit 5 --toon

# Optional MCP equivalent when using ticket-mcp
# next_tickets {"workspace":"default","root":"<ticket-id>"}
```

Or via MCP: `mcp_ticket-mcp_next_tickets` with `workspace`, optional `limit` and
`filter`.

The command returns tickets in **any non-terminal state** whose `depends_on`
edges all point to `done`/`cancelled` tickets. Results are sorted by:

1. **State progress** — tickets closest to `done` appear first (for example
   `in-review` > `in-implementation` > `planned` > `open`). Progress is determined
   by the state's index in the schema's `states` list.
2. **Priority** — `critical > high > medium > low > none`.
3. **Creation date** — oldest first (FIFO tiebreaker).

### Dependency Semantics

Use these rules to model planning, design, tracker, and implementation ticket
relationships correctly.

**Dependency direction convention:** Parents/epics `depends_on` their children
(an epic is done when all children are done). Children do **not** depend on their
parent — they depend on sibling prerequisites.

Planning or design tickets track the creation and refinement of specs, tickets,
and execution shape. Implementation tickets depend on the planning or design
ticket being completed before implementation starts. Tracker or epic tickets are
separate execution parents: the tracker ticket depends on the child
implementation tickets and closes when those children are done. Do not use a
planning or design ticket as the tracker for its own implementation work.

### Dependency Maintenance

After completing significant work, check whether finished tickets unblock others
and update those links:

```bash
# Find what a completed ticket blocks
./target/debug/ticket.exe topgraph <id> --json \
  | jq -r '.payload.nodes[] | select(.state=="open" or .state=="planned") | .id'
```

Add missing `depends_on` edges when you discover undocumented dependencies. Use
`--reason` on every link to explain *why* the dependency exists.

### Commit Checkpoint Suggestions

Suggest a `git commit` checkpoint to the user when any of the following is true:

- A ticket transitions to `closed` (work milestone reached).
- A batch of related tickets all reach `closed` or `in-implementation` together.
- A dependency graph changes materially (new links added/removed).
- A tracked bug is fixed and its ticket closed.

Phrase suggestions like:

> "Ticket `<title>` is now closed — good checkpoint for a commit. Suggested
> message: `<imperative summary of what was done>`."

### Aggressive Quality Improvement

Opportunistically improve ticket quality whenever you touch the store:

- Fill in missing `description`, `priority`, or `type` fields on tickets you
  encounter.
- Split vague tickets into concrete, actionable child tickets linked with
  `depends_on`.
- Remove or merge duplicate tickets.
- Verify that `in-implementation` tickets actually have an active owner/context;
  flag stale ones.
- After any structural refactor, re-run `ticket health --all` and resolve
  reported issues.

## Workflow Expectations

- Start implementation work by searching for existing tickets and creating or
  updating the required ticket set before code changes.
- Update or create the relevant spec before implementation when requirements,
  goals, or behavior are new or changing.
- For each ticket, implement the scoped change, run the required validation until
  it passes or repeatedly fails, update docs, verify the spec links the related
  tickets with openable `ticket.toml` targets plus the updated docs and
  validation results, then move the ticket to `in-review`.
- When capturing validation or documentation evidence in this repository, prefer
  the repo-local `workflow` CLI so the resulting artifact can be linked from
  specs and tickets.
- If validation repeatedly fails, do not silently skip it. Record the failing
  command or manual verification result and the blocker in the ticket/spec
  status summary.
- Summaries and handoffs must report implementation, validation, and
  documentation status.
- When dedicated test, doc, or cross-store-link tooling is missing or partial,
  use the strongest available substitute and note the gap explicitly.
- When mentioning tickets in chat output, use the exact canonical ticket folder
  path returned by ticket-api output as the base path for the markdown link
  target.
- Never synthesize a ticket folder path from a UUID, the current store root, or
  an example path; if the first ticket-api response omits the path, run a
  follow-up ticket-api command that returns the authoritative path before
  responding.
- Render ticket references per the Clickable Reference Policy in
  [AGENTS.md](../../../AGENTS.md).

## Health Checks

```bash
# Health-check a subgraph rooted at a ticket (BFS traversal)
ticket health <ticket-id> --toon

# Health-check a subgraph, filtering to a specific type
ticket health <ticket-id> --where type=tracker-improvement --toon

# Health-check all tickets
ticket health --all --toon

# Health-check all open tickets (--where filter)
ticket health --all --where state=open --toon
```

### Command Chaining (pipe via --stdin)

```bash
# List tickets → pipe IDs → health check
ticket list --where priority=high --json \
  | jq -r '.payload.items[].id' \
  | ticket health --stdin --toon

# Subgraph → filter open tickets → health check
ticket subgraph <ticket-id> --json \
  | jq -r '.payload.nodes[] | select(.state=="open") | .id' \
  | ticket health --stdin --toon

# Topgraph → health check all reverse dependencies
ticket topgraph <ticket-id> --json \
  | jq -r '.payload.nodes[].id' \
  | ticket health --stdin --toon
```

### Batch (CLI-syntax, transactional)

`ticket batch` reads one CLI command per line from stdin (or `--file`). All
commands execute against the same store in order. If any command fails, all
prior writes are rolled back automatically. Blank lines and `#` comments are
ignored.

```bash
# Heredoc — create tickets + link, all atomic
ticket batch --toon <<'EOF'
create --title "Extract GPU pipeline" --type tracker-improvement
create --title "Add shader cache" --type tracker-improvement
# link is resolved after creates succeed
link --from <UUID-A> --to <UUID-B> --kind depends_on
EOF

# From a checked-in batch file
ticket batch --file scripts/bootstrap-tickets.txt --toon

# Stdin from another process
echo -e "create --title 'Setup CI' --type tracker-improvement\nclose <UUID>" \
  | ticket batch --toon
```

**Rules:**
- Each line is parsed identically to a top-level `ticket <subcommand>` call.
- `serve`, `watch`, nested `batch`, `scan`, lease commands, and config commands
  (`add-root`, `workspace`, `export-command-schema`) are rejected with a clear
  error — they cannot be used inside a batch.
- On rollback: `create` → deleted, `update` → state/fields restored, `link` →
  edge removed. Read-only commands (`get`, `list`, `search`, `health`, etc.)
  produce no undo entry and are not affected by rollback.
- No `--index-root` requirement — uses normal workspace resolution like any
  other CLI command.
