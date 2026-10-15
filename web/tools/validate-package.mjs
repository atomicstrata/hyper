import {readFile, access} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {validateDistribution} from './distribution.mjs';

const root = new URL('../', import.meta.url);
const metadata = JSON.parse(await readFile(new URL('package.json', root), 'utf8'));
await validateDistribution(fileURLToPath(new URL('dist/', root)), metadata.version);
for (const path of ['dist/index.mjs', 'dist/index.d.ts', 'README.md', 'LICENSE', 'LICENSE-MIT', 'LICENSE-APACHE', 'NOTICE']) {
  await access(new URL(path, root));
}
