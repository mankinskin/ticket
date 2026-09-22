## Problem

Board check-in (`mcp_ticket-mcp_board_check_in` / `board check-in`) claims implementation **files only** via `owned_files`. It has no mechanism for claiming "I am authoring tickets/specs under topic X" or "I am authoring epic Y's children". Two agents can therefore concurrently author the same ticket/epic track without any conflict signal, because neither one is touching a shared *file* — they are each writing new ticket manifests to the store.

## Concrete incident

A duplicate ticket track (epic `84a9f497-fe5a-4c04-b1e1-ab99245e6ea0`, "PDF domain capability (pdf-api/pdf) exposed via CLI + MCP", plus its ~10 child tickets) had to be cancelled against a competing epic `322a4737` after two agents independently authored overlapping ticket/epic sets for the same scope. File-based ownership caught nothing because ticket authoring doesn't touch source files.

## Fix direction

Allow board check-in to claim a **ticket-authoring scope / topic lock** — e.g. a parent ticket id, epic id, or free-text topic label — as a peer concept to `owned_files`. A second check-in attempt against the same topic/epic scope should be rejected the same way a file-ownership conflict is rejected today. Reuse the existing conflict-detection, WIP-limit, and stale-entry machinery in `mcp_ticket-mcp_board_check_in` / `board_show` / `board_clean_preview` rather than inventing a parallel system.

## Surface

- `memory-api` ticket-api board/draftboard implementation backing `mcp_ticket-mcp_board_check_in`, `board_update_files`, `board_show`.
- `.agents/instructions/ticket/board.instructions.md` (board coordination rules).
- `.agents/instructions/ticket/workflow.instructions.md` (discovery-before-creating rule, which this complements rather than replaces).

## Notes

This is a distinct fix from ticket `2c019bce` (pre-create search gate): `2c019bce` addresses agents not *checking* before creating; this ticket addresses the board having no way to *prevent* concurrent authoring even when a check would have caught it, by giving agents a claimable scope beyond files.