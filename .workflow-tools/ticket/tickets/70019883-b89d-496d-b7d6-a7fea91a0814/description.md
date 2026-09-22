Objective
root-scoped `next_tickets` should surface actionable leaf work beneath an epic; scoping to an epic root must traverse containment edges (kind `linked`) to reach leaf tickets.

Context
Unscoped `next` (workspace=default, limit=20) returned 20 items (count 20) but did not include Phase 1 leaves beneath epic `1b58aaf5` in that top-20 slice. Running `next` scoped to root `1b58aaf5` returned count 0 and an empty `items` list. Running `subgraph` from `1b58aaf5` returned `nodes_returned: 43` and `edges_returned: 76`, demonstrating the epic reaches its children via `linked` containment edges. A spot-check subgraph for `1eb03085` shows `1eb03085 -> depends_on -> acebde24` (correct dependency direction). Measured counts: unscoped `next` top-20 count = 20; root-scoped `next` count = 0; epic subgraph nodes = 43, edges = 76.

Acceptance criteria
- `next --root=<EPIC>` surfaces the same actionable leaves beneath that epic that unscoped `next` surfaces for the workspace (filtered to that subtree). In particular, `next --workspace default --root 1b58aaf5` should return the Phase-1 leaf tickets that are unblocked within that subtree.
- Add a regression test that constructs an epic with `linked` containment edges and ensures `next --root` returns its actionable leaves.

Out of scope
- Changing edge kinds or restructuring the epic graph; this is a tooling traversal/visibility fix only.
