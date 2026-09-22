# Goal

Make aggregate scan the single source of truth for both index visibility and search visibility.

# Scope

- normal scan removes or tombstones search documents when tickets disappear, are deleted on disk, move, or leave a scan root
- normal scan and force scan share the same visible semantics even if force scan still performs a heavier rebuild internally
- aggregate path repair updates both indexed metadata and search documents together
- reconciliation emits actionable diagnostics for unresolved or unrecoverable rows instead of silently leaving drift behind

# Acceptance criteria

- `scan(false)` removes a nested ticket from search results when the manifest is marked deleted on disk
- `scan(false)` prunes search and list visibility for tickets that leave a configured scan root
- moved or repaired nested tickets remain readable and searchable after a normal scan
- search, list, and get agree on visibility after add, update, delete, and move cases in a parent aggregate store
- focused storage regression coverage covers deleted-on-disk, removed-root, moved-path, and stale-search-doc cases

# Required tests

- unit: deleted nested ticket disappears from Tantivy results after `scan(false)`
- unit: removed scan root prunes previously indexed child tickets from search/list visibility
- unit: moved child ticket path repair keeps title, state, and searchability aligned
- regression: parent aggregate store returns the same visibility set through list, search, and get after reconciliation

# Rigorous validation requirements

- Reuse the same fixture graph across `scan(false)` and `scan(true)` so the tests prove semantic equivalence instead of two unrelated happy paths.
- Exercise each fixture through both repository-root and direct `.ticket` entry points when the entry point can change ownership resolution.
- Every scenario must assert agreement across `search_tickets`, `list`, `get`, and indexed metadata; if one surface diverges, the test should fail loudly.
- Include at least one negative regression where stale search state or stale indexed metadata exists before reconciliation and is repaired or pruned afterward.
- Required command gate: focused `cargo test -p ticket-api scan_ -- --nocapture` coverage for the new fixtures, plus any additional targeted storage slice needed for moved-path or removed-root coverage.

# Inherited matrix rows for implementation

## Row 1: deleted nested ticket after `scan(false)`

- Fixture: parent aggregate store with a child workspace ticket already indexed in both ReDB and Tantivy, then `deleted = true` written on disk.
- Assertions: after `scan(false)`, `search_tickets`, `list`, and `get` all hide the ticket id.
- Command gate: `cargo test -p ticket-api scan_without_reindex_prunes_deleted_nested_ticket_from_search_and_index -- --nocapture`

## Row 2: removed scan root is pruned without `--reindex`

- Fixture: parent aggregate store scans a child root once, then that scan root is removed while old indexed and search state still exists.
- Assertions: after `scan(false)`, child tickets from the removed root disappear from `search_tickets`, `list`, and `get`.
- Command gate: `cargo test -p ticket-api scan_without_reindex_prunes_removed_scan_root_visibility -- --nocapture`

## Row 3: moved path and stale metadata repair stays aligned

- Fixture: parent aggregate store contains a stale local row and stale search document while the authoritative child ticket path, title, and state have changed.
- Assertions: after `scan(false)`, path, title, state, and searchability all match the child ticket.
- Command gate: `cargo test -p ticket-api scan_without_reindex_repairs_moved_nested_ticket_path_and_search_doc -- --nocapture`

## Row 4: visibility agreement regression

- Fixture: one shared aggregate fixture exercised through add, update, delete, move, and removed-root transitions.
- Assertions: `search_tickets`, `list`, `get`, and indexed metadata expose the same surviving ids after each reconciliation step.
- Command gate: `cargo test -p ticket-api scan_reconciliation_visibility_agreement_ -- --nocapture`

# Implementation summary

- `scan(false)` now prunes stale ticket ids from both ReDB and Tantivy so normal scan and force scan share the same visible semantics.
- Reconciliation now emits actionable diagnostics when a stale row is pruned because the ticket was deleted on disk, disappeared from disk, or left configured scan roots.
- Storage coverage now includes deleted nested tickets, removed scan roots, moved-path repair with search alignment, and shared visibility-agreement regressions for both `scan(false)` and `scan(true)`.

# Validation status

- Passed: `cargo test -p ticket-api scan_without_reindex_prunes_deleted_nested_ticket_from_search_and_index -- --nocapture`
- Passed: `cargo test -p ticket-api scan_without_reindex_prunes_removed_scan_root_visibility -- --nocapture`
- Passed: `cargo test -p ticket-api scan_without_reindex_repairs_moved_nested_ticket_path_and_search_doc -- --nocapture`
- Passed: `cargo test -p ticket-api scan_reconciliation_visibility_agreement_ -- --nocapture`
- Passed: `cargo test -p ticket-api scan_ -- --nocapture`

# Documentation status

- No public docs changed; linked specs already cover aggregate visibility and ticket-list semantics for this slice.

## Implementation rule

- Do not mark this ticket in-review until all four command gates pass or one is blocked with a documented failing command and root-cause note.
- If implementation changes the test names, update this checklist in the same edit so the ticket remains executable.
