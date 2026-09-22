Fix ticket health false positives across CLI, HTTP, and MCP surfaces.

Scope:
- stop reporting `unblocked_with_deps` for tickets whose `depends_on` targets are already terminal but are outside the currently checked ticket slice,
- stop reporting `missing_description` when the ticket store can already index and search the ticket's `description.md`,
- keep CLI, HTTP, and MCP health findings aligned so the same ticket graph produces the same result everywhere,
- preserve existing dangling-edge, title, and short-description checks.

Validation target:
- focused health regression reproducer against a ticket with a `done` dependency outside the checked slice,
- focused health regression reproducer against a ticket whose `description.md` exists on disk and is already searchable,
- relevant Rust test coverage for the corrected health logic where practical.

Implementation status:
- canonicalized ticket-store roots and indexed ticket paths in `ticket-api` so stale relative scan roots and indexed ticket paths resolve against the store root instead of the caller's current working directory,
- normalized public indexed-ticket reads plus lifecycle operations so `get`, `health`, and related flows recover from legacy relative index entries,
- normalized and de-duplicated scan roots so `scan --force` rewrites tickets from canonical roots instead of reintroducing stale relative paths,
- changed CLI, HTTP, and MCP health dependency evaluation to resolve dependency state from the store via `get_indexed_many`, so root-scoped health checks no longer treat out-of-slice `done` dependencies as unresolved.

Validation status:
- `cargo run --manifest-path ../memory-api/tools/cli/ticket-cli/Cargo.toml -- --json health 936d38d6-a238-4cb9-b00a-1b2a4b65dc04` from `memory-viewers/viewer-api` now returns zero findings,
- `cargo run --manifest-path ../memory-api/tools/cli/ticket-cli/Cargo.toml -- --json get 936d38d6-a238-4cb9-b00a-1b2a4b65dc04` from `memory-viewers/viewer-api` now succeeds,
- `cargo run --manifest-path ../memory-api/tools/cli/ticket-cli/Cargo.toml -- --json health 4d9293ab-b7a8-4113-b80a-bfe39297bad2` from `memory-viewers/viewer-api` now returns zero findings,
- `cargo test --manifest-path memory-api/crates/ticket-api/Cargo.toml recovers_ticket_paths_from_relative_index_entries` passed,
- `cargo check --manifest-path memory-api/tools/http/ticket-http/Cargo.toml` passed,
- `cargo check --manifest-path memory-api/tools/mcp/ticket-mcp/Cargo.toml` passed,
- `cargo run --manifest-path ../memory-api/tools/cli/ticket-cli/Cargo.toml -- --json scan --force` from `memory-viewers/viewer-api` reconciled 78 tickets and kept the repros green afterward.

Documentation status:
- no public docs changed; tool behavior and output shape stayed the same.
