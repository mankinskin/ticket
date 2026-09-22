# Goal

Replace basename-only workspace identity with a collision-safe public contract.

# Scope

- define a stable public workspace identifier that remains reversible even when two workspaces share the same folder name
- preserve readable display labels for UI surfaces while removing ambiguity from `ticket_ref.workspace`
- update registry, `/api/workspaces`, ticket refs, follow-up endpoints, and viewer routing to use the new contract consistently
- document compatibility behavior for existing callers that still assume folder-name-only workspace labels

# Acceptance criteria

- two workspaces with the same basename can coexist in the same registry without one silently shadowing the other
- list, detail, history, files, and asset flows remain reversible for both workspaces under the public contract
- ambiguous legacy labels either fail with a typed error or map through an explicit compatibility path; they do not silently resolve to the wrong workspace
- specs and HTTP tests document both the public identifier and the user-facing display label behavior

# Required tests

- integration: duplicate-basename workspaces both appear in `/api/workspaces` and resolve correctly
- integration: ticket refs from both workspaces remain reversible through follow-up endpoints
- integration: ambiguous legacy label requests fail predictably instead of selecting the wrong store
- regression: registry resolution never drops one colliding workspace on first-write-wins insertion

# Rigorous validation requirements

- Build at least one duplicate-basename fixture with two real workspaces in the same registry so this ticket proves collision handling instead of mocking string comparisons.
- Exercise both colliding workspaces through `/api/workspaces`, list, detail, history, files, and asset flows on the same fixture set.
- Add explicit compatibility tests for legacy folder-name-only callers; silent wrong-store resolution is a failure, not a compatibility mode.
- If public workspace identifiers change any viewer route or selector behavior, add release Playwright coverage and headed browser verification for the affected route flow.
- Required command gate: focused registry and `ticket-http` integration tests for duplicate-basename fixtures, plus viewer-side validation when the route or selector contract changes.
