import { expect, test, type APIResponse, type Page, type Response, type TestInfo } from '@playwright/test';
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

interface TicketFileEntry {
  name: string;
  path: string;
}

interface TicketFilesResponse {
  files?: TicketFileEntry[];
}

interface TicketRefPayload {
  workspace?: string;
  id?: string;
}

interface TicketListItem {
  id: string;
  ticket_ref?: TicketRefPayload;
}

interface TicketsResponse {
  items?: TicketListItem[];
}

interface WorkspacesResponse {
  active_workspace?: string;
  workspaces?: Array<{ name: string; label?: string }>;
}

interface TicketHistoryResponse {
  entries?: Array<unknown>;
}

interface SeededViewerFixture {
  tempDir: string;
  url: string;
  viewer: ChildProcessWithoutNullStreams;
  parentWorkspace: string;
  parentWorkspaceLabel: string;
  parentTicketId: string;
  parentTitle: string;
  parentDescriptionSnippet: string;
  childTicketId: string;
  childWorkspace: string;
  childWorkspaceLabel: string;
  childTitle: string;
  childAcceptanceCriteria: string;
  childImplementationSummary: string;
  childValidationSummary: string;
  childWorkflowStage: string;
  childReleaseTarget: string;
  childDescriptionSnippet: string;
  childAssetSnippet: string;
  secondChildTicketId: string;
  secondChildWorkspace: string;
  secondChildWorkspaceLabel: string;
  secondChildTitle: string;
  secondChildDescriptionSnippet: string;
  secondChildAssetSnippet: string;
}

let fixture: SeededViewerFixture | null = null;

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function escapeRegex(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

async function expectJson<T>(response: APIResponse, message: string): Promise<T> {
  expect(response.ok(), message).toBe(true);
  return (await response.json()) as T;
}

async function attachScreenshot(
  page: Page,
  testInfo: TestInfo,
  name: string,
): Promise<void> {
  await testInfo.attach(name, {
    body: await page.screenshot({ fullPage: true }),
    contentType: 'image/png',
  });
}

function matchesWorkspaceRequest(
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

async function runTicketCli<T>(args: string[]): Promise<T> {
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

async function seedMixedWorkspaceFixture(): Promise<SeededViewerFixture> {
  const tempDir = await fs.mkdtemp(path.join(os.tmpdir(), 'ticket-viewer-mixed-'));
  const parentDir = path.join(tempDir, 'parent');
  const childDir = path.join(parentDir, 'alpha', 'shared');
  const secondChildDir = path.join(parentDir, 'beta', 'shared');
  const parentTicketsDir = path.join(parentDir, '.ticket', 'tickets');
  const childTicketsDir = path.join(childDir, '.ticket', 'tickets');
  const secondChildTicketsDir = path.join(secondChildDir, '.ticket', 'tickets');
  const parentBodyPath = path.join(tempDir, 'parent-description.md');
  const childBodyPath = path.join(tempDir, 'child-description.md');
  const secondChildBodyPath = path.join(tempDir, 'second-child-description.md');
  const childAssetPath = path.join(tempDir, 'child-asset.md');
  const secondChildAssetPath = path.join(tempDir, 'second-child-asset.md');
  const parentWorkspaceLabel = path.basename(parentDir);
  const childWorkspaceLabel = 'shared';
  const secondChildWorkspaceLabel = 'shared';
  const parentTitle = 'Seeded parent ticket';
  const childTitle = 'Seeded first shared ticket';
  const secondChildTitle = 'Seeded second shared ticket';
  const childAcceptanceCriteria = 'Document metadata should expose the remaining top-level ticket fields.';
  const childImplementationSummary = 'Expanded integrated document coverage should render the summary fields inline.';
  const childValidationSummary = 'Playwright confirms extra seeded ticket fields appear in the document view.';
  const childWorkflowStage = 'delivery';
  const childReleaseTarget = '2026.05';
  const parentDescriptionSnippet = 'Default workspace root for mixed-workspace validation.';
  const childDescriptionSnippet = 'First shared-workspace ticket body used by Playwright validation.';
  const secondChildDescriptionSnippet = 'Second shared-workspace ticket body used by Playwright validation.';
  const childAssetSnippet = 'Mixed-workspace asset payload from the first shared workspace.';
  const secondChildAssetSnippet = 'Mixed-workspace asset payload from the second shared workspace.';

  await fs.mkdir(parentTicketsDir, { recursive: true });
  await fs.mkdir(childTicketsDir, { recursive: true });
  await fs.mkdir(secondChildTicketsDir, { recursive: true });
  await fs.writeFile(parentBodyPath, `# Parent ticket\n\n${parentDescriptionSnippet}\n`);
  await fs.writeFile(childBodyPath, `# Child ticket\n\n${childDescriptionSnippet}\n`);
  await fs.writeFile(secondChildBodyPath, `# Second child ticket\n\n${secondChildDescriptionSnippet}\n`);
  await fs.writeFile(childAssetPath, `# Asset\n\n${childAssetSnippet}\n`);
  await fs.writeFile(secondChildAssetPath, `# Asset\n\n${secondChildAssetSnippet}\n`);

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
  await runTicketCli<unknown>([
    'init',
    '--json',
    '--index-root',
    secondChildDir,
  ]);

  const parentTicket = await runTicketCli<CreatePayload>([
    'create',
    '--json',
    '--index-root',
    parentDir,
    '--type',
    'tracker-improvement',
    '--title',
    parentTitle,
    '--body-file',
    parentBodyPath,
  ]);

  const childTicket = await runTicketCli<CreatePayload>([
    'create',
    '--json',
    '--index-root',
    childDir,
    '--type',
    'tracker-improvement',
    '--title',
    childTitle,
    '--field',
    'component=ticket-viewer',
    '--field',
    'priority=high',
    '--field',
    'risk_level=medium',
    '--field',
    'tags=layout,document',
    '--field',
    'spec_refs=seeded-spec-01',
    '--field',
    `acceptance_criteria=${childAcceptanceCriteria}`,
    '--field',
    `implementation_summary=${childImplementationSummary}`,
    '--field',
    `validation_summary=${childValidationSummary}`,
    '--field',
    `workflow_stage=${childWorkflowStage}`,
    '--field',
    `release_target=${childReleaseTarget}`,
    '--body-file',
    childBodyPath,
  ]);

  const secondChildTicket = await runTicketCli<CreatePayload>([
    'create',
    '--json',
    '--index-root',
    secondChildDir,
    '--type',
    'tracker-improvement',
    '--title',
    secondChildTitle,
    '--body-file',
    secondChildBodyPath,
  ]);

  await runTicketCli<unknown>([
    'attach',
    '--json',
    '--index-root',
    childDir,
    childTicket.id,
    childAssetPath,
    '--as',
    'notes.md',
  ]);

  await runTicketCli<unknown>([
    'attach',
    '--json',
    '--index-root',
    secondChildDir,
    secondChildTicket.id,
    secondChildAssetPath,
    '--as',
    'notes.md',
  ]);

  await runTicketCli<unknown>([
    'add-root',
    '--json',
    '--index-root',
    parentDir,
    '--label',
    'shared-alpha',
    childTicketsDir,
  ]);

  await runTicketCli<unknown>([
    'add-root',
    '--json',
    '--index-root',
    parentDir,
    '--label',
    'shared-beta',
    secondChildTicketsDir,
  ]);

  await runTicketCli<unknown>([
    'scan',
    '--json',
    '--index-root',
    parentDir,
  ]);

  const { url, viewer } = await startSeededViewer(parentDir);
  const workspaceResponse = await fetch(`${url}/api/workspaces`);
  expect(
    workspaceResponse.ok,
    'seeded workspace list must respond',
  ).toBe(true);
  const workspaces = (await workspaceResponse.json()) as WorkspacesResponse;
  const parentWorkspace =
    workspaces.active_workspace ?? workspaces.workspaces?.[0]?.name;
  expect(
    parentWorkspace,
    'seeded parent workspace should resolve to a canonical workspace id',
  ).toBeTruthy();
  const sharedWorkspaceInfos = (workspaces.workspaces ?? []).filter(
    (workspace) => workspace.name !== parentWorkspace && workspace.label === childWorkspaceLabel,
  );
  expect(
    sharedWorkspaceInfos,
    'seeded viewer should expose both duplicate-basename child workspaces',
  ).toHaveLength(2);

  let childWorkspaceInfo: { name: string; label?: string } | undefined;
  let secondChildWorkspaceInfo: { name: string; label?: string } | undefined;

  for (const workspace of sharedWorkspaceInfos) {
    expect(
      workspace.name.startsWith('shared--'),
      'duplicate-basename child workspaces should use canonical shared-- ids',
    ).toBe(true);

    const ticketListResponse = await fetch(
      `${url}/api/tickets?workspace=${encodeURIComponent(workspace.name)}&limit=200`,
    );
    expect(
      ticketListResponse.ok,
      `seeded ticket list for ${workspace.name} must respond`,
    ).toBe(true);

    const ticketList = (await ticketListResponse.json()) as TicketsResponse;
    const listedIds = new Set((ticketList.items ?? []).map((ticket) => ticket.id));

    if (listedIds.has(childTicket.id)) {
      childWorkspaceInfo = workspace;
    }
    if (listedIds.has(secondChildTicket.id)) {
      secondChildWorkspaceInfo = workspace;
    }
  }

  const childWorkspace = childWorkspaceInfo?.name;
  expect(
    childWorkspace,
    'seeded first shared workspace should resolve to a canonical workspace id',
  ).toBeTruthy();
  const secondChildWorkspace = secondChildWorkspaceInfo?.name;
  expect(
    secondChildWorkspace,
    'seeded second shared workspace should resolve to a canonical workspace id',
  ).toBeTruthy();
  expect(
    childWorkspace,
    'duplicate-basename child workspaces must not collapse onto the same canonical id',
  ).not.toBe(secondChildWorkspace);

  return {
    tempDir,
    url,
    viewer,
    parentWorkspace: parentWorkspace!,
    parentWorkspaceLabel,
    parentTicketId: parentTicket.id,
    parentTitle,
    parentDescriptionSnippet,
    childTicketId: childTicket.id,
    childWorkspace: childWorkspace!,
    childWorkspaceLabel: childWorkspaceInfo?.label ?? childWorkspaceLabel,
    childTitle,
    childAcceptanceCriteria,
    childImplementationSummary,
    childValidationSummary,
    childWorkflowStage,
    childReleaseTarget,
    childDescriptionSnippet,
    childAssetSnippet,
    secondChildTicketId: secondChildTicket.id,
    secondChildWorkspace: secondChildWorkspace!,
    secondChildWorkspaceLabel:
      secondChildWorkspaceInfo?.label ?? secondChildWorkspaceLabel,
    secondChildTitle,
    secondChildDescriptionSnippet,
    secondChildAssetSnippet,
  };
}

async function gotoAndWaitForSeededViewer(page: Page, url: string): Promise<void> {
  await page.goto(url, { waitUntil: 'domcontentloaded' });
  await page.locator(TICKET_VIEWER.readySelector).first().waitFor({
    state: 'visible',
    timeout: TICKET_VIEWER.readyTimeout,
  });
}

test.beforeAll(async () => {
  fixture = await seedMixedWorkspaceFixture();
});

test.afterAll(async () => {
  if (!fixture) {
    return;
  }

  await stopSeededViewer(fixture.viewer);
  await fs.rm(fixture.tempDir, { recursive: true, force: true });
  fixture = null;
});

test('root route follows mixed-workspace ticket refs for history and file actions', async ({ page }) => {
  test.setTimeout(120_000);

  expect(fixture, 'seeded viewer fixture must be ready').not.toBeNull();
  const currentFixture = fixture!;
  const escapedBaseUrl = escapeRegex(currentFixture.url);

  await gotoAndWaitForSeededViewer(page, currentFixture.url);
  await expect(page).toHaveURL(new RegExp(`^${escapedBaseUrl}/(?:#.*)?$`));

  const ticketsResponse = await expectJson<TicketsResponse>(
    await page.request.get(
      `${currentFixture.url}/api/tickets?workspace=${encodeURIComponent(currentFixture.parentWorkspace)}&limit=200`,
    ),
    'seeded ticket list must respond',
  );
  const listedChildTicket = (ticketsResponse.items ?? []).find(
    (ticket) => ticket.id === currentFixture.childTicketId,
  );
  expect(listedChildTicket, 'seeded child ticket must appear in the default workspace list').toBeTruthy();
  expect(
    listedChildTicket?.ticket_ref?.workspace,
    'seeded child ticket list item must retain its owning workspace ref',
  ).toBe(currentFixture.childWorkspace);

  const ticketButton = page.getByTestId(`ticket-tree-ticket-${currentFixture.childTicketId}`);
  await expect(ticketButton).toBeVisible({ timeout: 30_000 });
  await expect(ticketButton).toContainText(currentFixture.childTitle);

  const historyResponsePromise = page.waitForResponse((response) => {
    return matchesWorkspaceRequest(
      response,
      `/api/tickets/${currentFixture.childTicketId}/history`,
      currentFixture.childWorkspace,
    );
  });
  await ticketButton.click();
  const historyResponse = await historyResponsePromise;
  expect(historyResponse.ok(), 'mixed-workspace history request must succeed').toBe(true);
  await expect(page).toHaveURL(
    new RegExp(
      `^${escapedBaseUrl}/#(?=.*ticket-id=${currentFixture.childTicketId})(?=.*ticket-workspace=${currentFixture.childWorkspace}).*$`,
    ),
  );
  await expect(page.getByTestId('ticket-detail-panel')).toBeVisible();
  await expect(page.getByTestId('ticket-content')).toBeVisible();

  const seededHistory = await expectJson<TicketHistoryResponse>(
    await page.request.get(
      `${currentFixture.url}/api/tickets/${currentFixture.childTicketId}/history?workspace=${encodeURIComponent(currentFixture.childWorkspace)}`,
    ),
    'seeded child history must respond',
  );
  expect(
    seededHistory.entries?.length ?? 0,
    'seeded child ticket should have visible history entries for the history tab flow',
  ).toBeGreaterThan(0);

  const row = page.getByTestId(`ticket-tree-row-${currentFixture.childTicketId}`);
  const filesResponsePromise = page.waitForResponse((response) => {
    return matchesWorkspaceRequest(
      response,
      `/api/tickets/${currentFixture.childTicketId}/files`,
      currentFixture.childWorkspace,
    );
  });
  await row.locator('button').first().click();
  const filesResponse = await filesResponsePromise;
  expect(filesResponse.ok(), 'mixed-workspace file list request must succeed').toBe(true);

  const fileList = await expectJson<TicketFilesResponse>(
    filesResponse,
    'mixed-workspace file list JSON must parse',
  );
  expect(fileList.files?.length ?? 0, 'mixed-workspace file list should return entries').toBeGreaterThan(0);

  const historyTab = page.getByTestId('tab-history');
  await expect(historyTab).toBeVisible();
  await historyTab.click();
  await expect(page.getByTestId('history-panel')).toBeVisible({ timeout: 20_000 });
  await expect(page).toHaveURL(
    new RegExp(
      `^${escapedBaseUrl}/#(?=.*ticket-id=${currentFixture.childTicketId})(?=.*ticket-workspace=${currentFixture.childWorkspace}).*$`,
    ),
  );
  await expect(page).not.toHaveURL(/\/workspace\/default/);
});

test('workspace route shows the display label while retaining the canonical workspace id', async ({ page }) => {
  test.setTimeout(120_000);

  expect(fixture, 'seeded viewer fixture must be ready').not.toBeNull();
  const currentFixture = fixture!;
  const escapedBaseUrl = escapeRegex(currentFixture.url);
  const escapedWorkspace = escapeRegex(currentFixture.childWorkspace);

  await page.goto(
    `${currentFixture.url}/workspace/${encodeURIComponent(currentFixture.childWorkspace)}`,
    { waitUntil: 'domcontentloaded' },
  );
  await page.locator(TICKET_VIEWER.readySelector).first().waitFor({
    state: 'visible',
    timeout: TICKET_VIEWER.readyTimeout,
  });

  await expect(page).toHaveURL(
    new RegExp(`^${escapedBaseUrl}/workspace/${escapedWorkspace}(?:#.*)?$`),
  );
  await expect(page.locator('.header-title').first()).toHaveText(
    currentFixture.childWorkspaceLabel,
  );
  if (currentFixture.childWorkspace !== currentFixture.childWorkspaceLabel) {
    await expect(page.locator('.header-subtitle').first()).toHaveText(
      currentFixture.childWorkspace,
    );
  } else {
    await expect(page.locator('.header-subtitle')).toHaveCount(0);
  }
});

test('colliding workspace routes keep list and follow-up requests pinned to canonical ids', async ({ page }) => {
  test.setTimeout(120_000);

  expect(fixture, 'seeded viewer fixture must be ready').not.toBeNull();
  const currentFixture = fixture!;
  const escapedBaseUrl = escapeRegex(currentFixture.url);
  const sharedWorkspaces = [
    {
      workspace: currentFixture.childWorkspace,
      label: currentFixture.childWorkspaceLabel,
      ticketId: currentFixture.childTicketId,
      title: currentFixture.childTitle,
      descriptionSnippet: currentFixture.childDescriptionSnippet,
      assetSnippet: currentFixture.childAssetSnippet,
    },
    {
      workspace: currentFixture.secondChildWorkspace,
      label: currentFixture.secondChildWorkspaceLabel,
      ticketId: currentFixture.secondChildTicketId,
      title: currentFixture.secondChildTitle,
      descriptionSnippet: currentFixture.secondChildDescriptionSnippet,
      assetSnippet: currentFixture.secondChildAssetSnippet,
    },
  ] as const;

  for (const workspaceFixture of sharedWorkspaces) {
    const otherWorkspace = sharedWorkspaces.find(
      (candidate) => candidate.workspace !== workspaceFixture.workspace,
    );
    expect(
      workspaceFixture.workspace.startsWith('shared--'),
      'duplicate-basename workspace routes must use canonical shared-- ids',
    ).toBe(true);

    const listResponsePromise = page.waitForResponse((response) => {
      return matchesWorkspaceRequest(
        response,
        '/api/tickets',
        workspaceFixture.workspace,
      );
    });

    await page.goto(
      `${currentFixture.url}/workspace/${encodeURIComponent(workspaceFixture.workspace)}`,
      { waitUntil: 'domcontentloaded' },
    );
    await page.locator(TICKET_VIEWER.readySelector).first().waitFor({
      state: 'visible',
      timeout: TICKET_VIEWER.readyTimeout,
    });

    const listResponse = await listResponsePromise;
    expect(
      listResponse.ok(),
      `workspace list request for ${workspaceFixture.workspace} must succeed`,
    ).toBe(true);

    await expect(page).toHaveURL(
      new RegExp(
        `^${escapedBaseUrl}/workspace/${escapeRegex(workspaceFixture.workspace)}(?:#.*)?$`,
      ),
    );
    await expect(page.locator('.header-title').first()).toHaveText(
      workspaceFixture.label,
    );
    await expect(page.locator('.header-subtitle').first()).toHaveText(
      workspaceFixture.workspace,
    );

    const ticketButton = page.getByTestId(
      `ticket-tree-ticket-${workspaceFixture.ticketId}`,
    );
    await expect(ticketButton).toBeVisible({ timeout: 30_000 });
    await expect(ticketButton).toContainText(workspaceFixture.title);

    const detailResponsePromise = page.waitForResponse((response) => {
      return matchesWorkspaceRequest(
        response,
        `/api/tickets/${workspaceFixture.ticketId}`,
        workspaceFixture.workspace,
      );
    });
    const historyResponsePromise = page.waitForResponse((response) => {
      return matchesWorkspaceRequest(
        response,
        `/api/tickets/${workspaceFixture.ticketId}/history`,
        workspaceFixture.workspace,
      );
    });

    await ticketButton.click();

    const [detailResponse, historyResponse] = await Promise.all([
      detailResponsePromise,
      historyResponsePromise,
    ]);
    expect(
      detailResponse.ok(),
      `ticket detail request for ${workspaceFixture.workspace} must succeed`,
    ).toBe(true);
    expect(
      historyResponse.ok(),
      `ticket history request for ${workspaceFixture.workspace} must succeed`,
    ).toBe(true);

    await expect(page).toHaveURL(
      new RegExp(
        `^${escapedBaseUrl}/workspace/${escapeRegex(workspaceFixture.workspace)}#(?=.*ticket-id=${workspaceFixture.ticketId})(?=.*ticket-workspace=${workspaceFixture.workspace}).*$`,
      ),
    );
    await expect(page.getByTestId('desc-markdown')).toContainText(
      workspaceFixture.descriptionSnippet,
    );
    if (otherWorkspace) {
      await expect(page.getByTestId('desc-markdown')).not.toContainText(
        otherWorkspace.descriptionSnippet,
      );
    }

    const row = page.getByTestId(`ticket-tree-row-${workspaceFixture.ticketId}`);
    const rowEntry = row.locator('xpath=..');
    const filesResponsePromise = page.waitForResponse((response) => {
      return matchesWorkspaceRequest(
        response,
        `/api/tickets/${workspaceFixture.ticketId}/files`,
        workspaceFixture.workspace,
      );
    });

    await row.locator('button').first().click();
    const filesResponse = await filesResponsePromise;
    expect(
      filesResponse.ok(),
      `ticket files request for ${workspaceFixture.workspace} must succeed`,
    ).toBe(true);

    const fileList = await expectJson<TicketFilesResponse>(
      filesResponse,
      `ticket files for ${workspaceFixture.workspace} must parse`,
    );
    const asset = (fileList.files ?? []).find((file) => file.path !== 'description.md');
    expect(asset, `ticket asset for ${workspaceFixture.workspace} should exist`).toBeTruthy();

    const assetButton = rowEntry.getByTestId(
      `ticket-tree-file-${workspaceFixture.ticketId}-${asset!.name}`,
    );
    await expect(assetButton).toBeVisible();

    const assetResponsePromise = page.waitForResponse((response) => {
      return matchesWorkspaceRequest(
        response,
        `/api/tickets/${workspaceFixture.ticketId}/asset`,
        workspaceFixture.workspace,
        { path: asset!.path },
      );
    });

    await assetButton.click();
    const assetResponse = await assetResponsePromise;
    expect(
      assetResponse.ok(),
      `ticket asset request for ${workspaceFixture.workspace} must succeed`,
    ).toBe(true);

    await expect(page.getByTestId('ticket-content')).toContainText(
      workspaceFixture.assetSnippet,
    );
    if (otherWorkspace) {
      await expect(page.getByTestId('ticket-content')).not.toContainText(
        otherWorkspace.assetSnippet,
      );
    }
  }
});

test('root route swaps content panel body when clicking different ticket rows', async ({ page }) => {
  test.setTimeout(120_000);

  expect(fixture, 'seeded viewer fixture must be ready').not.toBeNull();
  const currentFixture = fixture!;
  const escapedBaseUrl = escapeRegex(currentFixture.url);

  await gotoAndWaitForSeededViewer(page, currentFixture.url);
  await page.evaluate(() => window.localStorage.clear());
  await gotoAndWaitForSeededViewer(page, currentFixture.url);
  await expect(page.getByText('Select a ticket from the sidebar to view details.')).toBeVisible();

  const parentButton = page.getByTestId(`ticket-tree-ticket-${currentFixture.parentTicketId}`);
  const childButton = page.getByTestId(`ticket-tree-ticket-${currentFixture.childTicketId}`);
  await expect(parentButton).toBeVisible({ timeout: 30_000 });
  await expect(childButton).toBeVisible({ timeout: 30_000 });
  await expect(parentButton).toContainText(currentFixture.parentTitle);
  await expect(childButton).toContainText(currentFixture.childTitle);

  await parentButton.click();
  await expect(page.getByTestId('desc-markdown')).toContainText(currentFixture.parentDescriptionSnippet);
  await expect(page).toHaveURL(
    new RegExp(`^${escapedBaseUrl}/#(?=.*ticket-id=${currentFixture.parentTicketId}).*$`),
  );

  await childButton.click();
  await expect(page.getByTestId('desc-markdown')).toContainText(currentFixture.childDescriptionSnippet);
  await expect(page.getByTestId('desc-markdown')).not.toContainText(currentFixture.parentDescriptionSnippet);
  await expect(page).toHaveURL(
    new RegExp(
      `^${escapedBaseUrl}/#(?=.*ticket-id=${currentFixture.childTicketId})(?=.*ticket-workspace=${currentFixture.childWorkspace}).*$`,
    ),
  );
});

test('content mode renders an integrated ticket document and keeps asset context inline', async ({ page }, testInfo) => {
  test.setTimeout(120_000);

  expect(fixture, 'seeded viewer fixture must be ready').not.toBeNull();
  const currentFixture = fixture!;

  await gotoAndWaitForSeededViewer(page, currentFixture.url);

  const childButton = page.getByTestId(`ticket-tree-ticket-${currentFixture.childTicketId}`);
  await expect(childButton).toBeVisible({ timeout: 30_000 });
  await childButton.click();

  await page.getByRole('button', { name: 'Content' }).click();

  await expect(page.getByTestId('tab-description')).toHaveText('Document');
  await expect(page.getByTestId('ticket-document-header')).toContainText(currentFixture.childTitle);
  await expect(page.getByTestId('ticket-document-metadata')).toContainText(currentFixture.childWorkspace);
  await expect(page.getByTestId('ticket-document-metadata')).toContainText(currentFixture.childTicketId);
  await expect(page.getByTestId('desc-markdown')).toContainText(currentFixture.childDescriptionSnippet);
  await expect(page.getByTestId('ticket-document-fields')).toContainText('Acceptance Criteria');
  await expect(page.getByTestId('ticket-document-fields')).toContainText(currentFixture.childAcceptanceCriteria);
  await expect(page.getByTestId('ticket-document-fields')).toContainText('Implementation Summary');
  await expect(page.getByTestId('ticket-document-fields')).toContainText(currentFixture.childImplementationSummary);
  await expect(page.getByTestId('ticket-document-fields')).toContainText('Validation Summary');
  await expect(page.getByTestId('ticket-document-fields')).toContainText(currentFixture.childValidationSummary);
  await expect(page.getByTestId('ticket-document-fields')).toContainText('Workflow Stage');
  await expect(page.getByTestId('ticket-document-fields')).toContainText(currentFixture.childWorkflowStage);
  await expect(page.getByTestId('ticket-document-fields')).toContainText('Release Target');
  await expect(page.getByTestId('ticket-document-fields')).toContainText(currentFixture.childReleaseTarget);
  await expect(page.getByTestId('ticket-detail-panel')).toHaveCount(0);

  await gotoAndWaitForSeededViewer(
    page,
    `${currentFixture.url}/workspace/${encodeURIComponent(currentFixture.parentWorkspace)}#ticket-id=${currentFixture.childTicketId}&ticket-workspace=${encodeURIComponent(currentFixture.parentWorkspace)}`,
  );
  await page.getByRole('button', { name: 'Content' }).click();
  await expect(page.getByTestId('desc-markdown')).toContainText(currentFixture.childDescriptionSnippet);
  await expect(page.getByTestId('ticket-document-header')).toContainText(currentFixture.childTitle);
  await expect(page.getByTestId('ticket-document')).toContainText('tracker-improvement');
  await expect(page.getByTestId('ticket-document-metadata')).toContainText(currentFixture.childWorkspace);
  await expect(page.getByTestId('ticket-document-metadata')).toContainText(currentFixture.childTicketId);

  const row = page.getByTestId(`ticket-tree-row-${currentFixture.childTicketId}`);
  const rowEntry = row.locator('xpath=..');
  const filesResponsePromise = page.waitForResponse((response) => {
    return matchesWorkspaceRequest(
      response,
      `/api/tickets/${currentFixture.childTicketId}/files`,
      currentFixture.childWorkspace,
    );
  });
  await row.locator('button').first().click();
  const filesResponse = await filesResponsePromise;
  expect(filesResponse.ok(), 'mixed-workspace file list request must succeed').toBe(true);

  const fileList = await expectJson<TicketFilesResponse>(
    filesResponse,
    'mixed-workspace file list JSON must parse',
  );
  const asset = (fileList.files ?? []).find((file) => file.path !== 'description.md');
  expect(asset, 'seeded child ticket should expose a non-description asset').toBeTruthy();

  const assetButton = rowEntry.getByTestId(
    `ticket-tree-file-${currentFixture.childTicketId}-${asset!.name}`,
  );
  await expect(assetButton).toBeVisible();

  const assetResponsePromise = page.waitForResponse((response) => {
    return matchesWorkspaceRequest(
      response,
      `/api/tickets/${currentFixture.childTicketId}/asset`,
      currentFixture.childWorkspace,
      { path: asset!.path },
    );
  });
  await assetButton.click();
  const assetResponse = await assetResponsePromise;
  expect(assetResponse.ok(), 'mixed-workspace asset request must succeed').toBe(true);

  await expect(page.getByTestId('ticket-document-asset-banner')).toContainText(asset!.name);
  await expect(page.getByTestId('ticket-document-header')).toContainText(currentFixture.childTitle);
  await expect(page.getByTestId('ticket-content')).toContainText(currentFixture.childAssetSnippet);

  await attachScreenshot(page, testInfo, 'mixed-workspace-integrated-document');
});

test('asset file click should trigger owning-workspace asset request', async ({ page }) => {
  test.setTimeout(120_000);

  expect(fixture, 'seeded viewer fixture must be ready').not.toBeNull();
  const currentFixture = fixture!;

  await gotoAndWaitForSeededViewer(page, currentFixture.url);

  const ticketButton = page.getByTestId(`ticket-tree-ticket-${currentFixture.childTicketId}`);
  await expect(ticketButton).toBeVisible({ timeout: 30_000 });
  await ticketButton.click();

  const row = page.getByTestId(`ticket-tree-row-${currentFixture.childTicketId}`);
  const rowEntry = row.locator('xpath=..');
  const filesResponsePromise = page.waitForResponse((response) => {
    return matchesWorkspaceRequest(
      response,
      `/api/tickets/${currentFixture.childTicketId}/files`,
      currentFixture.childWorkspace,
    );
  });
  await row.locator('button').first().click();
  const filesResponse = await filesResponsePromise;
  expect(filesResponse.ok(), 'mixed-workspace file list request must succeed').toBe(true);

  const fileList = await expectJson<TicketFilesResponse>(
    filesResponse,
    'mixed-workspace file list JSON must parse',
  );
  const asset = (fileList.files ?? []).find((file) => file.path !== 'description.md');
  expect(asset, 'seeded child ticket should expose a non-description asset').toBeTruthy();

  const assetButton = rowEntry.getByTestId(
    `ticket-tree-file-${currentFixture.childTicketId}-${asset!.name}`,
  );
  await expect(assetButton).toBeVisible();

  const assetResponsePromise = page.waitForResponse((response) => {
    return matchesWorkspaceRequest(
      response,
      `/api/tickets/${currentFixture.childTicketId}/asset`,
      currentFixture.childWorkspace,
      { path: asset!.path },
    );
  });
  await assetButton.click();
  const assetResponse = await assetResponsePromise;
  expect(assetResponse.ok(), 'mixed-workspace asset request must succeed').toBe(true);
  await expect(page.getByTestId('ticket-content')).toContainText(currentFixture.childAssetSnippet);
  await expect(page.getByTestId('tab-description')).toHaveText(asset!.name);
});