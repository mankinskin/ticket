import { expect, test } from '@playwright/test';
import fs from 'node:fs/promises';
import { resolveActiveWorkspace, TICKET_VIEWER } from './shared/viewers';
import {
  activeSearchResultId,
  activeSidebarTicketId,
  activeSidebarTicketIsVisible,
  findBroadSearchCandidate,
  findSearchCandidate,
  findSubstringSearchCandidate,
  fetchTickets,
  gotoWorkspace,
  seedScrollableWorkspaceFixture,
  sidebarScrollMetrics,
  stopSeededViewer,
  viewportSidebarTicketIds,
  visibleSearchResultIds,
  visibleSidebarTicketIds,
} from './keyboard-navigation.helpers';

test.describe('ticket-viewer — keyboard navigation', () => {
  test('sidebar tree scrolls to deep tickets and keeps the keyboard-active row visible', async ({ page }) => {
    test.setTimeout(180_000);

    const fixture = await seedScrollableWorkspaceFixture();
    try {
      const workspace = await resolveActiveWorkspace(fixture.url);
      await page.addInitScript((activeWorkspace) => {
        localStorage.removeItem(`ticket-viewer:${activeWorkspace}:ui`);
      }, workspace);
      await page.goto(fixture.url, { waitUntil: 'domcontentloaded' });
      await page.locator(TICKET_VIEWER.readySelector).first().waitFor({
        state: 'visible',
        timeout: TICKET_VIEWER.readyTimeout,
      });

      await expect.poll(async () => (await visibleSidebarTicketIds(page)).length).toBeGreaterThan(20);

      const scrollRegion = page.getByTestId('ticket-tree-scroll-region');
      await expect(scrollRegion).toBeVisible();

      const initialMetrics = await sidebarScrollMetrics(page);
      expect(initialMetrics.scrollHeight).toBeGreaterThan(initialMetrics.clientHeight);

      const initialVisibleIds = await viewportSidebarTicketIds(page);
      expect(initialVisibleIds.length).toBeGreaterThan(0);

      await scrollRegion.evaluate((node) => {
        const element = node as HTMLElement;
        element.scrollTop = element.scrollHeight;
      });

      await expect.poll(async () => (await sidebarScrollMetrics(page)).scrollTop).toBeGreaterThan(0);

      const deepVisibleIds = await viewportSidebarTicketIds(page);
      expect(deepVisibleIds.length).toBeGreaterThan(0);
      expect(deepVisibleIds).not.toEqual(initialVisibleIds);

      const deepId = deepVisibleIds[deepVisibleIds.length - 1];
      await page.getByTestId(`ticket-tree-ticket-${deepId}`).click();
      await expect(page.getByTestId(`ticket-tree-ticket-${deepId}`)).toHaveAttribute(
        'data-selected',
        'true',
      );

      await scrollRegion.evaluate((node) => {
        (node as HTMLElement).scrollTop = 0;
      });
      await expect.poll(async () => (await sidebarScrollMetrics(page)).scrollTop).toBe(0);

      const filterInput = page.getByTestId('ticket-tree-filter');
      await filterInput.click();
      await expect(filterInput).toBeFocused();

      for (let i = 0; i < 30; i += 1) {
        await page.keyboard.press('ArrowDown');
      }

      await expect.poll(async () => (await sidebarScrollMetrics(page)).scrollTop).toBeGreaterThan(0);
      await expect.poll(async () => activeSidebarTicketIsVisible(page)).toBe(true);
    } finally {
      await stopSeededViewer(fixture.viewer);
      await fs.rm(fixture.tempDir, { recursive: true, force: true });
    }
  });

  test('sidebar filter keeps focus while arrows move the active ticket and Enter selects it', async ({ page }) => {
    await gotoWorkspace(page);

    await expect.poll(async () => (await visibleSidebarTicketIds(page)).length).toBeGreaterThan(1);

    const filterInput = page.getByTestId('ticket-tree-filter');
    const filterHint = page.getByTestId('ticket-tree-filter-hint');
    await expect(filterHint).toContainText('Free text searches titles');
    await expect(filterHint).toContainText('id:<value>');
    await expect(filterHint).not.toContainText('priority:');
    await filterInput.click();
    await expect(filterInput).toBeFocused();

    const visibleIds = await visibleSidebarTicketIds(page);
    await expect.poll(async () => activeSidebarTicketId(page)).not.toBeNull();
    const currentActiveId = await activeSidebarTicketId(page);
    expect(currentActiveId).toBeTruthy();

    const currentIndex = visibleIds.findIndex((id) => id === currentActiveId);
    expect(currentIndex).toBeGreaterThanOrEqual(0);

    const nextId = visibleIds[Math.min(currentIndex + 1, visibleIds.length - 1)];
    expect(nextId).toBeTruthy();

    await page.keyboard.press('ArrowDown');
    await expect(page.getByTestId(`ticket-tree-ticket-${nextId}`)).toHaveAttribute(
      'aria-selected',
      'true',
    );
    await expect(filterInput).toBeFocused();

    await page.keyboard.press('ArrowUp');
    await expect(page.getByTestId(`ticket-tree-ticket-${currentActiveId}`)).toHaveAttribute(
      'aria-selected',
      'true',
    );
    await expect(filterInput).toBeFocused();

    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('Enter');
    await expect(page.getByTestId(`ticket-tree-ticket-${nextId}`)).toHaveAttribute(
      'data-selected',
      'true',
    );
    await expect(filterInput).toBeFocused();
  });

  test('quick-search keeps input focus while arrows move the active result and Enter opens it', async ({ page }) => {
    const candidate = await findSearchCandidate(page);
    await gotoWorkspace(page);

    await page.locator('body').click();
    await page.keyboard.press('/');

    const searchInput = page.getByTestId('search-input');
    await expect(searchInput).toBeVisible();
    await expect(searchInput).toBeFocused();
    await searchInput.fill(candidate.query);

    await expect.poll(async () => (await visibleSearchResultIds(page)).length).toBeGreaterThan(1);

    const visibleIds = await visibleSearchResultIds(page);
    await expect.poll(async () => activeSearchResultId(page)).not.toBeNull();
    const currentActiveId = await activeSearchResultId(page);
    expect(currentActiveId).toBeTruthy();

    const currentIndex = visibleIds.findIndex((id) => id === currentActiveId);
    expect(currentIndex).toBeGreaterThanOrEqual(0);

    const nextId = visibleIds[Math.min(currentIndex + 1, visibleIds.length - 1)];
    expect(nextId).toBeTruthy();

    const currentResult = page.getByTestId(`search-result-${currentActiveId}`);
    const nextResult = page.getByTestId(`search-result-${nextId}`);
    await expect(currentResult).toHaveAttribute('aria-selected', 'true');

    await page.keyboard.press('ArrowDown');
    await expect(nextResult).toHaveAttribute('aria-selected', 'true');
    await expect(searchInput).toBeFocused();

    await page.keyboard.press('ArrowUp');
    await expect(currentResult).toHaveAttribute('aria-selected', 'true');
    await expect(searchInput).toBeFocused();

    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('Enter');

    await expect(page.getByTestId('search-input')).toHaveCount(0);
    await expect(page.getByTestId(`ticket-tree-ticket-${nextId}`)).toHaveAttribute(
      'data-selected',
      'true',
    );
  });

  test('quick-search documents supported syntax and id predicates narrow results', async ({ page }) => {
    const items = await fetchTickets(page);
    expect(items.length).toBeGreaterThan(0);

    const candidate = items[0];
    const titleCandidate = await findSubstringSearchCandidate(page);
    await gotoWorkspace(page);

    await page.locator('body').click();
    await page.keyboard.press('/');

    const searchInput = page.getByTestId('search-input');
    const syntaxHint = page.getByTestId('search-syntax-hint');
    await expect(searchInput).toBeVisible();
    await expect(searchInput).toHaveAttribute('placeholder', /id:, title:, state:, type:/);
    await expect(syntaxHint).toContainText('Free text searches titles');
    await expect(syntaxHint).toContainText('state:<value>/status:<value>');
    await expect(syntaxHint).toContainText('type:<value>/ticket_type:<value>');
    await expect(syntaxHint).not.toContainText('priority:');

    await searchInput.fill(`id:${candidate.id}`);

    await expect.poll(async () => visibleSearchResultIds(page)).toEqual([candidate.id]);

    await searchInput.fill(`title:${titleCandidate.query}`);

    await expect.poll(async () =>
      (await visibleSearchResultIds(page)).includes(titleCandidate.expectedId),
    ).toBe(true);
  });

  test('quick-search surfaces at least nine results for broad queries', async ({ page }) => {
    const candidate = await findBroadSearchCandidate(page, 9);
    await gotoWorkspace(page);

    await page.locator('body').click();
    await page.keyboard.press('/');

    const searchInput = page.getByTestId('search-input');
    await expect(searchInput).toBeVisible();
    await searchInput.fill(candidate.query);

    await expect.poll(async () => (await visibleSearchResultIds(page)).length).toBeGreaterThanOrEqual(
      candidate.ids.length,
    );
    await expect.poll(async () => (await visibleSearchResultIds(page)).slice(0, candidate.ids.length)).toEqual(
      candidate.ids,
    );
  });

  test('quick-search matches partial-word substrings', async ({ page }) => {
    const candidate = await findSubstringSearchCandidate(page);
    await gotoWorkspace(page);

    await page.locator('body').click();
    await page.keyboard.press('/');

    const searchInput = page.getByTestId('search-input');
    await expect(searchInput).toBeVisible();
    await searchInput.fill(candidate.query);

    await expect.poll(async () =>
      (await visibleSearchResultIds(page)).includes(candidate.expectedId),
    ).toBe(true);
  });
});