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

