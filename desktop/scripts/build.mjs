import { access, cp, mkdir } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const desktopRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const rendererOut = join(desktopRoot, 'dist-renderer');
const rendererSource = join(desktopRoot, 'src');

await mkdir(rendererOut, { recursive: true });
await cp(join(rendererSource, 'index.html'), join(rendererOut, 'index.html'));
await cp(join(rendererSource, 'styles.css'), join(rendererOut, 'styles.css'));
await cp(join(rendererSource, 'plan.css'), join(rendererOut, 'plan.css'));
await access(join(rendererOut, 'renderer.js'));
