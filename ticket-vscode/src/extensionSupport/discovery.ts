import * as vscode from 'vscode';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import type { TicketViewerConfig } from './configuration';

const execFileAsync = promisify(execFile);
const LAST_SERVER_URL_KEY = 'ticketViewer.lastServerUrl';
const DISCOVERY_PROBE_TIMEOUT_MS = 400;
const DISCOVERY_BATCH_SIZE = 8;


function normalizeServerUrl(value: string | undefined): string | undefined {
  const trimmed = value?.trim();
  if (!trimmed) {
    return undefined;
  }

  try {
    const url = new URL(trimmed);
    if (url.protocol !== 'http:' && url.protocol !== 'https:') {
      return undefined;
    }

    const host = url.hostname === '127.0.0.1' || url.hostname === '::1'
      ? 'localhost'
      : url.hostname;
    const port = url.port ? `:${url.port}` : '';
    return `${url.protocol}//${host}${port}`;
  } catch {
    return undefined;
  }
}

function extractServerPort(value: string | undefined): number | undefined {
  const normalized = normalizeServerUrl(value);
  if (!normalized) {
    return undefined;
  }

  try {
    const url = new URL(normalized);
    const port = Number(url.port);
    return Number.isInteger(port) && port > 0 ? port : undefined;
  } catch {
    return undefined;
  }
}

export function parseListeningPorts(output: string): number[] {
  const ports: number[] = [];
  const seen = new Set<number>();

  for (const rawLine of output.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (line === '' || !/listen/i.test(line)) {
      continue;
    }

    const match = line.match(/(?:127\.0\.0\.1|0\.0\.0\.0|\[::1\]|::1|\*)[:.](\d+)\b/);
    if (!match) {
      continue;
    }

    const port = Number(match[1]);
    if (!Number.isInteger(port) || port <= 0 || seen.has(port)) {
      continue;
    }

    seen.add(port);
    ports.push(port);
  }

  return ports;
}

export function buildServerDiscoveryCandidates(
  configuredUrl: string,
  storedUrl: string | undefined,
  listeningPorts: number[],
  preferredUrls: string[] = [],
): string[] {
  const candidates: string[] = [];
  const seen = new Set<string>();
  const preferredPorts = [
    extractServerPort(preferredUrls[0]),
    extractServerPort(storedUrl),
    extractServerPort(configuredUrl),
    3002,
  ].filter((port): port is number => port !== undefined);

  function addCandidate(url: string | undefined): void {
    const normalized = normalizeServerUrl(url);
    if (!normalized || seen.has(normalized)) {
      return;
    }
    seen.add(normalized);
    candidates.push(normalized);
  }

  for (const url of preferredUrls) {
    addCandidate(url);
  }
  addCandidate(storedUrl);
  addCandidate(configuredUrl);

  for (const port of preferredPorts) {
    addCandidate(`http://localhost:${port}`);
  }

  for (const port of listeningPorts) {
    addCandidate(`http://localhost:${port}`);
  }

  return candidates;
}

async function listListeningPorts(): Promise<number[]> {
  const commands = process.platform === 'win32'
    ? [['netstat', ['-ano', '-p', 'tcp']] as const]
    : [['ss', ['-ltnH']] as const, ['lsof', ['-nP', '-iTCP', '-sTCP:LISTEN']] as const];

  for (const [command, args] of commands) {
    try {
      const { stdout } = await execFileAsync(command, args, {
        windowsHide: true,
        maxBuffer: 1024 * 1024,
      });
      const ports = parseListeningPorts(stdout);
      if (ports.length > 0) {
        return ports;
      }
    } catch {
      // Try the next available platform command.
    }
  }

  return [];
}

export async function rememberServerUrl(
  context: vscode.ExtensionContext,
  serverUrl: string,
): Promise<void> {
  const normalized = normalizeServerUrl(serverUrl);
  if (!normalized) {
    return;
  }
  await context.workspaceState.update(LAST_SERVER_URL_KEY, normalized);
}

export async function discoverRunningServerUrl(
  context: vscode.ExtensionContext,
  config: TicketViewerConfig,
  outputChannel?: vscode.OutputChannel,
  preferredUrls: string[] = [],
): Promise<string | undefined> {
  const storedUrl = context.workspaceState.get<string>(LAST_SERVER_URL_KEY);
  const listeningPorts = await listListeningPorts();
  const candidates = buildServerDiscoveryCandidates(
    config.serverUrl,
    storedUrl,
    listeningPorts,
    preferredUrls,
  );

  if (outputChannel && candidates.length > 0) {
    outputChannel.appendLine(`[ticket-viewer] Probing ${candidates.length} ticket-server candidate URL(s).`);
  }

  for (let index = 0; index < candidates.length; index += DISCOVERY_BATCH_SIZE) {
    const batch = candidates.slice(index, index + DISCOVERY_BATCH_SIZE);
    const results = await Promise.all(batch.map(async candidate => (
      await pingServer(candidate, DISCOVERY_PROBE_TIMEOUT_MS) ? candidate : undefined
    )));
    const discovered = results.find((candidate): candidate is string => candidate !== undefined);
    if (discovered) {
      if (outputChannel) {
        outputChannel.appendLine(`[ticket-viewer] Discovered running server at ${discovered}`);
      }
      await rememberServerUrl(context, discovered);
      return discovered;
    }
  }

  return undefined;
}


export async function pingServer(baseUrl: string, timeoutMs = 2000): Promise<boolean> {
  try {
    const controller = new AbortController();
    const id = setTimeout(() => controller.abort(), timeoutMs);
    const response = await fetch(`${baseUrl}/api/workspaces`, { signal: controller.signal });
    clearTimeout(id);
    return response.ok;
  } catch {
    return false;
  }
}

export async function pollUntilReachable(baseUrl: string, timeoutMs: number): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const controller = new AbortController();
      const id = setTimeout(() => controller.abort(), 2000);
      const response = await fetch(`${baseUrl}/api/workspaces`, { signal: controller.signal });
      clearTimeout(id);
      if (response.ok) { return; }
    } catch { /* not ready yet */ }
    await new Promise<void>(resolve => setTimeout(resolve, 2000));
  }
}
