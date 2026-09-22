## Motivation
Explicit feedback was recorded against `ce://default/ticket/6a47ab0f-7e42-463e-afe0-bf51b85249c9` during session `e31bd0e5-ab29-4e76-9284-5f3d2067f40c` (tool call `toolu_01Mx5YQM8t3Fb2CyqEFkGpNW`).

## Feedback
- Rating: `not-helpful`
- Note: .agents/instructions/orchestration/orchestrator-delegation.instructions.md — no timeout/hang playbook (session 51701334: rule scan hit the 5-min cap 8x before switching approach) and no cost-gate escalation path (3x spec_create rejections sent an agent on a build-the-CLI detour). Add a one-strike timeout rule and a fast delegate/grant escalation on cost-gate rejection.

## Backtrace
- Session: `e31bd0e5-ab29-4e76-9284-5f3d2067f40c`
- Tool call: `toolu_01Mx5YQM8t3Fb2CyqEFkGpNW`
- Event id: `87c423ab-26ed-4c3c-9bc7-827460b02fc5`
- Dedupe key: `feedback-followup/e31bd0e5-ab29-4e76-9284-5f3d2067f40c/toolu_01Mx5YQM8t3Fb2CyqEFkGpNW`
- FeedbackEntry: the live `feedback_ingest` call already persisted its own entry for `ce://default/ticket/6a47ab0f-7e42-463e-afe0-bf51b85249c9`; cross-reference it via `feedback_inbox`/`entries_for(target)` filtered to this session and tool call (today's `feedback_ingest` transport does not yet echo back the created entry's id for direct linking here).

## Verification
Record a validation execution (test-api) confirming the reported issue is addressed before moving this ticket past `in-review`.
