import { expect, test, type Page } from '@playwright/test';
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

interface TicketListItem {
  id: string;
}

interface GraphEdge {
  source?: string;
  target?: string;
  from?: string;
  to?: string;
}

async function findGraphRootId(page: Page, workspace: string): Promise<string> {
  const listResp = await page.request.get(
    `${TICKET_VIEWER.url}/api/tickets?workspace=${workspace}&limit=500`,
  );
  expect(listResp.ok(), 'ticket list API must respond').toBe(true);

  const listBody = await listResp.json();
  const items: TicketListItem[] = listBody?.items ?? [];
  expect(items.length, 'workspace `' + workspace + '` must contain tickets').toBeGreaterThan(0);

  for (const item of items) {
    const graphResp = await page.request.get(
      `${TICKET_VIEWER.url}/api/graph/subgraph?workspace=${workspace}&root=${item.id}&depth=1`,
    );
    if (!graphResp.ok()) {
      continue;
    }

    const graph = await graphResp.json();
    const edges: GraphEdge[] = graph?.edges ?? [];
    if (edges.some((edge) => (edge.from ?? edge.source) === item.id)) {
      return item.id;
    }
  }

  throw new Error('No ticket with at least one outgoing graph edge was found');
}

test.describe('ticket-viewer — graph edge overlay renders as one filled shape', () => {
  test('graph edges use filled SVG paths instead of line markers', async ({ page }) => {
    test.setTimeout(120_000);

    const workspace = await resolveActiveWorkspace();
    const rootId = await findGraphRootId(page, workspace);

    await page.goto(`${TICKET_VIEWER.url}/workspace/${workspace}`, {
      waitUntil: 'domcontentloaded',
    });
    await page.locator(TICKET_VIEWER.readySelector).first().waitFor({
      state: 'visible',
      timeout: TICKET_VIEWER.readyTimeout,
    });

    await page.evaluate(
      ({ key, id }) => {
        const raw = localStorage.getItem(key);
        const obj = raw ? JSON.parse(raw) : {};
        obj.open_ticket_id = id;
        localStorage.setItem(key, JSON.stringify(obj));
      },
      { key: `ticket-viewer:${workspace}:ui`, id: rootId },
    );

    await page.goto(`${TICKET_VIEWER.url}/workspace/${workspace}#id=${rootId}`, {
      waitUntil: 'domcontentloaded',
    });
    await page.locator(TICKET_VIEWER.readySelector).first().waitFor({
      state: 'visible',
      timeout: TICKET_VIEWER.readyTimeout,
    });
    await expect(page.locator('#graph3d-container')).toBeAttached({ timeout: 30_000 });
    await expect(page.locator('#graph3d-container [data-edge-idx]').first()).toBeAttached({
      timeout: 30_000,
    });
    await page.waitForFunction(() => {
      const edges = Array.from(
        document.querySelectorAll('#graph3d-container [data-edge-idx]'),
      );
      return edges.some((edge) => {
        const fill = edge.getAttribute('fill');
        return typeof fill === 'string' && fill.includes('rgb(');
      });
    });

    const edges = await page
      .locator('#graph3d-container [data-edge-idx]')
      .evaluateAll((nodes) =>
        nodes.map((node) => ({
          tag: node.tagName.toLowerCase(),
          display: node.getAttribute('display'),
          fill: node.getAttribute('fill'),
          stroke: node.getAttribute('stroke'),
          markerEnd: node.getAttribute('marker-end'),
        })),
      );

    const renderedEdges = edges.filter(
      (edge) => edge.display !== 'none' && typeof edge.fill === 'string' && edge.fill.includes('rgb('),
    );

    expect(renderedEdges.length, 'graph overlay must expose at least one rendered edge').toBeGreaterThan(0);
    expect(renderedEdges.every((edge) => edge.tag === 'path'), 'edges should render as SVG paths').toBe(true);
    expect(renderedEdges.every((edge) => edge.markerEnd === null), 'edges must not use marker-end overlays').toBe(true);
    expect(
      renderedEdges.every((edge) => edge.stroke === 'none'),
      'edges should not keep an SVG stroke once rendered as filled shapes',
    ).toBe(true);
    expect(
      renderedEdges.every((edge) => typeof edge.fill === 'string' && edge.fill.includes('rgb(')),
      'edges should carry their color via fill on the single SVG shape',
    ).toBe(true);
  });
});