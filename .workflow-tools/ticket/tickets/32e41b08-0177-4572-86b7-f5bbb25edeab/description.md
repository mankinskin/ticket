## Problem

No `get_ticket` view profile exposes the ticket's affected/claimed file paths. Verified during a 2026-07-31 bulk triage:

- The `summary` profile omits affected file paths entirely.
- The `review` profile also omits them (`PATHS_IN_REVIEW_VIEW: no`).

Triage agents relying on view profiles therefore cannot determine which files a ticket claims to touch without falling back to reading the raw `description.md`. Lacking that signal, agents conclude "no code hits" and wrongly classify in-progress work as not-started.

## Impact evidence

In a 56-ticket bulk triage on 2026-07-31: a first pass using the `summary` profile classified 34 tickets as not-started. Re-running with the `review` profile plus direct `description.md` reads reduced that to 5 — a ~29-ticket false-negative rate (52% of the triage set) driven purely by the missing field in both view profiles.

## Acceptance Criteria

- [ ] At least one `get_ticket` view profile (`review` at minimum) includes the ticket's affected/claimed file paths in its response payload.
- [ ] A test asserts the affected-paths field is populated in that view's output for a ticket that declares affected paths (e.g. via board `owned_files` or an equivalent description-derived field).
- [ ] The field is documented in the ticket-mcp tool schema / `get_ticket` tool description so callers know which view profile to request for path visibility.

## Notes for implementer

Investigate whether affected paths should be sourced from board `owned_files` (see `mcp_ticket-mcp_board_update_files` / board check-in `files` param) or from a dedicated ticket field/description convention, and whether the `review` view is the right place to surface it versus a new dedicated field on `summary`.
