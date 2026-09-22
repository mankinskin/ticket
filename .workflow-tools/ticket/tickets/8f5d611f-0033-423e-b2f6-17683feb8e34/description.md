# Goal

Replace the split metadata/content treatment with a single compact ticket document area in the main layout.

# Scope

- show the ticket title above the main description/body area
- integrate important metadata into the same document surface instead of hiding it in a separate side panel
- present asset files as part of one contextual document flow instead of a bare disconnected file viewer whenever possible
- keep edit and history affordances reachable without fragmenting the primary reading surface

# Acceptance

- the selected ticket renders as one primary document surface in the main layout
- title, key metadata, and description are visible in the same reading context without opening a separate metadata panel
- asset content is contextualized within the document area rather than shown only as a raw detached tab
- the new document remains compact enough to work in split mode next to the graph

# Test design

- open a ticket in split mode and verify the title, metadata summary, and description appear in one scrollable document area
- open a ticket with at least one asset file and verify the document view exposes that asset context without losing the primary ticket body
- resize the main layout and verify the document remains readable in both split and content-only modes

# Status

## Implementation

- the content-panel lookup in `ticket-viewer/frontend/dioxus/src/routes/list/panels.rs` now resolves selected ticket summaries by ticket id, which keeps the integrated document header aligned even when the route workspace differs from the owning `ticket_ref.workspace`
- `ticket-viewer/frontend/dioxus/src/routes/list/panels.rs` now prefers the resolved owning ticket reference from the loaded summary, so mixed-workspace routes fetch the integrated document from the correct workspace instead of the aliased route workspace
- `ticket-viewer/frontend/dioxus/src/components/ticket_content/page.rs` now hydrates the document surface from direct ticket-detail data plus an exact summary lookup in the active route workspace, and the TOML tab now reuses those hydrated fields instead of the sparse initial list payload
- `ticket-viewer/frontend/dioxus/src/components/ticket_content/render.rs` now renders the remaining top-level ticket.toml fields in a generic document section, keeps `depends_on` readable as ticket ids, and falls back to fetched field timestamps so created and updated metadata survive the same mixed-workspace route path

## Validation

- passed `cargo check --manifest-path memory-viewers/ticket-viewer/frontend/dioxus/Cargo.toml`
- passed `viewer-ctl stop ticket-viewer && cd memory-viewers/ticket-viewer/frontend/dioxus && npm run test:e2e:release -- mixed-workspace-root-route.spec.ts -g "content mode renders an integrated ticket document and keeps asset context inline"`

## Documentation

- updated [8f5d611f ticket](C:/Users/linus_behrbohm/git/SECOND_CHECKOUT/graph_app/context-engine/memory-viewers/.ticket/tickets/8f5d611f-0033-423e-b2f6-17683feb8e34/ticket.toml) and the tracker spec at [8c4d51ef body](C:/Users/linus_behrbohm/git/SECOND_CHECKOUT/graph_app/context-engine/memory-viewers/.spec/specs/8c4d51ef-c0b6-437d-bc14-672b0802cef2/body.md) with the extra-field document rendering follow-up and refreshed validation evidence
