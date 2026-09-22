W1. The right details panel duplicates the floating content panel, but the content panel lacks the details panel's interactive buttons and edit capability. Remove the details panel and extend the content panel.

Requirements:
- Remove the right details panel entirely.
- Add inline edit + action buttons to the floating content panel (parity with the removed panel).
- Redesign the content-panel header to be compact (no full-width spill before main content).
- Move secondary metadata (ids, timestamps, tags, etc.) to a footer region.

Reconciles: obsoletes 2b3a6e2e (TicketDetail right-panel theme colors) since the panel is removed. Relates to spec 8c4d51ef. Precedes W4 (clickable neighbours live in the extended content panel).