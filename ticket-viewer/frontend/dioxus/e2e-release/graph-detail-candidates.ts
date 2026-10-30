import { expect, type Page, type TestInfo } from '@playwright/test';
import { gotoAndWaitForViewer, TICKET_VIEWER } from './shared/viewers';

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

interface WorkspacesResponse {
  active_workspace?: string;
  workspaces?: Array<{ name?: string }>;
}

export async function attachScreenshot(
  page: Page,
  testInfo: TestInfo,
  name: string,
): Promise<void> {
  await testInfo.attach(name, {
    body: await page.screenshot({ fullPage: true }),
    contentType: 'image/png',
  });
}

export async function openCandidateTicket(
  page: Page,
  candidate: CandidateSelection,
): Promise<void> {
  await openTicketById(page, candidate.workspace, candidate.rootId);
}

export async function openTicketById(
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

export function kanbanStateRank(state: string): number {
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

export async function resolveActiveWorkspace(page: Page): Promise<string> {
  await gotoAndWaitForViewer(page, TICKET_VIEWER);

  const resp = await page.request.get(`${TICKET_VIEWER.url}/api/workspaces`);
  expect(resp.ok(), 'workspace list API must respond').toBe(true);

  const body = (await resp.json()) as WorkspacesResponse;
  const workspace = body.active_workspace ?? body.workspaces?.[0]?.name;
  expect(workspace, 'viewer must expose at least one workspace').toBeTruthy();
  return workspace!;
}

export async function getTicketComponent(
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

export async function findGraphSelectionCandidate(page: Page): Promise<CandidateSelection> {
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

export async function findKanbanLayoutCandidate(page: Page): Promise<KanbanLayoutCandidate> {
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
