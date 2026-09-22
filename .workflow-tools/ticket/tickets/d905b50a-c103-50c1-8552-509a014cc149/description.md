## Motivation
Explicit feedback was recorded against `ce://default/ticket/3c6da958-f494-408f-b7dd-cc43997b8ead` during session `6f9208a4-c40e-4010-abf8-023505b4bf97` (tool call `call_xH9yX3C0tL8uxSpFxIhcG5wA`).

## Feedback
- Rating: `mixed`
- Note: Review 2026-07-25: AC2-AC5 pass. AC1 needs changes: WorkflowAddNodeInput.category MCP schema must include literal copy-ready example `kind="task", category="<your-label>"`; current prose-only redirect is insufficient. Follow-up: 6cc88405-62f2-4e36-b95e-a37f498175eb. Ticket update/state transition blocked because ticket type `task` has no registered schema.

## Backtrace
- Session: `6f9208a4-c40e-4010-abf8-023505b4bf97`
- Tool call: `call_xH9yX3C0tL8uxSpFxIhcG5wA`
- Event id: `unknown`
- Dedupe key: `feedback-followup/6f9208a4-c40e-4010-abf8-023505b4bf97/call_xH9yX3C0tL8uxSpFxIhcG5wA`
- FeedbackEntry: the live `feedback_ingest` call already persisted its own entry for `ce://default/ticket/3c6da958-f494-408f-b7dd-cc43997b8ead`; cross-reference it via `feedback_inbox`/`entries_for(target)` filtered to this session and tool call (today's `feedback_ingest` transport does not yet echo back the created entry's id for direct linking here).

## Verification
Record a validation execution (test-api) confirming the reported issue is addressed before moving this ticket past `in-review`.
