import type { Page } from '@playwright/test';

export interface ViewerConfig {
  name: string;
  url: string;
  readySelector: string;
  readyTimeout: number;
}

export const TICKET_VIEWER: ViewerConfig = {
  name: 'ticket-viewer',
  url: 'http://127.0.0.1:3002',
  // viewer-api Header component renders <header class="header">.
  readySelector: 'header.header',
  readyTimeout: 60_000,
};

interface WorkspacesResponse {
  active_workspace?: string;
  workspaces?: Array<{ name?: string }>;
}

export async function resolveActiveWorkspace(
  viewerUrl: string = TICKET_VIEWER.url,
): Promise<string> {
  const response = await fetch(viewerUrl + '/api/workspaces');
  if (!response.ok) {
    throw new Error('Workspace list API failed with ' + response.status);
  }

  const body = (await response.json()) as WorkspacesResponse;
  const workspace = body.active_workspace?.trim() || body.workspaces?.[0]?.name;
  if (!workspace) {
    throw new Error('No workspace returned by ' + viewerUrl + '/api/workspaces');
  }

  return workspace;
}

export async function gotoAndWaitForViewer(
  page: Page,
  viewer: ViewerConfig = TICKET_VIEWER,
): Promise<void> {
  await page.goto(viewer.url, { waitUntil: 'domcontentloaded' });
  await page.locator(viewer.readySelector).first().waitFor({
    state: 'visible',
    timeout: viewer.readyTimeout,
  });
}