import { expect, test } from '@playwright/test';
import fs from 'node:fs/promises';
import { TICKET_VIEWER } from './shared/viewers';
import {
  attachScreenshot,
  expectEvidenceIfPresent,
  fetchWorkflowNext,
  fetchWorkflowTree,
  flattenTreeEntries,
  frontierTreeIds,
  hasWorkflowEvidence,
  seedWorkflowFixture,
  selectTicketInBrowse,
  stopSeededViewer,
  visibleIds,
  visibleTreeEntries,
} from './workflow-sidebar.helpers';

test.describe('ticket-viewer — workflow sidebar', () => {
  test('workflow blockers mode keeps browse intact and shows a clear empty-root state', async ({ page }, testInfo) => {
    test.setTimeout(180_000);

    const fixture = await seedWorkflowFixture(page);
    try {
      await page.setViewportSize({ width: 1280, height: 900 });
      await page.goto(`${fixture.url}/`, {
        waitUntil: 'domcontentloaded',
      });
      await page.locator(TICKET_VIEWER.readySelector).first().waitFor({
        state: 'visible',
        timeout: TICKET_VIEWER.readyTimeout,
      });

      await expect(page.getByTestId('ticket-tree-filter')).toBeVisible();

      await page.getByTestId('sidebar-mode-blockers').click();
      await expect(page.getByTestId('workflow-root-empty')).toBeVisible();
      await attachScreenshot(page, testInfo, 'workflow-root-empty-sidebar');

      await page.getByTestId('sidebar-mode-browse').click();
      await expect(page.getByTestId('ticket-tree-filter')).toBeVisible();
    } finally {
      await stopSeededViewer(fixture.viewer);
      await fs.rm(fixture.tempDir, { recursive: true, force: true });
    }
  });

  test('workflow modes preserve server order, hierarchy, evidence, and narrow-width rendering', async ({ page }, testInfo) => {
    test.setTimeout(180_000);

    const fixture = await seedWorkflowFixture(page);
    try {
      const nextResponse = await fetchWorkflowNext(page, fixture);
      const blockersResponse = await fetchWorkflowTree(
        page,
        fixture,
        'blockers',
        fixture.blockersRootId,
      );
      const unblockedByResponse = await fetchWorkflowTree(
        page,
        fixture,
        'unblocked-by',
        fixture.unblockedByRootId,
      );

      expect(nextResponse.items.length).toBeGreaterThan(0);
      expect(blockersResponse.root.children?.length ?? 0).toBeGreaterThan(0);
      expect(unblockedByResponse.root.children?.length ?? 0).toBeGreaterThan(0);

      await page.setViewportSize({ width: 1280, height: 900 });
      await page.goto(`${fixture.url}/`, {
        waitUntil: 'domcontentloaded',
      });
      await page.locator(TICKET_VIEWER.readySelector).first().waitFor({
        state: 'visible',
        timeout: TICKET_VIEWER.readyTimeout,
      });

      await page.getByTestId('sidebar-mode-next').click();
      await expect(
        page.getByTestId(`workflow-next-item-${nextResponse.items[0].id}`),
      ).toBeVisible();
      await expect
        .poll(async () => await visibleIds(page, 'workflow-next-item-'))
        .toEqual(nextResponse.items.map((item) => item.id));
      for (const item of nextResponse.items) {
        await expectEvidenceIfPresent(
          page.getByTestId(`workflow-next-item-${item.id}`),
          item,
        );
      }
      await attachScreenshot(page, testInfo, 'workflow-next-sidebar');

      await page.setViewportSize({ width: 820, height: 900 });
      await expect(page.getByTestId('sidebar-mode-next')).toBeVisible();
      await expect(
        page.getByTestId(`workflow-next-item-${nextResponse.items[0].id}`),
      ).toBeVisible();

      await selectTicketInBrowse(page, fixture.blockersRootTitle, fixture.blockersRootId);
      await page.getByTestId('sidebar-mode-blockers').click();
      await expect(
        page.getByTestId(`workflow-tree-node-${fixture.blockersRootId}`),
      ).toBeVisible();
      await expect
        .poll(async () => await visibleTreeEntries(page))
        .toEqual(flattenTreeEntries(blockersResponse.root));
      await expect
        .poll(async () => await visibleIds(page, 'workflow-tree-frontier-'))
        .toEqual(frontierTreeIds(blockersResponse.root));
      await expect
        .poll(async () => await visibleIds(page, 'workflow-frontier-item-'))
        .toEqual((blockersResponse.frontier_items ?? []).map((item) => item.id));
      await expectEvidenceIfPresent(
        page.getByTestId('workflow-root-card'),
        blockersResponse.root,
      );
      for (const item of blockersResponse.frontier_items ?? []) {
        await expectEvidenceIfPresent(
          page.getByTestId(`workflow-frontier-item-${item.id}`),
          item,
        );
      }
      await attachScreenshot(page, testInfo, 'workflow-blockers-sidebar');

      await selectTicketInBrowse(
        page,
        fixture.unblockedByRootTitle,
        fixture.unblockedByRootId,
      );
      await page.getByTestId('sidebar-mode-unblocked-by').click();
      await expect(
        page.getByTestId(`workflow-tree-node-${fixture.unblockedByRootId}`),
      ).toBeVisible();
      await expect
        .poll(async () => await visibleTreeEntries(page))
        .toEqual(flattenTreeEntries(unblockedByResponse.root));
      await expect
        .poll(async () => await visibleIds(page, 'workflow-tree-frontier-'))
        .toEqual(frontierTreeIds(unblockedByResponse.root));
      await expect
        .poll(async () => await visibleIds(page, 'workflow-frontier-item-'))
        .toEqual((unblockedByResponse.frontier_items ?? []).map((item) => item.id));
      await expectEvidenceIfPresent(
        page.getByTestId('workflow-root-card'),
        unblockedByResponse.root,
      );
      const unblockedFrontierItems = unblockedByResponse.frontier_items ?? [];
      expect(
        unblockedFrontierItems.some((item) => hasWorkflowEvidence(item)),
      ).toBe(true);
      for (const item of unblockedFrontierItems) {
        await expectEvidenceIfPresent(
          page.getByTestId(`workflow-frontier-item-${item.id}`),
          item,
        );
      }
      await attachScreenshot(page, testInfo, 'workflow-unblocked-by-sidebar');
    } finally {
      await stopSeededViewer(fixture.viewer);
      await fs.rm(fixture.tempDir, { recursive: true, force: true });
    }
  });
});