import { access, chmod, copyFile, mkdir } from 'node:fs/promises';
import { constants } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawn } from 'node:child_process';

const desktopRoot = resolve(fileURLToPath(new URL('..', import.meta.url)));
const repoRoot = resolve(desktopRoot, '..');
const resourcesRoot = join(desktopRoot, 'resources');

function run(command, args) {
  return new Promise((resolvePromise, reject) => {
    const child = spawn(command, args, { cwd: repoRoot, stdio: 'inherit', windowsHide: true });
    child.once('error', reject);
    child.once('close', (code, signal) => {
      if (code === 0) { resolvePromise(); return; }
      reject(new Error(`${command} ${args.join(' ')} failed with ${signal ? `signal ${signal}` : `exit code ${code ?? 'unknown'}`}`));
    });
  });
}

function platformBinaryNames() {
  if (process.platform === 'win32') return { analyzer: 'duxo-analyzer.exe', adapter: 'duxo-adapter.exe' };
  return { analyzer: 'duxo-analyzer', adapter: 'duxo-adapter' };
}

const names = platformBinaryNames();
const targetRoot = join(repoRoot, 'target', 'release');
const analyzerSource = join(targetRoot, names.analyzer);
const adapterSource = join(targetRoot, names.adapter);
const analyzerDestination = join(resourcesRoot, names.analyzer);
const adapterDestination = join(resourcesRoot, names.adapter);

await mkdir(resourcesRoot, { recursive: true });
await run('cargo', ['build', '--release', '-p', 'duxo-analyzer', '-p', 'duxo-adapter']);

for (const [label, source, destination] of [
  ['Analyzer', analyzerSource, analyzerDestination],
  ['Adapter', adapterSource, adapterDestination],
]) {
  await access(source, constants.F_OK);
  await copyFile(source, destination);
  if (process.platform !== 'win32') await chmod(destination, 0o755);
  console.log(`Staged ${label}: ${destination}`);
}
