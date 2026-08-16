import { test, expect } from '@playwright/test';
import { TICKET_VIEWER } from './shared/viewers';

interface WorkspacesResponse {
  active_workspace?: string;
  workspaces?: Array<{ name?: string }>;
}

async function resolveActiveWorkspace(page: Parameters<typeof test>[0]['page']): Promise<string> {
  const resp = await page.request.get(`${TICKET_VIEWER.url}/api/workspaces`);
  expect(resp.ok(), 'workspace list API must respond').toBe(true);

  const body = (await resp.json()) as WorkspacesResponse;
  const workspace = body.active_workspace ?? body.workspaces?.[0]?.name;
  expect(workspace, 'viewer must expose at least one workspace').toBeTruthy();
  return workspace!;
}

test.describe('ticket-viewer — graph fetch failure UX', () => {
  test('graph API failure shows error and retry action', async ({ page }) => {
    test.setTimeout(60_000);

    const workspace = await resolveActiveWorkspace(page);

    const listResp = await page.request.get(
      `${TICKET_VIEWER.url}/api/tickets?workspace=${workspace}&limit=1`,
    );
    expect(listResp.ok(), 'ticket list API must respond').toBe(true);
    const listBody = await listResp.json();
    const firstId = listBody?.items?.[0]?.id as string | undefined;
    expect(firstId, 'active workspace must contain at least one ticket').toBeTruthy();

    await page.route('**/api/graph/workspace?*', async (route) => {
      await route.fulfill({
        status: 504,
        contentType: 'application/json',
        body: JSON.stringify({ error: 'forced timeout for e2e' }),
      });
    });

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

    await expect(page.getByText('Failed to load graph')).toBeVisible({ timeout: 20_000 });
    await expect(page.getByRole('button', { name: 'Retry' })).toBeVisible();
    await expect(page.getByText(/Loading graph/i)).toHaveCount(0);
  });
});