# Goal

Change the ticket-viewer graph mode so the full graph stays visible while the selected ticket becomes the active focus anchor.

# Scope

- render the full graph dataset in graph mode
- pan or center the viewport toward the focused ticket instead of replacing the graph with a different root-local fetch
- highlight the focused ticket and related edges or nearby nodes
- fade or cull distant unrelated nodes enough to keep the focused neighborhood readable

# Acceptance

- switching ticket focus updates highlighting and camera focus without replacing the whole graph dataset
- the overall workspace graph remains visible in graph mode
- related edges and nodes are emphasized while distant unrelated graph regions are visually de-emphasized
- the behavior works smoothly when switching among nearby tickets in the same workspace

# Status

## Implementation

- the ticket list page now keeps a persistent graph root per workspace in `ticket-viewer/frontend/dioxus/src/routes/list/page.rs`, while focused selection changes only the active graph-content ticket
- split and graph panels in `ticket-viewer/frontend/dioxus/src/routes/list/panels.rs` now pass a stable root id and a changing selected node id into the mounted graph so focus changes reuse the current workspace dataset instead of replacing it
- shared graph focus behavior in `viewer-api/viewer-api/frontend/dioxus/src/graph3d/mod.rs` now animates camera retargeting from selection changes through `selection_auto_focus`
- ticket-viewer graph cards in `ticket-viewer/frontend/dioxus/src/graph3d.rs` now dim unrelated nodes around the focused context and expose stable `data-node-id` selectors so focused navigation can be validated against the mounted graph

## Validation

- passed `cargo check --manifest-path memory-viewers/ticket-viewer/frontend/dioxus/Cargo.toml`
- passed `viewer-ctl stop ticket-viewer && cd memory-viewers/ticket-viewer/frontend/dioxus && npm run test:e2e:release -- graph-detail-sidebar.spec.ts`

## Documentation

- updated [6e7a15c9 ticket](C:/Users/linus_behrbohm/git/SECOND_CHECKOUT/graph_app/context-engine/memory-viewers/.ticket/tickets/6e7a15c9-d8e6-4bbe-bb34-b83bd651896b/ticket.toml) implementation and validation summaries and the tracker spec at [8c4d51ef body](C:/Users/linus_behrbohm/git/SECOND_CHECKOUT/graph_app/context-engine/memory-viewers/.spec/specs/8c4d51ef-c0b6-437d-bc14-672b0802cef2/body.md)
