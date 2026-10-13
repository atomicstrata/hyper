// Exercise the published tarball in a clean JavaScript/TypeScript consumer.
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {mkdtemp, readFile, writeFile, mkdir, rm, stat} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join, resolve, extname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {createServer} from 'node:http';
import {build} from 'esbuild';

const web = fileURLToPath(new URL('../', import.meta.url));
const metadata = JSON.parse(await readFile(join(web, 'package.json'), 'utf8'));
assert.notEqual(metadata.private, true, 'The npm package must be publishable');
const consumer = await mkdtemp(join(tmpdir(), 'hyper-npm-consumer-'));
function run(command, args, cwd = consumer) {
  const result = spawnSync(command, args, {cwd, encoding: 'utf8',
    env: {...process.env, npm_config_cache: join(tmpdir(), 'hyper-npm-cache')}});
  assert.equal(result.status, 0, `${command} ${args.join(' ')}\n${result.stdout}\n${result.stderr}`);
  return result.stdout;
}
let server, browser;
try {
  const [packed] = JSON.parse(run('npm', ['pack', '--json', '--pack-destination', consumer], web));
  const files = packed.files.map(file => file.path);
  assert.ok(files.includes('README.md'));
  for (const license of ['LICENSE', 'LICENSE-MIT', 'LICENSE-APACHE', 'NOTICE']) assert.ok(files.includes(license));
  assert.ok(files.every(path => /^(package\.json|README\.md|LICENSE(?:-MIT|-APACHE)?|NOTICE|bin\/|tools\/|dist\/(index\.mjs|index\.d\.ts|renderers\.json|renderers\/|licenses\/))/.test(path)),
    `Unexpected package files: ${files.join(', ')}`);
  await writeFile(join(consumer, 'package.json'), JSON.stringify({private: true, type: 'module'}));
  run('npm', ['install', '--ignore-scripts', '--no-audit', '--no-fund', '--package-lock=false', join(consumer, packed.filename)]);
  const installed = join(consumer, 'node_modules', ...metadata.name.split('/'));
  run(process.execPath, ['--input-type=module', '-e', `import {createHyperViewer} from '${metadata.name}'; if(typeof createHyperViewer !== 'function') throw Error('Missing ESM export');`]);
  const publicRoot = join(consumer, 'public');
  const assets = join(publicRoot, 'base/hyper');
  run('npm', ['exec', '--offline', '--', 'hyper-copy-assets', assets]);
  const manifest = JSON.parse(await readFile(join(assets, 'renderers.json'), 'utf8'));
  assert.equal(manifest.version, metadata.version);
  const notices = await readFile(join(assets, 'licenses/THIRD_PARTY_NOTICES.txt'), 'utf8');
  for (const font of ['UFL.txt', 'OFL.txt', 'Hack-Regular.txt', 'emoji-icon-font-mit-license.txt']) {
    assert.ok(notices.includes(`fonts/${font}`), `Bundled font notice missing: ${font}`);
  }
  assert.equal(notices, await readFile(join(installed, 'dist/licenses/THIRD_PARTY_NOTICES.txt'), 'utf8'));
  for (const backend of ['webgpu', 'webgl2']) {
    assert.ok((await stat(join(assets, manifest[backend].module))).size > 0);
    assert.ok(WebAssembly.validate(await readFile(join(assets, manifest[backend].wasm))));
  }
  await writeFile(join(consumer, 'types.mts'), `
import {createHyperViewer, type HyperViewer, type ViewerSettings} from '${metadata.name}';
async function mount(canvas: HTMLCanvasElement): Promise<HyperViewer> {
  const viewer = await createHyperViewer({canvas, backend: 'auto', manifestUrl: '/base/hyper/renderers.json'});
  await viewer.loadJson('{"version":"hypergraph.v1","vertices":[],"hyperedges":[]}');
  await viewer.setProjection('star');
  const settings: Partial<ViewerSettings> = {layoutRunning: false, opacity: 0.5};
  await viewer.setSettings(settings);
  const off = viewer.on('selectionchange', selection => selection.vertices.map(id => id.toUpperCase()));
  off(); viewer.getState().graph.vertices.toFixed(); await viewer.dispose(); return viewer;
}
// @ts-expect-error unsupported backend must fail type checking
createHyperViewer({canvas: document.createElement('canvas'), backend: 'vulkan'});
// @ts-expect-error unknown settings must fail type checking
const invalid: Partial<ViewerSettings> = {opacity: 'opaque'};
void mount; void invalid;
`);
  const tsc = join(web, 'node_modules/typescript/bin/tsc');
  for (const resolution of ['NodeNext', 'Bundler']) {
    run(process.execPath, [tsc, '--noEmit', '--strict', '--lib', 'ES2022,DOM', '--module', resolution === 'NodeNext' ? 'NodeNext' : 'ESNext', '--moduleResolution', resolution, 'types.mts']);
  }
  await writeFile(join(consumer, 'main.mjs'), `import {createHyperViewer} from '${metadata.name}'; window.createViewer = options => createHyperViewer({...options, manifestUrl: new URL('./hyper/renderers.json', location.href)});`);
  await build({entryPoints: [join(consumer, 'main.mjs')], bundle: true, format: 'esm', platform: 'browser', target: 'es2022', outfile: join(publicRoot, 'base/app.mjs')});
  console.log(`PASS ${metadata.name}@${metadata.version} tarball, clean install, asset CLI, ESM bundling, TypeScript NodeNext/Bundler (${(packed.size / 1024 / 1024).toFixed(1)} MiB compressed)`);
  if (process.argv.includes('--browser')) {
    const {chromium} = await import('playwright');
    await writeFile(join(publicRoot, 'base/index.html'), '<!doctype html><style>canvas{width:400px;height:300px}</style><canvas id="a"></canvas><canvas id="b"></canvas><script type="module" src="./app.mjs"></script>');
    server = createServer(async (request, response) => {
      try {
        const pathname = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
        const path = resolve(publicRoot, `.${pathname}`);
        if (!path.startsWith(publicRoot + '/')) throw Error('Invalid path');
        response.setHeader('Content-Type', ({'.wasm': 'application/wasm', '.mjs': 'text/javascript', '.json': 'application/json', '.html': 'text/html'})[extname(path)] ?? 'application/octet-stream');
        response.end(await readFile(path));
      } catch { response.writeHead(404); response.end(); }
    });
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    const software = process.env.HYPER_SOFTWARE_RENDERING === '1';
    browser = await chromium.launch({headless: true, executablePath: process.env.HYPER_BROWSER || undefined,
      args: software ? ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'] : ['--enable-unsafe-webgpu', ...(process.platform === 'darwin' ? ['--use-angle=metal'] : [])]});
    const page = await browser.newPage();
    page.setDefaultTimeout(90_000);
    await page.goto(`http://127.0.0.1:${server.address().port}/base/index.html`);
    await page.waitForFunction(() => window.createViewer);
    if (!software) {
      const adapter = await page.evaluate(async () => {
        const adapter = await navigator.gpu?.requestAdapter();
        return adapter ? {vendor: adapter.info.vendor, architecture: adapter.info.architecture,
          description: adapter.info.description, isFallbackAdapter: adapter.info.isFallbackAdapter} : null;
      });
      assert.ok(adapter, 'Hardware verification requires a WebGPU adapter');
      assert.notEqual(adapter.isFallbackAdapter, true, 'Fallback adapters cannot establish hardware support');
      assert.doesNotMatch(JSON.stringify(adapter), /swiftshader|llvmpipe|software/i);
      console.log(`Hardware evidence: ${browser.version()} ${JSON.stringify(adapter)}`);
    }
    const backends = software ? ['webgl2'] : ['webgpu', 'webgl2'];
    for (const backend of backends) {
      const requests = [];
      const listener = request => {if (request.url().endsWith('.wasm')) requests.push(request.url());};
      page.on('request', listener);
      const result = await page.evaluate(async backend => {
        const viewer = await createViewer({canvas: document.querySelector('#a'), backend});
        await viewer.loadJson('{"version":"hypergraph.v1","vertices":[{"id":"npm","label":"npm consumer"}],"hyperedges":[]}');
        await viewer.select({vertices: ['npm'], hyperedges: []});
        const result = {backend: viewer.backend, selection: viewer.getState().selection, renderer: viewer.getState().renderer};
        await viewer.dispose(); return result;
      }, backend);
      assert.equal(result.backend, backend);
      if (software) assert.match(JSON.stringify(result.renderer), /swiftshader|llvmpipe|software|"deviceType":"Cpu"/i);
      else assert.doesNotMatch(JSON.stringify(result.renderer), /swiftshader|llvmpipe|software|"deviceType":"Cpu"/i);
      assert.deepEqual(result.selection.vertices, ['npm']);
      assert.equal(requests.length, 1);
      assert.ok(requests[0].includes(`/base/hyper/renderers/${backend}/`));
      page.off('request', listener);
      console.log(`PASS ${software ? 'software' : 'hardware'} packed npm consumer: ${JSON.stringify(result)}`);
    }
    await page.close();
  }
} finally {
  await browser?.close();
  if (server) await new Promise(resolve => server.close(resolve));
  await rm(consumer, {recursive: true, force: true});
}
