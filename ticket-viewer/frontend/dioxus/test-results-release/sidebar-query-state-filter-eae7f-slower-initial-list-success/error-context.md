# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: sidebar-query-state-filter.spec.ts >> ticket-viewer — sidebar query + state filter >> latest filtered error wins over a slower initial list success
- Location: e2e-release\sidebar-query-state-filter.spec.ts:227:7

# Error details

```
Error: expect(received).toEqual(expected) // deep equality

- Expected  -  1
+ Received  + 14

- Array []
+ Array [
+   "f32e9df7-fead-4f34-bb29-012b5fe897fa",
+   "e7a9252f-5876-409f-85b7-3039e6c38d26",
+   "e108982f-f90f-40d4-be1d-fdc30475f3dc",
+   "cbf4d945-f288-4db3-9f8a-69d6bd4877fd",
+   "b3a60766-ec18-41c1-8430-dcd108165c5e",
+   "68e2a319-e225-4fa8-9ab9-9bb42180628f",
+   "5ea85741-084a-45f0-a1b7-bcf676440a18",
+   "4dfa31b5-c115-45f5-ae28-41471b4f18d9",
+   "3a1ec9f8-15ea-43f2-b6d3-89b88cbdcb17",
+   "0e7da84e-78af-413a-b102-6379f66fd525",
+   "0b4337f6-e6de-400c-b4b1-e8a8a808b601",
+   "070c8972-b606-4071-94bb-731cbdf13802",
+ ]

Call Log:
- Timeout 15000ms exceeded while waiting on the predicate
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
            - button "All" [ref=e39] [cursor=pointer]
            - button "planning" [ref=e40] [cursor=pointer]
            - button "ready" [ref=e41] [cursor=pointer]
            - button "impl" [ref=e42] [cursor=pointer]
            - button "review" [active] [pressed] [ref=e43] [cursor=pointer]
            - button "on-hold" [ref=e44] [cursor=pointer]
            - button "done" [ref=e45] [cursor=pointer]
            - button "cancelled" [ref=e46] [cursor=pointer]
            - button "Toggle batch selection" [ref=e47] [cursor=pointer]: ☑
            - button "Create new ticket" [ref=e48] [cursor=pointer]: + New
          - generic [ref=e49]: "Failed to load: 500 /api/tickets?workspace=e2e%2Drelease%2Dstore%2D%2D344e8f42&state=in%2Dreview&limit=200: {\"error\":\"forced filtered failure\"}"
          - generic [ref=e51]:
            - generic [ref=e53]:
              - button "Expand ticket files" [ref=e54] [cursor=pointer]: ▸
              - button "Release E2E search fixture alpha planning" [ref=e55] [cursor=pointer]:
                - generic [ref=e57]: Release E2E search fixture alpha
                - generic [ref=e58]: planning
            - generic [ref=e60]:
              - button "Expand ticket files" [ref=e61] [cursor=pointer]: ▸
              - button "Release E2E navigation fixture planning" [ref=e62] [cursor=pointer]:
                - generic [ref=e64]: Release E2E navigation fixture
                - generic [ref=e65]: planning
            - generic [ref=e67]:
              - button "Expand ticket files" [ref=e68] [cursor=pointer]: ▸
              - button "Release E2E completed prerequisite done" [ref=e69] [cursor=pointer]:
                - generic [ref=e71]: Release E2E completed prerequisite
                - generic [ref=e72]: done
            - generic [ref=e74]:
              - button "Expand ticket files" [ref=e75] [cursor=pointer]: ▸
              - button "Release E2E search fixture beta planning" [ref=e76] [cursor=pointer]:
                - generic [ref=e78]: Release E2E search fixture beta
                - generic [ref=e79]: planning
            - generic [ref=e81]:
              - button "Expand ticket files" [ref=e82] [cursor=pointer]: ▸
              - button "Release E2E graph root planning" [ref=e83] [cursor=pointer]:
                - generic [ref=e85]: Release E2E graph root
                - generic [ref=e86]: planning
            - generic [ref=e88]:
              - button "Expand ticket files" [ref=e89] [cursor=pointer]: ▸
              - button "Release E2E review prerequisite in-review" [ref=e90] [cursor=pointer]:
                - generic [ref=e92]: Release E2E review prerequisite
                - generic [ref=e93]: in-review
            - generic [ref=e95]:
              - button "Expand ticket files" [ref=e96] [cursor=pointer]: ▸
              - button "Release E2E search fixture delta planning" [ref=e97] [cursor=pointer]:
                - generic [ref=e99]: Release E2E search fixture delta
                - generic [ref=e100]: planning
            - generic [ref=e102]:
              - button "Expand ticket files" [ref=e103] [cursor=pointer]: ▸
              - button "Release E2E implementation prerequisite in-implementation" [ref=e104] [cursor=pointer]:
                - generic [ref=e106]: Release E2E implementation prerequisite
                - generic [ref=e107]: in-implementation
            - generic [ref=e109]:
              - button "Expand ticket files" [ref=e110] [cursor=pointer]: ▸
              - button "Release E2E legacy description fixture planning" [ref=e111] [cursor=pointer]:
                - generic [ref=e113]: Release E2E legacy description fixture
                - generic [ref=e114]: planning
            - generic [ref=e116]:
              - button "Expand ticket files" [ref=e117] [cursor=pointer]: ▸
              - button "Release E2E search fixture epsilon planning" [ref=e118] [cursor=pointer]:
                - generic [ref=e120]: Release E2E search fixture epsilon
                - generic [ref=e121]: planning
            - generic [ref=e123]:
              - button "Expand ticket files" [ref=e124] [cursor=pointer]: ▸
              - button "Release E2E search fixture gamma planning" [ref=e125] [cursor=pointer]:
                - generic [ref=e127]: Release E2E search fixture gamma
                - generic [ref=e128]: planning
            - generic [ref=e130]:
              - button "Expand ticket files" [ref=e131] [cursor=pointer]: ▸
              - button "Release E2E ready prerequisite ready" [ref=e132] [cursor=pointer]:
                - generic [ref=e134]: Release E2E ready prerequisite
                - generic [ref=e135]: ready
      - separator "Resize panel" [ref=e136]
    - generic [ref=e138]:
      - generic [ref=e139]: 🎫
      - generic [ref=e140]: Select a ticket from the sidebar to view details.
```

# Test source

```ts
  205 |         return false;
  206 |       }
  207 | 
  208 |       const url = new URL(response.url());
  209 |       return (
  210 |         url.pathname === '/api/tickets' &&
  211 |         url.searchParams.get('workspace') === activeWorkspace &&
  212 |         url.searchParams.get('state') === 'in-review'
  213 |       );
  214 |     });
  215 | 
  216 |     const reviewChip = page.getByTestId('ticket-tree-state-chip-in-review');
  217 |     await expect(reviewChip).toBeVisible();
  218 |     await reviewChip.click();
  219 |     await filteredResponsePromise;
  220 | 
  221 |     await expect.poll(async () => {
  222 |       const ids = await visibleTicketIds(page);
  223 |       return ids.sort();
  224 |     }).toEqual(expectedIds);
  225 |   });
  226 | 
  227 |   test('latest filtered error wins over a slower initial list success', async ({ page }) => {
  228 |     test.setTimeout(120_000);
  229 | 
  230 |     const activeWorkspace = await resolveActiveWorkspace(page);
  231 |     let delayedInitialList = false;
  232 |     const initialListResponsePromise = page.waitForResponse((response) => {
  233 |       if (!response.ok()) {
  234 |         return false;
  235 |       }
  236 | 
  237 |       const url = new URL(response.url());
  238 |       return (
  239 |         url.pathname === '/api/tickets' &&
  240 |         url.searchParams.get('workspace') === activeWorkspace &&
  241 |         !url.searchParams.has('query') &&
  242 |         !url.searchParams.has('state')
  243 |       );
  244 |     });
  245 | 
  246 |     await page.route('**/api/tickets?**', async (route) => {
  247 |       const url = new URL(route.request().url());
  248 |       const isInitialUnfilteredList =
  249 |         !delayedInitialList &&
  250 |         url.pathname === '/api/tickets' &&
  251 |         url.searchParams.get('workspace') === activeWorkspace &&
  252 |         !url.searchParams.has('query') &&
  253 |         !url.searchParams.has('state');
  254 | 
  255 |       if (isInitialUnfilteredList) {
  256 |         delayedInitialList = true;
  257 |         await new Promise((resolve) => setTimeout(resolve, 1500));
  258 |         await route.continue();
  259 |         return;
  260 |       }
  261 | 
  262 |       const isFilteredReviewList =
  263 |         url.pathname === '/api/tickets' &&
  264 |         url.searchParams.get('workspace') === activeWorkspace &&
  265 |         url.searchParams.get('state') === 'in-review';
  266 | 
  267 |       if (isFilteredReviewList) {
  268 |         await route.fulfill({
  269 |           status: 500,
  270 |           contentType: 'application/json',
  271 |           body: JSON.stringify({ error: 'forced filtered failure' }),
  272 |         });
  273 |         return;
  274 |       }
  275 | 
  276 |       await route.continue();
  277 |     });
  278 | 
  279 |     await page.goto(`${TICKET_VIEWER_URL}/`, {
  280 |       waitUntil: 'domcontentloaded',
  281 |     });
  282 |     await page.locator(TICKET_VIEWER.readySelector).first().waitFor({
  283 |       state: 'visible',
  284 |       timeout: TICKET_VIEWER.readyTimeout,
  285 |     });
  286 | 
  287 |     const filteredErrorPromise = page.waitForResponse((response) => {
  288 |       const url = new URL(response.url());
  289 |       return (
  290 |         response.status() === 500 &&
  291 |         url.pathname === '/api/tickets' &&
  292 |         url.searchParams.get('workspace') === activeWorkspace &&
  293 |         url.searchParams.get('state') === 'in-review'
  294 |       );
  295 |     });
  296 | 
  297 |     const reviewChip = page.getByTestId('ticket-tree-state-chip-in-review');
  298 |     await expect(reviewChip).toBeVisible();
  299 |     await reviewChip.click();
  300 | 
  301 |     await filteredErrorPromise;
  302 |     await initialListResponsePromise;
  303 | 
  304 |     await expect(page.getByText(/Failed to load:/)).toContainText('500');
> 305 |     await expect.poll(async () => await visibleTicketIds(page)).toEqual([]);
      |     ^ Error: expect(received).toEqual(expected) // deep equality
  306 |   });
  307 | 
  308 |   test('latest filtered success wins over a slower initial list error', async ({ page }) => {
  309 |     test.setTimeout(120_000);
  310 | 
  311 |     const activeWorkspace = await resolveActiveWorkspace(page);
  312 |     const expectedItems = await fetchTickets(page, activeWorkspace, undefined, 'in-review');
  313 |     const expectedIds = expectedItems.map((item) => item.id).sort();
  314 | 
  315 |     let delayedInitialList = false;
  316 |     const initialListErrorPromise = page.waitForResponse((response) => {
  317 |       const url = new URL(response.url());
  318 |       return (
  319 |         response.status() === 500 &&
  320 |         url.pathname === '/api/tickets' &&
  321 |         url.searchParams.get('workspace') === activeWorkspace &&
  322 |         !url.searchParams.has('query') &&
  323 |         !url.searchParams.has('state')
  324 |       );
  325 |     });
  326 | 
  327 |     await page.route('**/api/tickets?**', async (route) => {
  328 |       const url = new URL(route.request().url());
  329 |       const isInitialUnfilteredList =
  330 |         !delayedInitialList &&
  331 |         url.pathname === '/api/tickets' &&
  332 |         url.searchParams.get('workspace') === activeWorkspace &&
  333 |         !url.searchParams.has('query') &&
  334 |         !url.searchParams.has('state');
  335 | 
  336 |       if (isInitialUnfilteredList) {
  337 |         delayedInitialList = true;
  338 |         await new Promise((resolve) => setTimeout(resolve, 1500));
  339 |         await route.fulfill({
  340 |           status: 500,
  341 |           contentType: 'application/json',
  342 |           body: JSON.stringify({ error: 'forced initial failure' }),
  343 |         });
  344 |         return;
  345 |       }
  346 | 
  347 |       await route.continue();
  348 |     });
  349 | 
  350 |     await page.goto(`${TICKET_VIEWER_URL}/`, {
  351 |       waitUntil: 'domcontentloaded',
  352 |     });
  353 |     await page.locator(TICKET_VIEWER.readySelector).first().waitFor({
  354 |       state: 'visible',
  355 |       timeout: TICKET_VIEWER.readyTimeout,
  356 |     });
  357 | 
  358 |     const filteredResponsePromise = page.waitForResponse((response) => {
  359 |       if (!response.ok()) {
  360 |         return false;
  361 |       }
  362 | 
  363 |       const url = new URL(response.url());
  364 |       return (
  365 |         url.pathname === '/api/tickets' &&
  366 |         url.searchParams.get('workspace') === activeWorkspace &&
  367 |         url.searchParams.get('state') === 'in-review'
  368 |       );
  369 |     });
  370 | 
  371 |     const reviewChip = page.getByTestId('ticket-tree-state-chip-in-review');
  372 |     await expect(reviewChip).toBeVisible();
  373 |     await reviewChip.click();
  374 | 
  375 |     await filteredResponsePromise;
  376 |     await initialListErrorPromise;
  377 | 
  378 |     await expect(page.getByText(/Failed to load:/)).toHaveCount(0);
  379 |     await expect.poll(async () => {
  380 |       const ids = await visibleTicketIds(page);
  381 |       return ids.sort();
  382 |     }).toEqual(expectedIds);
  383 |   });
  384 | });
```