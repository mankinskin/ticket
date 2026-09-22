# Problem

The ticket-viewer sidebar works well for browsing known tickets, but it is too slow when users only know part of a title, body phrase, or ticket id. The viewer needs an in-app search surface that can query the existing ticket full-text endpoint and open matching tickets directly from the workspace page.

# Goal

Add a floating quick-search overlay for the active workspace that is keyboard accessible, supports both free-text and supported `field:value` predicates, and lets users navigate directly from search results to the ticket detail view.

# Acceptance Criteria

- Search bar opens from **Ctrl+K**, **Cmd+K**, or **/**.
- Free-text queries search ticket titles and description/body content through the existing ticket HTTP search path.
- Supported predicates include `id:<value>`, `title:<value>`, `state:<value>` / `status:<value>`, and `type:<value>` / `ticket_type:<value>`.
- Results are ranked by the backend full-text search and shown in a floating result panel.
- State and type facets are exposed as filter chips.
- Keyboard navigation moves through results and Enter opens the selected ticket in the detail route.
- Recent searches are stored in local storage per workspace.
- The UI documents the supported predicate syntax in the search helper tooltip.

# Implementation Status

- `SearchBar` is implemented as a floating overlay in the ticket-viewer frontend and is mounted per workspace page.
- The search flow delegates to `GET /api/tickets?workspace=<ws>&query=<q>` instead of inventing a separate search endpoint.
- The overlay supports document-level open shortcuts, outside-click and Escape dismissal, result activation, state/type filter chips, and workspace-scoped recent searches.
- Recent searches are persisted under `ticket-viewer:{workspace}:recent-searches` and surfaced when the overlay opens empty.

# Validation Status

- Release Playwright coverage verifies keyboard navigation keeps focus in the search input while arrow keys move the active result and Enter opens the selected ticket.
- The same E2E coverage verifies the helper text documents supported syntax and that `id:` predicates narrow results correctly.
- Additional Playwright coverage verifies broad queries return multiple results and that partial-word substring searches still match expected tickets.