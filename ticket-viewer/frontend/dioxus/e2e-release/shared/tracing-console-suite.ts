import { test, expect } from '@playwright/test';
import type { ViewerConfig } from './viewers';

/**
 * Suite: WASM structured tracing — console layer and network sink behaviour.
 */
export function registerTracingConsoleSuite(viewer: ViewerConfig): void {
  test.describe(`${viewer.name} — WASM tracing`, () => {
    test('startup: tracing subscriber emits first record to browser console', async ({ page }) => {
      test.setTimeout(90_000);

      const consoleLines: string[] = [];
      page.on('console', (msg) => consoleLines.push(msg.text()));

      await page.goto(viewer.url, { waitUntil: 'domcontentloaded' });
      await page.locator(viewer.readySelector).first().waitFor({
        state: 'visible',
        timeout: viewer.readyTimeout,
      });
      await page.waitForTimeout(2_000);

      expect(
        consoleLines.some((t) => t.includes('tracing subscriber installed')),
        `${viewer.name}: expected "tracing subscriber installed" record in console.\n` +
          `Captured ${consoleLines.length} lines:\n${consoleLines.slice(0, 20).join('\n')}`,
      ).toBe(true);
    });

    test('default: no POST to /api/client-log without log_sink flag', async ({ page }) => {
      test.setTimeout(30_000);

      const clientLogUrls: string[] = [];
      page.on('request', (req) => {
        if (req.url().includes('/api/client-log')) clientLogUrls.push(req.url());
      });

      await page.goto(viewer.url, { waitUntil: 'domcontentloaded' });
      await page.locator(viewer.readySelector).first().waitFor({
        state: 'visible',
        timeout: viewer.readyTimeout,
      });
      await page.waitForTimeout(5_000);

      expect(
        clientLogUrls,
        `${viewer.name}: unexpected POST to /api/client-log without log_sink flag`,
      ).toEqual([]);
    });

    test('?log_sink=on: network layer posts batched records to /api/client-log', async ({
      page,
    }) => {
      test.setTimeout(30_000);

      const postedBodies: unknown[] = [];
      page.on('request', (req) => {
        if (!req.url().includes('/api/client-log')) return;
        const body = req.postDataJSON();
        if (body) postedBodies.push(body);
      });

      await page.goto(`${viewer.url}?log_sink=on`, { waitUntil: 'domcontentloaded' });
      await page.locator(viewer.readySelector).first().waitFor({
        state: 'visible',
        timeout: viewer.readyTimeout,
      });
      await page.waitForTimeout(5_000);

      expect(
        postedBodies.length,
        `${viewer.name}: no POSTs to /api/client-log with ?log_sink=on`,
      ).toBeGreaterThan(0);

      const first = postedBodies[0] as { records: unknown[] };
      expect(first, 'payload missing "records" field').toHaveProperty('records');
      expect(Array.isArray(first.records), '"records" is not an array').toBe(true);
      expect(first.records.length, '"records" array is empty').toBeGreaterThan(0);
    });

    test('localStorage opt-in: network layer activates when viewer-api-log-sink=on', async ({
      page,
    }) => {
      test.setTimeout(30_000);

      await page.addInitScript(() => {
        localStorage.setItem('viewer-api-log-sink', 'on');
      });

      const clientLogUrls: string[] = [];
      page.on('request', (req) => {
        if (req.url().includes('/api/client-log')) clientLogUrls.push(req.url());
      });

      await page.goto(viewer.url, { waitUntil: 'domcontentloaded' });
      await page.locator(viewer.readySelector).first().waitFor({
        state: 'visible',
        timeout: viewer.readyTimeout,
      });
      await page.waitForTimeout(5_000);

      expect(
        clientLogUrls.length,
        `${viewer.name}: no POSTs to /api/client-log with localStorage opt-in`,
      ).toBeGreaterThan(0);
    });

    test('?log=off: filter blocks all events from reaching the network layer', async ({
      page,
    }) => {
      test.setTimeout(30_000);

      const clientLogUrls: string[] = [];
      page.on('request', (req) => {
        if (req.url().includes('/api/client-log')) clientLogUrls.push(req.url());
      });

      await page.goto(`${viewer.url}?log=off&log_sink=on`, {
        waitUntil: 'domcontentloaded',
      });
      await page.locator(viewer.readySelector).first().waitFor({
        state: 'visible',
        timeout: viewer.readyTimeout,
      });
      await page.waitForTimeout(5_000);

      expect(
        clientLogUrls,
        `${viewer.name}: records reached /api/client-log despite ?log=off`,
      ).toEqual([]);
    });

    test('localStorage filter: viewer-api-log-filter overrides default level', async ({
      page,
    }) => {
      test.setTimeout(30_000);

      await page.addInitScript(() => {
        localStorage.setItem('viewer-api-log-filter', 'off');
      });

      const clientLogUrls: string[] = [];
      page.on('request', (req) => {
        if (req.url().includes('/api/client-log')) clientLogUrls.push(req.url());
      });

      await page.goto(`${viewer.url}?log_sink=on`, { waitUntil: 'domcontentloaded' });
      await page.locator(viewer.readySelector).first().waitFor({
        state: 'visible',
        timeout: viewer.readyTimeout,
      });
      await page.waitForTimeout(5_000);

      expect(
        clientLogUrls,
        `${viewer.name}: records reached /api/client-log despite localStorage filter=off`,
      ).toEqual([]);
    });
  });
}