# Normalize Release E2E Suite for Workspace-Aware Flows

## Goal

Reduce false confidence from legacy default-workspace assumptions by updating release E2E tests to workspace-aware ticket-reference behavior.

## Scope

- Audit release specs that hardcode workspace=default assumptions.
- Convert relevant setup and assertions to workspace-aware refs where intended by contract.
- Keep truly default-workspace-only tests explicit and documented.

## Acceptance Criteria

- Updated specs no longer hide mixed-workspace regressions behind default-only assumptions.
- Route/hash assertions accept the modern ticket-workspace + ticket-id contract.
- Existing non-mixed-workspace behavior remains validated.
