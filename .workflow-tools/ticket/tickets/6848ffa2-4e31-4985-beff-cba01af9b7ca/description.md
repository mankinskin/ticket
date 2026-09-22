# Objective
Extend ticket ordering so `board`, `next`, `list`, and similar listing surfaces account for the new `effort` field.

# Status
Implemented.

## What changed
- Added shared `effort` parsing for token-budget strings in workflow ranking.
- Updated `ticket next` ordering to prefer lower-effort work before recency/priority tiebreakers.
- Updated shared list ordering so list-style commands sort by ascending effort.
- Surfaced `effort` in CLI/HTTP/MCP list and next payloads.
- Updated board recommendation rendering to show effort in JSON and human output.

## Validation
- `cargo test --manifest-path memory-api/crates/ticket-api/Cargo.toml sort_candidates_prefers_lower_effort_before_newer_tickets`
- `cargo test --manifest-path memory-api/crates/ticket-api/Cargo.toml parse_effort_accepts_numeric_token_budgets`
- `cargo test --manifest-path memory-api/tools/cli/ticket-cli/Cargo.toml integration_board_cli -- --nocapture`
- `cargo test --manifest-path memory-api/tools/http/ticket-http/Cargo.toml listing -- --nocapture`
- `cargo test --manifest-path memory-api/tools/mcp/ticket-mcp/Cargo.toml next_tickets -- --nocapture`
