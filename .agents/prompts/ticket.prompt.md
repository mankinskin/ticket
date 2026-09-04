---
description: "Create a single new ticket from the slash-command text using the ticket-api flow, then update the related spec when requirements or goals change."
name: "ticket"
argument-hint: "<your content>"
agent: "agent"
---

# Create Single Ticket

Create a single new ticket from the user's current slash-command request using the ticket-api flow.
Follow the repository workflow: ticket first, spec second, implementation later.

Reference [ticket-cli](../../memory-api/tools/cli/ticket-cli/README.md) and [ticket-mcp](../../memory-api/tools/mcp/ticket-mcp/README.md).

Install or build the ticket tools when needed:
- Build the CLI in this workspace with `cargo build -p ticket-cli --bin ticket` and use `./target/debug/ticket.exe`.
- Install the CLI onto your Cargo bin path with `cargo install --path memory-api/tools/cli/ticket-cli --bin ticket`.
- Run the MCP server with `cargo run -p ticket-mcp` when MCP access needs to be configured locally.

Workflow:
1. Treat the text typed after `/ticket` as the source request.
2. Search existing tickets first per [workflow.instructions.md#discovery-before-creating](../instructions/ticket/workflow.instructions.md#discovery-before-creating).
3. Search existing specs for the same work so you can update the relevant spec after the ticket is created or matched.
4. Prefer `ticket-mcp` tools such as `list_tickets`, `get_ticket_description`, `create_ticket`, and `workflow` when they are available.
5. If `ticket-mcp` is unavailable, fall back to `./target/debug/ticket.exe search`, `./target/debug/ticket.exe list`, and `./target/debug/ticket.exe create`; use `--index-root` when the intended `.ticket` store is not the nearest one.
6. Infer the best single ticket title, type, priority, and initial state from the request. Keep the result scoped to one actionable work item.
7. When the prompt includes enough detail, add a useful initial description covering motivation, scope, constraints, and acceptance criteria.
8. If a matching ticket already exists, return it instead of creating a duplicate, per [workflow.instructions.md#discovery-before-creating](../instructions/ticket/workflow.instructions.md#discovery-before-creating).
9. For work that introduces new or changed requirements, goals, or behavior, create or update the relevant spec after the ticket is created or matched. Prefer spec-mcp tools when they are available and fall back to `./target/debug/spec.exe` when needed.
10. When linking the ticket in chat output or the spec body, never synthesize the folder path from the UUID, the selected store, or an example path.
11. Resolve the exact canonical ticket folder path per [AGENTS.md](../../AGENTS.md#clickable-reference-policy)'s Clickable Reference Policy: run an immediate follow-up ticket-api command for the authoritative path if the first create or match response omits it.
12. Ensure the spec records the request's requirements or goals before implementation begins and renders ticket references per the Clickable Reference Policy in `AGENTS.md`.
13. Follow [AGENTS.md](../../AGENTS.md#escalation-rules)'s escalation rule: ask one concise clarification if the target store, scope, or ticket shape is still ambiguous after a focused search.
14. Do not split the request into multiple tickets unless the user explicitly asks; `/ticket` should create one ticket.
15. Do not implement code or change unrelated tickets, specs, edges, or board state unless the user explicitly asks.

Response:
- created or matched ticket folder path and title, rendered as a markdown link per the Clickable Reference Policy in `AGENTS.md`
- chosen type, priority, and state
- created or updated spec slug and id, or why no spec change was needed
- duplicate candidates considered, if any
- assumptions that still matter
