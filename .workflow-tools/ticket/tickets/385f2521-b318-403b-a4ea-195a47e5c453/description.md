# Problem

`ticket update` currently enforces a single-step state transition and optionally accepts `from_state`, which duplicates the current store state and rejects legitimate fast-forward workflows such as `new -> ready -> in-implementation` in one call. `close_ticket` already implements multi-step progression logic separately, which leaves duplicated transition-path handling across ticket surfaces.

## Scope

Unify ticket state-path validation so `update_ticket` can accept an explicit intermediate transition path, remove the need for a caller-provided `from_state`, and share the transition-walking logic with `close_ticket`.

## Assumptions To Prove

- The current store entry defines the authoritative current state; callers do not need to provide `from_state` for validation.
- `update_ticket` can accept a `transition_states` list for intermediate schema-validated states before `to_state`.
- `close_ticket` and `update_ticket` can share one transition-path implementation without changing required-state enforcement.
- CLI, HTTP, and MCP surfaces can adopt the new request shape compatibly.

## Test-Driven Plan

1. Add failing tests for multi-step `update_ticket` transitions and shared transition-path validation.
2. Refactor ticket-api state transition logic to share the path walker between `update_ticket` and `close_ticket`.
3. Update CLI, MCP, and HTTP request types and focused tests for the new `transition_states` behavior.

## Acceptance Criteria

- `ticket update` can progress through valid consecutive intermediate states in one request.
- `from_state` is no longer required for schema validation and is removed or ignored in favor of store state.
- `close_ticket` and `update_ticket` share the same transition-path validation logic.
- Focused ticket-api, ticket-cli, ticket-http, and ticket-mcp tests pass for the new behavior.

## Output Format Guidance

For token-efficient agent workflows, prefer `--toon` output over verbose `--json`:

- **Compact by default**: Use `--toon` for machine-readable output; `--json` adds 40–80% token overhead and should be reserved for external tool integration (e.g., piping to `jq`).
- **Transport consistency**: HTTP and MCP endpoints support both TOON and JSON, with TOON recommended for agent-to-agent communication.
- **Documentation examples**: Update all examples to show `--toon` usage first, with `--json` as a fallback for debugging.