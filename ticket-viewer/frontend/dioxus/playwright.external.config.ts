import { defineConfig, devices } from '@playwright/test';

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
    ['html', { outputFolder: 'playwright-report-external', open: 'never' }],
  ],
  outputDir: 'test-results-external',
  use: {
    ...devices['Desktop Chrome'],
    headless: true,
    trace: 'on-first-retry',
  },
});