import { expect, test } from '@playwright/test';
import fs from 'node:fs/promises';
import { TICKET_VIEWER } from './shared/viewers';
import {
  escapeRegex,
  expectJson,
  gotoAndWaitForSeededViewer,
  matchesWorkspaceRequest,
  stopSeededViewer,
} from './mixed-workspace-test-helpers';
import {
  seedMixedWorkspaceFixture,
  type SeededViewerFixture,
  type TicketFilesResponse,
  type TicketsResponse,
  type WorkspacesResponse,
} from './mixed-workspace-fixture';

let fixture: SeededViewerFixture | null = null;

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
