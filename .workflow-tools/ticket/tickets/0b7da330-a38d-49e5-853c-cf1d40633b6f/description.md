# Problem

The explorer still models ticket state filtering as a single selected value.

Evidence from the current code:

- `memory-viewers/ticket-viewer/frontend/dioxus/src/store.rs` stores `state_filter` as `String`.
- `memory-viewers/ticket-viewer/frontend/dioxus/src/components/ticket_tree.rs` exposes a single `state_filter: String` prop.
- `memory-viewers/ticket-viewer/frontend/dioxus/src/components/ticket_tree/header.rs` renders chips as single-select.

This conflicts with the original TicketTree delivery goals and prevents users from narrowing the explorer to multiple meaningful workflow buckets at once.

## Acceptance Criteria

1. The sidebar explorer supports selecting multiple states at the same time with OR semantics across selected states.
2. Query text and state filtering combine as AND logic: results must match the query and belong to at least one selected state.
3. The UI exposes a clear reset path (`All` or equivalent) and accurate active styling/`aria-pressed` state for every chip.
4. The persisted explorer UI state is updated from a single string to a stable multi-state representation and survives reloads without corrupting older saved state.
5. The backing ticket list API contract supports the multi-state request shape used by the viewer, and that request shape is covered by tests.
6. Browser tests cover selecting multiple states, reloading persisted state, and combining multi-state filters with a search query.

## Implementation Notes

- The shared viewer-api `FileTree` already accepts `active_filters: Vec<String>`; align the ticket-viewer state model with that direction instead of keeping a one-off single-string API.
- Decide and document the wire format for multiple states explicitly (for example repeated query params or a stable encoded list).
- Keep state-chip labels and colors consistent with the existing ticket state presentation.
