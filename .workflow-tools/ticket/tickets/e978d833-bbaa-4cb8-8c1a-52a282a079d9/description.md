W2. The immediate neighbourhood is hard to track when the full graph is rendered. Render only a focused set of nodes/edges within a limited distance of the selected node(s); load the full graph into cache only.

Requirements:
- Cache the full workspace graph client-side (nodes + edges + adjacency).
- Render a bounded neighbourhood (configurable hop distance) around the current selection.
- Recompute the visible subgraph from cache on selection change (no refetch).
- Keep camera/layout stable across focus changes.

Reconciles / re-scopes: 6e7a15c9 ("keep full workspace graph visible with focused navigation", in-implementation) took the opposite full-graph-visible approach and is superseded by this bounded-neighbourhood direction; 10c94251 (graph focus + 2D follow-up). Provides the cache that W4 and W6 build on. Spec 98b4f75d.