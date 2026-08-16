import { defineConfig, devices } from '@playwright/test';
import path from 'node:path';

/**
 * Playwright configuration for ticket-viewer release-binary E2E tests.
 *
 * Tests run against the managed `ticket-viewer` server on port 3002. The
 * server is prepared and started automatically through `install-ctl`.
 *
 * Run with:
 *   npm run test:e2e:release
 *   npm run test:e2e:release:headed
 *   npm run test:e2e:release:ui
 */

const repoRoot = path.resolve(__dirname, '../../..');
const RELEASE_SERVER_URL = 'http://127.0.0.1:3002';
const RELEASE_HEALTH_URL = `${RELEASE_SERVER_URL}/healthz`;

export default defineConfig({
  testDir: './e2e-release',
  timeout: 120_000,
  expect: { timeout: 15_000 },
  fullyParallel: false,
  workers: 1,
  forbidOnly: !!process.env['CI'],
  retries: process.env['CI'] ? 1 : 0,
  reporter: [
    ['list'],
    ['html', { outputFolder: 'playwright-report-release', open: 'never' }],
  ],
  outputDir: 'test-results-release',

  use: {
    ...devices['Desktop Chrome'],
    channel: 'msedge',
    headless: false,
    viewport: { width: 1920, height: 1080 },
    screen: { width: 1920, height: 1080 },
    launchOptions: {
      args: ['--start-fullscreen'],
    },
    trace: 'on-first-retry',
  },

  webServer: {
    command: 'node ticket-viewer/frontend/dioxus/e2e-release-server.mjs',
    url: RELEASE_HEALTH_URL,
    cwd: repoRoot,
    reuseExistingServer: false,
    timeout: 300_000,
  },
});