# Problem

The ticket store persists only direct `depends_on` edges. Two useful edge
classes are derived from the dependency DAG but are absent from the current
model:

1. **Transitive/latent dependency edges**: the reachability closure implied by
   stored edges, and the inverse transitive-reduction operation that identifies
   stored edges that are redundant because a longer path already implies them.
2. **Execution-order edges**: a linearization or wave assignment expressing
   that one ticket runs before another. Execution-order edges point in the
   exact inverse direction of dependency edges and are currently reconstructed
   manually whenever a workflow graph is drawn.

# Goal

Decide and specify how derived edges are produced and consumed so one rendered
graph can show both persisted dependency edges and derived execution-order
edges. The two classes must be visually distinct and include a legend, following
the rendering conventions in ticket `cd7aceca` (Mermaid/graph rendering
conventions for ticket graphs).

# Scope And Decisions To Make

This planning ticket deliberately leaves the following questions open. The
resulting design must choose an answer or create a named follow-up ticket for
each unresolved question:

1. Compute derived edges on demand, or materialize/cache them in the store;
   assess recomputation cost against cached-data staleness risk.
2. Locate the computation in `ticket-api`, a graph/query layer, or the
   renderer.
3. Decide whether transitive reduction becomes a warning on redundant stored
   edges, where a direct stored edge is already implied by a longer path and
   adds graph noise.
4. Define execution waves for parallel DAG branches: topological levels,
   longest-path levels, or priority-weighted levels.
5. Expose derived edges through the existing `subgraph`/`topgraph` query
   surface or a new query surface.
6. Decide whether the same surface reports cycle detection, since cycles make
   execution order undefined.

# Required Worked Example

Use the `7bc328d7` epic subgraph as the worked example. Record the direct
dependency edges observed in that subgraph, then describe:

- the transitive closure that would add latent reachability edges;
- the transitive reduction that would flag any direct edge already implied by a
  longer path;
- the selected execution-wave assignment and its inverse-direction
  execution-order edges.

The worked example should explicitly account for the existing renderer-related
chain: epic `7bc328d7` depends on renderer-audit ticket `b51c3460`, and
`b51c3460` depends on conventions ticket `cd7aceca`.

# Deliverable

Produce an implementation-ready derived-edge model and query/rendering contract
that renderer code can consume directly under ticket `cd7aceca`'s conventions,
without renderer-side graph transformation. Any implementation work should be
split into follow-up tickets after the decisions are recorded.