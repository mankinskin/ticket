# Problem

Ticket selection in the current ticket-viewer is still predominantly mouse-driven.

The user wants Up/Down navigation to work first in these two surfaces:

1. the sidebar ticket tree
2. the quick-search result list

Current evidence:

- `memory-viewers/ticket-viewer/frontend/dioxus/src/components/ticket_tree/*` has no keyboard focus model for the normal sidebar flow.
- `memory-viewers/ticket-viewer/frontend/dioxus/src/components/search/results.rs` supports hover and click, but not arrow-key focus or Enter activation.
- The shared viewer-api `TreeView` only handles ArrowUp/ArrowDown when `show_checkboxes` is enabled, which does not cover the normal ticket-viewer sidebar interaction.

## Acceptance Criteria

1. When the sidebar explorer itself is focused, Up/Down moves a visible ticket focus indicator through the currently rendered ticket entries.
2. Pressing Enter on the focused sidebar ticket opens that ticket.
3. The quick-search overlay supports Up/Down to move through visible results and Enter to open the focused result.
4. Keyboard handling does not steal input while a text field or textarea is focused.
5. Focus remains deterministic after filtering, result refresh, or selection changes.
6. Browser tests cover keyboard navigation in both the sidebar explorer and the quick-search overlay.

## Implementation Notes

- Keep this ticket local to ticket-list navigation. Do not expand it into global shortcut handling.
- Reuse shared viewer-api tree patterns where practical, but do not block on a full shared abstraction if the ticket-viewer needs a focused local implementation first.
- Include visible focus styling and accessible roles/selection state as part of the delivery, not as a follow-up.
