---
description: "Create new tickets from the slash-command text using the ticket-api flow, then update the related spec when requirements or goals change."
name: "tickets"
argument-hint: "<your content>"
agent: "agent"
---

# Create Ticket Set

Create one or more new tickets from the user's current slash-command request using the ticket-api flow.
Follow the repository workflow: tickets first, spec second, implementation later.

Reference [ticket-cli](../../memory-api/tools/cli/ticket-cli/README.md) and [ticket-mcp](../../memory-api/tools/mcp/ticket-mcp/README.md).

Install or build the ticket tools when needed:
- Build the CLI in this workspace with `cargo build -p ticket-cli --bin ticket` and use `./target/debug/ticket.exe`.
- Install the CLI onto your Cargo bin path with `cargo install --path memory-api/tools/cli/ticket-cli --bin ticket`.
- Run the MCP server with `cargo run -p ticket-mcp` when MCP access needs to be configured locally.

Workflow:
1. Treat the text typed after `/tickets` as the source request.
2. Search existing tickets first per [workflow.instructions.md#discovery-before-creating](../instructions/ticket/workflow.instructions.md#discovery-before-creating).
3. Search existing specs for the same work so you can update the relevant spec after the ticket set is created or matched.
4. Prefer `ticket-mcp` tools such as `list_tickets`, `get_ticket_description`, `create_ticket`, `add_edge`, and `workflow` when they are available.
5. If `ticket-mcp` is unavailable, fall back to `./target/debug/ticket.exe search`, `./target/debug/ticket.exe list`, `./target/debug/ticket.exe create`, and `./target/debug/ticket.exe link`; use `--index-root` when the intended `.ticket` store is not the nearest one.
6. Infer the smallest useful set of tickets that captures the request. Split only where the prompt implies distinct deliverables, dependencies, or workstreams.
7. Give each ticket a clear title plus reasonable type, priority, and initial state. Add descriptions when the prompt provides enough detail.
8. Add dependency edges only when the ordering is explicit or strongly implied. Do not invent extra structure.
9. Reuse existing tickets and create only the missing ones, per [workflow.instructions.md#discovery-before-creating](../instructions/ticket/workflow.instructions.md#discovery-before-creating).
10. For work that introduces new or changed requirements, goals, or behavior, create or update the relevant spec after the ticket set is created or matched. Prefer spec-mcp tools when they are available and fall back to `./target/debug/spec.exe` when needed.
11. In markdown outputs or spec bodies, never synthesize any ticket folder path from a UUID, the chosen store, or an example path.
12. Extract each exact canonical ticket folder path from ticket-api output. If the first create, match, or list response omits the folder path, run an immediate follow-up ticket-api command that returns the authoritative path for each referenced ticket before composing the chat response or updating the spec.
13. Ensure the spec records the request's requirements or goals before implementation begins and renders related ticket references per the Clickable Reference Policy in `AGENTS.md`.
14. Ask one concise clarification if the target store or ticket breakdown is still ambiguous after a focused search.
15. Do not implement code or change unrelated tickets, specs, or dependencies unless the user explicitly asks.

Response:
- target store and number of tickets created or matched
- ticket folder paths and titles created or reused, rendered as markdown links per the Clickable Reference Policy in `AGENTS.md`
- created or updated spec slug and id, or why no spec change was needed
- dependency edges added, if any
- assumptions or follow-up gaps
