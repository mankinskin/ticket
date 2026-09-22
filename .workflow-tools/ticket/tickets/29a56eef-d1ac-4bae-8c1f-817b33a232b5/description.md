## Defects

1. `create_ticket` accepts a `state` value that is not a member of the ticket
   type's schema `states` list. The created ticket then has zero outgoing
   transitions (an off-schema state has no `[[transitions]]` entries with
   `from = "<state>"`), so `update_ticket --to-state` fails permanently with:

   ```
   invalid state transition 'open' -> 'X'; current state 'open' allows next states []
   ```

   `create_ticket` should reject an unrecognized `state` value at creation
   time (or normalize it to the type's default initial state), not silently
   accept it and produce a permanently untransitionable ticket.

2. `health_check` does not detect tickets whose current `state` is absent from
   their type schema's `states` list. Run against root `322a4737` (11 child
   tickets beneath it, all frozen in the off-schema `open` state) it returned
   **0 findings** — the exact condition that should be its primary signal for
   this class of corruption went undetected.

## Evidence

- 12 tickets were created with `state = "open"` even though neither the
  `task` nor `feature` type schema
  (`memory-api/crates/ticket-api/schemas/task.toml`,
  `memory-api/crates/ticket-api/schemas/feature.toml`) defines `open` as a
  member of `states`.
- Affected ids (all originally `state = "open"`, repaired manually in this
  session): `0a602458`, `322a4737`, `42780b6e`, `7f39fae3`, `856c69c2`,
  `9590bdcf`, `a4d7df73`, `a59f35fb`, `c72f1fab`, `e135e28c`, `e9c0e280`,
  `fc94716e`.
- Exact error reproduced on every one of the 12 before repair:
  `invalid state transition 'open' -> 'X'; current state 'open' allows next states []`
- `mcp_ticket-mcp_health_check` against root `322a4737` returned 0 findings
  while all 11 tickets beneath it were frozen in `open` — confirming the
  detector has a blind spot for off-schema current-state values.
- Manual repair was required: `update_ticket` with `field_map: {"state": ...}`
  reported `{"status":"ok", "changed_fields": {...}}` but did **not** persist
  the change to disk (a second latent defect, not filed here — re-reads after
  the call still showed `state = "open"` on disk). The change only persisted
  after directly editing the `state = ` line in each ticket's `ticket.toml`.

## Acceptance Criteria

- [ ] `create_ticket` rejects a `state` argument that is not a member of the
      resolved ticket type's schema `states` list, with a clear error naming
      the invalid value and the legal set — OR normalizes it to the type's
      default initial state with a clear warning. Pick one behavior and
      document it.
- [ ] A unit/integration test asserts `create_ticket` cannot produce a ticket
      whose `state` is absent from its type schema.
- [ ] `health_check` (and `health_check --all`) flags any ticket whose current
      `state` is not a member of its resolved type schema's `states` list, as
      a distinct finding kind (e.g. `off_schema_state`).
- [ ] A regression test reproduces the exact scenario here: create/patch a
      ticket into an off-schema state, then assert `health_check` reports it.
- [ ] Existing store data (this repo's `.ticket/tickets/`) is unaffected by
      the fix landing — no bulk migration required as part of this ticket.

## Validation

```bash
cargo test -p ticket-api
```

## Depends On

None — this stands alone as a store-integrity bug, matching sibling `bf62e2f9`.
