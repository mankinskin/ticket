Implement blocker-tree workflow exploration and recently-unblocked ordering on top of the shared dependency-convergence model.

This tracker covers the next iteration of ticket workflow tooling after the shipped convergence ranking work.

Required outcomes:

- add shared upstream and downstream tree derivation to `ticket-api`
- add a `ticket blockers <id>` command and upgrade `ticket unblocked-by <id>` from flat lists to ordered nested trees
- integrate a recent-unblock ordering signal into global actionable ranking without weakening dependency-first convergence pressure
- keep large-store behavior efficient through materialized workflow facts and targeted graph traversal
