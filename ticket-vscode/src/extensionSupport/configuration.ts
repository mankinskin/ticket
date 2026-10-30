import * as vscode from 'vscode';

export const TICKET_STATES = [
  'open', 'planned', 'in-implementation',
  'in-review', 'done', 'cancelled',
];

export const TICKET_TYPES = ['tracker-improvement'];

export interface TicketViewerConfig {
  serverUrl: string;
  workspace: string;
  autoRefreshSeconds: number;
  autoStartServer: boolean;
  bridgePort: number;
  cdpPort: number;
  autoConnectCdp: boolean;
  serverBinaryPath: string;
  serverWorkingDirectory: string;
}

export function readConfig(): TicketViewerConfig {
  const cfg = vscode.workspace.getConfiguration('ticketViewer');
  return {
    serverUrl: cfg.get<string>('serverUrl', 'http://localhost:3002'),
    workspace: cfg.get<string>('workspace', ''),
    autoRefreshSeconds: cfg.get<number>('autoRefreshSeconds', 30),
    autoStartServer: cfg.get<boolean>('autoStartServer', true),
    bridgePort: cfg.get<number>('bridgePort', 0),
    cdpPort: cfg.get<number>('cdpPort', 0),
    autoConnectCdp: cfg.get<boolean>('autoConnectCdp', true),
    serverBinaryPath: cfg.get<string>('serverBinaryPath', ''),
    serverWorkingDirectory: cfg.get<string>('serverWorkingDirectory', ''),
  };
}
