import * as vscode from 'vscode';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { fetchWorkspaces, type WorkspaceInfo } from '../api';

export interface DetectedWorkspace {
  folderName: string;
  ticketPath: string;
  folder: vscode.WorkspaceFolder;
}

export interface ActiveWorkspace {
  workspace: string;
  displayName: string;
}

function normalizePath(value: string): string {
  return value.replace(/\\/g, '/').replace(/\/+$|\/$/g, '').toLowerCase();
}

function stripTicketSuffix(value: string): string {
  const normalized = normalizePath(value);
  return normalized.endsWith('/.ticket')
    ? normalized.slice(0, -'/.ticket'.length)
    : normalized;
}

function displayWorkspaceLabel(workspace: WorkspaceInfo): string {
  const label = workspace.label?.trim();
  return label && label !== '' ? label : workspace.name;
}

function matchesDetectedWorkspace(
  workspace: WorkspaceInfo,
  detected: DetectedWorkspace,
): boolean {
  const normalizedName = normalizePath(workspace.name);
  const normalizedLabel = normalizePath(workspace.label ?? '');
  const folderName = detected.folderName.trim().toLowerCase();
  const folderPath = normalizePath(detected.folder.uri.fsPath);
  const ticketPath = normalizePath(detected.ticketPath);

  if (workspace.name === detected.folderName) {
    return true;
  }

  if ((workspace.label ?? '').trim() === detected.folderName) {
    return true;
  }

  return [normalizedName, normalizedLabel].some(candidate => {
    if (candidate === '') {
      return false;
    }

    return candidate === folderPath
      || candidate === ticketPath
      || stripTicketSuffix(candidate) === folderPath
      || stripTicketSuffix(candidate) === ticketPath
      || candidate === folderName;
  });
}

function resolveWorkspaceSelection(
  serverWorkspaces: WorkspaceInfo[],
  activeWorkspace: string | undefined,
  detected: DetectedWorkspace | undefined,
): { workspace: string; displayName: string } {
  const activeName = activeWorkspace?.trim() || undefined;

  if (detected) {
    const exactMatch = serverWorkspaces.find(workspace =>
      matchesDetectedWorkspace(workspace, detected),
    );
    if (exactMatch) {
      return {
        workspace: exactMatch.name,
        displayName: detected.folderName,
      };
    }
  }

  if (activeName) {
    const activeMatch = serverWorkspaces.find(workspace => workspace.name === activeName);
    if (activeMatch) {
      return {
        workspace: activeMatch.name,
        displayName: detected?.folderName ?? displayWorkspaceLabel(activeMatch),
      };
    }
  }

  const fallback = serverWorkspaces[0];
  if (fallback) {
    return {
      workspace: fallback.name,
      displayName: detected?.folderName ?? displayWorkspaceLabel(fallback),
    };
  }

  const displayName = detected?.folderName ?? 'default';
  return { workspace: 'default', displayName };
}


export function detectTicketWorkspaces(): DetectedWorkspace[] {
  const folders = vscode.workspace.workspaceFolders ?? [];
  return folders.flatMap(folder => {
    const ticketDir = path.join(folder.uri.fsPath, '.ticket');
    try {
      if (fs.statSync(ticketDir).isDirectory()) {
        return [{ folderName: folder.name, ticketPath: ticketDir, folder }];
      }
    } catch { /* directory not found */ }
    return [];
  });
}

export async function resolveActiveWorkspace(
  serverUrl: string,
  configured: string,
  context: vscode.ExtensionContext,
): Promise<ActiveWorkspace> {
  const detected = detectTicketWorkspaces();
  const configuredName = configured.trim() || undefined;

  let serverWorkspaces: WorkspaceInfo[] = [];
  let activeWorkspace: string | undefined;
  try {
    const response = await fetchWorkspaces(serverUrl);
    serverWorkspaces = response.workspaces;
    activeWorkspace = response.active_workspace;
  } catch { /* server may not be running yet */ }

  if (configuredName) {
    const configuredMatch = serverWorkspaces.find(workspace => {
      if (workspace.name === configuredName) {
        return true;
      }

      return detected.some(candidate =>
        candidate.folderName === configuredName
        && matchesDetectedWorkspace(workspace, candidate),
      );
    });

    if (configuredMatch) {
      const detectedMatch = detected.find(candidate =>
        matchesDetectedWorkspace(configuredMatch, candidate),
      );
      return {
        workspace: configuredMatch.name,
        displayName: detectedMatch?.folderName ?? displayWorkspaceLabel(configuredMatch),
      };
    }

    if (serverWorkspaces.length === 0) {
      return { workspace: configuredName, displayName: configuredName };
    }
  }

  if (detected.length === 1) {
    return resolveWorkspaceSelection(serverWorkspaces, activeWorkspace, detected[0]);
  }

  if (detected.length > 1) {
    const stored = context.workspaceState.get<string>('activeTicketFolder');
    if (stored) {
      const match = detected.find(candidate => candidate.folderName === stored);
      if (match) {
        return resolveWorkspaceSelection(serverWorkspaces, activeWorkspace, match);
      }
    }

    const items = detected.map(candidate => ({
      label: candidate.folderName,
      description: candidate.ticketPath,
      folderName: candidate.folderName,
    }));
    const pick = await vscode.window.showQuickPick(items, {
      placeHolder: 'Multiple .ticket workspaces found — select one',
      title: 'Active Ticket Workspace',
    });
    if (pick) {
      await context.workspaceState.update('activeTicketFolder', pick.folderName);
      const detectedMatch = detected.find(candidate => candidate.folderName === pick.folderName);
      return resolveWorkspaceSelection(serverWorkspaces, activeWorkspace, detectedMatch);
    }
  }

  return resolveWorkspaceSelection(serverWorkspaces, activeWorkspace, undefined);
}

/**
 * Open the ticket-viewer URL in an external browser using VS Code's routing.
 *
 * Rule 4 (frozen, spec ticket-vscode/rust-wasm-port): viewer navigation routes
 * through asExternalUri so it is remote/Codespaces-safe by default.
 */
export function openTicketViewer(url: string): void {
  void (async () => {
    const external = await vscode.env.asExternalUri(vscode.Uri.parse(url));
    void vscode.env.openExternal(external);
  })();
}

export function resolveTicketsDir(workspaceName: string, displayName?: string): string | undefined {
  const detected = detectTicketWorkspaces();
  const normalizedWorkspace = normalizePath(workspaceName);
  const normalizedDisplay = displayName?.trim().toLowerCase();
  const match = detected.find(candidate => {
    const normalizedFolder = candidate.folderName.trim().toLowerCase();
    const normalizedFolderPath = normalizePath(candidate.folder.uri.fsPath);
    const normalizedTicketPath = normalizePath(candidate.ticketPath);

    return normalizedFolder === normalizedDisplay
      || normalizedFolder === workspaceName.trim().toLowerCase()
      || normalizedFolderPath === normalizedWorkspace
      || normalizedTicketPath === normalizedWorkspace
      || stripTicketSuffix(normalizedWorkspace) === normalizedFolderPath
      || stripTicketSuffix(normalizedWorkspace) === normalizedTicketPath;
  }) ?? detected[0];
  if (!match) { return undefined; }
  const dir = path.join(match.ticketPath, 'tickets');
  try {
    if (fs.statSync(dir).isDirectory()) { return dir; }
  } catch { /* not found */ }
  return undefined;
}

/**
 * URI-returning wrapper around resolveTicketsDir for the TicketTreeProvider.
 *
 * Rule 3 (frozen): TicketTreeProvider now accepts vscode.Uri instead of a
 * raw string path so virtual/web workspaces resolve correctly.  This helper
 * converts the desktop string path to a file URI; call sites in extension.ts
 * and extensionCommands.ts import this instead of resolveTicketsDir.
 */
export function resolveTicketsDirUri(workspaceName: string, displayName?: string): vscode.Uri | undefined {
  const dir = resolveTicketsDir(workspaceName, displayName);
  return dir !== undefined ? vscode.Uri.file(dir) : undefined;
}
