# Problem

Recent fixes closed two local failures but left the larger design problem intact:

- aggregate normal scan refreshed the indexed row without refreshing the Tantivy document
- the viewer list let an older unfiltered response overwrite a newer filtered response
- the HTTP search branch can still synthesize fallback summaries for unresolved hits instead of dropping them
- workspace identity is still derived from basename-only folder names, which is not collision-safe
- filtered live updates are not specified cleanly under SSE, reconnect, or workspace switches

# Design goals

1. Define a canonical workspace identity model that is collision-safe and reversible.
2. Define one reconciliation contract for add, update, delete, move, and removed-scan-root cases across index and search.
3. Define an authoritative resolved-hit policy for list, search, detail, history, files, and asset flows.
4. Define filtered explorer update semantics for overlapping requests, SSE events, snapshot refreshes, and workspace switches.
5. Define a regression matrix that locks these behaviors with focused unit, integration, and browser tests.

# Required outputs

## Workspace identity contract

- decide whether public transport uses a stable workspace id, a repo-relative path key, or a compatibility envelope that carries both id and display label
- keep UI-readable labels while removing ambiguity from `ticket_ref.workspace`
- document migration behavior for CLI, MCP, HTTP, ticket-viewer, and ticket-vscode callers

## Reconciliation contract

- normal scan and force scan may differ in implementation cost, but must not differ in user-visible search/list/get visibility semantics
- unresolved search hits must be repaired, pruned, or dropped with diagnostics; they must never become synthesized ghost tickets
- deleted or moved tickets must update both the index row and the search document in the same reconciliation pass

## Frontend synchronization contract

- newest relevant request wins for success and error paths
- responses from a previous workspace must not overwrite the current workspace list
- SSE `ticket.upsert` and `ticket.delete` must either respect active filters or schedule a silent refetch instead of mutating a filtered list directly

## Validation matrix

- duplicate-basename workspace fixtures
- deleted-on-disk nested ticket fixtures
- moved nested ticket fixtures
- removed scan-root fixtures
- unresolved search-only document fixtures
- filtered SSE update fixtures

# Validation deliverable

- For every invariant in this design, name the owning layer, the concrete fixture, and the exact focused command that proves it.
- Split the matrix by validation layer: storage unit tests, HTTP integration tests, and browser tests where viewer-visible behavior changes.
- Every failure mode must include a negative reproducer fixture, not just a post-fix happy-path assertion.
- The design is not complete until each child ticket carries explicit required tests and command gates instead of generic "add tests" wording.
- If a browser-visible contract changes, the design must require release Playwright coverage plus headed Chromium-family verification with the resolution recorded in the implementation summary.

# Concrete cross-layer test matrix

| Invariant | Owning layer | Concrete fixture | Required assertions | Focused command gate |
| --- | --- | --- | --- | --- |
| Normal aggregate scan reconciles a deleted nested ticket without `--reindex` | `ticket-api` storage | Parent aggregate store with a child workspace ticket already indexed in both ReDB and Tantivy, then `deleted = true` written on disk | After `scan(false)`, `search_tickets`, `list`, and `get` all hide the ticket id; no stale search hit remains reachable | `cargo test -p ticket-api scan_without_reindex_prunes_deleted_nested_ticket_from_search_and_index -- --nocapture` |
| Normal aggregate scan prunes tickets that leave a configured scan root | `ticket-api` storage | Parent aggregate store scans a child root once, then that scan root is removed while the old indexed/search state still exists | After `scan(false)`, child tickets from the removed root disappear from `search_tickets`, `list`, and `get`; no orphan visibility remains | `cargo test -p ticket-api scan_without_reindex_prunes_removed_scan_root_visibility -- --nocapture` |
| Normal aggregate scan repairs moved-path and stale-metadata drift | `ticket-api` storage | Parent aggregate store contains a stale local row and stale search document while the authoritative child ticket path, title, and state have changed | After `scan(false)`, path, title, state, and searchability all match the child ticket; `search_tickets`, `list`, and `get` agree on the repaired owner | `cargo test -p ticket-api scan_without_reindex_repairs_moved_nested_ticket_path_and_search_doc -- --nocapture` |
| Query branch drops unresolved Tantivy-only hits | `ticket-http` integration | Inject a search document id that has no resolvable indexed ticket in any workspace | `/api/tickets?query=...` excludes the id; no fallback `ticket_ref`, empty metadata, or epoch timestamps escape the handler | `cargo test -p ticket-http search_list_drops_unresolved_tantivy_only_hits -- --nocapture` |
| Query branch prefers the authoritative mixed-workspace owner over a stale local placeholder | `ticket-http` integration | Stale local indexed row plus an authoritative child-owned ticket for the same id | Query results use the authoritative owner workspace and remain reversible through detail/history/files/asset follow-ups | `cargo test -p ticket-http search_list_prefers_authoritative_mixed_workspace_hit -- --nocapture` |
| Duplicate-basename workspaces remain collision-safe in the public contract | Registry + `ticket-http` integration | Two distinct workspaces with the same basename registered in one runtime | `/api/workspaces` exposes two distinct public identifiers; list/detail/history/files/asset flows stay reversible for both | `cargo test -p ticket-http duplicate_basename_workspace_identity_ -- --nocapture` |
| Ambiguous legacy workspace labels fail typedly instead of selecting the wrong store | `ticket-http` integration | Same duplicate-basename fixture, but requests use the legacy basename-only label | The server returns a typed request error; it never silently resolves to one of the colliding workspaces | `cargo test -p ticket-http legacy_workspace_label_collision_ -- --nocapture` |
| Latest request wins across overlapping workspace switches | `ticket-viewer` browser | Browser test delays an earlier workspace request and lets the current workspace request finish first | Final DOM ticket ids, active workspace indicator, and follow-up selection state all match the latest workspace only | `cd memory-viewers/ticket-viewer/frontend/dioxus && npx playwright test ./e2e-release/sidebar-query-state-filter.spec.ts -c playwright.release.config.ts -g "workspace switch keeps latest workspace result"` |
| Non-matching SSE updates do not corrupt an active filter | `ticket-viewer` browser | Active query/state filter with injected `ticket.upsert` and `ticket.delete` events for matching and non-matching tickets | Non-matching upserts never appear; matching deletes remove only the expected rows; final DOM still matches the active filter | `cd memory-viewers/ticket-viewer/frontend/dioxus && npx playwright test ./e2e-release/sidebar-query-state-filter.spec.ts -c playwright.release.config.ts -g "SSE filtered list integrity"` |
| Stale success and stale error races cannot overwrite current filtered state | `ticket-viewer` browser | Deterministic delayed-success and delayed-error permutations against the same filtered sidebar flow | Final loading state, error state, and DOM ticket ids all reflect the newest request rather than the slower response | `cd memory-viewers/ticket-viewer/frontend/dioxus && npx playwright test ./e2e-release/sidebar-query-state-filter.spec.ts -c playwright.release.config.ts -g "stale success does not overwrite latest state|stale error does not overwrite latest state"` |

## Completion rule for this matrix

- Each child implementation ticket must copy the relevant rows above into its own implementation summary and keep the command gate focused.
- A row is not complete until the named fixture exists in code and the focused command passes against it.
- If one row needs more than one command, the ticket must explain why one command was insufficient instead of broadening the default command lazily.
- If a command name changes during implementation, update this matrix and the child ticket together so the design and implementation do not drift.

# Downstream work

This design ticket should produce the final work breakdown and acceptance criteria for:

- aggregate scan and search reconciliation
- authoritative resolved-hit transport behavior
- collision-safe public workspace identifiers
- filtered explorer live-update hardening
- cross-layer regression coverage
