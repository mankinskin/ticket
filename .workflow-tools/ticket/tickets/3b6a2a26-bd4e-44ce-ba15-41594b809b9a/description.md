Extend the shared workflow layer with explicit blocker and unlock tree derivation.

Scope:

- derive upstream blocker trees rooted at a queried ticket or epic
- derive downstream unlock trees rooted at a queried prerequisite
- preserve direct parent-child structure at every level instead of flattening transitive results
- compute frontier leaf counts, frontier leaf ids, blocker distance, and per-node ranking keys so tree commands can order sibling sets by closeness to progress

This ticket should stay inside `ticket-api` and land focused unit coverage for nested tree derivation and ordering.
