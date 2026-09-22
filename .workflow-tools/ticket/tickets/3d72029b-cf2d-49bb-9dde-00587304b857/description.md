Materialize recent-unblock and blocker-progress workflow facts for scalable ordering.

Scope:

- define and persist `became_actionable_at` when unresolved dependency count transitions from greater than zero to zero
- define and persist `last_blocker_progress_at` for subtree progress while a ticket remains blocked
- update workflow facts incrementally on ticket state transitions and `depends_on` edge mutations
- avoid per-query history scans for global ranking and tree commands on large stores
- document when Tantivy should be used only for text prefiltering instead of graph traversal

This ticket should focus on the store, index, and propagation logic rather than CLI rendering.
