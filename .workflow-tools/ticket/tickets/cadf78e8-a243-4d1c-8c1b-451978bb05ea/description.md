# Tracker status

Tracks remaining store scan/index/move performance follow-ups after baseline hot-path fixes.

## Current child slice status

- `bf094901-cdb6-4b25-8ccd-3eb7716f9d20`: in-review (baseline complete)
- `3e4718af-3fd3-40a4-ac89-d298c99c806a`: in-review (incremental workflow-facts recompute)
- `875919d5-558c-46a8-a83f-02a6756a1e0e`: in-review (batched residual SQLite/index writes)
- `013b57bd-2e8c-4d4d-87c8-6f8687a195c8`: in-review (targeted reconcile modes for move/tooling)

## Notes

- Spec remains open/aligned: `0adfbd09-15c7-46ee-be24-03da0564833d`.
- Perf validation suite remains passing after each child slice: `cargo test -p ticket-api --test e2e_perf_move_health -- --nocapture`.
- Move scan phases now execute targeted touched-set reconcile in ticket domain.

## Next decision gate

- Review and land the in-review child slices; then close tracker once dependencies reach terminal state.