# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: graph3d-right-drag.spec.ts >> ticket-viewer — graph3d right-drag does not open context menu >> right-drag on graph suppresses contextmenu; plain right-click does not
- Location: e2e-release\graph3d-right-drag.spec.ts:17:7

# Error details

```
Error: expect(locator).toBeAttached() failed

Locator: locator('#graph3d-nodes')
Expected: attached
Timeout: 30000ms
Error: element(s) not found

Call log:
  - Expect "toBeAttached" with timeout 30000ms
  - waiting for locator('#graph3d-nodes')

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
        - generic [ref=e22]: "13"
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
              - button "E2E structured-parts fixture ready" [ref=e54] [cursor=pointer]:
                - generic [ref=e56]: E2E structured-parts fixture
                - generic [ref=e57]: ready
            - generic [ref=e59]:
              - button "Expand ticket files" [ref=e60] [cursor=pointer]: ▸
              - button "Release E2E search fixture alpha planning" [ref=e61] [cursor=pointer]:
                - generic [ref=e63]: Release E2E search fixture alpha
                - generic [ref=e64]: planning
            - generic [ref=e66]:
              - button "Expand ticket files" [ref=e67] [cursor=pointer]: ▸
              - button "Release E2E navigation fixture planning" [ref=e68] [cursor=pointer]:
                - generic [ref=e70]: Release E2E navigation fixture
                - generic [ref=e71]: planning
            - generic [ref=e73]:
              - button "Expand ticket files" [ref=e74] [cursor=pointer]: ▸
              - button "Release E2E completed prerequisite done" [ref=e75] [cursor=pointer]:
                - generic [ref=e77]: Release E2E completed prerequisite
                - generic [ref=e78]: done
            - generic [ref=e80]:
              - button "Expand ticket files" [ref=e81] [cursor=pointer]: ▸
              - button "Release E2E search fixture beta planning" [ref=e82] [cursor=pointer]:
                - generic [ref=e84]: Release E2E search fixture beta
                - generic [ref=e85]: planning
            - generic [ref=e87]:
              - button "Expand ticket files" [ref=e88] [cursor=pointer]: ▸
              - button "Release E2E graph root planning" [ref=e89] [cursor=pointer]:
                - generic [ref=e91]: Release E2E graph root
                - generic [ref=e92]: planning
            - generic [ref=e94]:
              - button "Expand ticket files" [ref=e95] [cursor=pointer]: ▸
              - button "Release E2E review prerequisite in-review" [ref=e96] [cursor=pointer]:
                - generic [ref=e98]: Release E2E review prerequisite
                - generic [ref=e99]: in-review
            - generic [ref=e101]:
              - button "Expand ticket files" [ref=e102] [cursor=pointer]: ▸
              - button "Release E2E search fixture delta planning" [ref=e103] [cursor=pointer]:
                - generic [ref=e105]: Release E2E search fixture delta
                - generic [ref=e106]: planning
            - generic [ref=e108]:
              - button "Expand ticket files" [ref=e109] [cursor=pointer]: ▸
              - button "Release E2E implementation prerequisite in-implementation" [ref=e110] [cursor=pointer]:
                - generic [ref=e112]: Release E2E implementation prerequisite
                - generic [ref=e113]: in-implementation
            - generic [ref=e115]:
              - button "Expand ticket files" [ref=e116] [cursor=pointer]: ▸
              - button "Release E2E legacy description fixture planning" [ref=e117] [cursor=pointer]:
                - generic [ref=e119]: Release E2E legacy description fixture
                - generic [ref=e120]: planning
            - generic [ref=e122]:
              - button "Expand ticket files" [ref=e123] [cursor=pointer]: ▸
              - button "Release E2E search fixture epsilon planning" [ref=e124] [cursor=pointer]:
                - generic [ref=e126]: Release E2E search fixture epsilon
                - generic [ref=e127]: planning
            - generic [ref=e129]:
              - button "Expand ticket files" [ref=e130] [cursor=pointer]: ▸
              - button "Release E2E search fixture gamma planning" [ref=e131] [cursor=pointer]:
                - generic [ref=e133]: Release E2E search fixture gamma
                - generic [ref=e134]: planning
            - generic [ref=e136]:
              - button "Expand ticket files" [ref=e137] [cursor=pointer]: ▸
              - button "Release E2E ready prerequisite ready" [ref=e138] [cursor=pointer]:
                - generic [ref=e140]: Release E2E ready prerequisite
                - generic [ref=e141]: ready
      - separator "Resize panel" [ref=e142]
    - generic [ref=e144]:
      - generic [ref=e145]: 🎫
      - generic [ref=e146]: Select a ticket from the sidebar to view details.
```

# Test source

```ts
  1   | import { test, expect } from '@playwright/test';
  2   | import { resolveActiveWorkspace, TICKET_VIEWER } from './shared/viewers';
  3   | 
  4   | test.use({
  5   |   headless: false,
  6   |   launchOptions: {
  7   |     args: [
  8   |       '--enable-unsafe-webgpu',
  9   |       '--enable-features=Vulkan',
  10  |       '--use-vulkan=swiftshader',
  11  |       '--use-webgpu-adapter=swiftshader',
  12  |     ],
  13  |   },
  14  | });
  15  | 
  16  | test.describe('ticket-viewer — graph3d right-drag does not open context menu', () => {
  17  |   test('right-drag on graph suppresses contextmenu; plain right-click does not', async ({ page }) => {
  18  |     test.setTimeout(120_000);
  19  | 
  20  |     const consoleErrors: string[] = [];
  21  |     page.on('pageerror', (err) => {
  22  |       consoleErrors.push(`pageerror: ${err.message}`);
  23  |     });
  24  |     page.on('console', (msg) => {
  25  |       if (msg.type() !== 'error') {
  26  |         return;
  27  |       }
  28  |       const text = msg.text();
  29  |       if (/Failed to load resource.*404/.test(text)) return;
  30  |       consoleErrors.push(`console.error: ${text}`);
  31  |     });
  32  | 
  33  |     const workspace = await resolveActiveWorkspace();
  34  |     const listResp = await page.request.get(
  35  |       `${TICKET_VIEWER.url}/api/tickets?workspace=${workspace}&limit=500`,
  36  |     );
  37  |     expect(listResp.ok(), 'ticket list API must respond').toBe(true);
  38  |     const listBody = await listResp.json();
  39  |     const ids: string[] = (listBody?.items ?? []).map((it: { id: string }) => it.id);
  40  |     expect(ids.length, 'workspace `' + workspace + '` must contain tickets').toBeGreaterThan(0);
  41  | 
  42  |     let firstId: string | undefined;
  43  |     for (const id of ids) {
  44  |       const sgResp = await page.request.get(
  45  |         `${TICKET_VIEWER.url}/api/graph/subgraph?workspace=${workspace}&root=${id}&depth=2`,
  46  |       );
  47  |       if (!sgResp.ok()) continue;
  48  |       const sg = await sgResp.json();
  49  |       if ((sg?.edges?.length ?? 0) >= 1) {
  50  |         firstId = id;
  51  |         break;
  52  |       }
  53  |     }
  54  |     expect(firstId, 'no ticket with dependencies found in workspace').toBeTruthy();
  55  | 
  56  |     await page.goto(`${TICKET_VIEWER.url}/workspace/${workspace}`, {
  57  |       waitUntil: 'domcontentloaded',
  58  |     });
  59  |     await page.evaluate(
  60  |       ({ key, id }) => {
  61  |         const raw = localStorage.getItem(key);
  62  |         const obj = raw ? JSON.parse(raw) : {};
  63  |         obj.open_ticket_id = id;
  64  |         localStorage.setItem(key, JSON.stringify(obj));
  65  |       },
  66  |       { key: `ticket-viewer:${workspace}:ui`, id: firstId! },
  67  |     );
  68  |     await page.goto(`${TICKET_VIEWER.url}/workspace/${workspace}#id=${firstId}`, {
  69  |       waitUntil: 'domcontentloaded',
  70  |     });
  71  |     await page.locator(TICKET_VIEWER.readySelector).first().waitFor({
  72  |       state: 'visible',
  73  |       timeout: TICKET_VIEWER.readyTimeout,
  74  |     });
  75  | 
  76  |     const graphContainer = page.locator('#graph3d-nodes');
> 77  |     await expect(graphContainer).toBeAttached({ timeout: 30_000 });
      |                                  ^ Error: expect(locator).toBeAttached() failed
  78  | 
  79  |     const graphRoot = page.locator('#graph3d-container');
  80  |     await expect(graphRoot).toBeAttached({ timeout: 30_000 });
  81  |     await page.waitForFunction(() => {
  82  |       const el = document.getElementById('graph3d-container');
  83  |       if (!el) return false;
  84  |       const cards = el.querySelectorAll('[data-node-idx]');
  85  |       for (const card of Array.from(cards)) {
  86  |         if ((card as HTMLElement).style.display !== 'none') return true;
  87  |       }
  88  |       return false;
  89  |     }, null, { timeout: 30_000 });
  90  |     await page.waitForTimeout(1_000);
  91  | 
  92  |     await page.evaluate(() => {
  93  |       // @ts-expect-error attach test-only state to window
  94  |       window.__lastContextMenu = { fired: false, prevented: false };
  95  |       // @ts-expect-error attach test-only debug
  96  |       window.__rmbDebug = { mousedowns: 0, mousemoves: 0, mouseups: 0, cmCapture: 0, cmBubble: 0, cmTarget: '' };
  97  |       const dbg = (window as any).__rmbDebug;
  98  |       document.addEventListener('mousedown', (e) => {
  99  |         if ((e as MouseEvent).button === 2) dbg.mousedowns++;
  100 |       }, true);
  101 |       document.addEventListener('mousemove', (e) => {
  102 |         if ((e as MouseEvent).buttons & 2) dbg.mousemoves++;
  103 |       }, true);
  104 |       document.addEventListener('mouseup', (e) => {
  105 |         if ((e as MouseEvent).button === 2) dbg.mouseups++;
  106 |       }, true);
  107 |       document.addEventListener(
  108 |         'contextmenu',
  109 |         (evt) => {
  110 |           dbg.cmCapture++;
  111 |           dbg.cmTarget = (evt.target as HTMLElement)?.id || (evt.target as HTMLElement)?.tagName || '?';
  112 |         },
  113 |         true,
  114 |       );
  115 |       document.addEventListener(
  116 |         'contextmenu',
  117 |         (evt) => {
  118 |           dbg.cmBubble++;
  119 |           // @ts-expect-error
  120 |           window.__lastContextMenu = { fired: true, prevented: evt.defaultPrevented };
  121 |         },
  122 |         false,
  123 |       );
  124 |     });
  125 | 
  126 |     const container = page.locator('#graph3d-container');
  127 |     await expect(container).toBeVisible();
  128 |     const box = await container.boundingBox();
  129 |     expect(box).not.toBeNull();
  130 |     const cx = box!.x + box!.width / 2;
  131 |     const cy = box!.y + box!.height / 2;
  132 | 
  133 |     await page.evaluate(() => {
  134 |       // @ts-expect-error reset before the gesture
  135 |       window.__lastContextMenu = { fired: false, prevented: false };
  136 |     });
  137 | 
  138 |     await page.mouse.move(cx, cy);
  139 |     await page.mouse.down({ button: 'right' });
  140 |     await page.mouse.move(cx + 30, cy + 10, { steps: 5 });
  141 |     await page.mouse.move(cx + 60, cy + 25, { steps: 5 });
  142 |     await page.mouse.up({ button: 'right' });
  143 |     await page.waitForTimeout(150);
  144 | 
  145 |     const afterDrag = await page.evaluate(() => {
  146 |       // @ts-expect-error
  147 |       const lc = window.__lastContextMenu as { fired: boolean; prevented: boolean };
  148 |       // @ts-expect-error
  149 |       const dbg = window.__rmbDebug as { mousedowns: number; mousemoves: number; mouseups: number };
  150 |       return { ...lc, ...dbg };
  151 |     });
  152 | 
  153 |     expect(
  154 |       afterDrag.fired,
  155 |       'contextmenu event should still fire after right-drag (so we can check prevention)',
  156 |     ).toBe(true);
  157 |     expect(
  158 |       afterDrag.prevented,
  159 |       'right-drag on graph must call preventDefault() on the contextmenu event',
  160 |     ).toBe(true);
  161 | 
  162 |     // Let the graph interaction cleanup settle before asserting a fresh plain
  163 |     // right-click; otherwise the prior drag can occasionally leak into the
  164 |     // next gesture on slower runs.
  165 |     await page.mouse.move(cx - 24, cy - 16, { steps: 3 });
  166 |     await page.waitForTimeout(200);
  167 | 
  168 |     await page.evaluate(() => {
  169 |       // @ts-expect-error
  170 |       window.__lastContextMenu = { fired: false, prevented: false };
  171 |     });
  172 | 
  173 |     await page.mouse.move(cx, cy, { steps: 2 });
  174 |     await page.mouse.down({ button: 'right' });
  175 |     await page.mouse.up({ button: 'right' });
  176 |     await page.waitForTimeout(150);
  177 | 
```