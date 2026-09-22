## Motivation
Explicit feedback was recorded against `ce://default/ticket/9e450826-60e1-437f-b236-2c8839e4ab9e` during session `82c8b373-b0ef-4e29-b449-6b48d5fbd87e` (tool call `call_wh3i0rMlXK1KPKA7SepSmivh`).

## Feedback
- Rating: `not-helpful`
- Note: RETURN (track-readiness-review): plan output has no durable artifact location or approval record, so 7ef3f8db cannot consume a fixed contract.

## Backtrace
- Session: `82c8b373-b0ef-4e29-b449-6b48d5fbd87e`
- Tool call: `call_wh3i0rMlXK1KPKA7SepSmivh`
- Event id: `7d323583-46ea-4cd2-a8b9-d8c9f78f0024`
- Dedupe key: `feedback-followup/82c8b373-b0ef-4e29-b449-6b48d5fbd87e/call_wh3i0rMlXK1KPKA7SepSmivh`
- FeedbackEntry: the live `feedback_ingest` call already persisted its own entry for `ce://default/ticket/9e450826-60e1-437f-b236-2c8839e4ab9e`; cross-reference it via `feedback_inbox`/`entries_for(target)` filtered to this session and tool call (today's `feedback_ingest` transport does not yet echo back the created entry's id for direct linking here).

## Verification
Record a validation execution (test-api) confirming the reported issue is addressed before moving this ticket past `in-review`.
