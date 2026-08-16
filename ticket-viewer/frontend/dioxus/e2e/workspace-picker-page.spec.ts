/**
 * End-to-end tests for WorkspacePickerPage — the root route ("/") of the
 * Dioxus ticket-viewer SPA.
 *
 * All `/api/workspaces` calls are intercepted with `page.route()` so no real
 * `ticket serve` backend is required. The `trunk serve` dev server is started
 * automatically by the Playwright `webServer` config.
 *
 * ## trunk serve rebuild resilience
 *
 * `trunk serve` may show a loading state while rebuilding.
 */

import { test, expect, type Page, type Route } from '@playwright/test';

// ── Fixtures ──────────────────────────────────────────────────────────────────

const MOCK_WORKSPACES = {
  workspaces: [{ name: 'default' }, { name: 'my-project' }],
};

const MOCK_WORKSPACES_SINGLE = {
  workspaces: [{ name: 'default' }],
};

const MOCK_WORKSPACES_EMPTY = {
  workspaces: [],
};

// ── Helpers ───────────────────────────────────────────────────────────────────

/**
 * Wait for the Dioxus SPA to be fully ready:
 *  1. Any dx-serve rebuild overlay ("Your app is being rebuilt.") must be gone.
 *  2. The WASM app must have hydrated — confirmed by the h1 heading being visible.
 *
 * Step 1 can take 60-120 s after a Cargo.lock change or first compile.
 * Step 2 ensures assertions against the rendered DOM don't run too early.
 */
async function waitForApp(page: Page): Promise<void> {
  // 1. Wait for any rebuild overlay to disappear.
  await page
    .locator('h3', { hasText: 'Your app is being rebuilt.' })
    .waitFor({ state: 'detached', timeout: 90_000 })
    .catch(() => {
      /* element was never present or already detached — proceed */
    });

  // 2. Wait for the WASM runtime to hydrate and render the page heading.
  //    This guards against `page.evaluate()` running before the component tree
  //    exists. h1 is always rendered (loading / error / success all show it).
  await page
    .getByRole('heading', { level: 1 })
    .waitFor({ state: 'visible', timeout: 30_000 });
}

/** Intercept /api/workspaces and return the supplied payload. */
async function mockWorkspacesOk(page: Page, payload = MOCK_WORKSPACES) {
  await page.route('**/api/workspaces**', (route: Route) => {
    void route.fulfill({ json: payload });
  });
}

/** Intercept /api/workspaces and return a 500 error. */
async function mockWorkspacesError(page: Page, message = 'internal error') {
  await page.route('**/api/workspaces**', (route: Route) => {
    void route.fulfill({
      status: 500,
      contentType: 'text/plain',
      body: message,
    });
  });
}

// ── Tests ─────────────────────────────────────────────────────────────────────

test.describe('WorkspacePickerPage ("/")', () => {
  // ── Static chrome (always visible) ────────────────────────────────────────

  test('does not log the legacy /_dioxus websocket handshake error on startup', async ({
    page,
  }) => {
    const consoleErrors: string[] = [];
    await page.addInitScript(() => {
      localStorage.clear();
      sessionStorage.clear();
    });

    page.on('console', (message) => {
      if (message.type() === 'error') {
        consoleErrors.push(message.text());
      }
    });

    await mockWorkspacesOk(page);
    await page.goto('/');
    await page
      .locator('h3', { hasText: 'Your app is being rebuilt.' })
      .waitFor({ state: 'detached', timeout: 90_000 })
      .catch(() => {
        /* element was never present or already detached — proceed */
      });
    await page
      .getByRole('button', { name: 'Settings' })
      .waitFor({ state: 'visible', timeout: 30_000 });
    await page.waitForTimeout(1500);

    expect(
      consoleErrors.filter(
        (message) =>
          message.includes('/_dioxus')
          || message.includes('Unexpected response code: 200'),
      ),
    ).toEqual([]);
  });

  test('shows the page heading and subtitle', async ({ page }) => {
    await mockWorkspacesOk(page);
    await page.goto('/');
    await waitForApp(page);

    await expect(page.getByRole('heading', { name: 'Ticket Viewer' })).toBeVisible();
    await expect(page.getByText('Select a workspace to begin.')).toBeVisible();
  });

  test('page background is the expected dark colour', async ({ page }) => {
    await mockWorkspacesOk(page);
    await page.goto('/');
    await waitForApp(page);

    // Walk from the h1 upward to find the first ancestor div with an
    // explicit (non-transparent) background — that is the page wrapper.
    const bg = await page.evaluate(() => {
      const h1 = document.querySelector('h1');
      let el: Element | null = h1?.parentElement ?? null;
      while (el) {
        const computed = window.getComputedStyle(el).backgroundColor;
        if (computed !== 'rgba(0, 0, 0, 0)' && computed !== 'transparent') {
          return computed;
        }
        el = el.parentElement;
      }
      return '';
    });
    // #1a1a2e → rgb(26, 26, 46)
    expect(bg).toBe('rgb(26, 26, 46)');
  });

  // ── Loading state ──────────────────────────────────────────────────────────

  test('shows "Loading workspaces…" before the fetch resolves', async ({
    page,
  }) => {
    // Delay the response so we can observe the loading state.
    let resolveHold!: () => void;
    const hold = new Promise<void>((r) => { resolveHold = r; });

    await page.route('**/api/workspaces**', async (route: Route) => {
      await hold;
      void route.fulfill({ json: MOCK_WORKSPACES });
    });

    await page.goto('/');
    await waitForApp(page);

    // Loading text visible while fetch is held.
    await expect(page.getByText('Loading workspaces…')).toBeVisible();

    // Release the hold and verify loading text disappears.
    resolveHold();
    await expect(page.getByText('Loading workspaces…')).not.toBeVisible({
      timeout: 5000,
    });
  });

  // ── Error state ────────────────────────────────────────────────────────────

  test('shows error banner when the workspaces fetch fails', async ({
    page,
  }) => {
    await mockWorkspacesError(page, 'server unavailable');
    await page.goto('/');
    await waitForApp(page);

    const banner = page.getByText(/Failed to load workspaces:/);
    await expect(banner).toBeVisible();

    // Loading text must be gone.
    await expect(page.getByText('Loading workspaces…')).not.toBeVisible();

    // No workspace cards.
    await expect(page.getByText('default')).not.toBeVisible();
  });

  test('error banner has red-tinted styling', async ({ page }) => {
    await mockWorkspacesError(page);
    await page.goto('/');
    await waitForApp(page);

    const banner = page.getByText(/Failed to load workspaces:/).first();
    await expect(banner).toBeVisible();

    const color = await banner.evaluate(
      (el) => window.getComputedStyle(el).color,
    );
    // #ff8080 → rgb(255, 128, 128)
    expect(color).toBe('rgb(255, 128, 128)');
  });

  // ── Success state: workspace cards ────────────────────────────────────────

  test('renders one card per workspace on success', async ({ page }) => {
    await mockWorkspacesOk(page, MOCK_WORKSPACES);
    await page.goto('/');
    await waitForApp(page);

    await expect(page.getByText('default')).toBeVisible();
    await expect(page.getByText('my-project')).toBeVisible();

    // Loading text must be gone.
    await expect(page.getByText('Loading workspaces…')).not.toBeVisible();
  });

  test('shows "No workspaces found." when the list is empty', async ({
    page,
  }) => {
    await mockWorkspacesOk(page, MOCK_WORKSPACES_EMPTY);
    await page.goto('/');
    await waitForApp(page);

    await expect(page.getByText('No workspaces found.')).toBeVisible();

    // Neither loading nor error.
    await expect(page.getByText('Loading workspaces…')).not.toBeVisible();
    await expect(page.getByText(/Failed to load workspaces:/)).not.toBeVisible();
  });

  // ── Navigation ─────────────────────────────────────────────────────────────

  test('clicking a card navigates to /workspace/:name', async ({ page }) => {
    await mockWorkspacesOk(page, MOCK_WORKSPACES_SINGLE);
    await page.goto('/');
    await waitForApp(page);

    await page.getByText('default').click();

    await expect(page).toHaveURL(/\/workspace\/default$/);
  });

  test('each card links to the correct route (href attribute)', async ({
    page,
  }) => {
    await mockWorkspacesOk(page, MOCK_WORKSPACES);
    await page.goto('/');
    await waitForApp(page);

    const link = page.locator('a').filter({ has: page.getByText('my-project') });
    await expect(link).toHaveAttribute('href', '/workspace/my-project');
  });

  test('back-navigation from TicketListPage returns to the picker', async ({
    page,
  }) => {
    // Navigate to the picker, click a card to build history, then go back.
    await mockWorkspacesOk(page, MOCK_WORKSPACES_SINGLE);
    await page.goto('/');
    await waitForApp(page);
    await page.getByText('default').click();
    await expect(page).toHaveURL(/\/workspace\/default/);

    await page.goBack();

    await expect(page).toHaveURL('/');
    await expect(
      page.getByRole('heading', { name: 'Ticket Viewer' }),
    ).toBeVisible();
  });

  // ── Card styling ───────────────────────────────────────────────────────────

  test('cards have dark card background colour', async ({ page }) => {
    await mockWorkspacesOk(page, MOCK_WORKSPACES_SINGLE);
    await page.goto('/');
    await waitForApp(page);

    // The card text is rendered inside an immediate <div> with the background.
    const cardText = page.getByText('default');
    await expect(cardText).toBeVisible();

    const bg = await cardText.evaluate((el) => {
      // Start from el itself (the card div carries the background), then walk
      // up parents as a fallback.
      let node: Element | null = el;
      while (node) {
        const computed = window.getComputedStyle(node).backgroundColor;
        if (computed !== 'rgba(0, 0, 0, 0)' && computed !== 'transparent') {
          return computed;
        }
        node = node.parentElement;
      }
      return '';
    });
    // #16213e → rgb(22, 33, 62)
    expect(bg).toBe('rgb(22, 33, 62)');
  });

  test('cards show workspace names in bold (font-weight ≥ 600)', async ({
    page,
  }) => {
    await mockWorkspacesOk(page, MOCK_WORKSPACES_SINGLE);
    await page.goto('/');
    await waitForApp(page);

    const cardText = page.getByText('default');
    await expect(cardText).toBeVisible();

    const weight = await cardText.evaluate(
      (el) => window.getComputedStyle(el.closest('div') ?? el).fontWeight,
    );
    expect(Number(weight)).toBeGreaterThanOrEqual(600);
  });
});
