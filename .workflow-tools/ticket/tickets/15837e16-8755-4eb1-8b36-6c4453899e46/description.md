Integrate recent-unblock ordering and tree metadata into prioritized workflow surfaces.

Scope:

- extend the shared comparator used by `ticket next`, root-scoped next, and board recommendations so `became_actionable_at` influences ordering after convergence pressure
- preserve stronger dependency-convergence pressure ahead of the new recency signal
- expose the new workflow timing and tree-ranking metadata in machine-readable outputs
- add MCP parity for tree payloads and prioritized-list ordering where the ticket workflow surfaces are already exposed to agents

Focused validation should prove that a recently actionable ticket outranks otherwise equivalent work while urgent blocker repair still wins.
