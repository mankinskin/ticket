import { expect, test } from '@playwright/test';
import fs from 'node:fs/promises';
import { TICKET_VIEWER } from './shared/viewers';
import {
  type CreateTicketResponse,
  fetchTicketIds,
  resolveActiveWorkspace,
  resolveWorkspaceNames,
  seedFilterFixture,
  seedMixedWorkspaceFixture,
  stopSeededViewer,
  visibleTicketIds,
} from './live-refresh-authority.helpers';

test.describe('ticket-viewer — live refresh authority', () => {
  test('non-matching SSE upserts do not corrupt an active state filter', async ({ page }) => {
    test.setTimeout(120_000);

    const fixture = await seedFilterFixture();
    try {
      const activeWorkspace = await resolveActiveWorkspace(fixture.url);

      await page.goto(`${fixture.url}/`, {
        waitUntil: 'domcontentloaded',
      });
      await page.locator(TICKET_VIEWER.readySelector).first().waitFor({
        state: 'visible',
        timeout: TICKET_VIEWER.readyTimeout,
      });

      const readyChip = page.getByTestId('ticket-tree-state-chip-ready');
      await expect(readyChip).toBeVisible();
      await readyChip.click();

      await expect.poll(async () => {
        const ids = await visibleTicketIds(page);
        return ids.sort();
      }).toEqual([...fixture.readyTicketIds].sort());

      const createResponse = await page.request.post(
        `${fixture.url}/api/tickets?workspace=${encodeURIComponent(activeWorkspace)}`,
        {
          data: {
            type: 'tracker-improvement',
            title: 'Hidden new ticket',
          },
        },
      );
      expect(createResponse.ok(), 'seeded create request failed').toBe(true);
      const createBody = (await createResponse.json()) as CreateTicketResponse;
      const hiddenTicketId = createBody.ticket?.id;
      expect(hiddenTicketId, 'expected created ticket id').toBeTruthy();

      await expect.poll(async () => {
        const ids = await fetchTicketIds(fixture.url, activeWorkspace);
        return ids.includes(hiddenTicketId!);
      }).toBe(true);

      await expect.poll(async () => {
        const ids = await visibleTicketIds(page);
        return ids.sort();
      }).toEqual([...fixture.readyTicketIds].sort());
      await expect(page.getByText('Hidden new ticket')).toHaveCount(0);
    } finally {
      await stopSeededViewer(fixture.viewer);
      await fs.rm(fixture.tempDir, { recursive: true, force: true });
    }
  });

  test('ticket.delete keeps the active state filter authoritative', async ({ page }) => {
    test.setTimeout(120_000);

    const fixture = await seedFilterFixture();
    try {
      const activeWorkspace = await resolveActiveWorkspace(fixture.url);
      const [deletedTicketId, remainingTicketId] = fixture.readyTicketIds;

      await page.goto(`${fixture.url}/`, {
        waitUntil: 'domcontentloaded',
      });
      await page.locator(TICKET_VIEWER.readySelector).first().waitFor({
        state: 'visible',
        timeout: TICKET_VIEWER.readyTimeout,
      });

      const readyChip = page.getByTestId('ticket-tree-state-chip-ready');
      await expect(readyChip).toBeVisible();
      await readyChip.click();

      await expect.poll(async () => {
        const ids = await visibleTicketIds(page);
        return ids.sort();
      }).toEqual([...fixture.readyTicketIds].sort());

      const deleteResponse = await page.request.delete(
        `${fixture.url}/api/tickets/${deletedTicketId}?workspace=${encodeURIComponent(activeWorkspace)}`,
      );
      expect(deleteResponse.ok(), 'seeded delete request failed').toBe(true);

      await expect.poll(async () => {
        const ids = await visibleTicketIds(page);
        return ids.sort();
      }).toEqual([remainingTicketId]);
    } finally {
      await stopSeededViewer(fixture.viewer);
      await fs.rm(fixture.tempDir, { recursive: true, force: true });
    }
  });

  test('previous-workspace responses cannot overwrite the current list', async ({ page }) => {
    test.setTimeout(120_000);

    const fixture = await seedMixedWorkspaceFixture();
    try {
      const activeWorkspace = await resolveActiveWorkspace(fixture.url);
      const workspaceNames = await resolveWorkspaceNames(fixture.url);
      const childWorkspace = workspaceNames.find((workspace) => workspace !== activeWorkspace);
      expect(childWorkspace, 'expected a second workspace for the race test').toBeTruthy();

      const childIds = await fetchTicketIds(fixture.url, childWorkspace!);
      expect(childIds.length, 'expected seeded child workspace ticket').toBeGreaterThan(0);

      let delayedParentList = false;
      const parentRequestPromise = page.waitForRequest((request) => {
        const url = new URL(request.url());
        return (
          url.origin === new URL(fixture.url).origin &&
          url.pathname === '/api/tickets' &&
          url.searchParams.get('workspace') === activeWorkspace
        );
      });
      const parentResponsePromise = page.waitForResponse((response) => {
        if (!response.ok()) {
          return false;
        }

        const url = new URL(response.url());
        return (
          url.origin === new URL(fixture.url).origin &&
          url.pathname === '/api/tickets' &&
          url.searchParams.get('workspace') === activeWorkspace
        );
      });

      await page.route('**/api/tickets?**', async (route) => {
        const url = new URL(route.request().url());
        const isDelayedParentList =
          !delayedParentList &&
          url.origin === new URL(fixture.url).origin &&
          url.pathname === '/api/tickets' &&
          url.searchParams.get('workspace') === activeWorkspace;

        if (isDelayedParentList) {
          delayedParentList = true;
          await new Promise((resolve) => setTimeout(resolve, 1500));
        }

        await route.continue();
      });

      await page.goto(
        `${fixture.url}/workspace/${encodeURIComponent(activeWorkspace)}`,
        { waitUntil: 'domcontentloaded' },
      );
      await page.locator(TICKET_VIEWER.readySelector).first().waitFor({
        state: 'visible',
        timeout: TICKET_VIEWER.readyTimeout,
      });
      await parentRequestPromise;

      const childResponsePromise = page.waitForResponse((response) => {
        if (!response.ok()) {
          return false;
        }

        const url = new URL(response.url());
        return (
          url.origin === new URL(fixture.url).origin &&
          url.pathname === '/api/tickets' &&
          url.searchParams.get('workspace') === childWorkspace
        );
      });

      await page.evaluate((workspace) => {
        window.history.pushState({}, '', `/workspace/${encodeURIComponent(workspace)}`);
        window.dispatchEvent(new PopStateEvent('popstate'));
      }, childWorkspace!);

      await childResponsePromise;
      await parentResponsePromise;

      await expect.poll(async () => {
        const ids = await visibleTicketIds(page);
        return ids.sort();
      }).toEqual([...childIds].sort());
      await expect(page.getByText(fixture.childTitle)).toBeVisible();
      await expect(page.getByText(fixture.parentTitle)).toHaveCount(0);
    } finally {
      await stopSeededViewer(fixture.viewer);
      await fs.rm(fixture.tempDir, { recursive: true, force: true });
    }
  });
});