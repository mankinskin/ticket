# Problem

The ticket-viewer list route exposes a confusing header action set. The route wires `PageHeader.on_home` and `on_theme_toggle`, but the shared `HeaderActions` render the theme toggle as an `InfoIcon`, and there is no ticket-viewer shell spec defining which header actions are meaningful on the list route. In practice the visible controls are misleading: the theme action is not presented clearly as the ticket-viewer's header theme control, the info-like icon does not give users the expected result, and home/filter actions should not be shown when they do nothing useful.

## Evidence

- `memory-viewers/ticket-viewer/frontend/dioxus/src/routes/list/page.rs` wires `on_home` and `on_theme_toggle` on `PageHeader` for the list route.
- `viewer-api/viewer-api/frontend/dioxus/src/components/header.rs` renders `on_theme_toggle` as an `InfoIcon` button titled `Theme settings`.
- Existing ticket-viewer specs cover explorer interactions and theme-settings content, but not which header actions belong on the list route.

## Scope

- Define the ticket-viewer list-route header action contract.
- Expose the theme settings action in the header as a clear, working affordance.
- Remove home and filter actions from the list route when they have no meaningful effect.
- If ticket-viewer keeps an info/help affordance, clicking it must open a visible ticket-viewer help/about surface; otherwise it must not be rendered.
- Add browser coverage for the final header action set.

## Acceptance criteria

1. The ticket-viewer list route renders only meaningful, working header actions.
2. Theme settings open from the header consistently.
3. Dead home/filter actions are not rendered on the list route.
4. Any rendered info/help action has a visible effect and browser coverage.

## Track placement

This is Track A in the ticket-viewer shell plan and may run in parallel with the sidebar/tree parity ticket.

## Implementation plan

1. Reproduce the current header against a freshly prepared managed ticket-viewer before changing code.
2. Start at `memory-viewers/ticket-viewer/frontend/dioxus/src/routes/list/page.rs` and verify which `PageHeader` actions are actually wired on the list route.
3. Preserve the route-level hamburger lead control.
4. Keep a working theme-settings action in the header.
5. Remove header actions that are no-ops on the root list route.
6. If ticket-viewer retains an info/help affordance, implement a visible ticket-viewer help/about surface; otherwise do not render that affordance.
7. If the fix requires changing shared `PageHeader` / `HeaderActions` semantics, coordinate the minimal upstream change with `C:/Users/linus_behrbohm/git/SECOND_CHECKOUT/graph_app/context-engine/viewer-api/.ticket/tickets/bb1c32f5-5275-4e4f-85ae-a0fba09c522a`.

## Rigorous test design required before implementation

1. Before any code change, record the exact intended final header-action matrix for the ticket-viewer root list route.
2. The pre-implementation test design MUST identify whether the current user-visible problem is stale managed assets, misleading iconography, a dead control, or a real missing route-level behavior.
3. The test design MUST include the shared header assertions that need updating in `memory-viewers/ticket-viewer/frontend/dioxus/e2e-release/shared/common-viewer-suite.ts`.
4. The test design MUST include the theme-settings open/close assertions that need preserving or updating in `memory-viewers/ticket-viewer/frontend/dioxus/e2e-release/shared/dioxus-theme-suite.ts`.
5. If any info/help affordance remains, the test design MUST include an explicit browser assertion for its visible effect.
6. If shared header semantics change, the test design MUST include cross-viewer regression expectations and the required focused validation in the shared `viewer-api` surface.
7. The ticket should not move to `in-implementation` until this test contract is written down in the ticket or a linked asset.

## Suggested validation slice

- `viewer-ctl prepare ticket-viewer`
- Focused release Playwright assertions for shared header actions and theme settings panel behavior
- External headed Chromium-family browser verification that no dead header buttons remain

## Recorded asset

- Pre-implementation test design: `assets/test-design.md`