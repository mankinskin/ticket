Create the full implementation blueprint for blocker-tree workflow exploration and recently-unblocked ordering.

This planning pass should extend the shipped convergence model and the existing `unblocked-by` discovery contract without reopening broad implementation work yet.

Deliverables:

- a canonical spec that defines `ticket blockers <id>` and the nested-tree `ticket unblocked-by <id>` contract
- exact semantics for `became_actionable_at` versus `last_blocker_progress_at`
- an efficiency plan for indexed workflow facts and targeted graph traversal on large stores
- a coherent implementation tracker plus child tickets covering model, indexing, CLI, and MCP follow-up
