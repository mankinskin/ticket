---
description: "Use when editing ticket-system crates or tools that affect API, storage/index, transport, or viewer components. Covers layering, storage/index invariants, HTTP endpoints, force reconciliation, and validation."
---

## Scope

Applies to:
- `crates/ticket-api/`
- `tools/ticket-cli/`
- `tools/ticket-http/`
- `tools/ticket-mcp/`
- `tools/ticket-viewer/`

## Design Constraints

- Respect ticket lifecycle/state machine invariants — see [lifecycle.instructions.md](lifecycle.instructions.md) for the state machine and its `planned`-freeze/review-gate rules.
- Keep storage/index behavior backward compatible unless explicitly requested.
- Preserve clear separation between API, storage, transport, and UI layers.

## HTTP Endpoints

```
GET /api/graph/subgraph?workspace=default&root=<UUID>&depth=2
GET /api/graph/topgraph?workspace=default&root=<UUID>&depth=2
GET /api/graph/health?workspace=default&all=true
GET /api/graph/health?workspace=default&root=<UUID>&depth=4&direction=out
```

## Index Reconciliation (`scan --force`)

`scan` normally only integrates new/changed files it discovers. Use
`scan --force` to force a full reconciliation — every ticket.toml is re-read
from disk and both the SQLite index and Tantivy search index are rebuilt:

```bash
# Force-reconcile all indexes from disk
./target/debug/ticket.exe scan --force --toon
```

Output includes `"force": true` and `"reconciled": <count>` showing how many
tickets were re-indexed. Use this after manual edits to ticket.toml files or
when the index seems stale.

## Validation

- Prefer focused tests for changed modules before broader suites.
- Verify search/index behavior when touching ticket query paths.
- Confirm no regressions in CLI or MCP-facing flows for changed endpoints.
