## Objective
Refine spec `42e8d710` with exact traversal flags and BFS behavior.

## Context
Owner decision: “declare exact flags plus BFS semantics — depth, cycle handling, empty-result behavior. Cycle handling MUST be pinned; the ticket graph can contain cycles.”
Audit: `tmp/test-coverage-audit/03-requirements.md`.

## Acceptance criteria
- Define flags, depth semantics, and BFS ordering.
- Pin cycle handling.
- Define empty-result output.

## Out of scope
- Traversal implementation or tests.