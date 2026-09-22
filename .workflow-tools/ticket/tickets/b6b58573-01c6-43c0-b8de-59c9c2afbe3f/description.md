## Objective
Refine spec `1d62442b` for output defaults and transport negotiation.

## Context
Owner decision: “TOON by default. Stable field presence, NO byte cap. Tests assert both semantics and valid TOON format.”
Owner decision: “HTTP defaults to JSON; MCP defaults to TOON. Explicit selector on BOTH transports (Accept header / request field).”
Audit: `tmp/test-coverage-audit/03-requirements.md`.

## Acceptance criteria
- Specify CLI default and invalid-selector behavior.
- Specify HTTP and MCP defaults plus both explicit selectors.
- Define stable fields and format-validation obligations.

## Out of scope
- Implementing transport behavior or tests.