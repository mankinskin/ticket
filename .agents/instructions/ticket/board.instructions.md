---
description: "Use when coordinating multi-agent work on the ticket draftboard: check-in/check-out, heartbeats, WIP limits, stale-entry response, and file ownership."
---

## Board Coordination

The draftboard tracks which agent is working on each ticket and which files are
owned. Check in when starting implementation; check out when done.

### Check-In / Check-Out / Heartbeat

```bash
# Register yourself as actively working a ticket
./target/debug/ticket.exe board check-in <ticket-id> \
  --agent <agent-id> \
  --intent "brief description of planned work" \
  --file "src/foo.rs" \
  --file "src/bar.rs" \
  --ttl-secs 3600 \
  --toon

# Refresh your heartbeat before TTL elapses
./target/debug/ticket.exe board heartbeat <entry-id> --toon

# Check out when done (records handoff reason in audit trail)
./target/debug/ticket.exe board check-out <ticket-id> \
  --agent <agent-id> \
  --reason "implemented and tested" \
  --toon
```

A board entry has no dedicated branch column. For a worktree-backed task, record
the branch and worktree in the `--intent` text using this fixed prefix:

```bash
--intent "branch=agent/<ticket-short-id>-<slug> worktree=.worktrees/<ticket-short-id>-<slug> — <planned work>"
```

For a main-checkout task, identify the checkout without inventing a branch:

```bash
--intent "checkout=main — <planned work>"
```

Signal that a branch is integrable by checking out with a `ready-to-merge:` reason:

```bash
--reason "ready-to-merge: agent/<ticket-short-id>-<slug> @ <commit-sha> — rebased onto origin/main, <validation> passed"
```

The board claim covers ticket and file ownership only. Every main-checkout task
checks the board before editing and claims files when concurrent ownership is a
risk. A worktree-backed task additionally claims the authoritative
session-to-worktree-to-branch assignment with `session_check_in`. A conflict on
an applicable claim is an escalation per
[escalation-gate.instructions.md](../orchestration/escalation-gate.instructions.md), not something to work around. See
[worktree-claim.instructions.md](../commit/worktree-claim.instructions.md).

### WIP Limit

`board show` reports `wip_limit_reached` and `next` surfaces a warning when the
limit is hit. Do not start new implementation work when the WIP limit is
reached — finish or hand off an existing entry first.

Default limit: 5 simultaneous active entries. Configure:

```bash
./target/debug/ticket.exe board configure --max-wip 3 --toon
```

### Stale-Entry Response

An entry becomes **stale** when its heartbeat TTL elapses. `board show` lists
stale entries under `warnings[]` and `stale_count`.

Required responses:
1. Agent still active: run `board heartbeat <entry-id>` to renew.
2. Work abandoned: run `board check-out <ticket-id>` then clean.
3. Remove stale entries: `board clean preview --include-stale`, then
   `board clean apply --token <token> --include-stale`.

### File Ownership

Owned files block other agents from checking in with overlapping paths. Keep
owned file lists narrow and release them (via check-out or update-files) when no
longer needed.

Use the short flag forms shown below as the canonical CLI shape. The board
parser keeps the older `--agent-id`, `--files`, `--old-path`, and `--new-path`
spellings as compatibility aliases, but help text and docs should use the same
flag names as the rest of `ticket-cli`.

```bash
# Add / remove files from an active entry
./target/debug/ticket.exe board update-files <ticket-id> \
  --agent <agent-id> --add "new.rs" --remove "old.rs" --toon

# Rename a file in an active entry (atomic)
./target/debug/ticket.exe board rename-file <ticket-id> \
  --agent <agent-id> --from "old.rs" --to "new.rs" --toon
```
