import { expect, test, type Page } from '@playwright/test';
import { execFile, spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { promisify } from 'node:util';

import { TICKET_VIEWER } from './shared/viewers';

const execFileAsync = promisify(execFile);
const repoRoot = path.resolve(__dirname, '../../../..');
const ticketCli = path.join(repoRoot, 'target', 'debug', 'ticket.exe');

interface JsonEnvelope<T> {
  payload: T;
}

interface CreatePayload {
  id: string;
}

interface CreateTicketResponse {
  ticket?: {
    id?: string;
  };
}

interface WorkspacesResponse {
  active_workspace?: string;
  workspaces?: Array<{ name: string }>;
}

interface TicketsResponse {
  items?: Array<{ id: string }>;
}

interface SeededViewerFixture {
  tempDir: string;
  url: string;
  viewer: ChildProcessWithoutNullStreams;
}

interface FilterFixture extends SeededViewerFixture {
  indexRoot: string;
  readyTicketIds: string[];
}

interface MixedWorkspaceFixture extends SeededViewerFixture {
  parentTitle: string;
  childTitle: string;
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function expectJson<T>(response: Response, message: string): Promise<T> {
  expect(response.ok, message).toBe(true);
  return (await response.json()) as T;
}

async function runTicketCli<T>(args: string[]): Promise<T> {
  const { stdout } = await execFileAsync(ticketCli, args, {
    cwd: repoRoot,
    windowsHide: true,
  });
  const envelope = JSON.parse(stdout) as JsonEnvelope<T>;
  return envelope.payload;
}

async function resolveTicketViewerBin(): Promise<string> {
  const candidates = [
    process.env['TICKET_VIEWER_BIN'],
    path.join(repoRoot, 'target', 'release', 'ticket-viewer.exe'),
    process.env['USERPROFILE']
      ? path.join(process.env['USERPROFILE'], '.cargo', 'bin', 'ticket-viewer.exe')
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

  throw new Error('Could not locate ticket-viewer.exe for live-refresh validation.');
}

async function waitForViewer(url: string): Promise<void> {
  for (let attempt = 0; attempt < 60; attempt += 1) {
    try {
      const response = await fetch(`${url}/api/workspaces`);
      if (response.ok) {
        return;
      }
    } catch {
      // Server is still starting.
    }

    await sleep(500);
  }

  throw new Error(`Timed out waiting for seeded ticket-viewer at ${url}`);
}

async function startSeededViewer(indexRoot: string): Promise<{ url: string; viewer: ChildProcessWithoutNullStreams }> {
  const ticketViewerBin = await resolveTicketViewerBin();

  return await new Promise((resolve, reject) => {
    const viewer = spawn(
      ticketViewerBin,
      ['--port', '0', '--index-root', indexRoot],
      {
        cwd: repoRoot,
        stdio: ['ignore', 'pipe', 'pipe'],
        windowsHide: true,
      },
    );

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

    viewer.stdout.on('data', onData);
    viewer.stderr.on('data', (chunk: Buffer) => {
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

async function resolveWorkspaceNames(url: string): Promise<string[]> {
  const response = await fetch(`${url}/api/workspaces`);
  const body = await expectJson<WorkspacesResponse>(response, 'workspace list request failed');
  return body.workspaces?.map((workspace) => workspace.name) ?? [];
}

async function resolveActiveWorkspace(url: string): Promise<string> {
  const response = await fetch(`${url}/api/workspaces`);
  const body = await expectJson<WorkspacesResponse>(response, 'workspace list request failed');
  const workspace = body.active_workspace || body.workspaces?.[0]?.name;
  expect(workspace, 'expected a concrete active workspace name').toBeTruthy();
  return workspace!;
}

async function fetchTicketIds(url: string, workspace: string): Promise<string[]> {
  const response = await fetch(
    `${url}/api/tickets?workspace=${encodeURIComponent(workspace)}&limit=200`,
  );
  const body = await expectJson<TicketsResponse>(response, `ticket list request failed for ${workspace}`);
  return (body.items ?? []).map((item) => item.id);
}

async function visibleTicketIds(page: Page): Promise<string[]> {
  return await page.locator('button[data-testid^="ticket-tree-ticket-"]').evaluateAll((nodes) =>
    nodes
      .map((node) => node.getAttribute('data-testid') ?? '')
      .map((testId) => testId.replace('ticket-tree-ticket-', '')),
  );
}

async function seedFilterFixture(): Promise<FilterFixture> {
  const tempDir = await fs.mkdtemp(path.join(os.tmpdir(), 'ticket-viewer-filter-'));
  const indexRoot = path.join(tempDir, 'workspace');
  await fs.mkdir(indexRoot, { recursive: true });
  await runTicketCli<unknown>([
    'init',
    '--json',
    '--index-root',
    indexRoot,
  ]);

  const readyTicket = await runTicketCli<CreatePayload>([
    'create',
    '--json',
    '--index-root',
    indexRoot,
    '--type',
    'tracker-improvement',
    '--title',
    'Visible ready ticket',
  ]);

  const secondReadyTicket = await runTicketCli<CreatePayload>([
    'create',
    '--json',
    '--index-root',
    indexRoot,
    '--type',
    'tracker-improvement',
    '--title',
    'Second ready ticket',
  ]);

  await runTicketCli<unknown>([
    '--json',
    '--index-root',
    indexRoot,
    'update',
    readyTicket.id,
    '--to-state',
    'planned',
  ]);
  await runTicketCli<unknown>([
    '--json',
    '--index-root',
    indexRoot,
    'update',
    secondReadyTicket.id,
    '--to-state',
    'planned',
  ]);

  const { url, viewer } = await startSeededViewer(indexRoot);
  return {
    tempDir,
    indexRoot,
    url,
    viewer,
    readyTicketIds: [readyTicket.id, secondReadyTicket.id],
  };
}

async function seedMixedWorkspaceFixture(): Promise<MixedWorkspaceFixture> {
  const tempDir = await fs.mkdtemp(path.join(os.tmpdir(), 'ticket-viewer-workspace-race-'));
  const parentDir = path.join(tempDir, 'parent');
  const childDir = path.join(tempDir, 'child');
  const parentTicketsDir = path.join(parentDir, 'tickets');
  const childTicketsDir = path.join(childDir, 'tickets');
  const parentTitle = 'Workspace race parent ticket';
  const childTitle = 'Workspace race child ticket';

  await fs.mkdir(parentDir, { recursive: true });
  await fs.mkdir(childDir, { recursive: true });
  await runTicketCli<unknown>([
    'init',
    '--json',
    '--index-root',
    parentDir,
  ]);
  await runTicketCli<unknown>([
    'init',
    '--json',
    '--index-root',
    childDir,
  ]);
  await fs.mkdir(parentTicketsDir, { recursive: true });
  await fs.mkdir(childTicketsDir, { recursive: true });

  await runTicketCli<CreatePayload>([
    'create',
    '--json',
    '--index-root',
    parentDir,
    '--type',
    'tracker-improvement',
    '--title',
    parentTitle,
  ]);

  await runTicketCli<CreatePayload>([
    'create',
    '--json',
    '--index-root',
    childDir,
    '--type',
    'tracker-improvement',
    '--title',
    childTitle,
  ]);

  await runTicketCli<unknown>([
    'add-root',
    '--json',
    '--index-root',
    parentDir,
    '--label',
    'child',
    childTicketsDir,
  ]);

  await runTicketCli<unknown>([
    'scan',
    '--json',
    '--index-root',
    parentDir,
  ]);

  const { url, viewer } = await startSeededViewer(parentDir);
  return {
    tempDir,
    url,
    viewer,
    parentTitle,
    childTitle,
  };
}

test.describe('ticket-viewer — live refresh authority', () => {
  test('non-matching SSE upserts do not corrupt an active state filter', async ({ page }) => {
    test.setTimeout(120_000);

    const fixture = await seedFilterFixture();
    try {
      const activeWorkspace = await resolveActiveWorkspace(fixture.url);

      await page.goto(`${fixture.url}/`, {
        waitUntil: 'domcontentloaded',
      });
      await page.locator(TICKET_VIEWER.readySelector).first().waitFor({
        state: 'visible',
        timeout: TICKET_VIEWER.readyTimeout,
      });

      const readyChip = page.getByTestId('ticket-tree-state-chip-planned');
      await expect(readyChip).toBeVisible();
      await readyChip.click();

      await expect.poll(async () => {
        const ids = await visibleTicketIds(page);
        return ids.sort();
      }).toEqual([...fixture.readyTicketIds].sort());

      const createResponse = await page.request.post(
        `${fixture.url}/api/tickets?workspace=${encodeURIComponent(activeWorkspace)}`,
        {
          data: {
            type: 'tracker-improvement',
            title: 'Hidden new ticket',
          },
        },
      );
      expect(createResponse.ok(), 'seeded create request failed').toBe(true);
      const createBody = (await createResponse.json()) as CreateTicketResponse;
      const hiddenTicketId = createBody.ticket?.id;
      expect(hiddenTicketId, 'expected created ticket id').toBeTruthy();

      await expect.poll(async () => {
        const ids = await fetchTicketIds(fixture.url, activeWorkspace);
        return ids.includes(hiddenTicketId!);
      }).toBe(true);

      await expect.poll(async () => {
        const ids = await visibleTicketIds(page);
        return ids.sort();
      }).toEqual([...fixture.readyTicketIds].sort());
      await expect(page.getByText('Hidden new ticket')).toHaveCount(0);
    } finally {
      await stopSeededViewer(fixture.viewer);
      await fs.rm(fixture.tempDir, { recursive: true, force: true });
    }
  });

  test('ticket.delete keeps the active state filter authoritative', async ({ page }) => {
    test.setTimeout(120_000);

    const fixture = await seedFilterFixture();
    try {
      const activeWorkspace = await resolveActiveWorkspace(fixture.url);
      const [deletedTicketId, remainingTicketId] = fixture.readyTicketIds;

      await page.goto(`${fixture.url}/`, {
        waitUntil: 'domcontentloaded',
      });
      await page.locator(TICKET_VIEWER.readySelector).first().waitFor({
        state: 'visible',
        timeout: TICKET_VIEWER.readyTimeout,
      });

      const readyChip = page.getByTestId('ticket-tree-state-chip-planned');
      await expect(readyChip).toBeVisible();
      await readyChip.click();

      await expect.poll(async () => {
        const ids = await visibleTicketIds(page);
        return ids.sort();
      }).toEqual([...fixture.readyTicketIds].sort());

      const deleteResponse = await page.request.delete(
        `${fixture.url}/api/tickets/${deletedTicketId}?workspace=${encodeURIComponent(activeWorkspace)}`,
      );
      expect(deleteResponse.ok(), 'seeded delete request failed').toBe(true);

      await expect.poll(async () => {
        const ids = await visibleTicketIds(page);
        return ids.sort();
      }).toEqual([remainingTicketId]);
    } finally {
      await stopSeededViewer(fixture.viewer);
      await fs.rm(fixture.tempDir, { recursive: true, force: true });
    }
  });

  test('previous-workspace responses cannot overwrite the current list', async ({ page }) => {
    test.setTimeout(120_000);

    const fixture = await seedMixedWorkspaceFixture();
    try {
      const activeWorkspace = await resolveActiveWorkspace(fixture.url);
      const workspaceNames = await resolveWorkspaceNames(fixture.url);
      const childWorkspace = workspaceNames.find((workspace) => workspace !== activeWorkspace);
      expect(childWorkspace, 'expected a second workspace for the race test').toBeTruthy();

      const childIds = await fetchTicketIds(fixture.url, childWorkspace!);
      expect(childIds.length, 'expected seeded child workspace ticket').toBeGreaterThan(0);

      let delayedParentList = false;
      const parentRequestPromise = page.waitForRequest((request) => {
        const url = new URL(request.url());
        return (
          url.origin === new URL(fixture.url).origin &&
          url.pathname === '/api/tickets' &&
          url.searchParams.get('workspace') === activeWorkspace
        );
      });
      const parentResponsePromise = page.waitForResponse((response) => {
        if (!response.ok()) {
          return false;
        }

        const url = new URL(response.url());
        return (
          url.origin === new URL(fixture.url).origin &&
          url.pathname === '/api/tickets' &&
          url.searchParams.get('workspace') === activeWorkspace
        );
      });

      await page.route('**/api/tickets?**', async (route) => {
        const url = new URL(route.request().url());
        const isDelayedParentList =
          !delayedParentList &&
          url.origin === new URL(fixture.url).origin &&
          url.pathname === '/api/tickets' &&
          url.searchParams.get('workspace') === activeWorkspace;

        if (isDelayedParentList) {
          delayedParentList = true;
          await new Promise((resolve) => setTimeout(resolve, 1500));
        }

        await route.continue();
      });

      await page.goto(
        `${fixture.url}/workspace/${encodeURIComponent(activeWorkspace)}`,
        { waitUntil: 'domcontentloaded' },
      );
      await page.locator(TICKET_VIEWER.readySelector).first().waitFor({
        state: 'visible',
        timeout: TICKET_VIEWER.readyTimeout,
      });
      await parentRequestPromise;

      const childResponsePromise = page.waitForResponse((response) => {
        if (!response.ok()) {
          return false;
        }

        const url = new URL(response.url());
        return (
          url.origin === new URL(fixture.url).origin &&
          url.pathname === '/api/tickets' &&
          url.searchParams.get('workspace') === childWorkspace
        );
      });

      await page.evaluate((workspace) => {
        window.history.pushState({}, '', `/workspace/${encodeURIComponent(workspace)}`);
        window.dispatchEvent(new PopStateEvent('popstate'));
      }, childWorkspace!);

      await childResponsePromise;
      await parentResponsePromise;

      await expect.poll(async () => {
        const ids = await visibleTicketIds(page);
        return ids.sort();
      }).toEqual([...childIds].sort());
      await expect(page.getByText(fixture.childTitle)).toBeVisible();
      await expect(page.getByText(fixture.parentTitle)).toHaveCount(0);
    } finally {
      await stopSeededViewer(fixture.viewer);
      await fs.rm(fixture.tempDir, { recursive: true, force: true });
    }
  });
});