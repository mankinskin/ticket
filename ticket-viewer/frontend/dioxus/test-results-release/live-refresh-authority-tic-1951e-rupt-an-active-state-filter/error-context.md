# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: live-refresh-authority.spec.ts >> ticket-viewer — live refresh authority >> non-matching SSE upserts do not corrupt an active state filter
- Location: e2e-release\live-refresh-authority.spec.ts:344:7

# Error details

```
Error: Command failed: C:\Users\linus\git\3\meta-workspace\workflow-tools\ticket\target\debug\ticket.exe --json --index-root C:\Users\linus\AppData\Local\Temp\ticket-viewer-filter-wS97tP\workspace update 62467053-1811-4faf-8a21-1cad4b663875 --to-state ready
{
  "code": "invalid_request",
  "message": "storage error: schema validation: invalid state transition 'open' -> 'ready'; current state 'open' allows next states [cancelled, planned]; no direct transition to 'ready' is available"
}

```