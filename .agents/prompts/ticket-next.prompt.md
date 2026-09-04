---
description: "Pick the next actionable ticket slice, plan it, and continue implementation with validation and evidence tracking."
name: "ticket-next"
argument-hint: "[ticket-id|query|current]"
agent: "agent"
---

# Ticket Next

Work on the next iteration by following the repository workflow around actionable tickets, focused implementation slices, validation, and evidence tracking.

Reference [AGENTS](../../AGENTS.md), [ticket-cli](../../memory-api/tools/cli/ticket-cli/README.md), [ticket-mcp](../../memory-api/tools/mcp/ticket-mcp/README.md), [spec-cli](../../memory-api/tools/cli/spec-cli/README.md), and [audit-cli](../../workflow-tools/audit/crates/audit-cli/README.md).

## Workflow

1. Follow [workflow.instructions.md's Orientation](../instructions/ticket/workflow.instructions.md#orientation-start-of-every-session) and [Picking Next Work](../instructions/ticket/workflow.instructions.md#picking-next-work) sections before choosing work.
2. If the slash-command text names a ticket or query, prefer that scope when it is valid and actionable.
3. Otherwise pick the highest-value actionable ticket or the smallest unblocked child of the user's current track.
4. Gather only enough code, test, and spec context to define the first implementation slice.
5. Keep the plan concrete:
- what will change first
- what narrow validation will run first
- what `test-api` validation spec or execution should represent the check
- what `doc-api` or `log-api` evidence should be attached or referenced
6. Move the ticket through the correct state sequence as the work progresses.
7. If the ticket is too broad, split or refine it before implementation continues.

## Response

Return:
- the chosen ticket and why it is the right next slice
- all ticket/spec/code/log references rendered per the Clickable Reference Policy in `AGENTS.md`
- the first implementation step
- the first validation step
- the evidence plan for tests, docs, and logs
- any blocker that prevents immediate execution
