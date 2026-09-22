# Goal

Make the default ticket graph easier to read by tightening the layout, enlarging visible node tiers, and giving rich ticket nodes a more cubic high-LOD presentation.

# Scope

- reduce the default graph spacing so the initial graph footprint is more compact
- enlarge the ticket-viewer node footprints used by the minimal, compact, and rich tiers
- reshape the high-LOD ticket card so it shows more readable title and metadata content

# Acceptance

- default graph nodes are materially easier to read at the default camera than the previous layout
- the default layout is more compact horizontally without losing the top-to-bottom hierarchy ordering
- rich ticket nodes use a more cubic presentation and reveal more content than the previous one-line card

# Status

## Implementation

- `ticket-viewer/frontend/dioxus/src/layout.rs` now uses tighter horizontal, vertical, and z-stagger spacing so the default graph bounds are more compact before the camera frames the scene
- `viewer-api/viewer-api/frontend/dioxus/src/graph3d/data.rs` now increases the ticket-viewer minimal, compact, and rich node dimensions so the renderer keeps ticket nodes larger and easier to read
- `ticket-viewer/frontend/dioxus/src/graph3d.rs` now renders a taller, more cubic rich ticket card with multi-line title content, stronger metadata chips, and a denser compact summary layout

## Validation

- passed `viewer-ctl stop ticket-viewer && cd memory-viewers/ticket-viewer/frontend/dioxus && npm run test:e2e:release -- graph-detail-sidebar.spec.ts`

## Documentation

- updated [800f09ed ticket](C:/Users/linus_behrbohm/git/SECOND_CHECKOUT/graph_app/context-engine/memory-viewers/.ticket/tickets/800f09ed-beb0-4a12-be93-1392e45eadb8/ticket.toml) and the tracker spec at [8c4d51ef body](C:/Users/linus_behrbohm/git/SECOND_CHECKOUT/graph_app/context-engine/memory-viewers/.spec/specs/8c4d51ef-c0b6-437d-bc14-672b0802cef2/body.md) with the graph sizing and layout follow-up