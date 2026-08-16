import { test, expect } from '@playwright/test';
import type { ViewerConfig } from './viewers';

/**
 * Suite: POST /api/client-log server endpoint contract.
 */
export function registerClientLogApiSuite(viewer: ViewerConfig): void {
  test.describe(`${viewer.name} — POST /api/client-log`, () => {
    const endpoint = (): string => `${viewer.url}/api/client-log`;

    const minimalRecord = () => ({
      ts: new Date().toISOString(),
      level: 'info',
      target: 'e2e.test',
      message: 'playwright end-to-end test record',
      fields: { source: 'client-log-api-suite' },
    });

    test('valid payload returns 204 No Content', async ({ request }) => {
      const response = await request.post(endpoint(), {
        data: { records: [minimalRecord()] },
      });
      expect(response.status()).toBe(204);
    });

    test('empty records array returns 204 No Content', async ({ request }) => {
      const response = await request.post(endpoint(), {
        data: { records: [] },
      });
      expect(response.status()).toBe(204);
    });

    test('malformed JSON body returns 422 Unprocessable Entity', async ({ request }) => {
      const response = await request.post(endpoint(), {
        headers: { 'Content-Type': 'application/json' },
        data: '{ not: valid json }',
      });
      expect(response.status()).toBe(422);
    });

    test('JSON missing "records" field returns 422 Unprocessable Entity', async ({ request }) => {
      const response = await request.post(endpoint(), {
        data: { events: [minimalRecord()] },
      });
      expect(response.status()).toBe(422);
    });

    test('body exceeding 1 MiB returns 413 Payload Too Large', async ({ request }) => {
      const response = await request.post(endpoint(), {
        data: {
          records: [
            {
              ...minimalRecord(),
              overflow: 'x'.repeat(1_050_000),
            },
          ],
        },
      });
      expect(response.status()).toBe(413);
    });

    test('batch of multiple records returns 204 No Content', async ({ request }) => {
      const batch = Array.from({ length: 10 }, (_, i) => ({
        ...minimalRecord(),
        message: `batch record ${i}`,
      }));
      const response = await request.post(endpoint(), {
        data: { records: batch },
      });
      expect(response.status()).toBe(204);
    });
  });
}