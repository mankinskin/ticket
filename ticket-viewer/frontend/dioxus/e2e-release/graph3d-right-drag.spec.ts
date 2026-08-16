import { test, expect } from '@playwright/test';
import { resolveActiveWorkspace, TICKET_VIEWER } from './shared/viewers';

test.use({
  headless: false,
  launchOptions: {
    args: [
      '--enable-unsafe-webgpu',
      '--enable-features=Vulkan',
      '--use-vulkan=swiftshader',
      '--use-webgpu-adapter=swiftshader',
    ],
  },
});

test.describe('ticket-viewer — graph3d right-drag does not open context menu', () => {
  test('right-drag on graph suppresses contextmenu; plain right-click does not', async ({ page }) => {
    test.setTimeout(120_000);

    const consoleErrors: string[] = [];
    page.on('pageerror', (err) => {
      consoleErrors.push(`pageerror: ${err.message}`);
    });
    page.on('console', (msg) => {
      if (msg.type() !== 'error') {
        return;
      }
      const text = msg.text();
      if (/Failed to load resource.*404/.test(text)) return;
      consoleErrors.push(`console.error: ${text}`);
    });

    const workspace = await resolveActiveWorkspace();
    const listResp = await page.request.get(
      `${TICKET_VIEWER.url}/api/tickets?workspace=${workspace}&limit=500`,
    );
    expect(listResp.ok(), 'ticket list API must respond').toBe(true);
    const listBody = await listResp.json();
    const ids: string[] = (listBody?.items ?? []).map((it: { id: string }) => it.id);
    expect(ids.length, 'workspace `' + workspace + '` must contain tickets').toBeGreaterThan(0);

    let firstId: string | undefined;
    for (const id of ids) {
      const sgResp = await page.request.get(
        `${TICKET_VIEWER.url}/api/graph/subgraph?workspace=${workspace}&root=${id}&depth=2`,
      );
      if (!sgResp.ok()) continue;
      const sg = await sgResp.json();
      if ((sg?.edges?.length ?? 0) >= 1) {
        firstId = id;
        break;
      }
    }
    expect(firstId, 'no ticket with dependencies found in workspace').toBeTruthy();

    await page.goto(`${TICKET_VIEWER.url}/workspace/${workspace}`, {
      waitUntil: 'domcontentloaded',
    });
    await page.evaluate(
      ({ key, id }) => {
        const raw = localStorage.getItem(key);
        const obj = raw ? JSON.parse(raw) : {};
        obj.open_ticket_id = id;
        localStorage.setItem(key, JSON.stringify(obj));
      },
      { key: `ticket-viewer:${workspace}:ui`, id: firstId! },
    );
    await page.goto(`${TICKET_VIEWER.url}/workspace/${workspace}#id=${firstId}`, {
      waitUntil: 'domcontentloaded',
    });
    await page.locator(TICKET_VIEWER.readySelector).first().waitFor({
      state: 'visible',
      timeout: TICKET_VIEWER.readyTimeout,
    });

    const graphContainer = page.locator('#graph3d-nodes');
    await expect(graphContainer).toBeAttached({ timeout: 30_000 });

    const graphRoot = page.locator('#graph3d-container');
    await expect(graphRoot).toBeAttached({ timeout: 30_000 });
    await page.waitForFunction(() => {
      const el = document.getElementById('graph3d-container');
      if (!el) return false;
      const cards = el.querySelectorAll('[data-node-idx]');
      for (const card of Array.from(cards)) {
        if ((card as HTMLElement).style.display !== 'none') return true;
      }
      return false;
    }, null, { timeout: 30_000 });
    await page.waitForTimeout(1_000);

    await page.evaluate(() => {
      // @ts-expect-error attach test-only state to window
      window.__lastContextMenu = { fired: false, prevented: false };
      // @ts-expect-error attach test-only debug
      window.__rmbDebug = { mousedowns: 0, mousemoves: 0, mouseups: 0, cmCapture: 0, cmBubble: 0, cmTarget: '' };
      const dbg = (window as any).__rmbDebug;
      document.addEventListener('mousedown', (e) => {
        if ((e as MouseEvent).button === 2) dbg.mousedowns++;
      }, true);
      document.addEventListener('mousemove', (e) => {
        if ((e as MouseEvent).buttons & 2) dbg.mousemoves++;
      }, true);
      document.addEventListener('mouseup', (e) => {
        if ((e as MouseEvent).button === 2) dbg.mouseups++;
      }, true);
      document.addEventListener(
        'contextmenu',
        (evt) => {
          dbg.cmCapture++;
          dbg.cmTarget = (evt.target as HTMLElement)?.id || (evt.target as HTMLElement)?.tagName || '?';
        },
        true,
      );
      document.addEventListener(
        'contextmenu',
        (evt) => {
          dbg.cmBubble++;
          // @ts-expect-error
          window.__lastContextMenu = { fired: true, prevented: evt.defaultPrevented };
        },
        false,
      );
    });

    const container = page.locator('#graph3d-container');
    await expect(container).toBeVisible();
    const box = await container.boundingBox();
    expect(box).not.toBeNull();
    const cx = box!.x + box!.width / 2;
    const cy = box!.y + box!.height / 2;

    await page.evaluate(() => {
      // @ts-expect-error reset before the gesture
      window.__lastContextMenu = { fired: false, prevented: false };
    });

    await page.mouse.move(cx, cy);
    await page.mouse.down({ button: 'right' });
    await page.mouse.move(cx + 30, cy + 10, { steps: 5 });
    await page.mouse.move(cx + 60, cy + 25, { steps: 5 });
    await page.mouse.up({ button: 'right' });
    await page.waitForTimeout(150);

    const afterDrag = await page.evaluate(() => {
      // @ts-expect-error
      const lc = window.__lastContextMenu as { fired: boolean; prevented: boolean };
      // @ts-expect-error
      const dbg = window.__rmbDebug as { mousedowns: number; mousemoves: number; mouseups: number };
      return { ...lc, ...dbg };
    });

    expect(
      afterDrag.fired,
      'contextmenu event should still fire after right-drag (so we can check prevention)',
    ).toBe(true);
    expect(
      afterDrag.prevented,
      'right-drag on graph must call preventDefault() on the contextmenu event',
    ).toBe(true);

    // Let the graph interaction cleanup settle before asserting a fresh plain
    // right-click; otherwise the prior drag can occasionally leak into the
    // next gesture on slower runs.
    await page.mouse.move(cx - 24, cy - 16, { steps: 3 });
    await page.waitForTimeout(200);

    await page.evaluate(() => {
      // @ts-expect-error
      window.__lastContextMenu = { fired: false, prevented: false };
    });

    await page.mouse.move(cx, cy, { steps: 2 });
    await page.mouse.down({ button: 'right' });
    await page.mouse.up({ button: 'right' });
    await page.waitForTimeout(150);

    const afterClick = await page.evaluate(() => {
      // @ts-expect-error
      return window.__lastContextMenu as { fired: boolean; prevented: boolean };
    });

    expect(afterClick.fired, 'plain right-click should still trigger contextmenu').toBe(true);
    expect(
      afterClick.prevented,
      'plain right-click (no drag) must NOT be suppressed by the graph view',
    ).toBe(false);

    expect(consoleErrors, 'graph3d right-drag interaction produced JS errors').toEqual([]);
  });
});