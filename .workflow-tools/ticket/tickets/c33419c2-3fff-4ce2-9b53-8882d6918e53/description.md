# Mixed-Workspace Regression Tracking Summary

This tracker captures the regression-test completeness review for the ticket-viewer mixed-workspace rollout.

## Review Summary

- Mixed-workspace root route with child-owned ticket selection is covered and passing for history and files follow-up behavior.
- Asset follow-up behavior is currently represented as known broken behavior and needs implementation + test hardening.
- Several release specs still assume workspace=default and do not protect cross-workspace ownership semantics.
- There is no explicit contract matrix that validates endpoint-by-endpoint owning-workspace behavior from a selected mixed-workspace ticket reference.

## Evaluation Approach

- Build a contract matrix by endpoint and ownership context.
- Require at least one deterministic seeded E2E per critical flow.
- Keep release E2E split into stable pass paths plus explicit known-issue coverage while bugs are active.
- Add negative assertions for ambient-workspace fallback and legacy route/hash regressions.

## Delivery Goal

Close this tracker after the asset follow-up bug is fixed and release regression coverage includes matrix-level endpoint ownership checks.
