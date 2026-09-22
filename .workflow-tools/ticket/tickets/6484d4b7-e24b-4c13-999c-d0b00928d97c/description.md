# Problem

The current validation approach mostly proves ticket-cli, ticket-http, and ticket-mcp in isolation.

That is why the repository can celebrate green checks while the interfaces still disagree on discovery scope, board-aware exclusions, health findings, or required metadata. It is a remarkable little factory for false confidence.

# Scope

1. Create one reusable fixture workspace or store for larger-integration parity testing across ticket-cli, ticket-http, and ticket-mcp.
2. Define a normalized comparison contract for equivalent discovery and health flows, including:
   - actionable candidate ids and ordering
   - board-aware warnings or exclusions
   - scope and workspace metadata needed to explain why a ticket was included or excluded
   - health findings, severities, and messages
3. Add automated coverage that exercises the shared fixture through:
   - ticket-api focused tests for the minimal core contract
   - CLI commands against the fixture store
   - HTTP endpoints against a running server or handler harness
   - MCP tools against a running server or harness
4. Document a manual validation routine for the same fixture and the same flows before review.
5. Make the routine reproducible in CI or on a local checkout without ad hoc seeding or one-off shell archaeology.
6. Keep consolidated ticket detail/context parity out of scope here; that belongs to `61cb6557` if it is pursued separately.

# Acceptance Criteria

- One fixture builder or fixture store setup seeds parity tests for ticket-api, CLI, HTTP, and MCP.
- The automated suite compares equivalent workflow and health flows across the three interfaces and fails when normalized behavior diverges.
- The parity assertions ignore only documented transport-envelope differences.
- The documented manual checklist covers the larger integration path for CLI, HTTP, and MCP against the same fixture store.
- Review artifacts record the exact commands, test targets, or harness entrypoints used for the parity run.

# Implementation References

- Build on `0e375356` and `c031aeb0` instead of snapshotting current adapter quirks as the contract.
- Prefer one shared fixture source of truth that can boot HTTP and MCP against the same seeded data the CLI uses.
- Use process-level orchestration only where it adds confidence over a tighter handler or tool harness.

# Review Notes

The review should reject any parity harness that encodes today’s inconsistent results as the expected baseline or claims end-to-end coverage while never running HTTP and MCP against the same seeded fixture.