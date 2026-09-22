# Goal

Turn the current workflow and health surface inconsistency diagnosis into one coordinated implementation track with a ticket-api-owned minimal core, explicit adapter-boundary cleanup, and reproducible larger-integration parity validation across CLI, MCP, and HTTP.

# Why These Surfaces Drift Today

- Selector semantics and scope narrowing are still being coordinated by adapters instead of one shared contract.
- Health findings and workflow-result shaping are duplicated, so the same store can return different answers depending on which interface asked.
- CLI and HTTP still take on domain work that should live in ticket-api.
- Ticket-mcp is thin in the wrong places: it hides or omits parts of the real contract, so clients still have to reconstruct behavior externally.
- Validation mostly proves each surface alone, which is how parity bugs survive in broad daylight like the repo thinks embarrassment is optional.

# Child Tickets

- [0e375356 Implement scoped selectors for board and next](C:/Users/linus_behrbohm/git/SECOND_CHECKOUT/graph_app/workflow-tools/.workflow-tools/ticket/tickets/0e375356-b74e-48c4-8f1d-77cd28e055bc/ticket.toml)
- [c031aeb0 Define minimal workflow and health core plus adapter responsibilities](C:/Users/linus_behrbohm/git/SECOND_CHECKOUT/graph_app/workflow-tools/ticket/.workflow-tools/ticket/tickets/c031aeb0-f374-4d57-9d46-2463dfa8571d/ticket.toml)
- [6484d4b7 Build larger-integration parity routine for workflow and health surfaces](C:/Users/linus_behrbohm/git/SECOND_CHECKOUT/graph_app/workflow-tools/ticket/.workflow-tools/ticket/tickets/6484d4b7-e24b-4c13-999c-d0b00928d97c/ticket.toml)

# Upstream Prerequisites

- [790df512 Specify scoped selector contract for board and next](C:/Users/linus_behrbohm/git/SECOND_CHECKOUT/graph_app/workflow-tools/.workflow-tools/ticket/tickets/790df512-d8a9-42bd-b3d6-6e2b4d5eda9c/ticket.toml)
- [68a08b34 Scope-aware board and next for multi-root workspaces](C:/Users/linus_behrbohm/git/SECOND_CHECKOUT/graph_app/context-engine/memory-api/.workflow-tools/ticket/tickets/68a08b34-000b-4585-8354-4b1a26a15f4b/ticket.toml)
- [10cf2a19 Expose workflow trees and actionable ordering metadata](C:/Users/linus_behrbohm/git/SECOND_CHECKOUT/graph_app/workflow-tools/ticket/.workflow-tools/ticket/tickets/10cf2a19-356c-4e69-b0f3-b930d68dc0ce/ticket.toml) is already done and serves as the existing HTTP workflow baseline.

# Supporting References

- [07836f41 Make get/search/list workspace-aware across nested roots](C:/Users/linus_behrbohm/git/SECOND_CHECKOUT/graph_app/context-engine/memory-api/.ticket/tickets/07836f41-7fa5-4e41-8411-1c7cf8aeee75/ticket.toml) remains relevant where authoritative workspace and path metadata affect parity-critical read behavior, but it is not on the critical path unless selector rollout exposes path-metadata gaps.
- [61cb6557 Add consolidated ticket detail/context read surface](C:/Users/linus_behrbohm/git/SECOND_CHECKOUT/graph_app/context-engine/memory-api/.ticket/tickets/61cb6557-e559-4eae-8e59-ea0d520a3bee/ticket.toml) is related follow-up work, but it is not required for this workflow and health convergence track.

# Implementation Order

1. Phase 0: freeze the selector contract and multi-root scope semantics in [790df512 Specify scoped selector contract for board and next](C:/Users/linus_behrbohm/git/SECOND_CHECKOUT/graph_app/workflow-tools/.workflow-tools/ticket/tickets/790df512-d8a9-42bd-b3d6-6e2b4d5eda9c/ticket.toml) and [68a08b34 Scope-aware board and next for multi-root workspaces](C:/Users/linus_behrbohm/git/SECOND_CHECKOUT/graph_app/context-engine/memory-api/.workflow-tools/ticket/tickets/68a08b34-000b-4585-8354-4b1a26a15f4b/ticket.toml). No downstream implementation ticket should enter `in-implementation` before these are done.
2. Phase 1: implement the shared selector surface in [0e375356 Implement scoped selectors for board and next](C:/Users/linus_behrbohm/git/SECOND_CHECKOUT/graph_app/workflow-tools/.workflow-tools/ticket/tickets/0e375356-b74e-48c4-8f1d-77cd28e055bc/ticket.toml). Exit when CLI, MCP, and HTTP expose the agreed selector inputs and scope metadata.
3. Phase 2: move parity-critical workflow and health logic into ticket-api under [c031aeb0 Define minimal workflow and health core plus adapter responsibilities](C:/Users/linus_behrbohm/git/SECOND_CHECKOUT/graph_app/workflow-tools/ticket/.workflow-tools/ticket/tickets/c031aeb0-f374-4d57-9d46-2463dfa8571d/ticket.toml). Exit when the responsibility matrix is explicit and adapter-local logic is removed or documented.
4. Phase 3: build the shared fixture and larger-integration parity harness in [6484d4b7 Build larger-integration parity routine for workflow and health surfaces](C:/Users/linus_behrbohm/git/SECOND_CHECKOUT/graph_app/workflow-tools/ticket/.workflow-tools/ticket/tickets/6484d4b7-e24b-4c13-999c-d0b00928d97c/ticket.toml). Exit when automated and manual parity checks run against the same seeded fixture.
5. Phase 4: close the tracker only after all child tickets are done, the spec is current, and the review evidence is attached.

# Acceptance Criteria

- Ticket-api owns the minimal parity-critical core for selector-driven workflow discovery, health evaluation, and normalized result shaping used by the three interfaces.
- The adapter boundary is explicit: CLI and HTTP stop owning domain logic they should delegate, and ticket-mcp stops under-specifying inputs or outputs that clients need for parity.
- One larger-integration validation routine compares equivalent discovery and health flows across CLI, MCP, and HTTP against the same seeded fixture store.
- Review artifacts explain any remaining intentional transport differences and reject undocumented behavioral drift.
- The tracker only moves to `in-review` once the related spec links the exact ticket files, updated docs, and passing validation evidence.

# Review Workflow

1. Contract review gate: move [790df512 Specify scoped selector contract for board and next](C:/Users/linus_behrbohm/git/SECOND_CHECKOUT/graph_app/workflow-tools/.workflow-tools/ticket/tickets/790df512-d8a9-42bd-b3d6-6e2b4d5eda9c/ticket.toml) and [68a08b34 Scope-aware board and next for multi-root workspaces](C:/Users/linus_behrbohm/git/SECOND_CHECKOUT/graph_app/context-engine/memory-api/.workflow-tools/ticket/tickets/68a08b34-000b-4585-8354-4b1a26a15f4b/ticket.toml) through `in-review` first. Their review packet should freeze selector names, scope metadata, compatibility behavior, and the regression matrix used downstream.
2. Selector implementation gate: review [0e375356 Implement scoped selectors for board and next](C:/Users/linus_behrbohm/git/SECOND_CHECKOUT/graph_app/workflow-tools/.workflow-tools/ticket/tickets/0e375356-b74e-48c4-8f1d-77cd28e055bc/ticket.toml) only after the phase-0 tickets are done. This review must compare CLI, MCP, and HTTP selector inputs and selected-scope metadata side by side.
3. Core-boundary gate: review [c031aeb0 Define minimal workflow and health core plus adapter responsibilities](C:/Users/linus_behrbohm/git/SECOND_CHECKOUT/graph_app/workflow-tools/ticket/.workflow-tools/ticket/tickets/c031aeb0-f374-4d57-9d46-2463dfa8571d/ticket.toml) after the selector surface is stable. This review must include the responsibility matrix and explicitly call out any remaining adapter-local behavior.
4. Integration gate: review [6484d4b7 Build larger-integration parity routine for workflow and health surfaces](C:/Users/linus_behrbohm/git/SECOND_CHECKOUT/graph_app/workflow-tools/ticket/.workflow-tools/ticket/tickets/6484d4b7-e24b-4c13-999c-d0b00928d97c/ticket.toml) last. This review must include the automated parity run plus the manual fixture-based checklist.
5. Tracker closeout gate: move the tracker to `in-review` only after all prerequisite and child tickets are done and the spec links the exact ticket files, docs, and validation evidence. Close it only after the final reviewer confirms there is no undocumented behavior drift left in workflow or health surfaces.

# Manual Validation Checklist

1. Seed the shared fixture store with dependency edges, board state, and enough workspace metadata to expose scope and ownership differences.
2. Run the CLI discovery and health flows against that fixture, including `ticket next`, `ticket board show`, and the relevant health command.
3. Run the HTTP workflow and health flows against the same fixture and capture the normalized candidate ids, ordering, warnings or exclusions, findings, and metadata.
4. Run the MCP discovery and health flows against the same fixture and capture the same normalized fields.
5. Compare only intentional transport-local differences during review: CLI human formatting, HTTP status or envelope mapping, and MCP tool envelopes or alias names.
6. Re-run the automated parity suite, record the exact commands or test targets used, and attach the passing results before moving the tracker to `in-review`.

# Review Notes

Review should reject any solution that keeps selector semantics, health findings, workflow result shaping, or required MCP contract fields duplicated or missing across interface crates without a documented transport-only reason.