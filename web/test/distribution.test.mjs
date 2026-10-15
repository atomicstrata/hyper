import {test} from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp, mkdir, writeFile, readFile, rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {validateDistribution, copyAssets} from '../tools/distribution.mjs';

async function fixture() {
  const root = await mkdtemp(join(tmpdir(), 'hyper-distribution-'));
  const source = join(root, 'dist');
  const manifest = {schema: 1, version: '0.2.0'};
  for (const backend of ['webgpu', 'webgl2']) {
    const directory = `renderers/${backend}`;
    await mkdir(join(source, directory), {recursive: true});
    manifest[backend] = {module: `${directory}/hyper-test.mjs`, wasm: `${directory}/hyper-test.wasm`};
    await writeFile(join(source, manifest[backend].module), 'export function createBindings(){return {default: async()=>{}, start:()=>{}}}');
    await writeFile(join(source, manifest[backend].wasm), new Uint8Array([0,97,115,109,1,0,0,0]));
  }
  await writeFile(join(source, 'renderers.json'), JSON.stringify(manifest));
  await mkdir(join(source, 'licenses'));
  for (const name of ['LICENSE', 'LICENSE-MIT', 'LICENSE-APACHE', 'NOTICE', 'THIRD_PARTY_NOTICES.txt']) {
    await writeFile(join(source, 'licenses', name), 'bundled dependency notices');
  }
  return {root, source, manifest};
}

test('asset copy publishes a complete manifest and preserves unrelated host files', async () => {
  const {root, source} = await fixture();
  try {
    const destination = join(root, 'public/hyper');
    await mkdir(destination, {recursive: true});
    await writeFile(join(destination, 'host.txt'), 'keep me');
    await copyAssets(source, destination, '0.2.0');
    const manifest = await validateDistribution(destination, '0.2.0');
    assert.equal(manifest.version, '0.2.0');
    assert.equal(await readFile(join(destination, 'host.txt'), 'utf8'), 'keep me');
    assert.equal(await readFile(join(destination, 'licenses/THIRD_PARTY_NOTICES.txt'), 'utf8'), 'bundled dependency notices');
  } finally { await rm(root, {recursive: true, force: true}); }
});

test('incomplete renderer copy leaves the previously hosted manifest intact', async () => {
  const {root, source, manifest} = await fixture();
  try {
    const destination = join(root, 'public');
    await mkdir(destination); await writeFile(join(destination, 'renderers.json'), 'previous build');
    await rm(join(source, manifest.webgl2.wasm));
    await assert.rejects(copyAssets(source, destination, '0.2.0'), /ENOENT/);
    assert.equal(await readFile(join(destination, 'renderers.json'), 'utf8'), 'previous build');
  } finally { await rm(root, {recursive: true, force: true}); }
});

test('preflight rejects mismatched versions, invalid WASM and escaping asset paths', async () => {
  const {root, source, manifest} = await fixture();
  try {
    await assert.rejects(validateDistribution(source, '0.3.0'), /version/);
    await writeFile(join(source, manifest.webgpu.wasm), 'not wasm');
    await assert.rejects(validateDistribution(source, '0.2.0'), /WebAssembly/);
    manifest.webgpu.module = '../outside.mjs';
    await writeFile(join(source, 'renderers.json'), JSON.stringify(manifest));
    await assert.rejects(validateDistribution(source, '0.2.0'), /asset path/);
  } finally { await rm(root, {recursive: true, force: true}); }
});
