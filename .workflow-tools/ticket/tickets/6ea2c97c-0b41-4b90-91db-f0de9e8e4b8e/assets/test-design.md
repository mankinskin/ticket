# Test Design: Header Actions Cleanup

**Date:** 2026-05-21
**Applies to:** `6ea2c97c` — `[ticket-viewer] Bug: header actions cleanup for theme/info/home/filter`

## Goal

Define the exact validation contract for ticket-viewer root-route header actions before implementation begins.

## Route under test

- Managed release viewer route: `http://127.0.0.1:3002/workspace/default`
- Root shell source: `memory-viewers/ticket-viewer/frontend/dioxus/src/routes/list/page.rs`
- Shared header source: `viewer-api/viewer-api/frontend/dioxus/src/components/header.rs`

## Baseline preparation

1. Run `viewer-ctl prepare ticket-viewer` before any browser validation.
2. If header behavior changes require server-side rebuilds, rebuild/reinstall the managed `ticket-viewer` binary before final validation.
3. Capture the currently rendered root-route header controls before code changes and classify each visible control as:
   - current source behavior,
   - stale managed artifact,
   - misleading iconography,
   - dead control,
   - or intentionally working affordance.

## Final header-action matrix

This ticket will validate the following root-route matrix unless requirements change and this asset is updated first:

- Hamburger sidebar control in the header lead area: **present and working**.
- Theme settings action: **present and working**.
- Home shared action on the root list route: **absent**.
- Header filter-toggle action on the root list route: **absent**.
- Info/help action: **absent unless a visible help/about surface is implemented in this same slice**.

## Primary selectors

- Ready selector: `header.header`
- Theme button: `page.getByRole('button', { name: 'Theme settings' })`
- Home button: `page.getByRole('button', { name: 'Home' })`
- Theme panel root: `.theme-settings`
- Theme panel close button: `button[aria-label="Close theme settings"]`
- Header shared assertions currently live in:
  - `memory-viewers/ticket-viewer/frontend/dioxus/e2e-release/shared/common-viewer-suite.ts`
  - `memory-viewers/ticket-viewer/frontend/dioxus/e2e-release/shared/dioxus-theme-suite.ts`

## Automated validation plan

### Same-slice assertions

1. Ticket-viewer root route renders the final header matrix above.
2. Theme settings button remains visible and opens/closes the panel.
3. No dead header action is rendered.
4. If an info/help action exists, it produces a visible surface and can be closed without JS errors.

### Shared-contract update work

1. Update `common-viewer-suite.ts` so ticket-viewer no longer hardcodes `Home` visibility on the root route.
2. Update `dioxus-theme-suite.ts` so ticket-viewer theme-panel coverage does not depend on `Home` remaining visible.
3. If shared `PageHeader` / `HeaderActions` semantics change, validate the shared contract rather than only the ticket-viewer route.

### Recommended Playwright slices

- Focused release assertion for root-route header matrix.
- Focused release assertion for theme settings open/close.
- Focused assertion for info/help behavior only if that affordance remains.

## Manual validation plan

### Desktop

- Browser: external Chromium-family browser
- Resolution: `1440x900`
- Verify the final header matrix visually.
- Click every visible header control and confirm each has an immediate, user-visible effect.

### Narrow/mobile shell

- Browser: external Chromium-family browser
- Resolution: `390x844`
- Verify the hamburger control still works and no dead header action appears when the layout compresses.

## Failure classification before implementation

### Same-slice blockers

- Current shared Playwright expectations that require `Home` on the ticket-viewer root route are **same-slice** and must be updated as part of this work.
- Any root-route button that renders but has no visible effect is **same-slice**.

### Background / unrelated failures

- Known explorer quick-search selection failures are **background** for this ticket unless the header refactor unexpectedly changes route selection behavior.
- Known graph-detail or graph3d HTTP 500 failures are **background** for this ticket.

## Exit criteria

1. The final header matrix matches this asset or an explicitly updated version of it.
2. Shared header/theme suites are updated to reflect the new contract.
3. Focused release Playwright coverage passes for the touched header/theme slice.
4. External headed browser verification is recorded with the exact resolution used.
