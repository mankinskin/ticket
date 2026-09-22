# Goal

Improve the `.ticket/README.md` generated index so each entry is compact, actionable-first, and navigable.

## Problem

Current output has multi-line bullet blocks per ticket (H4 heading + summary line + ref line). With 700+ tickets the file is unwieldy. Entries are ordered alphabetically within each state/component bucket, not by actionability. State sections are flat — no way to collapse. The ref is a quoted string, not a clickable link.

## Scope

**Refined per Q1-Q3 decisions:**

- Change `render_entry_block` in `ticket-api/src/store_index.rs` so each entry is a single line:
  `- [short_id title](relative/path/to/ticket.toml)`
  The link target is the relative path to `ticket.toml` within the workspace.
  
- **Q1 Grouping & Ordering**: Group entries by state, following schema progression order (new → ready → in-implementation → in-review → done → cancelled). Within each state group, order entries by effort (ascending), then by created_at (oldest first). This prioritizes quick wins before heavy work, and orders same-effort tickets FIFO.
  
- **Q2 Collapsing**: Keep all state sections open by default (expand all). Omit `<details>`/`<summary>` wrapping for clarity and simplicity in this version.
  
- **Q3 Link Format**: The entire entry line is a clickable link. State is implied by the section header — no additional `state` or `priority` metadata on the link line. Just `[short_id title](path)`.

- Remove the `#### [short_id]` H4 heading — the linked line replaces it.
- Keep provenance comment `<!-- ticket-index:entry ... -->` on the line preceding each entry.
- Update `render_catalog_markdown` ordering: state ordering must follow the schema progression.
- Update integration tests to assert single-line link format and effort→created_at ordering within each state.

## Acceptance criteria

- `.ticket/README.md` entries each occupy exactly one output line in format `- [short_id title](path)`.
- Each entry is a markdown link to the relative `ticket.toml` path.
- State sections appear in workflow progression order (new first, cancelled last).
- Within each state, entries are ordered by effort (ascending), then by created_at (FIFO).
- All state sections are open (expanded) by default.
- Re-running `ticket store-index --check` after generation returns `drift:false`.

## Non-goals

- No change to `index.toon` sidecar format.
- No change to the agent-hook file format.
- No UI changes.
