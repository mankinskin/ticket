import { expect } from '@playwright/test';
import { type ChildProcessWithoutNullStreams } from 'node:child_process';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';

import { runTicketCli, startSeededViewer } from './mixed-workspace-test-helpers';

export interface JsonEnvelope<T> {
  payload: T;
}

export interface CreatePayload {
  id: string;
}

export interface TicketFileEntry {
  name: string;
  path: string;
}

export interface TicketFilesResponse {
  files?: TicketFileEntry[];
}

export interface TicketRefPayload {
  workspace?: string;
  id?: string;
}

export interface TicketListItem {
  id: string;
  ticket_ref?: TicketRefPayload;
}

export interface TicketsResponse {
  items?: TicketListItem[];
}

export interface WorkspacesResponse {
  active_workspace?: string;
  workspaces?: Array<{ name: string; label?: string }>;
}

export interface TicketHistoryResponse {
  entries?: Array<unknown>;
}

export interface SeededViewerFixture {
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

export async function seedMixedWorkspaceFixture(): Promise<SeededViewerFixture> {
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
