## Motivation
Explicit feedback was recorded against `ce://default/ticket/85012858-cbf3-40df-b55e-b82e89f72434` during session `82c8b373-b0ef-4e29-b449-6b48d5fbd87e` (tool call `call_GIBRALy3hHNbenu044bp6Wx3`).

## Feedback
- Rating: `not-helpful`
- Note: RETURN (track-readiness-review): research brief has no durable target path, format, or acceptance evidence for the planning ticket to consume.

## Backtrace
- Session: `82c8b373-b0ef-4e29-b449-6b48d5fbd87e`
- Tool call: `call_GIBRALy3hHNbenu044bp6Wx3`
- Event id: `bf886c4b-95e1-4f36-b511-bfd7894d63cb`
- Dedupe key: `feedback-followup/82c8b373-b0ef-4e29-b449-6b48d5fbd87e/call_GIBRALy3hHNbenu044bp6Wx3`
- FeedbackEntry: the live `feedback_ingest` call already persisted its own entry for `ce://default/ticket/85012858-cbf3-40df-b55e-b82e89f72434`; cross-reference it via `feedback_inbox`/`entries_for(target)` filtered to this session and tool call (today's `feedback_ingest` transport does not yet echo back the created entry's id for direct linking here).

## Verification
Record a validation execution (test-api) confirming the reported issue is addressed before moving this ticket past `in-review`.
