import { expect, type APIResponse, type Page, type Response, type TestInfo } from '@playwright/test';
import { execFile, spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import fs from 'node:fs/promises';
import path from 'node:path';
import { promisify } from 'node:util';

import { TICKET_VIEWER } from './shared/viewers';
import type { JsonEnvelope } from './mixed-workspace-fixture';

const execFileAsync = promisify(execFile);
const repoRoot = path.resolve(__dirname, '../../../..');
const ticketCli = path.join(repoRoot, 'target', 'debug', 'ticket.exe');

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export function escapeRegex(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

export async function expectJson<T>(response: APIResponse | Response, message: string): Promise<T> {
  expect(response.ok(), message).toBe(true);
  return (await response.json()) as T;
}

export async function attachScreenshot(
  page: Page,
  testInfo: TestInfo,
  name: string,
): Promise<void> {
  await testInfo.attach(name, {
    body: await page.screenshot({ fullPage: true }),
    contentType: 'image/png',
  });
}

export function matchesWorkspaceRequest(
  response: Response,
  pathSuffix: string,
  workspace: string,
  expectedSearchParams: Record<string, string> = {},
): boolean {
  const url = new URL(response.url());
  return (
    response.request().method() === 'GET' &&
    url.pathname.endsWith(pathSuffix) &&
    url.searchParams.get('workspace') === workspace &&
    Object.entries(expectedSearchParams).every(
      ([key, value]) => url.searchParams.get(key) === value,
    )
  );
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

  throw new Error('Could not locate ticket-viewer.exe for seeded mixed-workspace validation.');
}

export async function runTicketCli<T>(args: string[]): Promise<T> {
  const { stdout } = await execFileAsync(ticketCli, args, {
    cwd: repoRoot,
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
      // Server is still starting.
    }

    await sleep(500);
  }

  throw new Error(`Timed out waiting for seeded ticket-viewer at ${url}`);
}

export async function startSeededViewer(indexRoot: string): Promise<{ url: string; viewer: ChildProcessWithoutNullStreams }> {
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

export async function gotoAndWaitForSeededViewer(page: Page, url: string): Promise<void> {
  await page.goto(url, { waitUntil: 'domcontentloaded' });
  await page.locator(TICKET_VIEWER.readySelector).first().waitFor({
    state: 'visible',
    timeout: TICKET_VIEWER.readyTimeout,
  });
}
