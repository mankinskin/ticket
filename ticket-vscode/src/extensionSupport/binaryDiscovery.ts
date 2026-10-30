import * as fs from 'node:fs';
import * as path from 'node:path';

function preferredBrowserCandidates(): string[] {
  if (process.platform === 'win32') {
    const roots = [
      process.env.PROGRAMFILES,
      process.env['PROGRAMFILES(X86)'],
      process.env.LOCALAPPDATA,
    ].filter((value): value is string => typeof value === 'string' && value !== '');

    return [
      'chrome.exe',
      'chromium.exe',
      'msedge.exe',
      ...roots.flatMap(root => [
        path.join(root, 'Google', 'Chrome', 'Application', 'chrome.exe'),
        path.join(root, 'Chromium', 'Application', 'chrome.exe'),
        path.join(root, 'Microsoft', 'Edge', 'Application', 'msedge.exe'),
      ]),
    ];
  }

  if (process.platform === 'darwin') {
    return [
      '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
      '/Applications/Chromium.app/Contents/MacOS/Chromium',
      '/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge',
    ];
  }

  return [
    'google-chrome',
    'google-chrome-stable',
    'chromium-browser',
    'chromium',
    'microsoft-edge',
    'microsoft-edge-stable',
  ];
}

function resolvePreferredBrowserBinary(): string | undefined {
  for (const candidate of preferredBrowserCandidates()) {
    if (path.isAbsolute(candidate)) {
      if (fs.existsSync(candidate)) {
        return candidate;
      }
      continue;
    }

    const resolved = resolveBinaryOnPath(candidate);
    if (resolved) {
      return resolved;
    }
  }

  return undefined;
}


export function resolveBinaryOnPath(binaryName: string): string | undefined {
  const pathValue = process.env.PATH ?? '';
  for (const entry of pathValue.split(path.delimiter)) {
    if (entry.trim() === '') {
      continue;
    }

    const candidate = path.join(entry, binaryName);
    try {
      const stat = fs.statSync(candidate);
      if (!stat.isFile()) {
        continue;
      }
      if (process.platform === 'win32' || (stat.mode & 0o111) !== 0) {
        return candidate;
      }
    } catch { /* not found */ }
  }

  return undefined;
}
