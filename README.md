Back to [workflow-tools](..).

# ticket

Ticket store and workflow engine: create, query, and transition tickets, track dependency graphs, and coordinate multi-agent work through a draftboard.

## Primary Use Case

Track implementation work as tickets with typed states, dependency edges, and acceptance criteria, so agents and humans can pick unblocked work, check in on a shared board, and record evidence without editing store files by hand.

## Usage

Build the desired transport from the `workflow-tools` workspace root:

```bash
cargo run -p ticket --features cli --bin ticket -- --help
cargo run -p ticket --features mcp --bin ticket-mcp
cargo run -p ticket --features http --bin ticket-http
```

`ticket` finds the nearest `.ticket` workspace by walking up from the current directory.

For board commands, an explicit `--workspace` pointing to a submodule or a
checkout with a separate Git directory uses that checkout's own ticket store.
A managed linked worktree at `.worktrees/<session>/<slug>` instead shares its
main checkout's board. A `.git` file alone does not imply a linked worktree;
submodules normally have no `commondir` file. Invalid linked-worktree metadata
is still rejected. An explicit `--index-root` overrides this board routing.

## Examples

```bash
# List open tickets
cargo run -p ticket --features cli --bin ticket -- list --state open

# Create a ticket
cargo run -p ticket --features cli --bin ticket -- create --type task --title "Example"

# Show unblocked next work
cargo run -p ticket --features cli --bin ticket -- next
```

## Related Crates

- [crates/ticket-api](crates/ticket-api): core domain library (storage, query, workspace management).
- [crates/ticket-vscode-core](crates/ticket-vscode-core): shared core for the VS Code extension.
- [ticket-viewer](ticket-viewer): standalone viewer application.
- [ticket-vscode](ticket-vscode): VS Code extension frontend.
