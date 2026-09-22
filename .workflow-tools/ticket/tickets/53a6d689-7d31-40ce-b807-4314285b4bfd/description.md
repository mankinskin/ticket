# Add Mixed-Workspace Contract Matrix Regression Coverage

## Goal

Create endpoint-matrix regression tests that validate workspace ownership semantics for mixed-workspace ticket references.

## Target Matrix

- Endpoints: detail, description, history, files, asset, graph follow-up, mutation follow-up, and SSE refresh paths.
- Ownership contexts: active-workspace ticket, child-owned ticket, ancestor-owned ticket.

## Acceptance Criteria

- At least one deterministic seeded E2E flow validates each required follow-up endpoint for a child-owned ticket.
- Matrix assertions fail on ambient-workspace fallback.
- Tests validate hash/route behavior for root route (no forced /workspace/default requirement).
- Coverage docs in test comments or companion markdown describe what remains intentionally out of scope.
