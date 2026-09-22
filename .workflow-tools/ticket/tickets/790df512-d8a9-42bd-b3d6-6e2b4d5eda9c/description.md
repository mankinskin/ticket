# Problem

`ticket board show`, `ticket next`, MCP `board_show` / `next_tickets`, and HTTP `/api/workflow/next` expose only narrow, inconsistent scoping knobs.

Current facilities are fragmented:

- `ticket list` supports repeated `--where key=value` field filters, including schema fields like `component`.
- `ticket search` supports full-text search over ticket content.
- `ticket subgraph` / `ticket topgraph` already support `root`, `depth`, `direction`, and `edge-kind` for graph traversal.
- CLI `ticket next` supports only a title-prefix `--filter` and an optional reverse-dependent `root`.
- MCP `next_tickets` supports only `workspace`, `limit`, and title-prefix `filter`.
- CLI / MCP board reads only allow agent filtering today.
- HTTP `/api/workflow/next` supports `workspace`, `root`, `filter`, and `limit`, but not field predicates or richer graph scoping.

Operators need to focus workflow discovery on a related slice of tickets: a module, a component, a text-search match, a hierarchy under one or more roots, a specific workspace, or a constrained edge set. Today they have to mentally combine multiple commands and cannot ask board / next directly for "the best next work inside this selected subgraph".

# Scope

1. Define a reusable ticket-selector contract for workflow discovery surfaces.
2. Decide which existing primitives become first-class selector axes: workspace or root selection, field predicates (`component`, `state`, `type`, `tags`, and similar), text query, graph roots, traversal depth and direction, edge-kind, and whether multi-root selection is supported.
3. Specify how selector dimensions compose: intersection by default, explicit union modes where needed, and stable empty-result or omission semantics.
4. Preserve backward compatibility for the current `--filter` and `root` inputs or document an explicit migration path.
5. Define stable field names and response metadata across CLI, MCP, and HTTP workflow surfaces so downstream viewers can consume one contract.
6. Document how this selector contract relates to existing discovery commands (`list`, `search`, `subgraph`, and `topgraph`) so the product surface stays coherent instead of drifting into parallel query dialects.

# Acceptance Criteria

- Canonical spec and operator docs define the selector inputs and combination semantics for board / next discovery.
- The spec names the minimum shared contract for CLI `ticket next`, CLI `ticket board show`, MCP `next_tickets`, MCP `board_show`, and HTTP `/api/workflow/next`.
- The spec explains which current surfaces remain specialized (`list`, `search`, `subgraph`, `topgraph`) and which selector pieces are reused from them.
- The spec defines machine-readable scope metadata plus empty-state or omitted-ticket behavior.
- The spec records a regression matrix for CLI, MCP, and HTTP parity coverage.

# Implementation Summary

**Spec created:** `.spec/specs/a595eb0c-f9f1-4e29-a425-120df5334f7d/` (slug `ticket-api/workflow/scoped-selector-contract`).

The spec defines:
- Five selector axes: workspace, title-prefix filter, graph-root scope, field predicates (Phase 1+), and future text-query (Phase 1+).
- Intersection composition semantics.
- Machine-readable `scope` object required in every workflow-discovery response.
- Backward compatibility table for all existing CLI/MCP/HTTP inputs.
- Phase 0 surface scope: CLI `ticket next`, CLI `ticket board show`, MCP `next_tickets`, MCP `board_show`.
- Regression matrix for Phase 0 (CLI + MCP) and Phase 1 (HTTP).

**Phase 0 implementation in `68a08b34`:** adds `root` param to MCP `next_tickets`, adds `scope` object to CLI next/board-show and MCP next_tickets/board_show responses.

# Related Spec

- `.spec/specs/a595eb0c-f9f1-4e29-a425-120df5334f7d/` — scoped selector contract spec

# Related Existing Tickets

- `68a08b34` covers workspace or root-aware board / next selection in multi-root repositories.
- `68e3c713` covers `next --filter` semantics.
- `10cf2a19` adds HTTP workflow ordering and tree payloads that this selector contract should extend rather than fork.

# Likely Surfaces

- `memory-api/.spec/`
- `memory-api/tools/cli/ticket-cli/`
- `memory-api/tools/mcp/ticket-mcp/`
- `memory-api/tools/http/ticket-http/`
- `memory-api/crates/ticket-api/`
- `memory-api/README.md`