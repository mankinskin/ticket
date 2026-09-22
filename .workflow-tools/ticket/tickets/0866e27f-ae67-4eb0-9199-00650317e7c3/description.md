# Fix Asset Follow-Up Selection in Ticket Sidebar

## Problem

Clicking an expanded asset row in the ticket tree does not reliably propagate selected_file state and does not trigger the owning-workspace /asset follow-up request.

## Scope

- Identify why file-row selection is not applied for non-description assets.
- Ensure selected_file updates for same-ticket asset rows.
- Ensure TicketContent fetch path reacts to selected_file and requests /asset with owning workspace + file path.

## Acceptance Criteria

- Clicking an expanded non-description file row marks that file row active.
- The UI requests /api/tickets/{id}/asset with workspace set to the ticket_ref workspace and path set to the selected asset path.
- The content panel renders the returned asset content.
- A deterministic seeded release E2E test passes for this behavior.
