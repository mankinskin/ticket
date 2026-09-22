## Objective

Make MCP reads trustworthy after external mutations to the ticket store.

## Reproduction

Start ticket-mcp; mutate a ticket description via CLI, another MCP server, or direct file write; run get_ticket_description through original server; section present on disk can be reported absent. Observed specifically `## Planning Output Obligation` reported absent while present in .ticket/tickets/*/description.md.

## Root Cause

- memory-api/tools/mcp/ticket-mcp/src/server/query.rs lines 120-127: `get_ticket_description` resolves ticket path from the INDEX, then reads `"description": TicketFs::read_description(&indexed.path),`.
- memory-api/crates/ticket-api/src/storage/store.rs lines 468-474: `get_indexed` reads id-to-path from the internal tickets.db index, not a filesystem scan.
- memory-api/crates/ticket-api/src/watcher/reconciler.rs lines 66-74: a filesystem watcher/reconciler exists in ticket-api and the CLI can start it.
- memory-api/tools/mcp/ticket-mcp/src/server.rs lines 702-708: ticket-mcp constructs `TicketServer::new(index_root)` and serves stdio without starting the reconciler.

## Impact

Concurrent external writes leave in-process MCP index stale; MCP can return stale or missing data despite correct description.md, invalidating MCP readback verification.

## Acceptance Criteria

1. MCP read reflects content written by another process, by starting existing reconciler or revalidating index entry on read.
2. Regression test proves a section present in on-disk description.md is not reported absent by get_ticket_description after external write.
3. Any intentional residual staleness is surfaced to caller, not silently returned.
