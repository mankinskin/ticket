Back to [workflow-tools](https://github.com/mankinskin/workflow-tools/blob/main/README.md).

# ticket

Ticket store and workflow engine: create, query, and transition tickets, track dependency graphs, and coordinate multi-agent work through a draftboard.

## Primary Use Case

Track implementation work as tickets with typed states, dependency edges, and acceptance criteria, so agents and humans can pick unblocked work, check in on a shared board, and record evidence without editing store files by hand.

## Usage

Ticket initialization maintains a store-local `.gitignore` for SQLite/search
indexes, hidden advisory locks and temporary write files. It also excludes the
`.ticket-generation` and `.ticket-transaction.json` control files and
`.ticket-txn-*.stage` / `.ticket-txn-*.backup` transaction staging artifacts at
any depth. Existing custom rules are preserved; missing defaults are appended
on initialization. Ticket manifests, parts, history, attachments and durable
move journals remain trackable. Ignoring a file never removes it from disk or
from Git's existing tracked files.

Build the desired transport from the `workflow-tools` workspace root:

```bash
cargo run -p ticket --features cli --bin ticket -- --help
cargo run -p ticket --features mcp --bin ticket-mcp
cargo run -p ticket --features http --bin ticket-http
```

The MCP transport supports MCP 2026-07-28 `server/discover` and the legacy
`initialize` handshake. Run its stdio regression with the `mcp` feature enabled:

```bash
cargo test -p ticket --features mcp --test integration_stdio_discovery --locked -j 1
```

MCP startup reconciles workspace policy and on-disk tickets incrementally:
unchanged rows and search documents are retained, new/changed tickets are
indexed, and deleted or newly excluded tickets are pruned before serving.
Missing or inconsistent search indexes still trigger a full recovery rebuild.
Explicit policy rescans retain their full-reindex behavior.

Startup diagnostics go to stderr (never MCP stdout). Default INFO logs include
store resolution/open/close and protocol-handshake durations, policy discovery
and root-registration durations, actual scan mode/rebuild reason, scanned and
updated Ticket counts, skipped roots, per-root counts, and scan-phase timings.
`startup_ms` ends before the handshake; `handshake_ms` includes waiting for the
client. `RUST_LOG` overrides the defaults; use
`RUST_LOG=warn,ticket=info,ticket_mcp=info,ticket_api::storage::store=debug`
for detailed storage phase events.

Compare warm policy-reconciliation costs on a disposable fixture:

```bash
cargo bench -p ticket-api --bench fixture_scan --locked -- fixture_startup_policy
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
