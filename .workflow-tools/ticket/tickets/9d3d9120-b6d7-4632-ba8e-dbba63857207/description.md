## Problem

`ticket scan --reindex --force` fails while rebuilding the ticket store search index on Windows. The authoritative on-disk ticket store remains readable, but the derived index cannot be reconciled.

## Reproduction

Run from `c:/Users/linus/git/context-engine`:

```bash
./target/debug/ticket.exe --index-root /c/Users/linus/git/context-engine/.ticket scan --reindex --force --toon
```

First observed failure:

```text
code: invalid_request
message: "storage error: search index error: Failed to open file for read: 'FileDoesNotExist(\"\\\\?\\C:\\Users\\linus\\git\\context-engine\\.ticket\\search_index\\f0719dc3c6214f468ac92e12dd17cf38.term\")'"
```

`.ticket/search_index/` was confirmed git-ignored and untracked, then deleted entirely with `rm -rf .ticket/search_index`. Re-running the same command from an empty index directory failed again, with a newly generated segment hash:

```text
FileDoesNotExist("\\?\C:\Users\linus\git\context-engine\.ticket\search_index\f8b927c4a5684b019e19ba5fae2cdcf1.term")
```

The fresh-directory failure is decisive: the missing `.term` file is generated during the failing rebuild, so the defect is not stale-cache corruption. The rebuild writes a segment and subsequently cannot read the segment back.

## Non-failing controls

Against the same ticket store:

- `./target/debug/ticket.exe get 0afe45b5 --view summary --toon` succeeds.
- `./target/debug/ticket.exe list --toon` succeeds and reports 1538 tickets.
- `./target/debug/ticket.exe board show --toon` succeeds.
- `./target/debug/ticket.exe health --toon` runs and reports only content warnings: `missing_effort_estimation`, `missing_description`, and `graph_participation`.

## Impact

The on-disk store and cached search index can drift with no successful reconciliation path. Normal reads remain available, so the issue is not a total outage, but drift is silent and unbounded for affected Windows stores.

## Located code paths

- CLI command: `memory-api/tools/cli/ticket-cli/src/cli/commands/ops.rs` (`cmd_scan`) maps `--reindex` or `--force` to `TicketStore::scan(reindex)`.
- Rebuild path: `memory-api/crates/ticket-api/src/storage/store/scan.rs` (`scan_once`) calls `self.search.reset_dir()` for a forced rebuild before reintegrating store entries.
- Index writer/reader: `memory-api/crates/memory-api/src/storage/search.rs` implements `TantivySearchIndex`, including `open_or_create`, `reset_dir`, and writer commits.
- Index library: `tantivy = "0.22"` in `memory-api/crates/memory-api/Cargo.toml`.

## Investigation hypotheses

The Windows extended-length-path prefix (`\\?\`) in the error suggests a Windows path-handling issue, but no root cause is confirmed. Other plausible hypotheses are a missing flush or commit before segment readback, write-then-read ordering or race in the segment writer, or a Windows-specific Tantivy index-directory issue. The 1538-ticket store is large enough to produce multiple segments.

## Acceptance criteria

- On Windows, `scan --reindex --force` completes successfully against a 1500+ ticket store from an empty `search_index/` directory.
- A regression test covers rebuilding from an empty search index and verifies the rebuilt index can be queried/read back.
- When the underlying cause is environmental, the failure reports a diagnosable cause and remediation context rather than only a bare `FileDoesNotExist` segment path.
