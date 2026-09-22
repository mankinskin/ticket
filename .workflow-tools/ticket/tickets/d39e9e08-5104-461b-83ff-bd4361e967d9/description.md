Add an upstream `ticket blockers <id>` command and upgrade `ticket unblocked-by <id>` to nested tree output.

Scope:

- add the new CLI command surface and JSON contract for `blockers`
- render all deep blocker-tree tickets beneath the root and emphasize frontier leaves
- change `unblocked-by` to return nested parent-child trees below the queried root while preserving derived frontier leaf summaries
- order nested siblings by closeness to being fully unblocked, measured primarily by unresolved frontier leaves and blocker distance
- add human-readable rendering that shows tree structure instead of flattened object lists

Focused integration tests should lock both the nested JSON shape and the human-readable rendering.
