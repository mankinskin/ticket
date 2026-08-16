import { defineConfig, devices } from '@playwright/test';
import path from 'node:path';

const dioxusRoot = __dirname;

export default defineConfig({
  testDir: path.join(dioxusRoot, 'e2e-release'),
  timeout: 120_000,
  expect: { timeout: 15_000 },
  fullyParallel: false,
  workers: 1,
  forbidOnly: !!process.env['CI'],
  retries: process.env['CI'] ? 1 : 0,
  reporter: [
    ['list'],
    [
      'html',
      {
        outputFolder: path.join(dioxusRoot, 'playwright-report-release-edge'),
        open: 'never',
      },
    ],
  ],
  outputDir: path.join(dioxusRoot, 'test-results-release-edge'),
  use: {
    ...devices['Desktop Chrome'],
    channel: 'msedge',
    headless: true,
    trace: 'on-first-retry',
  },
});