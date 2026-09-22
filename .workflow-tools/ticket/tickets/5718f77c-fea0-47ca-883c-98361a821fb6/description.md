## Problem

`enforce_dependency_progress` in [memory-api/crates/ticket-api/src/storage/store.rs](memory-api/crates/ticket-api/src/storage/store.rs#L924-L955) applies the dependency-ordering guard to ALL non-`cancelled` target states, including backward/parking transitions like `on-hold`. It only special-cases `target_state == "cancelled"` as always permitted (line ~928). There is no distinction between forward progress (toward completion) and backward/parking moves.

Parking a ticket with `on-hold` specifically BECAUSE it is blocked on an unfinished dependency is the primary legitimate use case for that state. The guard makes this escape hatch unreachable: a ticket can never be parked while its dependency is unfinished, which is exactly when parking is most needed.

Mechanism: this is triggered via `ticket update <id> --to-state <state>` / MCP `update_ticket{to_state}` (state-machine progression), NOT the `move` subcommand (which is cross-workspace ticket relocation and is unrelated).

## Exact observed errors (from 2026-07-31 bulk triage)

- "cannot move ticket 10c94251-1c0c-4542-a282-ea3d75a205b5 to 'on-hold' because dependency 68eaae1f-b230-4aab-8572-cbf41d1d3b6d is only at 'in-implementation'"
- "cannot move ticket 27558fde-37b0-43eb-86c6-cfbe2d99a0b8 to 'on-hold' because dependency ef0ebf38-7f55-4bd7-bf0c-0b416650ee0b is only at 'in-implementation'"
- "cannot move ticket a71c2da8-0972-4c2d-9754-0a0e06db5272 to 'on-hold' because dependency db9bad13-ae43-4300-8037-7165c0e9a7b0 is only at 'in-review'"

Error string source: `StorageError::DependencyNotProgressed` message template in [memory-api/crates/memory-api/src/error.rs](memory-api/crates/memory-api/src/error.rs#L120): "dependency not progressed: cannot move ticket {ticket} to '{target_state}' because dependency {dependency} is only at '{dependency_state}'".

## Impact evidence

56 tickets had accumulated in `in-implementation` (48 simultaneously in `in-review`, 86 in `planned`; 192 active total) as of the 2026-07-31 bulk triage. The unreachable `on-hold` parking transition is a plausible direct contributor to this WIP sprawl: agents blocked on unfinished dependencies had no valid way to park the ticket and had to leave it sitting in an active state instead.

## Acceptance Criteria

- [ ] The dependency-ordering guard in `enforce_dependency_progress` applies only to FORWARD transitions toward completion (i.e. transitions where `target_rank` represents forward workflow progress), not to backward/parking transitions.
- [ ] Transitioning a ticket to `on-hold` succeeds regardless of the state of its `depends_on` targets.
- [ ] Other backward/parking transitions (if any exist in the schema) are likewise exempted from the guard.
- [ ] A regression test in `memory-api/crates/ticket-api/src/storage/tests/workflow_tests.rs` covers: ticket X with an unfinished dependency Y (e.g. Y at `in-implementation`) can be moved to `on-hold` without error.
- [ ] Forward transitions remain guarded: moving X to `in-review` while Y is unfinished still fails with `DependencyNotProgressed`, unchanged from current behavior.

## Location

Guard implementation: [memory-api/crates/ticket-api/src/storage/store.rs](memory-api/crates/ticket-api/src/storage/store.rs#L924) (`fn enforce_dependency_progress`), rank comparison at line ~949, `cancelled`-only exemption at line ~928.
Existing regression test pattern to extend: [memory-api/crates/ticket-api/src/storage/tests/workflow_tests.rs](memory-api/crates/ticket-api/src/storage/tests/workflow_tests.rs#L240) (`update_guards_transition_ahead_of_dependency_state`).


## Additional Evidence (2026-07-31)

This guard is broader than `on-hold`: it also blocks backward demotions to `planned`.

Observed errors during 2026-07-31 bulk triage:
- "cannot move ticket 322a4737-9fae-4804-8053-6ea1c85205da to 'planned' because dependency 9590bdcf-8c3c-4cd1-bd60-df2ec6ca65f1 is only at 'open'"
- "cannot move ticket 5f9542bf-483a-4da6-9c78-fcbe588af973 to 'planned' because dependency 819f2e97-4cd4-410b-af3b-f196ba80d720 is only at 'open'"

Corrected root-cause hypothesis: `enforce_dependency_progress` compares every dependency against the TARGET state's rank, independent of transition direction. With the workflow ordering in [memory-api/crates/ticket-api/schemas/tracker-improvement.toml](memory-api/crates/ticket-api/schemas/tracker-improvement.toml) (`open -> planned -> in-implementation -> in-review -> on-hold -> done -> cancelled`), this means:
- `on-hold` outranks `in-review`, so parking is treated as forward progress and gets guarded.
- Demoting to `planned` requires every dependency to already be at least `planned`, so any `open` dependency makes that demotion fail.
- Net effect: a ticket can become permanently stuck in `in-implementation` because it can neither advance nor retreat.

Implementation pointer: [memory-api/crates/ticket-api/src/storage/store.rs](memory-api/crates/ticket-api/src/storage/store.rs#L924), with the cancelled-only exemption around line ~928 and rank comparison around line ~949.

Measured impact from the same triage: 4 of 56 `in-implementation` tickets were immovable in both directions for this reason: 322a4737-9fae-4804-8053-6ea1c85205da, 5f9542bf-483a-4da6-9c78-fcbe588af973, 79dd2d35, and 6e72756f.

## Revised Acceptance Criteria

- [ ] Demotion to an earlier state (`planned`, `open`) succeeds regardless of dependency states.
- [ ] Parking (`on-hold`) succeeds regardless of dependency states.
- [ ] No ticket can be rendered immovable in both directions; a regression test asserts a ticket with an `open` dependency can be demoted to `planned`.
- [ ] Forward transitions to `in-review`/`done` remain guarded.
