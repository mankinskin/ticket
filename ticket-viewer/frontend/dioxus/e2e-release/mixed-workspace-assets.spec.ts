import { expect, test } from '@playwright/test';
import fs from 'node:fs/promises';
import { TICKET_VIEWER } from './shared/viewers';
import {
  attachScreenshot,
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
  type TicketHistoryResponse,
  type TicketsResponse,
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
