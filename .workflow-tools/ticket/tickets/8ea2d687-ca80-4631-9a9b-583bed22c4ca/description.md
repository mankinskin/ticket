# Problem
When the ticket-viewer backend is offline, clicking anywhere in the ticket-viewer frontend can throw `Uncaught RuntimeError: unreachable` and leave the UI unresponsive.

# Goal
Make the frontend degrade into visible error handling instead of panicking when interactive views are clicked after backend loss.

# Acceptance Criteria
- With the backend/API offline, clicking interactive ticket-viewer surfaces does not panic the WASM app.
- The UI stays responsive and shows an existing or new actionable error state.
- External Playwright validation runs in a visible Chromium-family browser.
