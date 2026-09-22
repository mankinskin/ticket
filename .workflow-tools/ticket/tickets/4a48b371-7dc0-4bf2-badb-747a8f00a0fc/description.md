# Problem

`ticket next` and MCP `next_tickets` already apply board-aware filtering: tickets
with active or stale board entries stay out of `items`, while the response still
surfaces them in `excluded_by_board` with any relevant board warnings.

HTTP `/api/workflow/next` still returns the raw actionable candidates instead of
the board-aware result. That leaves workflow discovery inconsistent across the
three transports even though the selector contract and compatibility notes already
expect board-aware exclusions to remain on `next` surfaces.

# Goal

Move board-aware `next` filtering behind one shared ticket-api helper, then make
HTTP `/api/workflow/next` return the same board-aware `items`,
`excluded_by_board`, and `warnings` contract as CLI and MCP.

# Acceptance Criteria

- ticket-api exposes one shared helper for board-aware `next` filtering and
  warnings so transports stop duplicating that logic.
- HTTP `/api/workflow/next` filters board-active or stale tickets out of `items`
  and returns them in `excluded_by_board` instead.
- HTTP `/api/workflow/next` surfaces the same board warnings as CLI and MCP,
  including WIP-limit and stale-entry warnings.
- CLI and MCP delegate to the shared helper so the board-aware behavior has one
  backend implementation.
- Focused tests cover the HTTP handler contract and the shared parity suite
  fails if HTTP drifts from CLI/MCP board-aware `next` behavior again.

# References

- Spec: `.spec/specs/a595eb0c-f9f1-4e29-a425-120df5334f7d/spec.toml`
- Tracker: `.ticket/tickets/cf4246c3-6539-4f1c-a876-6d34073db7b3/ticket.toml`
- Shared selector work: `.ticket/tickets/0e375356-b74e-48c4-8f1d-77cd28e055bc/ticket.toml`
- Core-boundary work: `.ticket/tickets/c031aeb0-f374-4d57-9d46-2463dfa8571d/ticket.toml`
- Parity harness: `.ticket/tickets/6484d4b7-e24b-4c13-999c-d0b00928d97c/ticket.toml`

# Validation Plan

- `cargo test -p ticket-http workflow_next_filters_board_active_candidates_into_excluded_by_board`
- `cargo test -p ticket-http --test integration_parity`
- `cargo test -p ticket-mcp next_tickets_excludes_board_active_and_surfaces_wip_warning`