import { spawn, spawnSync } from 'node:child_process';
import { createWriteStream, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '../../..');
const targetDir = join(repoRoot, 'target');
const fixtureRoot = join(targetDir, 'e2e-release-store');
const storeRoot = resolve(fixtureRoot, '.ticket');
const logPath = join(targetDir, 'e2e-release-ticket-viewer.log');
const viewerBinary = join(
  targetDir,
  'release',
  `ticket-viewer${process.platform === 'win32' ? '.exe' : ''}`,
);

function runCargo(args, options = {}) {
  const result = spawnSync('cargo', args, {
    cwd: repoRoot,
    encoding: 'utf8',
    stdio: options.captureOutput ? ['ignore', 'pipe', 'inherit'] : 'inherit',
  });

  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }

  return options.captureOutput ? result.stdout : undefined;
}

function runTicket(args, options) {
  return runCargo([
    'run', '--features', 'cli', '--bin', 'ticket', '--',
    ...args,
    '--index-root', storeRoot,
    '--workspace', fixtureRoot,
  ], options);
}

function createFixtureTicket({ id, title, fields = [], bodyFile }) {
  const args = ['create', '--json', '--type', 'task', '--title', title];
  if (id) {
    args.push('--id', id);
  }
  for (const field of fields) {
    args.push('--field', field);
  }
  if (bodyFile) {
    args.push('--body-file', bodyFile);
  }

  const output = runTicket(args, { captureOutput: true });
  return JSON.parse(output).payload.id;
}

function transitionFixtureTicket(id, state) {
  runTicket(['update', id, '--to-state', state]);
}

// This directory and log are owned solely by the release E2E harness.
rmSync(fixtureRoot, { recursive: true, force: true });
mkdirSync(targetDir, { recursive: true });

runTicket(['init']);
runTicket([
  'workspace', 'policy', 'set',
  '--include-descendants', 'false', '--include-ancestors', 'false',
  '--deny-external-paths', 'true',
]);
runTicket(['workspace', 'rescan', '--apply-policy']);
runTicket(['add-root', '--label', 'default', storeRoot]);

const graphRootId = createFixtureTicket({
  title: 'Release E2E graph root',
  fields: ['component=graph-root'],
});
const readyChildId = createFixtureTicket({
  title: 'Release E2E ready prerequisite',
  fields: ['component=graph-ready'],
});
const implementationChildId = createFixtureTicket({
  title: 'Release E2E implementation prerequisite',
  fields: ['component=graph-implementation'],
});
const reviewChildId = createFixtureTicket({
  title: 'Release E2E review prerequisite',
  fields: ['component=graph-review'],
});
const doneChildId = createFixtureTicket({
  title: 'Release E2E completed prerequisite',
  fields: ['component=graph-done'],
});

transitionFixtureTicket(readyChildId, 'planned');
transitionFixtureTicket(implementationChildId, 'in-implementation');
transitionFixtureTicket(reviewChildId, 'in-review');
transitionFixtureTicket(doneChildId, 'done');

for (const childId of [
  readyChildId,
  implementationChildId,
  reviewChildId,
  doneChildId,
]) {
  runTicket(['link', '--from', graphRootId, '--to', childId, '--kind', 'depends_on']);
}

for (const title of [
  'Release E2E navigation fixture',
  'Release E2E search fixture alpha',
  'Release E2E search fixture beta',
  'Release E2E search fixture gamma',
  'Release E2E search fixture delta',
  'Release E2E search fixture epsilon',
]) {
  createFixtureTicket({ title, fields: ['component=search'] });
}

const legacyBodyPath = join(fixtureRoot, 'legacy-description.md');
writeFileSync(legacyBodyPath, '# Legacy ticket\n\nLegacy description-only fixture.\n');
createFixtureTicket({
  id: '3a1ec9f8-15ea-43f2-b6d3-89b88cbdcb17',
  title: 'Release E2E legacy description fixture',
  bodyFile: legacyBodyPath,
});

runTicket(['scan']);
runCargo(['build', '--manifest-path', 'ticket-viewer/Cargo.toml', '--release']);

const log = createWriteStream(logPath, { flags: 'w' });
const server = spawn(
  viewerBinary,
  ['--port', '3002', '--index-root', storeRoot],
  { cwd: repoRoot, stdio: ['ignore', 'pipe', 'pipe'] },
);

for (const stream of [server.stdout, server.stderr]) {
  stream.on('data', (chunk) => {
    process.stdout.write(chunk);
    log.write(chunk);
  });
}

let stopping = false;
function stopServer() {
  if (stopping) {
    return;
  }

  stopping = true;
  server.kill();
}

process.once('SIGINT', stopServer);
process.once('SIGTERM', stopServer);
process.once('exit', stopServer);

server.on('error', (error) => {
  console.error(`ticket-viewer launch failed: ${error.message}`);
  process.exitCode = 1;
});
server.on('exit', (code, signal) => {
  log.end();
  if (!stopping) {
    process.exitCode = code ?? 1;
    console.error(`ticket-viewer exited unexpectedly (${signal ?? code})`);
  }
});
