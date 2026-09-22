# Problem

The ticket-viewer list route still mounts a local `TicketTree` inside a shell that aggressively hides overflow, and in practice long sidebar trees are not reachable by scrolling. The result diverges from the spec-viewer tree experience, where the primary navigation surface preserves tree reachability and scroll state.

## Evidence

- `memory-viewers/ticket-viewer/frontend/dioxus/src/routes/list/page.rs` mounts a local `TicketTree` inside `Sidebar` and wraps the route in multiple containers with `overflow: hidden`.
- `memory-viewers/ticket-viewer/frontend/dioxus/src/main.rs` sets `html, body, #main { overflow: hidden; }`.
- `memory-viewers/ticket-viewer/frontend/dioxus/src/components/ticket_tree.rs` describes the widget as a scrollable sorted list, but user testing shows long lists cannot be scrolled to lower rows.
- `memory-viewers/.spec/specs/8c2bbb42-4ca0-43a3-8196-334218bf543e/body.md` defines the spec-viewer tree contract, including preserved scroll position and a primary tree navigation surface.

## Scope

This ticket focuses on sidebar/tree reachability and parity, not on changing explorer filter semantics already tracked elsewhere.

- Make the ticket-viewer sidebar tree scrollable for long lists on wheel, trackpad, and keyboard-driven navigation.
- Align the ticket-viewer tree container and row behavior with the spec-viewer/shared tree contract where practical, without regressing ticket-specific file rows.
- Preserve or add stable scroll-state restoration when opening a ticket from deeper in the tree.
- Add browser coverage for long-list reachability.

## Acceptance criteria

1. A long ticket list can be scrolled to lower rows in the sidebar on desktop and mobile-sized layouts.
2. Opening a ticket from lower in the list does not strand the user at the top of the sidebar.
3. The sidebar tree container either reuses the shared/tree parity primitives from spec-viewer or matches their reachability and scroll-preservation contract.
4. Browser coverage proves lower rows are reachable and interactive.

## Track placement

This is Track B in the ticket-viewer shell plan and may run in parallel with the header cleanup ticket.

## Implementation plan

1. Reproduce the failure against a freshly prepared managed ticket-viewer before changing code.
2. Start locally in `memory-viewers/ticket-viewer/frontend/dioxus/src/components/ticket_tree/page.rs` and `rows.rs` before changing shared `Sidebar` semantics.
3. Verify whether the real scroll trap is in local explorer-body composition, row rendering, or shared sidebar sizing.
4. Preserve expanded file rows and ticket-specific interaction while restoring long-list reachability.
5. Add focused-row visibility or scroll restoration only where needed for deep-row selection and keyboard movement.
6. Touch shared `viewer-api` sidebar or CSS only when a local fix cannot satisfy the parity contract.

## Rigorous test design required before implementation

1. Before any code change, record a baseline reproduction using a long-enough workspace and specify whether the failure is wheel scrolling, trackpad scrolling, keyboard visibility, deep-row selection persistence, or multiple of those at once.
2. The pre-implementation test design MUST name the exact selectors or test ids that will prove the fix, including the sidebar ticket row/test ids already rendered by the explorer.
3. The test design MUST include at least one focused release Playwright test for long-list reachability and one focused release Playwright or manual check for deep-row interaction after scrolling.
4. The test design MUST include a keyboard-oriented case proving that the active/focused row remains visible while moving deeper into the list.
5. The test design MUST include external headed browser verification at desktop width and a narrower mobile-sidebar width.
6. If shared `Sidebar` or shared layout CSS changes are proposed, the test design MUST include a cross-viewer regression check so spec-viewer or other viewers do not lose sidebar scrolling.
7. The ticket should not move to `in-implementation` until this test design is recorded in the ticket or a linked asset.

## Suggested validation slice

- `viewer-ctl prepare ticket-viewer`
- Focused release Playwright for sidebar reachability / deep-row selection
- External headed Chromium-family browser verification against the managed ticket-viewer

## Recorded asset

- Pre-implementation test design: `assets/test-design.md`