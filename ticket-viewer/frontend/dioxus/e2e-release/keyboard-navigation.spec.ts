import { expect, test, type Page } from '@playwright/test';
import { execFile, spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { promisify } from 'node:util';

import { resolveActiveWorkspace, TICKET_VIEWER } from './shared/viewers';

const execFileAsync = promisify(execFile);
const repoRoot = path.resolve(__dirname, '../../../..');
const ticketCli = path.join(repoRoot, 'target', 'debug', 'ticket.exe');
const TICKET_VIEWER_URL = process.env.TICKET_VIEWER_URL ?? TICKET_VIEWER.url;

interface TicketListItem {
  id: string;
  title?: string;
}

interface JsonEnvelope<T> {
  payload: T;
}

interface CreatePayload {
  id: string;
}

interface SeededScrollFixture {
  tempDir: string;
  url: string;
  viewer: ChildProcessWithoutNullStreams;
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
      // Try the next candidate.
    }
  }

  throw new Error('Could not locate ticket-viewer.exe for seeded sidebar-scroll validation.');
}

async function runTicketCli<T>(args: string[], cwd = repoRoot): Promise<T> {
  const { stdout } = await execFileAsync(ticketCli, args, {
    cwd,
    windowsHide: true,
  });
  const envelope = JSON.parse(stdout) as JsonEnvelope<T>;
  return envelope.payload;
}

async function waitForViewer(url: string): Promise<void> {
  for (let attempt = 0; attempt < 60; attempt += 1) {
    try {
      const response = await fetch(`${url}/api/workspaces`);
      if (response.ok) {
        return;
      }
    } catch {
      // Viewer is still starting.
    }

    await new Promise((resolve) => setTimeout(resolve, 500));
  }

  throw new Error(`Timed out waiting for seeded ticket-viewer at ${url}`);
}

async function startSeededViewer(
  indexRoot: string,
): Promise<{ url: string; viewer: ChildProcessWithoutNullStreams }> {
  const ticketViewerBin = await resolveTicketViewerBin();

  return await new Promise((resolve, reject) => {
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
        reject(new Error(`Timed out waiting for ticket-viewer port announcement. Output:\n${output}`));
      }
    }, 60_000);

    const onData = async (chunk: Buffer) => {
      output += chunk.toString();
      const match = output.match(/TICKET_VIEWER_PORT=(\d+)/);
      if (!match || settled) {
        return;
      }

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

    viewer.stdout.on('data', (chunk) => {
      void onData(chunk);
    });
    viewer.stderr.on('data', (chunk) => {
      output += chunk.toString();
    });
    viewer.on('exit', (code) => {
      if (!settled) {
        clearTimeout(timeout);
        reject(new Error(`ticket-viewer exited before startup completed (code ${code}). Output:\n${output}`));
      }
    });
  });
}

async function stopSeededViewer(viewer: ChildProcessWithoutNullStreams): Promise<void> {
  if (viewer.exitCode !== null) {
    return;
  }

  viewer.kill();
  await new Promise<void>((resolve) => {
    viewer.once('exit', () => resolve());
    setTimeout(resolve, 5_000);
  });
}

async function seedScrollableWorkspaceFixture(
  ticketCount = 36,
): Promise<SeededScrollFixture> {
  const tempDir = await fs.mkdtemp(path.join(os.tmpdir(), 'ticket-viewer-scroll-'));
  const indexRoot = path.join(tempDir, 'workspace');

  await fs.mkdir(indexRoot, { recursive: true });
  await runTicketCli<unknown>(['init', '--json'], indexRoot);

  for (let index = 0; index < ticketCount; index += 1) {
    await runTicketCli<CreatePayload>([
      '--json',
      '--workspace',
      indexRoot,
      '--index-root',
      indexRoot,
      'create',
      '--type',
      'tracker-improvement',
      '--title',
      `Scroll seed ticket ${String(index + 1).padStart(2, '0')}`,
    ], indexRoot);
  }

  await runTicketCli<unknown>([
    '--json',
    '--workspace',
    indexRoot,
    '--index-root',
    indexRoot,
    'scan',
  ], indexRoot);
  const { url, viewer } = await startSeededViewer(indexRoot);

  return {
    tempDir,
    url,
    viewer,
  };
}

function candidateWords(title: string | undefined): string[] {
  if (!title) {
    return [];
  }

  return title
    .toLowerCase()
    .split(/[^a-z0-9]+/)
    .filter((word) => word.length >= 4);
}

async function gotoWorkspace(page: Page): Promise<void> {
  await gotoWorkspaceAt(page, TICKET_VIEWER_URL);
}

async function gotoWorkspaceAt(page: Page, viewerUrl: string): Promise<void> {
  const workspace = await resolveActiveWorkspace(viewerUrl);
  await page.addInitScript((activeWorkspace) => {
    localStorage.removeItem(`ticket-viewer:${activeWorkspace}:ui`);
  }, workspace);
  await page.goto(`${viewerUrl}/workspace/${workspace}`, {
    waitUntil: 'domcontentloaded',
  });
  await page.locator(TICKET_VIEWER.readySelector).first().waitFor({
    state: 'visible',
    timeout: TICKET_VIEWER.readyTimeout,
  });
}

async function visibleSidebarTicketIds(page: Page): Promise<string[]> {
  return page
    .locator('button[data-testid^="ticket-tree-ticket-"]')
    .evaluateAll((nodes) =>
      nodes
        .map((node) => node.getAttribute('data-testid') ?? '')
        .map((testId) => testId.replace('ticket-tree-ticket-', '')),
    );
}

async function activeSidebarTicketId(page: Page): Promise<string | null> {
  const active = page.locator('button[data-testid^="ticket-tree-ticket-"][aria-selected="true"]').first();
  if ((await active.count()) === 0) {
    return null;
  }

  const testId = await active.getAttribute('data-testid');
  return testId?.replace('ticket-tree-ticket-', '') ?? null;
}

async function sidebarScrollMetrics(
  page: Page,
): Promise<{ clientHeight: number; scrollHeight: number; scrollTop: number }> {
  return page.getByTestId('ticket-tree-scroll-region').evaluate((node) => {
    const element = node as HTMLElement;
    return {
      clientHeight: element.clientHeight,
      scrollHeight: element.scrollHeight,
      scrollTop: element.scrollTop,
    };
  });
}

async function viewportSidebarTicketIds(page: Page): Promise<string[]> {
  return page.getByTestId('ticket-tree-scroll-region').evaluate((node) => {
    const container = node as HTMLElement;
    const containerRect = container.getBoundingClientRect();

    return Array.from(
      container.querySelectorAll('button[data-testid^="ticket-tree-ticket-"]'),
    )
      .filter((candidate) => {
        const rect = (candidate as HTMLElement).getBoundingClientRect();
        return rect.bottom > containerRect.top && rect.top < containerRect.bottom;
      })
      .map((candidate) => candidate.getAttribute('data-testid') ?? '')
      .map((testId) => testId.replace('ticket-tree-ticket-', ''));
  });
}

async function activeSidebarTicketIsVisible(page: Page): Promise<boolean> {
  return page.getByTestId('ticket-tree-scroll-region').evaluate((node) => {
    const container = node as HTMLElement;
    const containerRect = container.getBoundingClientRect();
    const active = container.querySelector(
      'button[data-testid^="ticket-tree-ticket-"][aria-selected="true"]',
    ) as HTMLElement | null;

    if (!active) {
      return false;
    }

    const rect = active.getBoundingClientRect();
    return rect.top >= containerRect.top && rect.bottom <= containerRect.bottom;
  });
}

async function visibleSearchResultIds(page: Page): Promise<string[]> {
  return page
    .locator('button[data-testid^="search-result-"]')
    .evaluateAll((nodes) =>
      nodes
        .map((node) => node.getAttribute('data-testid') ?? '')
        .map((testId) => testId.replace('search-result-', '')),
    );
}

async function activeSearchResultId(page: Page): Promise<string | null> {
  const active = page.locator('button[data-testid^="search-result-"][aria-selected="true"]').first();
  if ((await active.count()) === 0) {
    return null;
  }

  const testId = await active.getAttribute('data-testid');
  return testId?.replace('search-result-', '') ?? null;
}

async function fetchTickets(
  page: Page,
  query?: string,
): Promise<TicketListItem[]> {
  const workspace = await resolveActiveWorkspace(TICKET_VIEWER_URL);
  const params = new URLSearchParams({
    workspace,
    limit: '200',
  });
  if (query) {
    params.set('query', query);
  }

  const response = await page.request.get(
    `${TICKET_VIEWER_URL}/api/tickets?${params.toString()}`,
  );
  expect(response.ok(), `ticket list query failed for ${params.toString()}`).toBe(true);
  const body = await response.json();
  return body?.items ?? [];
}

async function findSearchCandidate(page: Page): Promise<{ query: string; ids: string[] }> {
  const items = await fetchTickets(page);
  const words = new Set<string>();
  for (const item of items) {
    for (const word of candidateWords(item.title)) {
      words.add(word);
    }
  }

  for (const word of words) {
    const matches = await fetchTickets(page, word);
    if (matches.length >= 2) {
      return {
        query: word,
        ids: matches.slice(0, 2).map((item) => item.id),
      };
    }
  }

  throw new Error('No quick-search query with at least two results was found');
}

async function findBroadSearchCandidate(
  page: Page,
  minimumResults: number,
): Promise<{ query: string; ids: string[] }> {
  const items = await fetchTickets(page);
  const words = new Set<string>();
  for (const item of items) {
    for (const word of candidateWords(item.title)) {
      words.add(word);
    }
  }

  for (const word of words) {
    const matches = await fetchTickets(page, word);
    if (matches.length >= minimumResults) {
      return {
        query: word,
        ids: matches.slice(0, minimumResults).map((item) => item.id),
      };
    }
  }

  throw new Error(`No quick-search query with at least ${minimumResults} results was found`);
}

async function findSubstringSearchCandidate(
  page: Page,
): Promise<{ query: string; expectedId: string }> {
  const items = await fetchTickets(page);
  for (const item of items) {
    for (const word of candidateWords(item.title)) {
      if (word.length < 8) {
        continue;
      }

      const substring = word.slice(2, word.length - 1);
      const matches = await fetchTickets(page, substring);
      if (matches.some((candidate) => candidate.id === item.id)) {
        return {
          query: substring,
          expectedId: item.id,
        };
      }
    }
  }

  throw new Error('No quick-search substring candidate could be resolved for the current workspace');
}

test.describe('ticket-viewer — keyboard navigation', () => {
  test('sidebar tree scrolls to deep tickets and keeps the keyboard-active row visible', async ({ page }) => {
    test.setTimeout(180_000);

    const fixture = await seedScrollableWorkspaceFixture();
    try {
      const workspace = await resolveActiveWorkspace(fixture.url);
      await page.addInitScript((activeWorkspace) => {
        localStorage.removeItem(`ticket-viewer:${activeWorkspace}:ui`);
      }, workspace);
      await page.goto(fixture.url, { waitUntil: 'domcontentloaded' });
      await page.locator(TICKET_VIEWER.readySelector).first().waitFor({
        state: 'visible',
        timeout: TICKET_VIEWER.readyTimeout,
      });

      await expect.poll(async () => (await visibleSidebarTicketIds(page)).length).toBeGreaterThan(20);

      const scrollRegion = page.getByTestId('ticket-tree-scroll-region');
      await expect(scrollRegion).toBeVisible();

      const initialMetrics = await sidebarScrollMetrics(page);
      expect(initialMetrics.scrollHeight).toBeGreaterThan(initialMetrics.clientHeight);

      const initialVisibleIds = await viewportSidebarTicketIds(page);
      expect(initialVisibleIds.length).toBeGreaterThan(0);

      await scrollRegion.evaluate((node) => {
        const element = node as HTMLElement;
        element.scrollTop = element.scrollHeight;
      });

      await expect.poll(async () => (await sidebarScrollMetrics(page)).scrollTop).toBeGreaterThan(0);

      const deepVisibleIds = await viewportSidebarTicketIds(page);
      expect(deepVisibleIds.length).toBeGreaterThan(0);
      expect(deepVisibleIds).not.toEqual(initialVisibleIds);

      const deepId = deepVisibleIds[deepVisibleIds.length - 1];
      await page.getByTestId(`ticket-tree-ticket-${deepId}`).click();
      await expect(page.getByTestId(`ticket-tree-ticket-${deepId}`)).toHaveAttribute(
        'data-selected',
        'true',
      );

      await scrollRegion.evaluate((node) => {
        (node as HTMLElement).scrollTop = 0;
      });
      await expect.poll(async () => (await sidebarScrollMetrics(page)).scrollTop).toBe(0);

      const filterInput = page.getByTestId('ticket-tree-filter');
      await filterInput.click();
      await expect(filterInput).toBeFocused();

      for (let i = 0; i < 30; i += 1) {
        await page.keyboard.press('ArrowDown');
      }

      await expect.poll(async () => (await sidebarScrollMetrics(page)).scrollTop).toBeGreaterThan(0);
      await expect.poll(async () => activeSidebarTicketIsVisible(page)).toBe(true);
    } finally {
      await stopSeededViewer(fixture.viewer);
      await fs.rm(fixture.tempDir, { recursive: true, force: true });
    }
  });

  test('sidebar filter keeps focus while arrows move the active ticket and Enter selects it', async ({ page }) => {
    await gotoWorkspace(page);

    await expect.poll(async () => (await visibleSidebarTicketIds(page)).length).toBeGreaterThan(1);

    const filterInput = page.getByTestId('ticket-tree-filter');
    const filterHint = page.getByTestId('ticket-tree-filter-hint');
    await expect(filterHint).toContainText('Free text searches titles');
    await expect(filterHint).toContainText('id:<value>');
    await expect(filterHint).not.toContainText('priority:');
    await filterInput.click();
    await expect(filterInput).toBeFocused();

    const visibleIds = await visibleSidebarTicketIds(page);
    await expect.poll(async () => activeSidebarTicketId(page)).not.toBeNull();
    const currentActiveId = await activeSidebarTicketId(page);
    expect(currentActiveId).toBeTruthy();

    const currentIndex = visibleIds.findIndex((id) => id === currentActiveId);
    expect(currentIndex).toBeGreaterThanOrEqual(0);

    const nextId = visibleIds[Math.min(currentIndex + 1, visibleIds.length - 1)];
    expect(nextId).toBeTruthy();

    await page.keyboard.press('ArrowDown');
    await expect(page.getByTestId(`ticket-tree-ticket-${nextId}`)).toHaveAttribute(
      'aria-selected',
      'true',
    );
    await expect(filterInput).toBeFocused();

    await page.keyboard.press('ArrowUp');
    await expect(page.getByTestId(`ticket-tree-ticket-${currentActiveId}`)).toHaveAttribute(
      'aria-selected',
      'true',
    );
    await expect(filterInput).toBeFocused();

    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('Enter');
    await expect(page.getByTestId(`ticket-tree-ticket-${nextId}`)).toHaveAttribute(
      'data-selected',
      'true',
    );
    await expect(filterInput).toBeFocused();
  });

  test('quick-search keeps input focus while arrows move the active result and Enter opens it', async ({ page }) => {
    const candidate = await findSearchCandidate(page);
    await gotoWorkspace(page);

    await page.locator('body').click();
    await page.keyboard.press('/');

    const searchInput = page.getByTestId('search-input');
    await expect(searchInput).toBeVisible();
    await expect(searchInput).toBeFocused();
    await searchInput.fill(candidate.query);

    await expect.poll(async () => (await visibleSearchResultIds(page)).length).toBeGreaterThan(1);

    const visibleIds = await visibleSearchResultIds(page);
    await expect.poll(async () => activeSearchResultId(page)).not.toBeNull();
    const currentActiveId = await activeSearchResultId(page);
    expect(currentActiveId).toBeTruthy();

    const currentIndex = visibleIds.findIndex((id) => id === currentActiveId);
    expect(currentIndex).toBeGreaterThanOrEqual(0);

    const nextId = visibleIds[Math.min(currentIndex + 1, visibleIds.length - 1)];
    expect(nextId).toBeTruthy();

    const currentResult = page.getByTestId(`search-result-${currentActiveId}`);
    const nextResult = page.getByTestId(`search-result-${nextId}`);
    await expect(currentResult).toHaveAttribute('aria-selected', 'true');

    await page.keyboard.press('ArrowDown');
    await expect(nextResult).toHaveAttribute('aria-selected', 'true');
    await expect(searchInput).toBeFocused();

    await page.keyboard.press('ArrowUp');
    await expect(currentResult).toHaveAttribute('aria-selected', 'true');
    await expect(searchInput).toBeFocused();

    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('Enter');

    await expect(page.getByTestId('search-input')).toHaveCount(0);
    await expect(page.getByTestId(`ticket-tree-ticket-${nextId}`)).toHaveAttribute(
      'data-selected',
      'true',
    );
  });

  test('quick-search documents supported syntax and id predicates narrow results', async ({ page }) => {
    const items = await fetchTickets(page);
    expect(items.length).toBeGreaterThan(0);

    const candidate = items[0];
    const titleCandidate = await findSubstringSearchCandidate(page);
    await gotoWorkspace(page);

    await page.locator('body').click();
    await page.keyboard.press('/');

    const searchInput = page.getByTestId('search-input');
    const syntaxHint = page.getByTestId('search-syntax-hint');
    await expect(searchInput).toBeVisible();
    await expect(searchInput).toHaveAttribute('placeholder', /id:, title:, state:, type:/);
    await expect(syntaxHint).toContainText('Free text searches titles');
    await expect(syntaxHint).toContainText('state:<value>/status:<value>');
    await expect(syntaxHint).toContainText('type:<value>/ticket_type:<value>');
    await expect(syntaxHint).not.toContainText('priority:');

    await searchInput.fill(`id:${candidate.id}`);

    await expect.poll(async () => visibleSearchResultIds(page)).toEqual([candidate.id]);

    await searchInput.fill(`title:${titleCandidate.query}`);

    await expect.poll(async () =>
      (await visibleSearchResultIds(page)).includes(titleCandidate.expectedId),
    ).toBe(true);
  });

  test('quick-search surfaces at least nine results for broad queries', async ({ page }) => {
    const candidate = await findBroadSearchCandidate(page, 9);
    await gotoWorkspace(page);

    await page.locator('body').click();
    await page.keyboard.press('/');

    const searchInput = page.getByTestId('search-input');
    await expect(searchInput).toBeVisible();
    await searchInput.fill(candidate.query);

    await expect.poll(async () => (await visibleSearchResultIds(page)).length).toBeGreaterThanOrEqual(
      candidate.ids.length,
    );
    await expect.poll(async () => (await visibleSearchResultIds(page)).slice(0, candidate.ids.length)).toEqual(
      candidate.ids,
    );
  });

  test('quick-search matches partial-word substrings', async ({ page }) => {
    const candidate = await findSubstringSearchCandidate(page);
    await gotoWorkspace(page);

    await page.locator('body').click();
    await page.keyboard.press('/');

    const searchInput = page.getByTestId('search-input');
    await expect(searchInput).toBeVisible();
    await searchInput.fill(candidate.query);

    await expect.poll(async () =>
      (await visibleSearchResultIds(page)).includes(candidate.expectedId),
    ).toBe(true);
  });
});