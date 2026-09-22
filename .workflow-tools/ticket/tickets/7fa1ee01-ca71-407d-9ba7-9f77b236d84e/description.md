## Objective
Refine spec `6e63979a` so TOON documentation examples are generated and validated.

## Context
Owner decision: “generate examples from command schemas and validate the generated output. `ticket-cli export-command-schema` already exists as the input.”
Audit: `tmp/test-coverage-audit/03-requirements.md`.

## Acceptance criteria
- Define schema-to-example generation contract.
- Define generated-output validation.
- Define source-of-truth and regeneration expectations.

## Out of scope
- Implementing generation or docs.