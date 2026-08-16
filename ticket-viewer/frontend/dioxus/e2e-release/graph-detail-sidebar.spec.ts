import { expect, test, type Page, type TestInfo } from '@playwright/test';
import {
  gotoAndWaitForViewer,
  TICKET_VIEWER,
} from './shared/viewers';

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
  title?: string;
  ticket_ref?: {
    workspace?: string;
    id?: string;
  };
}

interface GraphNode {
  id: string;
  title?: string;
  state?: string;
}

interface GraphEdge {
  source?: string;
  target?: string;
  from?: string;
  to?: string;
}

interface CandidateSelection {
  workspace: string;
  rootId: string;
  rootTitle: string;
  childId: string;
  childTitle: string;
  childComponent: string;
}

interface KanbanLayoutCandidate {
  workspace: string;
  rootId: string;
  stateSamples: Array<{
    id: string;
    state: string;
  }>;
}

interface GraphNodeMetrics {
  x: number;
  y: number;
  width: number;
  height: number;
  centerX: number;
  centerY: number;
  lod: string | null;
  layoutX: number | null;
  layoutY: number | null;
  layoutZ: number | null;
}

interface WorkspacesResponse {
  active_workspace?: string;
  workspaces?: Array<{ name?: string }>;
}

const GRAPH_ACTIVE_STYLE = /#93bbff|79,140,255/;

async function attachScreenshot(
  page: Page,
  testInfo: TestInfo,
  name: string,
): Promise<void> {
  await testInfo.attach(name, {
    body: await page.screenshot({ fullPage: true }),
    contentType: 'image/png',
  });
}

async function openCandidateTicket(
  page: Page,
  candidate: CandidateSelection,
): Promise<void> {
  await openTicketById(page, candidate.workspace, candidate.rootId);
}

async function openTicketById(
  page: Page,
  workspace: string,
  rootId: string,
): Promise<void> {
  await page.goto(`${TICKET_VIEWER.url}/workspace/${workspace}`, {
    waitUntil: 'domcontentloaded',
  });
  await page.locator(TICKET_VIEWER.readySelector).first().waitFor({
    state: 'visible',
    timeout: TICKET_VIEWER.readyTimeout,
  });

  const searchBox = page.getByRole('textbox').first();
  await searchBox.fill(`id:${rootId}`);

  const rootTicketButton = page.getByTestId(`ticket-tree-ticket-${rootId}`);
  await expect(rootTicketButton).toBeVisible({ timeout: 30_000 });
  await rootTicketButton.click();
}

function normalizeTicketState(state?: string | null): string {
  const normalized = state?.trim().toLowerCase();
  return normalized && normalized.length > 0 ? normalized : 'new';
}

function kanbanStateRank(state: string): number {
  switch (state) {
    case 'new':
      return 0;
    case 'ready':
      return 1;
    case 'blocked':
      return 2;
    case 'in-implementation':
      return 3;
    case 'in-review':
      return 4;
    case 'done':
      return 5;
    case 'cancelled':
    case 'canceled':
      return 6;
    default:
      return 7;
  }
}

async function graphLodMetrics(
  page: Page,
  selectedId: string,
): Promise<null | {
  selectedLod: string | null;
  visibleNodeCount: number;
  collapsedNodes: number;
  compactNodes: number;
  minimalNodes: number;
}> {
  return page.evaluate((activeNodeId) => {
    const container = document.getElementById('graph3d-container');
    if (!container) {
      return null;
    }

    const visibleNodes = Array.from(container.querySelectorAll('[data-node-id]')).filter((node) => {
      const element = node as HTMLElement;
      return element.style.display !== 'none';
    }) as HTMLElement[];
    const selectedNode = visibleNodes.find((node) => node.dataset.nodeId === activeNodeId);
    if (!selectedNode) {
      return null;
    }

    let compactNodes = 0;
    let minimalNodes = 0;
    let collapsedNodes = 0;
    for (const node of visibleNodes) {
      const lod = node.getAttribute('data-node-lod');
      if (lod === 'compact') {
        compactNodes += 1;
      }
      if (lod === 'minimal') {
        minimalNodes += 1;
      }
      if (node.dataset.nodeId !== activeNodeId && (lod === 'compact' || lod === 'minimal')) {
        collapsedNodes += 1;
      }
    }

    return {
      selectedLod: selectedNode.getAttribute('data-node-lod'),
      visibleNodeCount: visibleNodes.length,
      collapsedNodes,
      compactNodes,
      minimalNodes,
    };
  }, selectedId);
}

async function graphHierarchyMetrics(
  page: Page,
  rootId: string,
  childId: string,
): Promise<null | {
  visibleNodeCount: number;
  rootX: number;
  rootY: number;
  childX: number;
  childY: number;
}> {
  return page.evaluate(({ activeRootId, activeChildId }) => {
    const container = document.getElementById('graph3d-container');
    if (!container) {
      return null;
    }

    const visibleNodes = Array.from(container.querySelectorAll('[data-node-id]')).filter((node) => {
      const element = node as HTMLElement;
      return element.style.display !== 'none';
    }) as HTMLElement[];
    const rootNode = visibleNodes.find((node) => node.dataset.nodeId === activeRootId);
    const childNode = visibleNodes.find((node) => node.dataset.nodeId === activeChildId);
    if (!rootNode || !childNode) {
      return null;
    }

    const rootRect = rootNode.getBoundingClientRect();
    const childRect = childNode.getBoundingClientRect();
    return {
      visibleNodeCount: visibleNodes.length,
      rootX: rootRect.x,
      rootY: rootRect.y,
      childX: childRect.x,
      childY: childRect.y,
    };
  }, { activeRootId: rootId, activeChildId: childId });
}

async function graphNodeMetrics(
  page: Page,
  nodeId: string,
): Promise<GraphNodeMetrics | null> {
  return page.evaluate((activeNodeId) => {
    const container = document.getElementById('graph3d-container');
    if (!container) {
      return null;
    }

    const node = Array.from(container.querySelectorAll('[data-node-id]')).find((candidate) => {
      const element = candidate as HTMLElement;
      return element.style.display !== 'none' && element.dataset.nodeId === activeNodeId;
    }) as HTMLElement | undefined;
    if (!node) {
      return null;
    }

    const rect = node.getBoundingClientRect();
    const parseCoordinate = (name: string): number | null => {
      const value = node.getAttribute(name);
      if (!value) {
        return null;
      }
      const parsed = Number.parseFloat(value);
      return Number.isFinite(parsed) ? parsed : null;
    };
    return {
      x: rect.x,
      y: rect.y,
      width: rect.width,
      height: rect.height,
      centerX: rect.x + rect.width / 2,
      centerY: rect.y + rect.height / 2,
      lod: node.getAttribute('data-node-lod'),
      layoutX: parseCoordinate('data-layout-x'),
      layoutY: parseCoordinate('data-layout-y'),
      layoutZ: parseCoordinate('data-layout-z'),
    };
  }, nodeId);
}

async function graphCameraDistance(page: Page): Promise<number | null> {
  return page.evaluate(() => {
    const container = document.getElementById('graph3d-container');
    if (!container) {
      return null;
    }
    const distance = container.getAttribute('data-camera-distance');
    if (!distance) {
      return null;
    }
    const parsed = Number.parseFloat(distance);
    return Number.isFinite(parsed) ? parsed : null;
  });
}

async function visibleGuideCount(
  page: Page,
  selector: string,
): Promise<number> {
  return page.locator(selector).evaluateAll((elements) => {
    return elements.filter((element) => {
      const html = element as HTMLElement;
      const style = window.getComputedStyle(html);
      return style.display !== 'none' && style.visibility !== 'hidden' && style.opacity !== '0';
    }).length;
  });
}

async function visibleGuideMetrics(
  page: Page,
  selector: string,
): Promise<null | {
  width: number;
  height: number;
}> {
  return page.evaluate((activeSelector) => {
    const container = document.getElementById('graph3d-container');
    if (!container) {
      return null;
    }

    const guide = Array.from(container.querySelectorAll(activeSelector)).find((candidate) => {
      const element = candidate as HTMLElement;
      const style = window.getComputedStyle(element);
      return style.display !== 'none' && style.visibility !== 'hidden' && style.opacity !== '0';
    }) as HTMLElement | undefined;
    if (!guide) {
      return null;
    }

    const rect = guide.getBoundingClientRect();
    return {
      width: rect.width,
      height: rect.height,
    };
  }, selector);
}

async function visibleRowLabelOverlapsVisibleNode(page: Page): Promise<boolean> {
  return page.evaluate(() => {
    const container = document.getElementById('graph3d-container');
    if (!container) {
      return true;
    }

    const isVisible = (candidate: Element): candidate is HTMLElement => {
      const element = candidate as HTMLElement;
      const style = window.getComputedStyle(element);
      return style.display !== 'none' && style.visibility !== 'hidden' && style.opacity !== '0';
    };

    const labels = Array.from(container.querySelectorAll('[data-kanban-row-label]')).filter(isVisible);
    const nodes = Array.from(container.querySelectorAll('[data-node-idx]')).filter(isVisible);

    return labels.some((label) => {
      const labelRect = label.getBoundingClientRect();
      return nodes.some((node) => {
        const nodeRect = node.getBoundingClientRect();
        return labelRect.left < nodeRect.right
          && labelRect.right > nodeRect.left
          && labelRect.top < nodeRect.bottom
          && labelRect.bottom > nodeRect.top;
      });
    });
  });
}

async function graphNodeLod(
  page: Page,
  nodeId: string,
): Promise<string | null> {
  return page.evaluate((activeNodeId) => {
    const container = document.getElementById('graph3d-container');
    if (!container) {
      return null;
    }

    const node = Array.from(container.querySelectorAll('[data-node-id]')).find((candidate) => {
      const element = candidate as HTMLElement;
      return element.style.display !== 'none' && element.dataset.nodeId === activeNodeId;
    }) as HTMLElement | undefined;
    return node?.getAttribute('data-node-lod') ?? null;
  }, nodeId);
}

async function zoomGraph(
  page: Page,
  deltaY: number,
  repetitions: number,
): Promise<void> {
  await page.locator('#graph3d-container').hover();
  for (let repeatIndex = 0; repeatIndex < repetitions; repeatIndex += 1) {
    await page.mouse.wheel(0, deltaY);
  }
}

async function dragGraphNode(
  page: Page,
  nodeId: string,
  deltaX: number,
  deltaY: number,
): Promise<void> {
  const metrics = await graphNodeMetrics(page, nodeId);
  expect(metrics, `graph node ${nodeId} should expose drag metrics`).not.toBeNull();

  await page.mouse.move(metrics!.centerX, metrics!.centerY);
  await page.mouse.down();
  await page.mouse.move(
    metrics!.centerX + deltaX,
    metrics!.centerY + deltaY,
    { steps: 12 },
  );
  await page.mouse.up();
}

async function resolveActiveWorkspace(page: Page): Promise<string> {
  await gotoAndWaitForViewer(page, TICKET_VIEWER);

  const resp = await page.request.get(`${TICKET_VIEWER.url}/api/workspaces`);
  expect(resp.ok(), 'workspace list API must respond').toBe(true);

  const body = (await resp.json()) as WorkspacesResponse;
  const workspace = body.active_workspace ?? body.workspaces?.[0]?.name;
  expect(workspace, 'viewer must expose at least one workspace').toBeTruthy();
  return workspace!;
}

async function getTicketComponent(
  page: Page,
  workspace: string,
  id: string,
): Promise<string> {
  const resp = await page.request.get(
    `${TICKET_VIEWER.url}/api/tickets/${id}?workspace=${workspace}`,
  );
  if (!resp.ok()) {
    return '';
  }
  const body = await resp.json();
  return body?.ticket?.fields?.component ?? '';
}

async function findGraphSelectionCandidate(page: Page): Promise<CandidateSelection> {
  const workspace = await resolveActiveWorkspace(page);

  const listResp = await page.request.get(
    `${TICKET_VIEWER.url}/api/tickets?workspace=${workspace}&limit=500`,
  );
  expect(listResp.ok(), 'ticket list API must respond').toBe(true);

  const listBody = await listResp.json();
  const items: TicketListItem[] = listBody?.items ?? [];
  expect(items.length, `workspace \`${workspace}\` must contain tickets`).toBeGreaterThan(0);

  for (const item of items) {
    const rootComponent = await getTicketComponent(page, workspace, item.id);

    const graphResp = await page.request.get(
      `${TICKET_VIEWER.url}/api/graph/subgraph?workspace=${workspace}&root=${item.id}&depth=1`,
    );
    if (!graphResp.ok()) {
      continue;
    }

    const graph = await graphResp.json();
    const nodes: GraphNode[] = graph?.nodes ?? [];
    const edges: GraphEdge[] = graph?.edges ?? [];
    if (edges.length < 1) {
      continue;
    }

    const directChildIds = new Set(
      edges
        .filter((edge) => (edge.from ?? edge.source) === item.id)
        .map((edge) => edge.to ?? edge.target)
        .filter((id): id is string => Boolean(id) && id !== item.id),
    );

    for (const childId of directChildIds) {
      const childComponent = await getTicketComponent(page, workspace, childId);
      if (!childComponent || childComponent === rootComponent) {
        continue;
      }

      const childNode = nodes.find((node) => node.id === childId);
      const childTitle = childNode?.title?.trim() || childId;
      return {
        workspace,
        rootId: item.id,
        rootTitle: item.title?.trim() || item.id,
        childId,
        childTitle,
        childComponent,
      };
    }
  }

  throw new Error('No root ticket with a direct graph child and unique component field was found');
}

async function findKanbanLayoutCandidate(page: Page): Promise<KanbanLayoutCandidate> {
  const workspace = await resolveActiveWorkspace(page);

  const listResp = await page.request.get(
    `${TICKET_VIEWER.url}/api/tickets?workspace=${workspace}&limit=500`,
  );
  expect(listResp.ok(), 'ticket list API must respond').toBe(true);

  const listBody = await listResp.json();
  const items: TicketListItem[] = listBody?.items ?? [];
  expect(items.length, `workspace \`${workspace}\` must contain tickets`).toBeGreaterThan(0);

  for (const item of items) {
    const graphWorkspace = item.ticket_ref?.workspace ?? workspace;
    const graphResp = await page.request.get(
      `${TICKET_VIEWER.url}/api/graph/subgraph?workspace=${graphWorkspace}&root=${item.id}&depth=2`,
    );
    if (!graphResp.ok()) {
      continue;
    }

    const graph = await graphResp.json();
    const nodes: GraphNode[] = graph?.nodes ?? [];
    if (nodes.length < 3) {
      continue;
    }

    const stateGroups = new Map<string, string[]>();
    for (const node of nodes) {
      const state = normalizeTicketState(node.state);
      const group = stateGroups.get(state) ?? [];
      group.push(node.id);
      stateGroups.set(state, group);
    }

    const orderedStates = Array.from(stateGroups.keys()).sort((left, right) => {
      const rankDelta = kanbanStateRank(left) - kanbanStateRank(right);
      return rankDelta === 0 ? left.localeCompare(right) : rankDelta;
    });
    if (orderedStates.length < 2) {
      continue;
    }

    return {
      workspace,
      rootId: item.id,
      stateSamples: orderedStates.slice(0, 4).map((state) => ({
        state,
        id: stateGroups.get(state)![0],
      })),
    };
  }

  throw new Error('No graph candidate with at least two distinct workflow states was found');
}

test.describe('ticket-viewer — graph selection updates right detail sidebar', () => {
  test('clicking a graph node updates the detail pane without changing the main ticket route', async ({ page }) => {
    test.setTimeout(120_000);

    const candidate = await findGraphSelectionCandidate(page);

    await openCandidateTicket(page, candidate);

    const detailPanel = page.getByTestId('ticket-detail-panel');
    const componentField = page.getByTestId('ticket-detail-field-component');
    await expect(detailPanel).toBeVisible();
    await expect(componentField).toBeVisible();
    await expect(componentField).not.toContainText(candidate.childComponent);

    await page.getByRole('button', { name: /^Graph$/ }).first().click();
    await expect(page.locator('#graph3d-container')).toBeVisible({ timeout: 30_000 });
    await page.waitForFunction(
      (childTitle) => {
        const root = document.getElementById('graph3d-container');
        if (!root) {
          return false;
        }
        return Array.from(root.querySelectorAll('[data-node-idx]')).some((node) => {
          const element = node as HTMLElement;
          return element.style.display !== 'none' && (element.textContent || '').includes(childTitle);
        });
      },
      candidate.childTitle,
      { timeout: 30_000 },
    );

    const childNode = page
      .locator('#graph3d-container [data-node-idx]:visible')
      .filter({ hasText: candidate.childTitle })
      .first();
    await expect(childNode).toBeVisible();
    await childNode.click();

    await page.getByRole('button', { name: /^Split$/ }).first().click();
    await expect(detailPanel).toBeVisible();
    await expect(componentField).toBeVisible();
    await expect(componentField).toContainText(candidate.childComponent, { timeout: 20_000 });
    await expect
      .poll(() => {
        const currentUrl = new URL(page.url());
        const hashParams = new URLSearchParams(currentUrl.hash.slice(1));
        return {
          pathname: currentUrl.pathname,
          ticketId: hashParams.get('ticket-id') ?? hashParams.get('id'),
        };
      })
      .toEqual({
        pathname: `/workspace/${candidate.workspace}`,
        ticketId: candidate.rootId,
      });
  });

  test('graph selection recenters the active node and dims unrelated nodes without replacing the graph', async ({ page }, testInfo) => {
    test.setTimeout(120_000);

    const candidate = await findGraphSelectionCandidate(page);

    await openCandidateTicket(page, candidate);

    const container = page.locator('#graph3d-container');
    await page.getByRole('button', { name: /^Graph$/ }).first().click();
    await expect(container).toBeVisible({ timeout: 30_000 });

    await expect.poll(async () => page.evaluate(({ rootId, childId }) => {
      const container = document.getElementById('graph3d-container');
      if (!container) {
        return false;
      }
      const visibleNodes = Array.from(container.querySelectorAll('[data-node-id]')).filter((node) => {
        const element = node as HTMLElement;
        return element.style.display !== 'none';
      }) as HTMLElement[];
      return visibleNodes.some((node) => node.dataset.nodeId === rootId) &&
        visibleNodes.some((node) => node.dataset.nodeId === childId);
    }, {
      rootId: candidate.rootId,
      childId: candidate.childId,
    }), {
      timeout: 30_000,
    }).toBe(true);

    const childNode = page.locator(`#graph3d-container [data-node-id="${candidate.childId}"]`).first();
    await expect(childNode).toBeVisible();

    const initialDistance = await page.evaluate((childId) => {
      const container = document.getElementById('graph3d-container');
      if (!container) {
        return null;
      }
      const child = Array.from(container.querySelectorAll('[data-node-id]')).find((node) => {
        const element = node as HTMLElement;
        return element.style.display !== 'none' && element.dataset.nodeId === childId;
      }) as HTMLElement | undefined;
      if (!child) {
        return null;
      }

      const containerRect = container.getBoundingClientRect();
      const childRect = child.getBoundingClientRect();
      const containerCx = containerRect.left + containerRect.width / 2;
      const containerCy = containerRect.top + containerRect.height / 2;
      const childCx = childRect.left + childRect.width / 2;
      const childCy = childRect.top + childRect.height / 2;
      return Math.hypot(childCx - containerCx, childCy - containerCy);
    }, candidate.childId);
    expect(initialDistance, 'selected child node must expose an initial distance').not.toBeNull();

    await childNode.click();

    await expect.poll(async () => page.evaluate(({ childId, maxDistance }) => {
      const container = document.getElementById('graph3d-container');
      if (!container) {
        return false;
      }
      const visibleNodes = Array.from(container.querySelectorAll('[data-node-id]')).filter((node) => {
        const element = node as HTMLElement;
        return element.style.display !== 'none';
      }) as HTMLElement[];
      const child = visibleNodes.find((node) => node.dataset.nodeId === childId);
      if (!child) {
        return false;
      }

      const containerRect = container.getBoundingClientRect();
      const childRect = child.getBoundingClientRect();
      const containerCx = containerRect.left + containerRect.width / 2;
      const containerCy = containerRect.top + containerRect.height / 2;
      const childCx = childRect.left + childRect.width / 2;
      const childCy = childRect.top + childRect.height / 2;
      const dimmedCount = visibleNodes.filter((node) => {
        if (node.dataset.nodeId === childId) {
          return false;
        }
        return Number.parseFloat(getComputedStyle(node).opacity || '1') < 0.5;
      }).length;
      const childDistance = Math.hypot(childCx - containerCx, childCy - containerCy);

      return childDistance <= maxDistance && dimmedCount > 0;
    }, {
      childId: candidate.childId,
      maxDistance: Math.max(48, initialDistance! * 0.75),
    }), {
      timeout: 20_000,
    }).toBe(true);

    const focusedMetrics = await page.evaluate((childId) => {
      const container = document.getElementById('graph3d-container');
      if (!container) {
        return null;
      }
      const visibleNodes = Array.from(container.querySelectorAll('[data-node-id]')).filter((node) => {
        const element = node as HTMLElement;
        return element.style.display !== 'none';
      }) as HTMLElement[];
      const child = visibleNodes.find((node) => node.dataset.nodeId === childId);
      if (!child) {
        return null;
      }

      const containerRect = container.getBoundingClientRect();
      const childRect = child.getBoundingClientRect();
      const containerCx = containerRect.left + containerRect.width / 2;
      const containerCy = containerRect.top + containerRect.height / 2;
      const childCx = childRect.left + childRect.width / 2;
      const childCy = childRect.top + childRect.height / 2;
      const dimmedCount = visibleNodes.filter((node) => {
        if (node.dataset.nodeId === childId) {
          return false;
        }
        return Number.parseFloat(getComputedStyle(node).opacity || '1') < 0.5;
      }).length;

      return {
        childDistance: Math.hypot(childCx - containerCx, childCy - containerCy),
        dimmedCount,
      };
    }, candidate.childId);

    expect(focusedMetrics, 'focused graph metrics must be measurable').not.toBeNull();
    expect(
      focusedMetrics!.childDistance,
      'focused node should move closer to the graph center after selection',
    ).toBeLessThanOrEqual(Math.max(48, initialDistance! * 0.75));
    expect(
      focusedMetrics!.dimmedCount,
      'graph selection should dim at least one unrelated visible node',
    ).toBeGreaterThan(0);

    await attachScreenshot(page, testInfo, 'graph-focused-selection');
  });

  test('graph node LOD keeps the active selection rich while collapsing other visible nodes', async ({ page }, testInfo) => {
    test.setTimeout(120_000);

    const candidate = await findGraphSelectionCandidate(page);

    await openCandidateTicket(page, candidate);

    const container = page.locator('#graph3d-container');
    await page.getByRole('button', { name: /^Graph$/ }).first().click();
    await expect(container).toBeVisible({ timeout: 30_000 });

    await expect.poll(() => graphLodMetrics(page, candidate.rootId), {
      timeout: 20_000,
    }).toMatchObject({
      selectedLod: 'rich',
      visibleNodeCount: expect.any(Number),
    });

    const initialMetrics = await graphLodMetrics(page, candidate.rootId);
    expect(initialMetrics, 'root graph node should expose LOD metrics').not.toBeNull();
    expect(
      initialMetrics!.collapsedNodes,
      'workspace graph should collapse at least one non-selected visible node to a smaller LOD tier',
    ).toBeGreaterThan(0);

    await zoomGraph(page, 480, 6);

    await expect.poll(() => graphLodMetrics(page, candidate.rootId), {
      timeout: 20_000,
    }).toMatchObject({
      selectedLod: 'rich',
      visibleNodeCount: expect.any(Number),
    });

    const zoomedOutMetrics = await graphLodMetrics(page, candidate.rootId);
    expect(zoomedOutMetrics, 'zoomed-out root graph node should expose LOD metrics').not.toBeNull();
    expect(
      zoomedOutMetrics!.collapsedNodes,
      'zooming out should keep at least one non-selected visible node in a smaller LOD tier',
    ).toBeGreaterThan(0);

    const childNode = page.locator(`#graph3d-container [data-node-id="${candidate.childId}"]`).first();
    await expect(childNode).toBeVisible();

    const zoomedOutChildLod = await graphNodeLod(page, candidate.childId);
    expect(
      zoomedOutChildLod === 'compact' || zoomedOutChildLod === 'minimal',
      'zoomed-out child node should remain interactive while rendered in a smaller LOD tier',
    ).toBe(true);

    await childNode.click();

    await expect.poll(() => graphLodMetrics(page, candidate.childId), {
      timeout: 20_000,
    }).toMatchObject({
      selectedLod: 'rich',
      visibleNodeCount: expect.any(Number),
    });

    const childMetrics = await graphLodMetrics(page, candidate.childId);
    expect(childMetrics, 'selected child node should expose LOD metrics').not.toBeNull();
    expect(
      childMetrics!.collapsedNodes,
      'after selection, other visible nodes should still use smaller compact or minimal tiers',
    ).toBeGreaterThan(0);

    await zoomGraph(page, -480, 4);

    await expect.poll(() => graphLodMetrics(page, candidate.childId), {
      timeout: 20_000,
    }).toMatchObject({
      selectedLod: 'rich',
      visibleNodeCount: expect.any(Number),
    });

    const zoomedInMetrics = await graphLodMetrics(page, candidate.childId);
    expect(zoomedInMetrics, 'zoomed-in child graph node should expose LOD metrics').not.toBeNull();
    expect(
      zoomedInMetrics!.minimalNodes,
      'zooming back in should not increase the number of minimal visible nodes',
    ).toBeLessThanOrEqual(zoomedOutMetrics!.minimalNodes);

    await attachScreenshot(page, testInfo, 'graph-node-lod-tiers');
  });

  test('dragged graph layout and camera zoom persist when focus changes inside the same graph', async ({ page }, testInfo) => {
    test.setTimeout(120_000);

    const candidate = await findGraphSelectionCandidate(page);

    await openCandidateTicket(page, candidate);

    await page.getByRole('button', { name: /^Graph$/ }).first().click();
    await expect(page.locator('#graph3d-container')).toBeVisible({ timeout: 30_000 });

    await expect.poll(() => graphNodeMetrics(page, candidate.rootId), {
      timeout: 30_000,
    }).not.toBeNull();
    await expect.poll(() => graphNodeMetrics(page, candidate.childId), {
      timeout: 30_000,
    }).not.toBeNull();

    const childBeforeDrag = await graphNodeMetrics(page, candidate.childId);
    expect(childBeforeDrag, 'child graph node should expose drag metrics before movement').not.toBeNull();

    await dragGraphNode(page, candidate.childId, 120, 60);

    await expect.poll(async () => {
      const metrics = await graphNodeMetrics(page, candidate.childId);
      if (!metrics || !childBeforeDrag) {
        return 0;
      }
      return Math.hypot(
        metrics.centerX - childBeforeDrag.centerX,
        metrics.centerY - childBeforeDrag.centerY,
      );
    }, {
      timeout: 20_000,
    }).toBeGreaterThan(40);

    await zoomGraph(page, 480, 4);

    const rootBeforeFocus = await graphNodeMetrics(page, candidate.rootId);
    const childBeforeFocus = await graphNodeMetrics(page, candidate.childId);
    const cameraDistanceBeforeFocus = await graphCameraDistance(page);
    expect(rootBeforeFocus, 'root graph node should remain visible after dragging and zooming').not.toBeNull();
    expect(childBeforeFocus, 'dragged child graph node should remain visible after zooming').not.toBeNull();
    expect(cameraDistanceBeforeFocus, 'graph container should expose current camera distance').not.toBeNull();

    const childNode = page.locator(`#graph3d-container [data-node-id="${candidate.childId}"]`).first();
    await expect(childNode).toBeVisible();
    await childNode.click();

    await expect.poll(() => graphLodMetrics(page, candidate.childId), {
      timeout: 20_000,
    }).toMatchObject({
      selectedLod: 'rich',
      visibleNodeCount: expect.any(Number),
    });

    const rootAfterFocus = await graphNodeMetrics(page, candidate.rootId);
    const childAfterFocus = await graphNodeMetrics(page, candidate.childId);
    const cameraDistanceAfterFocus = await graphCameraDistance(page);
    expect(rootAfterFocus, 'root node should remain visible after focus change').not.toBeNull();
    expect(childAfterFocus, 'dragged child node should remain visible after focus change').not.toBeNull();
    expect(cameraDistanceAfterFocus, 'camera distance should still be exposed after focus change').not.toBeNull();

    const deltaBeforeFocusX = childBeforeFocus!.centerX - rootBeforeFocus!.centerX;
    const deltaBeforeFocusY = childBeforeFocus!.centerY - rootBeforeFocus!.centerY;
    const deltaAfterFocusX = childAfterFocus!.centerX - rootAfterFocus!.centerX;
    const deltaAfterFocusY = childAfterFocus!.centerY - rootAfterFocus!.centerY;

    expect(
      Math.abs(deltaAfterFocusX - deltaBeforeFocusX),
      'same-graph focus changes should preserve the dragged horizontal node offset',
    ).toBeLessThan(24);
    expect(
      Math.abs(deltaAfterFocusY - deltaBeforeFocusY),
      'same-graph focus changes should preserve the dragged vertical node offset',
    ).toBeLessThan(24);
    expect(
      Math.abs(cameraDistanceAfterFocus! - cameraDistanceBeforeFocus!),
      'focus changes should preserve the user-adjusted camera zoom distance',
    ).toBeLessThan(0.25);

    await attachScreenshot(page, testInfo, 'graph-persisted-layout-and-zoom');
  });

  test('graph mode defaults to hierarchical isometric controls and preserves top-to-bottom depth ordering', async ({ page }, testInfo) => {
    test.setTimeout(120_000);

    const candidate = await findGraphSelectionCandidate(page);

    await openCandidateTicket(page, candidate);

    await page.getByRole('button', { name: /^Graph$/ }).first().click();
    await expect(page.locator('#graph3d-container')).toBeVisible({ timeout: 30_000 });

    await page.waitForFunction(
      ({ rootTitle, childTitle }) => {
        const root = document.getElementById('graph3d-container');
        if (!root) {
          return false;
        }
        const text = Array.from(root.querySelectorAll('[data-node-idx]'))
          .filter((node) => (node as HTMLElement).style.display !== 'none')
          .map((node) => node.textContent || '');
        return text.some((value) => value.includes(rootTitle)) &&
          text.some((value) => value.includes(childTitle));
      },
      { rootTitle: candidate.rootTitle, childTitle: candidate.childTitle },
      { timeout: 30_000 },
    );

    const rootNode = page
      .locator('#graph3d-container [data-node-idx]:visible')
      .filter({ hasText: candidate.rootTitle })
      .first();
    const childNode = page
      .locator('#graph3d-container [data-node-idx]:visible')
      .filter({ hasText: candidate.childTitle })
      .first();

    await expect(rootNode).toBeVisible();
    await expect(childNode).toBeVisible();

    const rootBox = await rootNode.boundingBox();
    const childBox = await childNode.boundingBox();
    expect(rootBox, 'root graph node must expose a screen box').not.toBeNull();
    expect(childBox, 'child graph node must expose a screen box').not.toBeNull();
    expect(
      childBox!.y,
      'child node should render below the root in the default hierarchy view',
    ).toBeGreaterThan(rootBox!.y + 10);

    await page.getByTitle('Graph settings').click();
    const hierarchicalButton = page.getByRole('button', { name: 'Hierarchical 3D' });
    const orthographicButton = page.getByRole('button', { name: 'Orthographic' });
    await expect(hierarchicalButton).toBeVisible();
    await expect(orthographicButton).toBeVisible();
    await expect(hierarchicalButton).toHaveAttribute('style', GRAPH_ACTIVE_STYLE);
    await expect(orthographicButton).toHaveAttribute('style', GRAPH_ACTIVE_STYLE);

    await attachScreenshot(page, testInfo, 'graph-default-isometric-layout');
  });

  test('graph settings can switch away from and restore the default layout and projection', async ({ page }, testInfo) => {
    test.setTimeout(120_000);

    const candidate = await findGraphSelectionCandidate(page);

    await openCandidateTicket(page, candidate);

    await page.getByRole('button', { name: /^Graph$/ }).first().click();
    await expect(page.locator('#graph3d-container')).toBeVisible({ timeout: 30_000 });

    await expect.poll(() => graphHierarchyMetrics(page, candidate.rootId, candidate.childId), {
      timeout: 30_000,
    }).toMatchObject({
      visibleNodeCount: expect.any(Number),
    });

    const defaultMetrics = await graphHierarchyMetrics(page, candidate.rootId, candidate.childId);
    expect(defaultMetrics, 'default graph layout should expose root and child positions').not.toBeNull();
    expect(
      defaultMetrics!.childY,
      'default hierarchical view should render the child below the root',
    ).toBeGreaterThan(defaultMetrics!.rootY + 10);

    await page.getByTitle('Graph settings').click();

    const hierarchicalButton = page.getByRole('button', { name: 'Hierarchical 3D' });
    const flatButton = page.getByRole('button', { name: 'Flat 2D' });
    const perspectiveButton = page.getByRole('button', { name: 'Perspective' });
    const orthographicButton = page.getByRole('button', { name: 'Orthographic' });

    await expect(hierarchicalButton).toHaveAttribute('style', GRAPH_ACTIVE_STYLE);
    await expect(orthographicButton).toHaveAttribute('style', GRAPH_ACTIVE_STYLE);

    await flatButton.click();
    await perspectiveButton.click();

    await expect(flatButton).toHaveAttribute('style', GRAPH_ACTIVE_STYLE);
    await expect(perspectiveButton).toHaveAttribute('style', GRAPH_ACTIVE_STYLE);

    const switchedMetrics = await graphHierarchyMetrics(page, candidate.rootId, candidate.childId);
    expect(switchedMetrics, 'graph should stay mounted after switching layout and projection').not.toBeNull();
    expect(
      switchedMetrics!.visibleNodeCount,
      'root and child nodes should remain visible after switching away from defaults',
    ).toBeGreaterThanOrEqual(2);

    await hierarchicalButton.click();
    await orthographicButton.click();

    await expect(hierarchicalButton).toHaveAttribute('style', GRAPH_ACTIVE_STYLE);
    await expect(orthographicButton).toHaveAttribute('style', GRAPH_ACTIVE_STYLE);

    await expect.poll(() => graphHierarchyMetrics(page, candidate.rootId, candidate.childId), {
      timeout: 20_000,
    }).toMatchObject({
      visibleNodeCount: expect.any(Number),
    });

    const restoredMetrics = await graphHierarchyMetrics(page, candidate.rootId, candidate.childId);
    expect(restoredMetrics, 'restored default graph layout should expose root and child positions').not.toBeNull();
    expect(
      restoredMetrics!.childY,
      'restoring the default layout should preserve top-to-bottom hierarchy ordering',
    ).toBeGreaterThan(restoredMetrics!.rootY + 10);

    await attachScreenshot(page, testInfo, 'graph-restored-default-layout');
  });

  test('graph settings can switch to kanban state columns', async ({ page }, testInfo) => {
    test.setTimeout(180_000);

    const candidate = await findKanbanLayoutCandidate(page);

    await openTicketById(page, candidate.workspace, candidate.rootId);

    await page.getByRole('button', { name: /^Graph$/ }).first().click();
    await expect(page.locator('#graph3d-container')).toBeVisible({ timeout: 30_000 });

    await page.getByTitle('Graph settings').click();
    const kanbanButton = page.getByRole('button', { name: 'Kanban' });
    await expect(kanbanButton).toBeVisible();
    await kanbanButton.click();
    await expect(kanbanButton).toHaveAttribute('style', GRAPH_ACTIVE_STYLE);
    await attachScreenshot(page, testInfo, 'graph-kanban-guides-after-switch');

    for (const sample of candidate.stateSamples) {
      await expect.poll(
        () => visibleGuideCount(page, `#graph3d-container [data-kanban-column-header="${sample.state}"]`),
        { timeout: 30_000 },
      ).toBeGreaterThan(0);
    }

    await expect.poll(
      () => visibleGuideCount(page, '#graph3d-container [data-kanban-column-separator]'),
      { timeout: 30_000 },
    ).toBeGreaterThan(0);
    await expect.poll(
      () => visibleGuideCount(page, '#graph3d-container [data-kanban-row-label]'),
      { timeout: 30_000 },
    ).toBeGreaterThan(0);

    await expect.poll(
      async () => (await visibleGuideMetrics(page, '#graph3d-container [data-kanban-row-label]'))?.height ?? 0,
      { timeout: 30_000 },
    ).toBeGreaterThan(0);
    await expect.poll(() => visibleRowLabelOverlapsVisibleNode(page), {
      timeout: 30_000,
    }).toBe(false);

    await zoomGraph(page, -480, 3);

    await expect.poll(
      async () => (await visibleGuideMetrics(page, '#graph3d-container [data-kanban-row-label]'))?.height ?? 0,
      { timeout: 30_000 },
    ).toBeGreaterThan(5.0);

    await expect.poll(() => visibleRowLabelOverlapsVisibleNode(page), {
      timeout: 30_000,
    }).toBe(false);

    await zoomGraph(page, 480, 7);

    await expect.poll(
      async () => (await visibleGuideMetrics(page, '#graph3d-container [data-kanban-row-label]'))?.height ?? Number.POSITIVE_INFINITY,
      { timeout: 30_000 },
    ).toBeLessThan(2.5);

    await expect.poll(async () => {
      const metrics = await Promise.all(
        candidate.stateSamples.map((sample) => graphNodeMetrics(page, sample.id)),
      );
      return metrics.every((sample) => sample !== null);
    }, {
      timeout: 30_000,
    }).toBe(true);

    const orderedSamples = (await Promise.all(
      candidate.stateSamples.map(async (sample) => ({
        ...sample,
        metrics: await graphNodeMetrics(page, sample.id),
      })),
    ))
      .filter((sample): sample is typeof sample & { metrics: GraphNodeMetrics } => sample.metrics !== null)
      .sort((left, right) => {
        const rankDelta = kanbanStateRank(left.state) - kanbanStateRank(right.state);
        return rankDelta === 0 ? left.state.localeCompare(right.state) : rankDelta;
      });

    expect(
      orderedSamples.length,
      'kanban candidate should provide visible state samples after switching layouts',
    ).toBeGreaterThanOrEqual(2);

    for (let index = 1; index < orderedSamples.length; index += 1) {
      expect(
        orderedSamples[index].metrics.layoutX,
        `state ${orderedSamples[index].state} should render to the right of ${orderedSamples[index - 1].state} in kanban mode`,
      ).not.toBeNull();
      expect(
        orderedSamples[index - 1].metrics.layoutX,
        `state ${orderedSamples[index - 1].state} should expose a layout x-coordinate in kanban mode`,
      ).not.toBeNull();
      expect(
        orderedSamples[index].metrics.layoutX!,
        `state ${orderedSamples[index].state} should render to the right of ${orderedSamples[index - 1].state} in kanban mode`,
      ).toBeGreaterThan(orderedSamples[index - 1].metrics.layoutX! + 0.25);
    }

    await attachScreenshot(page, testInfo, 'graph-kanban-state-columns');
  });
});