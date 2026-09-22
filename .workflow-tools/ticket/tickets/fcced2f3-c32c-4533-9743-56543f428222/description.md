# Search syntax hints and pattern contract

## Problem

The ticket-viewer currently exposes text search, but the intended contract is not visible enough to users:

- plain search should be understood as matching ticket titles and description/body content
- structured search patterns and field predicates must work when passed through the viewer
- the UI should advertise the supported syntax instead of requiring users to infer it from backend behavior
- the canonical syntax rules and examples need to be documented in the relevant specs

## Scope

- Preserve plain free-text search as title/body matching rather than a broad implicit field search.
- Ensure structured query patterns continue to round-trip from the viewer to the ticket-http backend.
- Surface concise syntax hints near the viewer search inputs or overlay.
- Document the search grammar, supported examples, and free-text versus field-pattern behavior in the relevant specs and user-facing docs.
- Add regression coverage for the viewer/backend contract.

## Acceptance criteria

- Free-text viewer searches are documented and validated as matching ticket titles and description/body content.
- Structured patterns such as field predicates work in the viewer flows that call `/api/tickets`.
- The viewer exposes discoverable hints or help text for the supported search syntax.
- The canonical spec documents the syntax rules, examples, and grammar expectations for explorer and quick-search behavior.
- The ticket-http contract documents how free text and explicit field predicates are interpreted.
- Regression tests cover at least one structured-pattern search path and one free-text path.
