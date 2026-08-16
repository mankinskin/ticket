/**
 * End-to-end tests verifying that the ticket-viewer search surfaces tickets
 * by partial UUID — both in the quick-search overlay (press '/') and in the
 * sidebar ticket-tree filter.
 *
 * Each test spins up a fresh seeded ticket-viewer instance so it is
 * independent of the default workspace contents. The real Tantivy search
 * index is exercised end-to-end; no mocking.
 *
 * Run with:
 *   npm run test:e2e:release -- search-id-substring
 */

import { expect, test, type Page } from '@playwright/test';
import { execFile, spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);
const repoRoot = path.resolve(__dirname, '../../../..');
const ticketCli = path.join(repoRoot, 'target', 'debug', 'ticket.exe');

// ── Helpers ───────────────────────────────────────────────────────────────────

interface JsonEnvelope<T> {
  payload: T;
}

interface CreatePayload {
  id: string;
}

async function runTicketCli<T>(args: string[], cwd: string): Promise<T> {
  const { stdout } = await execFileAsync(ticketCli, args, {
    cwd,
    windowsHide: true,
  });
  const envelope = JSON.parse(stdout) as JsonEnvelope<T>;
  return envelope.payload;
}

async function resolveTicketViewerBin(): Promise<string> {
  const candidates = [
    process.env.TICKET_VIEWER_BIN,
    path.join(repoRoot, 'target', 'release', 'ticket-viewer.exe'),
    process.env.USERPROFILE
      ? path.join(process.env.USERPROFILE, '.cargo', 'bin', 'ticket-viewer.exe')
      : undefined,
  ].filter((candidate): candidate is string => Boolean(candidate));

  for (const candidate of candidates) {
    try {
      await fs.access(candidate);
      return candidate;
    } catch {
      // try next
    }
  }

  throw new Error('Could not locate ticket-viewer.exe. Build with: cargo build --release -p ticket-viewer');
}

async function waitForViewer(url: string): Promise<void> {
  for (let attempt = 0; attempt < 60; attempt += 1) {
    try {
      const response = await fetch(`${url}/api/workspaces`);
      if (response.ok) {
        return;
      }
    } catch {
      // still starting
    }
    await new Promise((resolve) => setTimeout(resolve, 500));
  }
  throw new Error(`Timed out waiting for seeded ticket-viewer at ${url}`);
}

async function startSeededViewer(
  indexRoot: string,
): Promise<{ url: string; viewer: ChildProcessWithoutNullStreams }> {
  const ticketViewerBin = await resolveTicketViewerBin();

  return new Promise((resolve, reject) => {
    const viewer = spawn(ticketViewerBin, ['--port', '0', '--index-root', indexRoot], {
      cwd: repoRoot,
      stdio: ['ignore', 'pipe', 'pipe'],
      windowsHide: true,
    });

    let settled = false;
    let output = '';

    const timeout = setTimeout(() => {
      if (!settled) {
        viewer.kill();
        reject(new Error(`Timed out waiting for ticket-viewer port announcement.\nOutput:\n${output}`));
      }
    }, 60_000);

    const onData = async (chunk: Buffer) => {
      output += chunk.toString();
      const match = output.match(/TICKET_VIEWER_PORT=(\d+)/);
      if (!match || settled) return;

      settled = true;
      clearTimeout(timeout);
      const url = `http://127.0.0.1:${match[1]}`;
      try {
        await waitForViewer(url);
        resolve({ url, viewer });
      } catch (error) {
        viewer.kill();
        reject(error);
      }
    };

    viewer.stdout.on('data', (chunk) => { void onData(chunk); });
    viewer.stderr.on('data', (chunk) => { output += chunk.toString(); });
    viewer.on('exit', (code) => {
      if (!settled) {
        clearTimeout(timeout);
        reject(new Error(`ticket-viewer exited before startup (code ${code}).\nOutput:\n${output}`));
      }
    });
  });
}

async function stopViewer(viewer: ChildProcessWithoutNullStreams): Promise<void> {
  if (viewer.exitCode !== null) return;
  viewer.kill();
  await new Promise<void>((resolve) => {
    viewer.once('exit', () => resolve());
    setTimeout(resolve, 5_000);
  });
}

/**
 * Query the running viewer to discover its primary workspace name.
 * The name is derived from the index-root path (e.g. "workspace--abc12345"),
 * NOT the hardcoded string "default".
 */
async function getWorkspaceName(viewerUrl: string): Promise<string> {
  const response = await fetch(`${viewerUrl}/api/workspaces`);
  const data = (await response.json()) as { workspaces?: Array<{ name: string }> };
  const name = data.workspaces?.[0]?.name;
  if (!name) throw new Error(`No workspaces returned by ${viewerUrl}/api/workspaces`);
  return name;
}

/** Navigate to the viewer's primary workspace and wait for the page header. */
async function gotoWorkspace(page: Page, viewerUrl: string): Promise<void> {
  const workspaceName = await getWorkspaceName(viewerUrl);
  await page.addInitScript((ws: string) => {
    localStorage.removeItem(`ticket-viewer:${ws}:ui`);
  }, workspaceName);
  await page.goto(`${viewerUrl}/workspace/${workspaceName}`, { waitUntil: 'domcontentloaded' });
  await page.locator('header.header').first().waitFor({ state: 'visible', timeout: 60_000 });
}

/** Collect all `data-testid="search-result-<id>"` button IDs currently in the DOM. */
async function visibleSearchResultIds(page: Page): Promise<string[]> {
  return page
    .locator('button[data-testid^="search-result-"]')
    .evaluateAll((nodes) =>
      nodes
        .map((node) => node.getAttribute('data-testid') ?? '')
        .map((testId) => testId.replace('search-result-', '')),
    );
}

/** Collect all `data-testid="ticket-tree-ticket-<id>"` button IDs in the sidebar. */
async function visibleSidebarTicketIds(page: Page): Promise<string[]> {
  return page
    .locator('button[data-testid^="ticket-tree-ticket-"]')
    .evaluateAll((nodes) =>
      nodes
        .map((node) => node.getAttribute('data-testid') ?? '')
        .map((testId) => testId.replace('ticket-tree-ticket-', '')),
    );
}

// ── Fixtures ──────────────────────────────────────────────────────────────────

interface IdSubstringFixture {
  /** Absolute path to the temp dir (cleaned up in `finally`). */
  tempDir: string;
  /** Base URL of the seeded ticket-viewer. */
  url: string;
  /** Process handle for cleanup. */
  viewer: ChildProcessWithoutNullStreams;
  /** Full UUID of the created ticket. */
  ticketId: string;
  /** First 8 characters of `ticketId` — the short form shown in the UI. */
  ticketIdShort: string;
}

/**
 * Creates a fresh ticket store with a single ticket, scans it into the
 * Tantivy index, and starts a ticket-viewer pointing at that store.
 */
async function seedIdSubstringFixture(): Promise<IdSubstringFixture> {
  const tempDir = await fs.mkdtemp(path.join(os.tmpdir(), 'ticket-viewer-id-sub-'));
  const workspaceRoot = path.join(tempDir, 'workspace');
  const indexRoot = path.join(workspaceRoot, '.ticket');
  await fs.mkdir(workspaceRoot, { recursive: true });

  await runTicketCli<unknown>(
    ['--json', '--workspace', workspaceRoot, '--index-root', indexRoot, 'init'],
    workspaceRoot,
  );

  const created = await runTicketCli<CreatePayload>(
    [
      '--json',
      '--workspace',
      workspaceRoot,
      '--index-root',
      indexRoot,
      'create',
      '--type',
      'tracker-improvement',
      '--title',
      'ID substring search target',
    ],
    workspaceRoot,
  );
  const ticketId: string = created.id;

  // scan builds the Tantivy index so searches work
  await runTicketCli<unknown>(
    ['--json', '--workspace', workspaceRoot, '--index-root', indexRoot, 'scan'],
    workspaceRoot,
  );

  const { url, viewer } = await startSeededViewer(indexRoot);

  return {
    tempDir,
    url,
    viewer,
    ticketId,
    ticketIdShort: ticketId.slice(0, 8),
  };
}

// ── Tests ─────────────────────────────────────────────────────────────────────

test.describe('ticket-viewer — ID substring search', () => {

  // ── Quick-search overlay (press '/' to open) ────────────────────────────

  test('quick-search overlay finds ticket by first-8-char UUID prefix (free text)', async ({ page }) => {
    test.setTimeout(180_000);

    const fixture = await seedIdSubstringFixture();
    try {
      await gotoWorkspace(page, fixture.url);

      // open the quick-search overlay
      await page.locator('body').click();
      await page.keyboard.press('/');

      const searchInput = page.getByTestId('search-input');
      await expect(searchInput).toBeVisible();
      await expect(searchInput).toBeFocused();

      await searchInput.fill(fixture.ticketIdShort);

      await expect.poll(
        async () => (await visibleSearchResultIds(page)).includes(fixture.ticketId),
        { message: `Expected ${fixture.ticketId} in results when searching "${fixture.ticketIdShort}"` },
      ).toBe(true);
    } finally {
      await stopViewer(fixture.viewer);
      await fs.rm(fixture.tempDir, { recursive: true, force: true });
    }
  });

  test('quick-search overlay finds ticket by explicit id:<partial-uuid> prefix query', async ({ page }) => {
    test.setTimeout(180_000);

    const fixture = await seedIdSubstringFixture();
    try {
      await gotoWorkspace(page, fixture.url);

      await page.locator('body').click();
      await page.keyboard.press('/');

      const searchInput = page.getByTestId('search-input');
      await expect(searchInput).toBeVisible();

      // explicit id: prefix with partial UUID
      await searchInput.fill(`id:${fixture.ticketIdShort}`);

      await expect.poll(
        async () => (await visibleSearchResultIds(page)).includes(fixture.ticketId),
        { message: `Expected ${fixture.ticketId} in results when searching "id:${fixture.ticketIdShort}"` },
      ).toBe(true);
    } finally {
      await stopViewer(fixture.viewer);
      await fs.rm(fixture.tempDir, { recursive: true, force: true });
    }
  });

  test('quick-search overlay finds ticket by full UUID (exact id match)', async ({ page }) => {
    test.setTimeout(180_000);

    const fixture = await seedIdSubstringFixture();
    try {
      await gotoWorkspace(page, fixture.url);

      await page.locator('body').click();
      await page.keyboard.press('/');

      const searchInput = page.getByTestId('search-input');
      await expect(searchInput).toBeVisible();

      await searchInput.fill(`id:${fixture.ticketId}`);

      await expect.poll(
        async () => visibleSearchResultIds(page),
        { message: `Expected exactly [${fixture.ticketId}] in results` },
      ).toEqual([fixture.ticketId]);
    } finally {
      await stopViewer(fixture.viewer);
      await fs.rm(fixture.tempDir, { recursive: true, force: true });
    }
  });

  // ── Sidebar ticket-tree filter ──────────────────────────────────────────

  test('sidebar ticket-tree filter finds ticket by first-8-char UUID prefix', async ({ page }) => {
    test.setTimeout(180_000);

    const fixture = await seedIdSubstringFixture();
    try {
      await gotoWorkspace(page, fixture.url);

      // The sidebar should initially show the ticket
      await expect.poll(
        async () => (await visibleSidebarTicketIds(page)).includes(fixture.ticketId),
      ).toBe(true);

      const filterInput = page.getByTestId('ticket-tree-filter');
      await filterInput.click();
      await expect(filterInput).toBeFocused();

      await filterInput.fill(fixture.ticketIdShort);

      // After filtering by partial UUID the ticket must still appear
      await expect.poll(
        async () => (await visibleSidebarTicketIds(page)).includes(fixture.ticketId),
        { message: `Expected ${fixture.ticketId} in sidebar after filtering by "${fixture.ticketIdShort}"` },
      ).toBe(true);
    } finally {
      await stopViewer(fixture.viewer);
      await fs.rm(fixture.tempDir, { recursive: true, force: true });
    }
  });

  test('sidebar ticket-tree filter hides unmatched tickets when filtering by UUID prefix', async ({ page }) => {
    test.setTimeout(180_000);

    // Seed two tickets; filter by the first one's short ID — only that one should appear.
    const tempDir = await fs.mkdtemp(path.join(os.tmpdir(), 'ticket-viewer-id-sub2-'));
    const workspaceRoot = path.join(tempDir, 'workspace');
    const indexRoot = path.join(workspaceRoot, '.ticket');
    await fs.mkdir(workspaceRoot, { recursive: true });
    await runTicketCli<unknown>(
      ['--json', '--workspace', workspaceRoot, '--index-root', indexRoot, 'init'],
      workspaceRoot,
    );

    const first = await runTicketCli<CreatePayload>(
      [
        '--json',
        '--workspace',
        workspaceRoot,
        '--index-root',
        indexRoot,
        'create',
        '--type',
        'tracker-improvement',
        '--title',
        'First ticket',
      ],
      workspaceRoot,
    );
    const second = await runTicketCli<CreatePayload>(
      [
        '--json',
        '--workspace',
        workspaceRoot,
        '--index-root',
        indexRoot,
        'create',
        '--type',
        'tracker-improvement',
        '--title',
        'Second ticket',
      ],
      workspaceRoot,
    );

    await runTicketCli<unknown>(
      ['--json', '--workspace', workspaceRoot, '--index-root', indexRoot, 'scan'],
      workspaceRoot,
    );

    const { url, viewer } = await startSeededViewer(indexRoot);
    try {
      await gotoWorkspace(page, url);

      await expect.poll(async () => (await visibleSidebarTicketIds(page)).length).toBeGreaterThanOrEqual(2);

      const filterInput = page.getByTestId('ticket-tree-filter');
      await filterInput.click();
      await filterInput.fill(first.id.slice(0, 8));

      await expect.poll(
        async () => (await visibleSidebarTicketIds(page)).includes(first.id),
        { message: `Expected ${first.id} to remain visible after UUID-prefix filter` },
      ).toBe(true);

      await expect.poll(
        async () => (await visibleSidebarTicketIds(page)).includes(second.id),
        { message: `Expected ${second.id} to be hidden after UUID-prefix filter for ${first.id.slice(0, 8)}` },
      ).toBe(false);
    } finally {
      await stopViewer(viewer);
      await fs.rm(tempDir, { recursive: true, force: true });
    }
  });
});
