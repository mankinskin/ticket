import { test, expect } from '@playwright/test';
import type { ViewerConfig } from './viewers';

async function loadAndInspectViewer(
  page: import('@playwright/test').Page,
  url: string,
  readySelector: string,
  readyTimeout: number,
): Promise<{ errors: string[]; missingAssets: string[] }> {
  const errors: string[] = [];
  const missingAssets: string[] = [];
  const staticAsset = /\.(js|ts|css|wasm|png|svg|ico|woff2?)(\?.*)?$/i;

  page.on('pageerror', (error) => errors.push(`pageerror: ${error.message}`));
  page.on('console', (message) => {
    if (message.type() === 'error') {
      errors.push(`console.error: ${message.text()}`);
    }
  });
  page.on('response', (response) => {
    if (response.status() === 404 && staticAsset.test(response.url())) {
      missingAssets.push(response.url());
    }
  });

  await page.goto(url, { waitUntil: 'domcontentloaded' });
  await page.locator(readySelector).first().waitFor({
    state: 'visible',
    timeout: readyTimeout,
  });
  await page.waitForTimeout(2_000);
  return { errors, missingAssets };
}

/**
 * Baseline suite contract for the managed ticket-viewer release binary.
 */
export function registerCommonViewerSuite(viewer: ViewerConfig): void {
  test.describe(`${viewer.name} — common suite`, () => {
    test('renders without console errors or uncaught exceptions', async ({ page }) => {
      test.setTimeout(90_000);

      const { errors } = await loadAndInspectViewer(
        page,
        viewer.url,
        viewer.readySelector,
        viewer.readyTimeout,
      );

      expect(errors, `${viewer.name} produced JS errors after loading`).toEqual([]);
    });

    test('no missing static assets (no 404 for JS/CSS/WASM)', async ({ page }) => {
      test.setTimeout(90_000);

      const { missingAssets } = await loadAndInspectViewer(
        page,
        viewer.url,
        viewer.readySelector,
        viewer.readyTimeout,
      );

      expect(missingAssets, `${viewer.name} has missing static assets`).toEqual([]);
    });

    test('ready-selector is visible after load', async ({ page }) => {
      test.setTimeout(90_000);

      await page.goto(viewer.url, { waitUntil: 'domcontentloaded' });
      await expect(page.locator(viewer.readySelector).first()).toBeVisible({
        timeout: viewer.readyTimeout,
      });
    });

    test('root-route header renders Theme settings without a redundant Home action', async ({ page }) => {
      test.setTimeout(90_000);

      await page.goto(viewer.url, { waitUntil: 'domcontentloaded' });
      await expect(page.locator(viewer.readySelector).first()).toBeVisible({
        timeout: viewer.readyTimeout,
      });

      await expect(page.getByRole('button', { name: 'Theme settings' })).toBeVisible();
      await expect(page.getByRole('button', { name: 'Home' })).toHaveCount(0);
    });
  });
}