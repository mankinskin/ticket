import { defineConfig, devices } from '@playwright/test';

/**
 * Playwright configuration for the Dioxus ticket-viewer SPA end-to-end tests.
 *
 * Tests run against a `trunk serve` dev server started automatically before the
 * suite. API calls are intercepted via `page.route()` — no real backend is
 * required.
 *
 * Run with:
 *   npm run test:e2e            (headless)
 *   npm run test:e2e:headed     (visible browser)
 *   npm run test:e2e:ui         (Playwright UI mode)
 */

const DEV_SERVER_URL = 'http://127.0.0.1:8090';

export default defineConfig({
  testDir: './e2e',
  // trunk serve is a single dev-server process; run tests sequentially to avoid
  // parallel navigation fighting over the same WASM rebuild state.
  fullyParallel: false,
  workers: 1,
  forbidOnly: !!process.env['CI'],
  retries: process.env['CI'] ? 1 : 0,
  reporter: [['list'], ['html', { outputFolder: 'playwright-report', open: 'never' }]],

  use: {
    baseURL: DEV_SERVER_URL,
    trace: 'on-first-retry',
    // WASM hydration and trunk serve rebuilds are slower than JS-only apps.
    actionTimeout: 20_000,
    navigationTimeout: 30_000,
  },

  // Per-test timeout covers WASM load + any rebuild overlay wait.
  timeout: 90_000,

  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],

  webServer: {
    command: 'trunk serve --port 8090',
    url: DEV_SERVER_URL,
    // trunk serve keeps compiled WASM in dist/, so subsequent runs are fast.
    reuseExistingServer: !process.env['CI'],
    timeout: 120_000,
    stdout: 'ignore',
    stderr: 'pipe',
  },
});
