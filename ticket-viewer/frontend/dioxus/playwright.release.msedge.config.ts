// Temporary local override: runs the release suite against the system
// Microsoft Edge (Chromium-family) install instead of Playwright's bundled
// Chromium, whose download/extraction is unreliable in this environment.
import { defineConfig, devices } from '@playwright/test';
import baseConfig from './playwright.release.config';

export default defineConfig({
  ...baseConfig,
  use: {
    ...devices['Desktop Chrome'],
    channel: 'msedge',
    headless: false,
    trace: 'on-first-retry',
  },
});
