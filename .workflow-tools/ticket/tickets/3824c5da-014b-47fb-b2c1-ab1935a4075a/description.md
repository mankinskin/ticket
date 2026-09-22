## Objective
Refine spec `9074b2ef` with a strict machine-readable transport parity matrix.

## Context
Owner decision: “publish a strict field matrix for CLI/MCP/HTTP so parity becomes a generated test.”
Audit: `tmp/test-coverage-audit/03-requirements.md`.

## Acceptance criteria
- Define artifact format and all required fields.
- Define generation/consumption boundary for parity tests.
- Define change-review expectations for field differences.

## Out of scope
- Implementing the generator or matrix tests.