import { test, expect, type APIRequestContext } from '@playwright/test';
import { TICKET_VIEWER } from './shared/viewers';

/**
 * Covers ticket 89fa0c25 ACs 1-7: parts render as separate collapsible
 * sections, frozen parts are marked, amendments render inline beneath the
 * part they supersede, typed refs render (spec deep-link + dangling
 * marking), and legacy description-only tickets still render.
 */

interface CreatedPart {
  id: string;
}

async function openTicket(page: import('@playwright/test').Page, workspace: string, id: string): Promise<void> {
  await page.goto(`${TICKET_VIEWER.url}/workspace/${workspace}`, { waitUntil: 'domcontentloaded' });
  await page.locator(TICKET_VIEWER.readySelector).first().waitFor({ state: 'visible', timeout: TICKET_VIEWER.readyTimeout });
  await page.evaluate(
    ({ key, ticketId }) => {
      const raw = localStorage.getItem(key);
      const obj = raw ? JSON.parse(raw) : {};
      obj.open_ticket_id = ticketId;
      localStorage.setItem(key, JSON.stringify(obj));
    },
    { key: `ticket-viewer:${workspace}:ui`, ticketId: id },
  );
  await page.goto(`${TICKET_VIEWER.url}/workspace/${workspace}#id=${id}`, { waitUntil: 'domcontentloaded' });
  await page.locator(TICKET_VIEWER.readySelector).first().waitFor({ state: 'visible', timeout: TICKET_VIEWER.readyTimeout });
}

async function resolveActiveWorkspace(request: APIRequestContext): Promise<string> {
  const resp = await request.get(`${TICKET_VIEWER.url}/api/workspaces`);
  expect(resp.ok(), 'workspace list API must respond').toBe(true);
  const body = (await resp.json()) as { active_workspace?: string; workspaces?: Array<{ name?: string }> };
  const workspace = body.active_workspace ?? body.workspaces?.[0]?.name;
  expect(workspace, 'viewer must expose at least one workspace').toBeTruthy();
  return workspace!;
}

async function writePart(
  request: APIRequestContext,
  workspace: string,
  ticketId: string,
  kind: string,
  content: string,
): Promise<string> {
  const resp = await request.post(
    `${TICKET_VIEWER.url}/api/tickets/${ticketId}/parts?workspace=${workspace}`,
    { data: { kind, content } },
  );
  expect(resp.ok(), `write_part(${kind}) must succeed`).toBe(true);
  const body = (await resp.json()) as { part: CreatedPart };
  return body.part.id;
}

/** Creates a fresh ticket with every planning part kind, freezes it via the
 * `planned` transition, and writes an amendment superseding `objective`. */
async function createStructuredFixtureTicket(
  request: APIRequestContext,
  workspace: string,
): Promise<{ id: string; objectiveId: string }> {
  const createResp = await request.post(`${TICKET_VIEWER.url}/api/tickets?workspace=${workspace}`, {
    data: { type: 'task', title: 'E2E structured-parts fixture' },
  });
  expect(createResp.ok(), 'create_ticket must succeed').toBe(true);
  const created = (await createResp.json()) as { ticket: { id: string } };
  const id = created.ticket.id;

  const objectiveId = await writePart(request, workspace, id, 'objective', 'Fixture objective content.');
  await writePart(request, workspace, id, 'requirements', 'Fixture requirements content.');
  await writePart(request, workspace, id, 'design', 'Fixture design content.');
  await writePart(request, workspace, id, 'examples', 'Fixture examples content.');
  await writePart(request, workspace, id, 'acceptance_criteria', 'Fixture acceptance criteria content.');
  await writePart(request, workspace, id, 'review', 'Fixture review content.');

  const planResp = await request.patch(`${TICKET_VIEWER.url}/api/tickets/${id}?workspace=${workspace}`, {
    data: { state: 'planned' },
  });
  expect(planResp.ok(), 'transition to planned (freezes planning parts) must succeed').toBe(true);

  const amendResp = await request.post(
    `${TICKET_VIEWER.url}/api/tickets/${id}/parts/amendment?workspace=${workspace}`,
    { data: { supersedes: objectiveId, content: 'Amendment: corrected fixture objective.' } },
  );
  expect(amendResp.ok(), 'write_amendment must succeed').toBe(true);

  return { id, objectiveId };
}

test.describe('ticket-viewer — structured parts rendering', () => {
  test('parts render as independently collapsible sections with frozen and amendment markers', async ({
    page,
    request,
  }) => {
    test.setTimeout(90_000);

    const workspace = await resolveActiveWorkspace(request);
    const { id } = await createStructuredFixtureTicket(request, workspace);

    await openTicket(page, workspace, id);
    await page.locator('[data-testid="ticket-part"]').first().waitFor({ state: 'visible', timeout: 30_000 });

    // Top-level sections only — the amendment renders nested inside the
    // `objective` section it supersedes, so it is excluded from this count.
    const topLevelParts = page.locator('[data-testid="ticket-parts"] > [data-testid="ticket-part"]');
    await expect(topLevelParts).toHaveCount(6, { timeout: 15_000 });

    const frozenBadges = page.locator('[data-testid="ticket-part-frozen-badge"]');
    await expect(frozenBadges.first()).toBeVisible();
    await expect(frozenBadges.first()).toContainText('planned');

    const amendment = page.locator('[data-testid="ticket-part-amendment"]');
    await expect(amendment).toHaveCount(1);
    await expect(amendment).toContainText('supersedes objective');

    await amendment.scrollIntoViewIfNeeded();
    await page.screenshot({
      path: 'test-results-release/structured-parts-expanded.png',
      fullPage: true,
    });

    // Collapse the first top-level part; its body must disappear while the
    // others stay open (AC4 — independent collapse state). `> ` scopes to
    // the direct child body, excluding the nested amendment's own body.
    const firstHeader = topLevelParts.first().locator('> [data-testid="ticket-part-header"]');
    const firstBody = topLevelParts.first().locator('> [data-testid="ticket-part-body"]');
    await expect(firstBody).toBeVisible();
    await firstHeader.click();
    await expect(firstBody).toHaveCount(0);
    await expect(topLevelParts.nth(1).locator('> [data-testid="ticket-part-body"]')).toBeVisible();

    await topLevelParts.first().scrollIntoViewIfNeeded();
    await page.screenshot({
      path: 'test-results-release/structured-parts-collapsed.png',
      fullPage: true,
    });
  });

  test('typed refs render as links or dangling markers', async ({ page, request }) => {
    test.setTimeout(60_000);

    // This fixture lives in the `memory-api` nested store specifically
    // (ref-writing has no HTTP transport yet — ticket 9d69e93d scope — so
    // its `[[refs]]` table was seeded directly in its manifest there).
    const wsResp = await request.get(`${TICKET_VIEWER.url}/api/workspaces`);
    const wsBody = (await wsResp.json()) as { workspaces?: Array<{ name?: string; label?: string }> };
    const memoryApiWorkspace = wsBody.workspaces?.find((w) => w.label === 'memory-api')?.name;
    test.skip(!memoryApiWorkspace, 'memory-api workspace is not registered in this environment');
    const workspace = memoryApiWorkspace!;

    const fixtureId = '4460cad4-c137-45c4-893e-6e340e16bbe7';
    const checkResp = await request.get(`${TICKET_VIEWER.url}/api/tickets/${fixtureId}?workspace=${workspace}&view=full`);
    test.skip(!checkResp.ok(), 'refs fixture ticket is not present in this environment');

    await openTicket(page, workspace, fixtureId);
    await page.locator('[data-testid="ticket-refs"]').waitFor({ state: 'visible', timeout: 30_000 });

    const specLink = page.locator('[data-testid="ticket-ref-link"]');
    await expect(specLink.first()).toBeVisible();
    await expect(specLink.first()).toHaveAttribute('href', /localhost:4002\/specs\//);

    const dangling = page.locator('[data-testid="ticket-ref-dangling-badge"]');
    await expect(dangling.first()).toBeVisible();

    await page.locator('[data-testid="ticket-refs"]').scrollIntoViewIfNeeded();
    await page.screenshot({
      path: 'test-results-release/structured-parts-refs.png',
      fullPage: true,
    });
  });

  test('legacy description-only ticket still renders without regression', async ({ page, request }) => {
    test.setTimeout(60_000);
    const workspace = await resolveActiveWorkspace(request);

    // A ticket predating `[[parts]]`; the backend synthesizes a sole
    // `objective` part so the viewer must still render usefully (AC — no
    // blank pages for legacy tickets).
    const legacyId = '3a1ec9f8-15ea-43f2-b6d3-89b88cbdcb17';
    const checkResp = await request.get(`${TICKET_VIEWER.url}/api/tickets/${legacyId}?workspace=${workspace}&view=full`);
    test.skip(!checkResp.ok(), 'legacy fixture ticket is not present in this environment');

    await openTicket(page, workspace, legacyId);

    const parts = page.locator('[data-testid="ticket-part"]');
    await expect(parts).toHaveCount(1, { timeout: 15_000 });
    await expect(parts.first()).toHaveAttribute('data-part-kind', 'objective');
    await expect(page.locator('[data-testid="desc-error"]')).toHaveCount(0);
    await expect(page.locator('[data-testid="ticket-parts-empty"]')).toHaveCount(0);

    await parts.first().scrollIntoViewIfNeeded();
    await page.screenshot({
      path: 'test-results-release/structured-parts-legacy.png',
      fullPage: true,
    });
  });
});
