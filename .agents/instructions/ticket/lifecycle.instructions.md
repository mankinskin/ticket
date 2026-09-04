---
description: "Use when moving a ticket through its states: transitions, auto-walk vs strict single-hop, undo/revert, schema-enforced required_states, and the review gate before closing."
---

## Ticket State Machine

The ticket lifecycle is a **one-way** state machine: transitions only go
forward, and every state must be visited in order — do not skip states. The
schema defines `required_states` (for example `["in-review"]`) that **must**
appear in a ticket's history before it can reach a terminal state (`done`).

### Continuous State Updates

Update ticket state immediately when the work status changes — do not defer to
the end of a session:

| Situation | Action |
|---|---|
| Starting implementation | `update --to-state in-implementation` |
| Implementation complete, moving to review | `update --to-state in-review` |
| All acceptance criteria met and validated | `close <id>` |
| Ticket is no longer relevant | `cancel <id>` with a reason |

> If a state was reached prematurely, use `update --undo` to revert the last
> transition and re-progress correctly.

### Transition Auto-walk and Strict Recovery

`update --to-state <state>` **auto-walks** the shortest legal path by default.
If the target is not a direct neighbor of the current state (for example
`open -> in-implementation`, which must pass through `planned`), the update
traverses the required intermediate waypoints automatically and lands on the
requested state.

Pass `--single-hop` (CLI) / `single_hop: true` (ticket-MCP and HTTP) to opt into
**strict one-hop mode**. In strict mode a target that would skip a required
waypoint is **rejected with recovery guidance** rather than walked. The error
names the current state, the legal next states, and the mandatory intermediate
waypoint(s), for example:

```
invalid state transition 'open' -> 'in-implementation'; current state 'open'
allows next states [cancelled, planned]; to reach 'in-implementation', first
transition through: planned
```

To advance explicitly across multiple states you can also name the waypoints
with `--transition-state`, or use `close <id> --to-state <state>` which
fast-forwards along the shortest legal path. A genuinely unreachable target is
always rejected regardless of mode.

Inspect the legal transition graph for any ticket with:

```bash
# Current state, allowed next states, full transition graph, required/terminal states
./target/debug/ticket.exe transitions <id> --toon
```

The same recovery-field shape (current + allowed-next + intermediate states) is
surfaced identically across the CLI, ticket-MCP, and HTTP mutation surfaces.

### Correcting State Transitions (Undo / Revert)

If a ticket was advanced to the wrong state, use `--undo` to roll back the last
transition:

```bash
# Undo the most recent state change (reverts to the previous state)
./target/debug/ticket.exe update <id> --undo --toon
```

For deeper rollbacks, use `revert --to <rev>` to restore a specific historical
revision:

```bash
# Revert to revision 6 (re-applies fields from that point in history)
./target/debug/ticket.exe revert <id> --to 6 --toon
```

> `--undo` is a convenience for the common case of "I advanced too far" and is
> equivalent to reverting to rev N-2. Neither command deletes history — a new
> revision is appended recording the rollback.

### Schema-Enforced Workflow (`required_states`)

The ticket type schema can declare `required_states` — a list of states that
**must** appear in a ticket's history before the store allows a transition to a
terminal state (default terminal: `done`).

For `tracker-improvement` tickets, the schema enforces:

```toml
required_states = ["in-review"]
```

This means the store will **reject** `close` (or `update --to-state done`) if
`in-review` has never been visited. This is enforced at the API layer, so it
applies to CLI, MCP, and HTTP equally.

To customize enforcement per ticket type, edit the corresponding schema file
under `crates/ticket-api/schemas/<type>.toml`.

## Plan Freezing at `planned`

Entering `planned` freezes five **planning** part kinds: `objective`,
`requirements`, `design`, `examples`, `acceptance_criteria`. Writing to a
frozen part is **hard-rejected** — `enforce_part_write_gate` is the sole write
path and applies identically across CLI, MCP, and HTTP. `review`,
`validation`, `notes`, `amendment`, and free-form kinds are never frozen and
stay writable in every state, so recording progress never requires touching
the plan.

Two recovery paths when a frozen part needs correcting:

- **Amendment (preferred).** Write a new part with `--supersedes <part_id>`
  (CLI `write-amendment`, MCP `write_amendment`) to record the correction
  without unfreezing the original.
- **Re-plan.** Transition the ticket back to `open`, which clears the frozen
  flag on all five planning kinds; edit freely, then re-enter `planned` to
  re-freeze.

Never record a review outcome, status update, or validation result by editing
`objective` — write it as its own part (`review` or `validation` kind,
`description_mode: append` when using whole-description writes at all). See
[workflow.instructions.md](workflow.instructions.md) for the read-side view
profiles.

## Review Gate Before Closing

**Never `close` a ticket directly from `in-implementation`.** Always move
through `in-review` first, even for small changes. The schema's
`required_states` enforcement prevents skipping `in-review`, but you should
still follow the full progression diligently. Review readiness means the
implementation, required validation, documentation updates, and spec
traceability are current before the state change.

### Step 1 — Move to in-review

```bash
./target/debug/ticket.exe update <id> --to-state in-review
```

### Step 2 — Code Review Checklist

Before moving to validation, verify each of the following. Fix any issue found
before proceeding, including missing documentation or spec traceability needed
for review.

**Correctness & Reactivity (frontend)**
- [ ] All signal reads that must re-run on change are inside reactive closures,
      not computed once outside the `view!` macro.
- [ ] State updated correctly on all paths (including edge cases like empty data).

**Memory & Cleanup**
- [ ] No unbounded `Closure::forget()` calls; use `Closure::into_js_value()` to
      transfer ownership to the JS GC instead.
- [ ] Document-level event listeners registered with a `on_cleanup` removal hook
      so they are unregistered if the component unmounts mid-gesture.
- [ ] No `Rc`/`RefCell` or wasm-bindgen closures that outlive component scope
      without an explicit cleanup path.

**CSS & Layout**
- [ ] Elements with negative positioning checked against any `overflow: hidden`
      ancestors — they will be clipped.
- [ ] Responsive/resize behavior tested at both min-width and large widths.
- [ ] `aria-label` or role attributes on interactive elements without visible text.

**Security**
- [ ] User-controlled strings inserted into the DOM use text-node APIs (for
      example `set_text_content`) — not `set_inner_html` — to prevent XSS.
- [ ] URLs derived from external data are validated before fetch/navigation.

**General**
- [ ] No dead code, unused imports, or unreachable branches left behind.
- [ ] Public API changes reflected in docs/changelogs if applicable.
- [ ] The relevant spec links the exact ticket folder path(s), the updated docs,
      and the passing or blocked validation results for this work.
- [ ] The implementation summary captures implementation, validation, and
      documentation status.

### Step 3 — Validate Acceptance Criteria

Run the relevant test suite(s) against the ticket's acceptance criteria. Keep
iterating on the nearest required validation until it passes or you have a
clearly repeated failure with enough evidence to stop and report the blocker:

```bash
# Native unit tests (pure-Rust logic, no browser needed)
cargo test -p <crate>

# WASM browser tests (requires wasm-pack + Chrome)
wasm-pack test --headless --chrome <path/to/crate>

# Cargo check for WASM target (quick compile gate)
cargo check --target wasm32-unknown-unknown -p <crate>
```

Confirm each acceptance criterion listed in the ticket description is met with a
passing test or a documented manual verification step.

### Step 4 — Close

Only close after the review checklist is complete and tests pass:

```bash
./target/debug/ticket.exe close <id>
```
