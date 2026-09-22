# Interview: Update Strategy

**Date:** 2026-04-08
**Applies to:** `8672684c` (SSE integration), `44d22e8f` (ticket-viewer scaffold), store design

## Question

With write operations, the UI needs an update strategy: optimistic updates, wait-for-server, or SSE-driven?

## Answer

**Optimistic updates but waiting for the server to respond before accepting any further updates.**

## Implications

- **Optimistic-with-gate model:** apply the change to the UI immediately, but block further mutations until the server confirms
- This is a middle ground: faster than pure wait-for-server, safer than fire-and-forget optimistic
- UI state: after mutation, show the updated value but enter a "pending confirmation" state
- During pending: disable mutation controls (buttons grayed, inputs locked) until server response arrives
- On success: unlock controls, confirm the optimistic state
- On failure: rollback the optimistic change, show error, re-enable controls
- SSE still needed for multi-user scenarios (other users' changes must propagate)
- Store needs a "pending mutations" queue with at most one in-flight mutation at a time
- Consider a visual indicator (subtle loading bar or spinner) during the confirmation wait
