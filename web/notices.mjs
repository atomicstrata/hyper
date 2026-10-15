import {execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {readFile, writeFile, mkdir, cp, readdir} from 'node:fs/promises';
import {join, dirname} from 'node:path';

const execute = promisify(execFile);
export async function generateNotices(root, destination) {
  const directory = join(destination, 'licenses');
  await mkdir(directory, {recursive: true});
  const output = join(directory, 'THIRD_PARTY_NOTICES.txt');
  // cargo-about resolves metadata for every platform before filtering licenses
  // to WASM. A fresh WASM build alone does not cache native-only dependencies.
  await execute('cargo', ['fetch', '--locked',
    '--manifest-path', join(root, 'crates/hyper-viz-web/Cargo.toml')],
  {cwd: root, maxBuffer: 8 * 1024 * 1024});
  await execute(process.env.CARGO_ABOUT ?? 'cargo-about', ['generate', '--locked', '--offline', '--fail',
    '--no-default-features', '--features', 'webgpu webgl2',
    '--manifest-path', join(root, 'crates/hyper-viz-web/Cargo.toml'),
    '--config', join(root, 'web/about.toml'), join(root, 'web/about.hbs'), '-o', output],
  {cwd: root, maxBuffer: 8 * 1024 * 1024});
  const {stdout} = await execute('cargo', ['metadata', '--locked', '--offline', '--format-version', '1',
    '--filter-platform', 'wasm32-unknown-unknown', '--manifest-path', join(root, 'crates/hyper-viz-web/Cargo.toml'),
    '--no-default-features', '--features', 'webgpu,webgl2'], {cwd: root, maxBuffer: 16 * 1024 * 1024});
  const metadata = JSON.parse(stdout);
  const fonts = metadata.packages.filter(package_ => package_.name === 'epaint_default_fonts');
  if (!fonts.length) throw Error('Default font license sources were not found');
  let notices = await readFile(output, 'utf8');
  for (const package_ of fonts) {
    const source = join(dirname(package_.manifest_path), 'fonts');
    // Preserve every font's original copyright and full license, in addition to SPDX texts.
    for (const file of (await readdir(source)).filter(file => file.endsWith('.txt')).sort()) {
      notices += `\n${'='.repeat(80)}\nBundled font notice: ${package_.name} ${package_.version} / fonts/${file}\n\n`;
      notices += await readFile(join(source, file), 'utf8');
    }
  }
  await writeFile(output, notices);
  for (const name of ['LICENSE', 'LICENSE-MIT', 'LICENSE-APACHE', 'NOTICE']) await cp(join(root, name), join(directory, name));
}
