import { expect, type Page } from '@playwright/test';
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

export async function stopSeededViewer(viewer: ChildProcessWithoutNullStreams): Promise<void> {
  if (viewer.exitCode !== null) {
    return;
  }

  viewer.kill();
  await new Promise<void>((resolve) => {
    viewer.once('exit', () => resolve());
    setTimeout(resolve, 5_000);
  });
}

export async function seedScrollableWorkspaceFixture(
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

export async function gotoWorkspace(page: Page): Promise<void> {
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

export async function visibleSidebarTicketIds(page: Page): Promise<string[]> {
  return page
    .locator('button[data-testid^="ticket-tree-ticket-"]')
    .evaluateAll((nodes) =>
      nodes
        .map((node) => node.getAttribute('data-testid') ?? '')
        .map((testId) => testId.replace('ticket-tree-ticket-', '')),
    );
}

export async function activeSidebarTicketId(page: Page): Promise<string | null> {
  const active = page.locator('button[data-testid^="ticket-tree-ticket-"][aria-selected="true"]').first();
  if ((await active.count()) === 0) {
    return null;
  }

  const testId = await active.getAttribute('data-testid');
  return testId?.replace('ticket-tree-ticket-', '') ?? null;
}

export async function sidebarScrollMetrics(
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

export async function viewportSidebarTicketIds(page: Page): Promise<string[]> {
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

export async function activeSidebarTicketIsVisible(page: Page): Promise<boolean> {
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

export async function visibleSearchResultIds(page: Page): Promise<string[]> {
  return page
    .locator('button[data-testid^="search-result-"]')
    .evaluateAll((nodes) =>
      nodes
        .map((node) => node.getAttribute('data-testid') ?? '')
        .map((testId) => testId.replace('search-result-', '')),
    );
}

export async function activeSearchResultId(page: Page): Promise<string | null> {
  const active = page.locator('button[data-testid^="search-result-"][aria-selected="true"]').first();
  if ((await active.count()) === 0) {
    return null;
  }

  const testId = await active.getAttribute('data-testid');
  return testId?.replace('search-result-', '') ?? null;
}

export async function fetchTickets(
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

export async function findSearchCandidate(page: Page): Promise<{ query: string; ids: string[] }> {
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

export async function findBroadSearchCandidate(
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

export async function findSubstringSearchCandidate(
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
