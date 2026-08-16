import { test, expect, type ConsoleMessage, type Page, type Route } from '@playwright/test';

function isRuntimePanic(message: string): boolean {
  return [
    'runtime.rs:223',
    'RuntimeError: unreachable',
    'RefCell already borrowed',
    'Option::unwrap',
  ].some((needle) => message.includes(needle));
}

async function waitForWorkspacePage(page: Page): Promise<void> {
  await page
    .locator('h3', { hasText: 'Your app is being rebuilt.' })
    .waitFor({ state: 'detached', timeout: 90_000 })
    .catch(() => {
      /* element was never present or already detached */
    });

  await page.getByRole('heading', { name: 'Tickets' }).waitFor({
    state: 'visible',
    timeout: 30_000,
  });
}

async function mockOfflineWorkspace(page: Page): Promise<() => number> {
  let streamRequests = 0;

  await page.route('**/api/tickets?workspace=default**', (route: Route) => {
    void route.fulfill({
      status: 500,
      contentType: 'text/plain',
      body: 'backend offline',
    });
  });

  await page.route('**/api/stream?workspace=default**', (route: Route) => {
    streamRequests += 1;
    void route.fulfill({
      status: 500,
      contentType: 'text/plain',
      body: 'backend offline',
    });
  });

  return () => streamRequests;
}

test('offline workspace route stays interactive without runtime panics', async ({ page }) => {
  const runtimePanics: string[] = [];

  page.on('console', (message: ConsoleMessage) => {
    if (message.type() === 'error' && isRuntimePanic(message.text())) {
      runtimePanics.push(message.text());
    }
  });

  page.on('pageerror', (error: Error) => {
    if (isRuntimePanic(error.message)) {
      runtimePanics.push(error.message);
    }
  });

  const getStreamRequests = await mockOfflineWorkspace(page);

  await page.goto('/workspace/default');
  await waitForWorkspacePage(page);

  await expect(page.getByText(/Failed to load: 500 \/api\/tickets\?workspace=default&limit=200/)).toBeVisible();
  await expect(page.getByText('Select a ticket from the sidebar to view details.')).toBeVisible();

  await page.locator('body').click();

  await expect.poll(getStreamRequests, { timeout: 5_000 }).toBeGreaterThanOrEqual(2);
  await expect(page).toHaveURL(/\/workspace\/default$/);
  await expect(page.getByText('Select a ticket from the sidebar to view details.')).toBeVisible();
  await expect(runtimePanics).toEqual([]);
});