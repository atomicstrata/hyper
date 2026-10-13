#!/usr/bin/env node
import {readFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {copyAssets} from '../tools/distribution.mjs';

const arguments_ = process.argv.slice(2);
if (arguments_.length !== 1 || arguments_[0].startsWith('-')) {
  console.log('Usage: hyper-copy-assets <destination>\nCopies prebuilt renderer assets for use with createHyperViewer({manifestUrl}).');
  process.exit(arguments_[0] === '--help' ? 0 : 1);
}
try {
  const metadata = JSON.parse(await readFile(new URL('../package.json', import.meta.url), 'utf8'));
  const destination = resolve(arguments_[0]);
  await copyAssets(fileURLToPath(new URL('../dist/', import.meta.url)), destination, metadata.version);
  console.log(`Hyper renderer assets copied to ${destination}`);
} catch (error) { console.error(error.message); process.exitCode = 1; }
