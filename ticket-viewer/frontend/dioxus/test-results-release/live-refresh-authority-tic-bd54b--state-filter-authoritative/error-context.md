# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: live-refresh-authority.spec.ts >> ticket-viewer — live refresh authority >> ticket.delete keeps the active state filter authoritative
- Location: e2e-release\live-refresh-authority.spec.ts:398:7

# Error details

```
Error: Command failed: C:\Users\linus\git\3\meta-workspace\workflow-tools\ticket\target\debug\ticket.exe --json --index-root C:\Users\linus\AppData\Local\Temp\ticket-viewer-filter-tigYBL\workspace update 079b2cb8-c44e-4bd8-88f8-4964e8356065 --to-state ready
{
  "code": "invalid_request",
  "message": "storage error: schema validation: invalid state transition 'open' -> 'ready'; current state 'open' allows next states [cancelled, planned]; no direct transition to 'ready' is available"
}

```