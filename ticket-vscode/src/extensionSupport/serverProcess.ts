import * as vscode from 'vscode';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { spawn, type ChildProcess } from 'node:child_process';
import type { TicketViewerConfig } from './configuration';
import { resolveBinaryOnPath } from './binaryDiscovery';
import { detectTicketWorkspaces, type DetectedWorkspace } from './workspace';

export interface ServerHandle {
  process: ChildProcess;
  serverUrl: string;
}

export function resolveServerLaunch(config: TicketViewerConfig): {
  cmd: string;
  args: string[];
  cwd: string | undefined;
} {
  const detected = detectTicketWorkspaces();

  const cwd = config.serverWorkingDirectory.trim() !== ''
    ? config.serverWorkingDirectory.trim()
    : (detected[0]?.folder.uri.fsPath
        ?? vscode.workspace.workspaceFolders?.[0]?.uri.fsPath);

  const indexRoot = detected[0]?.ticketPath ?? undefined;
  const indexRootArgs = indexRoot ? ['--index-root', indexRoot] : [];

  if (config.serverBinaryPath.trim() !== '') {
    return { cmd: config.serverBinaryPath.trim(), args: indexRootArgs, cwd };
  }

  const binaryName = process.platform === 'win32' ? 'ticket-viewer.exe' : 'ticket-viewer';
  const pathBinary = resolveBinaryOnPath(binaryName);
  if (pathBinary) {
    return { cmd: pathBinary, args: indexRootArgs, cwd };
  }

  if (detected[0]?.folder.uri.fsPath) {
    const devBinary = path.join(detected[0].folder.uri.fsPath, 'target', 'debug', binaryName);
    if (fs.existsSync(devBinary)) {
      return { cmd: devBinary, args: indexRootArgs, cwd };
    }
  }

  return { cmd: binaryName, args: indexRootArgs, cwd };
}

export function startServerTask(
  outputChannel: vscode.OutputChannel,
  config: TicketViewerConfig,
): Promise<ServerHandle> {
  const { cmd, args, cwd } = resolveServerLaunch(config);
  const finalArgs = [...args, '--port', '0'];

  outputChannel.appendLine(`[ticket-viewer] Starting: ${cmd} ${finalArgs.join(' ')}`);
  outputChannel.appendLine(`[ticket-viewer] Working directory: ${cwd ?? '(inherited)'}`);

  const proc = spawn(cmd, finalArgs, {
    cwd,
    detached: false,
    windowsHide: true,
    stdio: ['ignore', 'pipe', 'pipe'],
  });

  return new Promise<ServerHandle>((resolve, reject) => {
    let resolved = false;
    const portRe = /TICKET_VIEWER_PORT=(\d+)/;

    proc.stdout?.on('data', (data: Buffer) => {
      const text = data.toString();
      outputChannel.append(text);

      if (!resolved) {
        const match = portRe.exec(text);
        if (match) {
          resolved = true;
          const port = Number(match[1]);
          const serverUrl = `http://localhost:${port}`;
          outputChannel.appendLine(`[ticket-viewer] Detected server on ${serverUrl}`);
          resolve({ process: proc, serverUrl });
        }
      }
    });

    proc.stderr?.on('data', (data: Buffer) => outputChannel.append(data.toString()));

    proc.on('error', err => {
      outputChannel.appendLine(`[ticket-viewer] Error: ${err.message}`);
      if (!resolved) {
        resolved = true;
        reject(err);
      }
    });

    proc.on('exit', code => {
      outputChannel.appendLine(`[ticket-viewer] Exited with code ${code}`);
      if (!resolved) {
        resolved = true;
        reject(new Error(`Server exited with code ${code} before reporting a port`));
      }
    });

    setTimeout(() => {
      if (!resolved) {
        resolved = true;
        reject(new Error('Timed out waiting for server to report its port'));
      }
    }, 30_000);
  });
}
