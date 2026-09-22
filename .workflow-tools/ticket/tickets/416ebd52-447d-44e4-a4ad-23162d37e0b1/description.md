# Goal

HTTP query responses must only expose tickets that resolve to authoritative indexed paths and workspace ownership.

# Scope

- remove fallback summary synthesis for unresolved search hits
- prefer authoritative resolved metadata over stale local placeholders when a mixed-workspace ticket is available from the registry
- deleted or unresolved ids are dropped from search results and recorded in diagnostics or logs
- query, list, detail, history, files, and asset flows share one ownership-resolution policy
- integrate authoritative folder-path work so follow-up tooling uses the same resolved owner and path

# Acceptance criteria

- a stale Tantivy document whose id no longer resolves is not returned by `/api/tickets`
- deleted tickets never reappear through the search branch
- mixed-workspace query results preserve authoritative owner workspace and follow-up path information
- transport responses never fabricate epoch-timestamp ghost summaries for unresolved ids
- focused HTTP integration tests lock these behaviors

# Required tests

- integration: unresolved search-only doc is dropped from the ticket list response
- integration: wrong local path plus authoritative child ticket resolves to the child owner or is dropped cleanly
- integration: deleted ticket does not reappear through the query branch
- integration: follow-up detail/history/files requests stay reversible for mixed-workspace search hits

# Rigorous validation requirements

- Explicitly inject each bad state: a stale Tantivy-only document, a deleted ticket that still has residual search state, and a wrong local row that competes with an authoritative mixed-workspace result.
- Assert that no fallback summary using ambient workspace ownership, empty metadata, or epoch timestamps can escape the query branch.
- Use the same fixture ids across list, detail, history, files, and asset follow-ups so ownership reversibility is proven end to end rather than per endpoint in isolation.
- Required command gate: focused `cargo test -p ticket-http search_list_ -- --nocapture` coverage for query/list behavior, plus targeted follow-up endpoint integration tests for reversible mixed-workspace hits.

# Inherited matrix rows for implementation

## Row 1: unresolved Tantivy-only hit is dropped

- Fixture: inject a search document id that has no resolvable indexed ticket in any workspace.
- Assertions: `/api/tickets?query=...` excludes the id and emits no fallback `ticket_ref`, empty metadata, or epoch timestamps.
- Command gate: `cargo test -p ticket-http search_list_drops_unresolved_tantivy_only_hits -- --nocapture`

## Row 2: authoritative mixed-workspace owner beats stale local placeholder

- Fixture: stale local indexed row plus an authoritative child-owned ticket for the same id.
- Assertions: query results use the authoritative owner workspace and remain reversible through detail, history, files, and asset follow-ups.
- Command gate: `cargo test -p ticket-http search_list_prefers_authoritative_mixed_workspace_hit -- --nocapture`

## Row 3: deleted ticket does not reappear through query

- Fixture: deleted ticket with residual search state or stale local row.
- Assertions: query results omit the deleted id and no follow-up endpoint can resolve it as active.
- Command gate: `cargo test -p ticket-http search_list_excludes_deleted_hits_and_followups -- --nocapture`

## Row 4: reversible mixed-workspace follow-ups

- Fixture: one shared mixed-workspace search fixture reused across list, detail, history, files, and asset routes.
- Assertions: every follow-up route resolves through the same authoritative owner and path chosen by the list result.
- Command gate: `cargo test -p ticket-http mixed_workspace_search_followups_remain_reversible -- --nocapture`

# Implementation summary

- Query search now treats Tantivy hits as candidate ids only and builds list responses from authoritative resolved metadata or valid same-store scan-root locals.
- Unresolved and deleted residual search hits are dropped instead of synthesized, and the drop site now emits a debug trace with enough context to investigate drift.
- Detail, description, history, files, and asset follow-ups now share the same preference rule: same-store scan-root locals keep their human label, but nested-workspace authoritative hits beat stale parent placeholders.

# Validation status

- Passed: `cargo test -p ticket-http search_list_drops_unresolved_tantivy_only_hits -- --nocapture`
- Passed: `cargo test -p ticket-http search_list_ -- --nocapture`
- Passed: `cargo test -p ticket-http mixed_workspace_search_followups_remain_reversible -- --nocapture`
- Passed: `cargo test -p ticket-http serve::handlers::tickets::tests::listing:: -- --nocapture`

# Documentation status

- No public docs changed; linked specs already cover aggregate visibility and mixed-workspace ownership semantics for this transport slice.

## Implementation rule

- Do not mark this ticket in-review until all four command gates pass or one is blocked with a documented failing command and root-cause note.
- If implementation changes the test names, update this checklist in the same edit so the ticket remains executable.
