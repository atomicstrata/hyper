import {readFile, mkdir, copyFile, writeFile, rename, rm} from 'node:fs/promises';
import {join} from 'node:path';
import {pathToFileURL} from 'node:url';

const licenseFiles = ['LICENSE', 'LICENSE-MIT', 'LICENSE-APACHE', 'NOTICE', 'THIRD_PARTY_NOTICES.txt'];

/** Fail before packing or copying an incomplete/mismatched renderer build. */
export async function validateDistribution(directory, version) {
  let manifest;
  try { manifest = JSON.parse(await readFile(join(directory, 'renderers.json'), 'utf8')); }
  catch (cause) { throw new Error('Renderer build missing or invalid. Run npm run build before packing.', {cause}); }
  if (manifest.schema !== 1 || manifest.version !== version) throw Error('Renderer build version differs from package.json. Rebuild the SDK.');
  for (const backend of ['webgpu', 'webgl2']) {
    const entry = manifest[backend];
    for (const [kind, extension] of [['module', 'mjs'], ['wasm', 'wasm']]) {
      const path = entry?.[kind];
      if (typeof path !== 'string' || !new RegExp(`^renderers/${backend}/[a-zA-Z0-9-]+\\.${extension}$`).test(path)) {
        throw Error(`Invalid ${backend} ${kind} asset path`);
      }
    }
    const bytes = await readFile(join(directory, entry.wasm));
    if (!WebAssembly.validate(bytes)) throw Error(`Invalid ${backend} WebAssembly artifact`);
    const factory = await import(pathToFileURL(join(directory, entry.module)).href);
    if (typeof factory.createBindings !== 'function') throw Error(`Invalid ${backend} binding factory`);
    const bindings = factory.createBindings();
    if (typeof bindings.default !== 'function' || typeof bindings.start !== 'function') throw Error(`Incomplete ${backend} binding exports`);
  }
  for (const name of licenseFiles) {
    if (!(await readFile(join(directory, 'licenses', name), 'utf8')).trim()) throw Error(`Missing license notice: ${name}`);
  }
  return manifest;
}

/** Copy only this build's assets, retaining unrelated host files and old hashed assets. */
export async function copyAssets(source, destination, version) {
  const manifest = await validateDistribution(source, version);
  for (const backend of ['webgpu', 'webgl2']) {
    await mkdir(join(destination, 'renderers', backend), {recursive: true});
    for (const path of [manifest[backend].module, manifest[backend].wasm]) await copyFile(join(source, path), join(destination, path));
  }
  await mkdir(join(destination, 'licenses'), {recursive: true});
  for (const name of licenseFiles) await copyFile(join(source, 'licenses', name), join(destination, 'licenses', name));
  const temporary = join(destination, `.renderers-${crypto.randomUUID()}.tmp`);
  try {
    await writeFile(temporary, JSON.stringify(manifest, null, 2) + '\n');
    await rename(temporary, join(destination, 'renderers.json'));
  } finally { await rm(temporary, {force: true}); }
  return manifest;
}
