---
description: "Review the highest-ranked in-review tickets using ticket-system ranking, specs, audits, and referenced code."
name: "reviews"
argument-hint: "[limit=5|infinite|endlessly|until no more]"
agent: "agent"
---

# Review In-Review Tickets

Use this workflow to review the highest-ranked tickets currently in `in-review`.

Reference [ticket-cli](../../memory-api/tools/cli/ticket-cli/README.md), [ticket-mcp](../../memory-api/tools/mcp/ticket-mcp/README.md), [spec-cli](../../memory-api/tools/cli/spec-cli/README.md), [spec-mcp](../../memory-api/tools/mcp/spec-mcp/README.md), [audit-cli](../../workflow-tools/audit/crates/audit-cli/README.md), and [audit-mcp](../../workflow-tools/audit/crates/audit-mcp/README.md).

## Limit Handling

1. Read the slash-command text and determine the per-call review limit.
2. Default to `5` when the user does not provide a limit.
3. Treat `infinite`, `endlessly`, or `until no more candidates are found` as unbounded mode.
4. In bounded mode, stop after reviewing the requested number of tickets.
5. In unbounded mode, continue until there are no more eligible `in-review` candidates.

## Candidate Discovery

1. Use ticket-system tools to confirm whether any `in-review` tickets exist.
2. Prefer MCP when available:
- `mcp_ticket-mcp_list_tickets` with `{"workspace":"default","state":"in-review"}`
3. Fall back to CLI when MCP is unavailable:
- `ticket list --state in-review --toon`
4. If there are no `in-review` tickets, return immediately with a concise note and do not continue.

## Ranking

1. Use the ticket system's built-in ranking instead of inventing your own ordering.
2. Prefer:
- `mcp_ticket-mcp_next_tickets`
3. Fall back to:
- `ticket next --toon`
4. Because the ranking surface includes all non-terminal tickets, collect tickets with `state == "in-review"` in returned order.
5. If the first ranked page does not yield enough `in-review` tickets to satisfy the requested limit, increase the ranking query limit and repeat until either:
- enough `in-review` tickets have been collected
- or no more ranked candidates remain
6. Preserve the ticket-system order exactly.

## Per-Ticket Review Workflow

For each selected ticket, work in rank order.

1. Audit the ticket itself.
- Read the manifest and description with `get_ticket` and `get_ticket_description`, or `ticket get` and `ticket describe`.
- Run `mcp_ticket-mcp_health_check` scoped to the ticket, or `ticket health <id> --depth 0 --toon`.
- Confirm the ticket is valid, well-defined, and actionable: title, description, dependencies, state, and acceptance criteria must be coherent.

2. Gather context.
- Read dependency context with `subgraph` and `topgraph`, or `ticket subgraph` and `ticket topgraph`.
- Read related tickets that explain prerequisites, dependents, and design intent.
- Search specs with `spec search "<keywords>"`, read the best matches with `spec get <id-or-slug>`, and inspect code references with `spec refs <id>` and `spec refs <id> validate` when useful.
- Read the referenced code, docs, and nearby tests instead of trusting summaries alone.

3. Audit the implementation slice.
- Use audit tools when the ticket touches code that needs quality or complexity review.
- Prefer `audit.exe run . --toon` or `audit.exe summary --by path . --toon` when a repository-level audit helps explain risk or missing validation.
- Use the strongest available audit or validation signal for the affected code paths.

4. Verify the acceptance criteria.
- Check every acceptance criterion against the implementation, specs, docs, tests, and validation evidence.
- Run the narrowest relevant validation available.
- Record missing evidence explicitly instead of assuming the ticket is complete.

5. Decide and report the outcome. **Do not apply any state transition yourself.**
- If the ticket is incomplete, ambiguous, or missing required validation, report a `fail` verdict and recommend the ticket return to `in-implementation`, with the findings that justify it. Record the recommendation as a ticket field patch (without `to_state`) or via `feedback-mcp`. The Iteration Agent owns the transition.
- If the ticket satisfies its acceptance criteria and review evidence is sufficient, report a `pass` verdict and recommend the ticket advance to `done`. Record the recommendation durably. The Iteration Agent closes the ticket.
- Do not skip the decision; every reviewed ticket must end with an explicit reported verdict.

## Stop Conditions

1. Stop when the bounded review limit is reached.
2. In unbounded mode, stop only when no more eligible `in-review` tickets remain.
3. If a ticket cannot be reviewed because required context is missing, report the blocker and continue to the next candidate unless the missing context blocks the whole review pass.

## Output Format

Return a concise review summary containing:
- requested limit and whether it was bounded or unbounded
- the candidate discovery method used
- for each reviewed ticket: rank, ticket id, title, decision, evidence checked, and blockers or follow-up work
- whether additional `in-review` tickets remain after the pass
- all ticket/spec/code/log references rendered per the Clickable Reference Policy in `AGENTS.md`
