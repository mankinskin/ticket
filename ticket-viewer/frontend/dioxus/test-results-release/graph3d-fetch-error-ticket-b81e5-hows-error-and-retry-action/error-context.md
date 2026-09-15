# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: graph3d-fetch-error.spec.ts >> ticket-viewer — graph fetch failure UX >> graph API failure shows error and retry action
- Location: e2e-release\graph3d-fetch-error.spec.ts:20:7

# Error details

```
Error: expect(locator).toBeVisible() failed

Locator: getByText('Failed to load graph')
Expected: visible
Timeout: 20000ms
Error: element(s) not found

Call log:
  - Expect "toBeVisible" with timeout 20000ms
  - waiting for getByText('Failed to load graph')

```

# Page snapshot

```yaml
- generic [ref=e5]:
  - banner [ref=e6]:
    - generic [ref=e7]:
      - button "Collapse tickets sidebar" [ref=e8] [cursor=pointer]:
        - img [ref=e9]
      - generic [ref=e10]: 🎫
      - generic [ref=e11]: e2e-release-store
      - generic [ref=e12]: e2e-release-store--344e8f42
    - button "Theme settings" [ref=e15] [cursor=pointer]:
      - img [ref=e16]
  - generic [ref=e18]:
    - generic [ref=e19]:
      - generic [ref=e20]:
        - heading "Tickets" [level=2] [ref=e21]
        - generic [ref=e22]: "12"
        - button "Collapse sidebar" [ref=e23] [cursor=pointer]:
          - img [ref=e25]
      - generic [ref=e28]:
        - generic [ref=e29]:
          - button "Browse" [pressed] [ref=e30] [cursor=pointer]
          - button "Next" [ref=e31] [cursor=pointer]
          - button "Blockers" [ref=e32] [cursor=pointer]
          - button "Unblocked" [ref=e33] [cursor=pointer]
        - generic [ref=e34]:
          - generic [ref=e35]:
            - textbox "Search titles/descriptions or use id:, title:, state:, type:" [ref=e36]
            - generic [ref=e37]: "Free text searches titles, descriptions, and ticket IDs (including partial-UUID substrings). Patterns: id:<value>, title:<value>, state:<value>/status:<value>, type:<value>/ticket_type:<value>. Terms are ANDed; quote phrases."
          - generic [ref=e38]:
            - button "All" [pressed] [ref=e39] [cursor=pointer]
            - button "planning" [ref=e40] [cursor=pointer]
            - button "ready" [ref=e41] [cursor=pointer]
            - button "impl" [ref=e42] [cursor=pointer]
            - button "review" [ref=e43] [cursor=pointer]
            - button "on-hold" [ref=e44] [cursor=pointer]
            - button "done" [ref=e45] [cursor=pointer]
            - button "cancelled" [ref=e46] [cursor=pointer]
            - button "Toggle batch selection" [ref=e47] [cursor=pointer]: ☑
            - button "Create new ticket" [ref=e48] [cursor=pointer]: + New
          - generic [ref=e50]:
            - generic [ref=e52]:
              - button "Expand ticket files" [ref=e53] [cursor=pointer]: ▸
              - button "Release E2E search fixture alpha planning" [ref=e54] [cursor=pointer]:
                - generic [ref=e56]: Release E2E search fixture alpha
                - generic [ref=e57]: planning
            - generic [ref=e59]:
              - button "Expand ticket files" [ref=e60] [cursor=pointer]: ▸
              - button "Release E2E navigation fixture planning" [ref=e61] [cursor=pointer]:
                - generic [ref=e63]: Release E2E navigation fixture
                - generic [ref=e64]: planning
            - generic [ref=e66]:
              - button "Expand ticket files" [ref=e67] [cursor=pointer]: ▸
              - button "Release E2E completed prerequisite done" [ref=e68] [cursor=pointer]:
                - generic [ref=e70]: Release E2E completed prerequisite
                - generic [ref=e71]: done
            - generic [ref=e73]:
              - button "Expand ticket files" [ref=e74] [cursor=pointer]: ▸
              - button "Release E2E search fixture beta planning" [ref=e75] [cursor=pointer]:
                - generic [ref=e77]: Release E2E search fixture beta
                - generic [ref=e78]: planning
            - generic [ref=e80]:
              - button "Expand ticket files" [ref=e81] [cursor=pointer]: ▸
              - button "Release E2E graph root planning" [ref=e82] [cursor=pointer]:
                - generic [ref=e84]: Release E2E graph root
                - generic [ref=e85]: planning
            - generic [ref=e87]:
              - button "Expand ticket files" [ref=e88] [cursor=pointer]: ▸
              - button "Release E2E review prerequisite in-review" [ref=e89] [cursor=pointer]:
                - generic [ref=e91]: Release E2E review prerequisite
                - generic [ref=e92]: in-review
            - generic [ref=e94]:
              - button "Expand ticket files" [ref=e95] [cursor=pointer]: ▸
              - button "Release E2E search fixture delta planning" [ref=e96] [cursor=pointer]:
                - generic [ref=e98]: Release E2E search fixture delta
                - generic [ref=e99]: planning
            - generic [ref=e101]:
              - button "Expand ticket files" [ref=e102] [cursor=pointer]: ▸
              - button "Release E2E implementation prerequisite in-implementation" [ref=e103] [cursor=pointer]:
                - generic [ref=e105]: Release E2E implementation prerequisite
                - generic [ref=e106]: in-implementation
            - generic [ref=e108]:
              - button "Expand ticket files" [ref=e109] [cursor=pointer]: ▸
              - button "Release E2E legacy description fixture planning" [ref=e110] [cursor=pointer]:
                - generic [ref=e112]: Release E2E legacy description fixture
                - generic [ref=e113]: planning
            - generic [ref=e115]:
              - button "Expand ticket files" [ref=e116] [cursor=pointer]: ▸
              - button "Release E2E search fixture epsilon planning" [ref=e117] [cursor=pointer]:
                - generic [ref=e119]: Release E2E search fixture epsilon
                - generic [ref=e120]: planning
            - generic [ref=e122]:
              - button "Expand ticket files" [ref=e123] [cursor=pointer]: ▸
              - button "Release E2E search fixture gamma planning" [ref=e124] [cursor=pointer]:
                - generic [ref=e126]: Release E2E search fixture gamma
                - generic [ref=e127]: planning
            - generic [ref=e129]:
              - button "Expand ticket files" [ref=e130] [cursor=pointer]: ▸
              - button "Release E2E ready prerequisite ready" [ref=e131] [cursor=pointer]:
                - generic [ref=e133]: Release E2E ready prerequisite
                - generic [ref=e134]: ready
      - separator "Resize panel" [ref=e135]
    - generic [ref=e137]:
      - generic [ref=e138]: 🎫
      - generic [ref=e139]: Select a ticket from the sidebar to view details.
```

# Test source

```ts
  1  | import { test, expect } from '@playwright/test';
  2  | import { TICKET_VIEWER } from './shared/viewers';
  3  | 
  4  | interface WorkspacesResponse {
  5  |   active_workspace?: string;
  6  |   workspaces?: Array<{ name?: string }>;
  7  | }
  8  | 
  9  | async function resolveActiveWorkspace(page: Parameters<typeof test>[0]['page']): Promise<string> {
  10 |   const resp = await page.request.get(`${TICKET_VIEWER.url}/api/workspaces`);
  11 |   expect(resp.ok(), 'workspace list API must respond').toBe(true);
  12 | 
  13 |   const body = (await resp.json()) as WorkspacesResponse;
  14 |   const workspace = body.active_workspace ?? body.workspaces?.[0]?.name;
  15 |   expect(workspace, 'viewer must expose at least one workspace').toBeTruthy();
  16 |   return workspace!;
  17 | }
  18 | 
  19 | test.describe('ticket-viewer — graph fetch failure UX', () => {
  20 |   test('graph API failure shows error and retry action', async ({ page }) => {
  21 |     test.setTimeout(60_000);
  22 | 
  23 |     const workspace = await resolveActiveWorkspace(page);
  24 | 
  25 |     const listResp = await page.request.get(
  26 |       `${TICKET_VIEWER.url}/api/tickets?workspace=${workspace}&limit=1`,
  27 |     );
  28 |     expect(listResp.ok(), 'ticket list API must respond').toBe(true);
  29 |     const listBody = await listResp.json();
  30 |     const firstId = listBody?.items?.[0]?.id as string | undefined;
  31 |     expect(firstId, 'active workspace must contain at least one ticket').toBeTruthy();
  32 | 
  33 |     await page.route('**/api/graph/workspace?*', async (route) => {
  34 |       await route.fulfill({
  35 |         status: 504,
  36 |         contentType: 'application/json',
  37 |         body: JSON.stringify({ error: 'forced timeout for e2e' }),
  38 |       });
  39 |     });
  40 | 
  41 |     await page.goto(`${TICKET_VIEWER.url}/workspace/${workspace}`, {
  42 |       waitUntil: 'domcontentloaded',
  43 |     });
  44 |     await page.evaluate(
  45 |       ({ key, id }) => {
  46 |         const raw = localStorage.getItem(key);
  47 |         const obj = raw ? JSON.parse(raw) : {};
  48 |         obj.open_ticket_id = id;
  49 |         localStorage.setItem(key, JSON.stringify(obj));
  50 |       },
  51 |       { key: `ticket-viewer:${workspace}:ui`, id: firstId! },
  52 |     );
  53 |     await page.goto(`${TICKET_VIEWER.url}/workspace/${workspace}#id=${firstId}`, {
  54 |       waitUntil: 'domcontentloaded',
  55 |     });
  56 |     await page.locator(TICKET_VIEWER.readySelector).first().waitFor({
  57 |       state: 'visible',
  58 |       timeout: TICKET_VIEWER.readyTimeout,
  59 |     });
  60 | 
> 61 |     await expect(page.getByText('Failed to load graph')).toBeVisible({ timeout: 20_000 });
     |                                                          ^ Error: expect(locator).toBeVisible() failed
  62 |     await expect(page.getByRole('button', { name: 'Retry' })).toBeVisible();
  63 |     await expect(page.getByText(/Loading graph/i)).toHaveCount(0);
  64 |   });
  65 | });
```