# Problem

The same ticket store can still produce different workflow and health answers because parity-critical domain behavior is split across the interface crates.

- CLI and HTTP still compute or normalize domain behavior locally.
- Ticket-mcp is too thin in the wrong places and does not expose the full selector or health contract needed for parity.
- Shared selector work exists, but health findings, workflow-result shaping, and adapter-boundary decisions are still scattered.

That is a classic distributed-responsibility mess: too much logic in the adapters that like to cosplay as applications, not enough contract completeness in the adapter that is supposed to expose the application cleanly.

# Scope

1. Build a minimal ticket-api application surface for workflow discovery and health evaluation that reuses the selector contract from `0e375356` instead of redefining it.
2. Move parity-critical domain logic into ticket-api, including:
   - health finding generation and normalization
   - board-aware workflow result shaping and shared metadata assembly
   - any remaining graph-oriented workflow evaluation needed for consistent cross-interface answers
3. Define and document the adapter responsibility boundary:
   - CLI owns parsing, help, formatting, exit semantics, and local bootstrap concerns only
   - HTTP owns routing, query decoding, and HTTP envelope or status mapping only
   - MCP owns tool registration, schema aliases, and MCP envelopes, but must expose the full selector and health contract instead of partial tool shapes
4. Remove or explicitly document any remaining adapter-local domain behavior that still affects parity.
5. Keep consolidated ticket detail/context read surfaces out of scope here; that follow-up belongs to `61cb6557`.

# Acceptance Criteria

- Ticket-api exposes the minimal reusable workflow and health surface consumed by CLI, HTTP, and MCP.
- CLI and HTTP no longer compute parity-critical workflow or health behavior outside ticket-api.
- Ticket-mcp exposes the required selector, workflow, and health inputs or outputs instead of forcing clients to reconstruct missing contract pieces.
- The work produces a reviewable responsibility matrix showing what remains local to each adapter and why.
- Focused tests prove the shared ticket-api surface returns the normalized results that adapters consume.

# Implementation References

- `0e375356` owns selector normalization for board and next; this ticket should build on that contract rather than duplicating it.
- Reuse existing workspace and metadata contracts where they are already authoritative instead of inventing another path-resolution shape.
- Keep review-ready consolidated detail/context reads out of scope for this ticket.

# Review Notes

The review should reject any solution that moves logic sideways into a new wrapper crate or leaves MCP parity gaps explained away as "transport-specific" when the real issue is a missing contract.