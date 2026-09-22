## Motivation
Explicit feedback was recorded against `ce://default/ticket/6a47ab0f-7e42-463e-afe0-bf51b85249c9` during session `e31bd0e5-ab29-4e76-9284-5f3d2067f40c` (tool call `toolu_01Gkjst5pUyW8UfkdQgamdPA`).

## Feedback
- Rating: `not-helpful`
- Note: .agents/instructions/orchestration/model-routing.instructions.md — the "When NOT to Delegate (The Floor)" section is advisory and was not honored: a 3-line edit was delegated and cost 20 min (session 51701334 events.json L80467). Needs an enforced, concrete delegation floor (min task size/scope) and a mandatory verify-before-accept gate; "Verify Subagent Output Before Acting" was ignored while sub-agents reported PASS with shipped defects.

## Backtrace
- Session: `e31bd0e5-ab29-4e76-9284-5f3d2067f40c`
- Tool call: `toolu_01Gkjst5pUyW8UfkdQgamdPA`
- Event id: `96192adf-b071-4048-b29a-2c0a3023dd74`
- Dedupe key: `feedback-followup/e31bd0e5-ab29-4e76-9284-5f3d2067f40c/toolu_01Gkjst5pUyW8UfkdQgamdPA`
- FeedbackEntry: the live `feedback_ingest` call already persisted its own entry for `ce://default/ticket/6a47ab0f-7e42-463e-afe0-bf51b85249c9`; cross-reference it via `feedback_inbox`/`entries_for(target)` filtered to this session and tool call (today's `feedback_ingest` transport does not yet echo back the created entry's id for direct linking here).

## Verification
Record a validation execution (test-api) confirming the reported issue is addressed before moving this ticket past `in-review`.
