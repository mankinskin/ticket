# Test Design: Sidebar Tree Parity and Long-List Scrolling

**Date:** 2026-05-21
**Applies to:** `c10cc92e` — `[ticket-viewer] Bug: sidebar tree parity with spec-viewer + long-list scrolling`

## Goal

Define the pre-implementation validation contract for restoring long-list sidebar reachability and spec-viewer-style parity without regressing ticket-specific tree behavior.

## Route under test

- Managed release viewer route: `http://127.0.0.1:3002/workspace/default`
- Local explorer source:
  - `memory-viewers/ticket-viewer/frontend/dioxus/src/components/ticket_tree/page.rs`
  - `memory-viewers/ticket-viewer/frontend/dioxus/src/components/ticket_tree/rows.rs`
- Shared shell/layout source:
  - `viewer-api/viewer-api/frontend/dioxus/src/components/layout/sidebar.rs`
  - `viewer-api/viewer-api/frontend/dioxus/public/css/layout.css`

## Baseline preparation

1. Run `viewer-ctl prepare ticket-viewer` before any browser validation.
2. Use an unfiltered workspace with enough visible tickets to require scrolling. The default managed workspace is the first candidate.
3. If the visible list is too short to exercise scrolling, reset saved UI state and confirm the fully expanded default list before switching to any alternate workspace strategy.
4. Capture the current failure mode before code changes and classify whether the breakage is:
   - wheel scrolling blocked,
   - trackpad scrolling blocked,
   - lower rows not reachable,
   - active row not kept visible during keyboard movement,
   - deep-row selection losing useful sidebar position,
   - or a combination of these.

## Core selectors and observability

- Ready selector: `header.header`
- Filter input: `page.getByTestId('ticket-tree-filter')`
- Ticket row button pattern: `button[data-testid^="ticket-tree-ticket-"]`
- Selected ticket row: `button[data-testid^="ticket-tree-ticket-"][data-selected="true"]`
- Keyboard-active row: `button[data-testid^="ticket-tree-ticket-"][aria-selected="true"]`
- Optional row wrapper: `div[data-testid^="ticket-tree-row-"]`
- Sidebar scroll container candidate: `.sidebar-content`

## Automated validation plan

### Same-slice assertions

1. Lower rows become reachable through actual sidebar scrolling.
2. The visible ticket set changes after scrolling deeper into a long list.
3. A deep row can be clicked and becomes selected.
4. After deep-row selection, the sidebar retains a useful position instead of snapping back to the top unexpectedly.
5. Keyboard movement to deeper rows keeps the active row visible.

### Candidate Playwright coverage

#### Scroll reachability

- Add a dedicated release spec or focused block that:
  - captures the initial visible ticket ids,
  - scrolls the sidebar container downward,
  - confirms newly visible lower-row ids appear,
  - confirms a lower row is clickable and selectable.

#### Keyboard visibility

- Extend keyboard-navigation coverage so repeated `ArrowDown` movement over a long list proves the active row remains within the scrollable viewport.

#### Mobile-sidebar reachability

- Add a narrow-width release check with the mobile sidebar open, then prove lower rows are still reachable and selectable.

## Manual validation plan

### Desktop

- Browser: external Chromium-family browser
- Resolution: `1440x900`
- Verify that wheel or trackpad scrolling reaches lower rows.
- Open a ticket from deep in the list and confirm the sidebar does not strand the user back at the top.

### Narrow/mobile shell

- Browser: external Chromium-family browser
- Resolution: `390x844`
- Open the mobile sidebar drawer and verify lower rows are still reachable.

## Cross-viewer regression plan

If this work touches shared `Sidebar` behavior or shared layout CSS, add a narrow regression check for spec-viewer so its sidebar/spec tree remains scrollable and reachable.

## Failure classification before implementation

### Same-slice blockers

- Any inability to expose a stable scroll container for the ticket tree is **same-slice**.
- Any fix that only restores pointer scrolling while leaving keyboard-driven visibility broken is **same-slice incomplete**.

### Background / unrelated failures

- The existing quick-search selection failure in `keyboard-navigation.spec.ts` is **background** unless the sidebar scroll work changes the same active-row bookkeeping.
- Known graph-detail / graph3d HTTP 500 failures are **background** for this ticket.

## Exit criteria

1. Focused release Playwright proves lower-row reachability and selection.
2. Keyboard-focused row visibility is covered or explicitly verified in the same release slice.
3. External headed browser verification is recorded at desktop and narrow/mobile resolutions.
4. Any shared-sidebar change includes at least one cross-viewer regression check.
