# Goal

Add release Playwright coverage for the graph review checklist items around layout restoration and zoom-driven node detail interactions.

# Scope

- extend `ticket-viewer/frontend/dioxus/e2e-release/graph-detail-sidebar.spec.ts` with coverage for switching layout and projection away from the defaults and restoring them
- verify zoomed-out compact or minimal graph nodes remain clickable and the selected node stays rich after focus changes
- keep the added checks in the existing ticket-viewer release Playwright suite

# Acceptance

- the graph release suite verifies switching away from and back to the default hierarchical orthographic graph view
- the graph release suite verifies zoomed-out compact or minimal nodes remain interactive and the selected node stays rich
- the added checks pass in `graph-detail-sidebar.spec.ts`

# Status

## Implementation

- `ticket-viewer/frontend/dioxus/e2e-release/graph-detail-sidebar.spec.ts` now includes a layout restore regression that toggles Flat 2D plus Perspective and then restores Hierarchical 3D plus Orthographic while asserting the root and child nodes remain mounted and the restored view preserves top-to-bottom hierarchy ordering
- the same Playwright suite now zooms the Graph3D surface out before selecting a compact or minimal child node, verifies the smaller tier stays clickable, and asserts the selected node remains rich after focus changes and zooming back in

## Validation

- passed `viewer-ctl stop ticket-viewer && cd memory-viewers/ticket-viewer/frontend/dioxus && npm run test:e2e:release -- graph-detail-sidebar.spec.ts`

## Documentation

- updated [1f39ba8f ticket](C:/Users/linus_behrbohm/git/SECOND_CHECKOUT/graph_app/context-engine/memory-viewers/.ticket/tickets/1f39ba8f-650b-417d-b664-1878f08af669/ticket.toml) and the tracker spec at [8c4d51ef body](C:/Users/linus_behrbohm/git/SECOND_CHECKOUT/graph_app/context-engine/memory-viewers/.spec/specs/8c4d51ef-c0b6-437d-bc14-672b0802cef2/body.md) with the added graph review browser coverage