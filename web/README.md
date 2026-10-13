# @atomicstrata/hyper

An embeddable hypergraph viewer for JavaScript and TypeScript applications.
Each canvas owns its own WASM instance, graph, camera, selection, settings, and
lifecycle. WebGPU is preferred, with automatic WebGL2 fallback. The package
includes both prebuilt renderers and has no runtime npm dependencies.

## Install and host the renderer assets

```sh
npm install @atomicstrata/hyper
npx hyper-copy-assets public/hyper
```

Serve the copied files as static assets. For a Vite app, `public/hyper` is
available at `${import.meta.env.BASE_URL}hyper/`; for other frameworks use their
public/static directory. Re-run the copy command after updating the package,
for example in your app's build script. The SDK loads only the selected backend
on successful startup. The renderers are about 50 MiB each before HTTP
compression; enable gzip or Brotli on your host.
Keep the copied `licenses/` directory with the renderer assets; it includes
notices for bundled dependencies and fonts.

```ts
import {createHyperViewer, type HyperViewer} from '@atomicstrata/hyper';

const canvas = document.querySelector<HTMLCanvasElement>('#graph')!;
const viewer: HyperViewer = await createHyperViewer({
  canvas,
  backend: 'auto',
  manifestUrl: new URL('./hyper/renderers.json', document.baseURI),
});

await viewer.loadJson(jsonText); // HIF or native hypergraph.v1 JSON text
await viewer.setProjection('bipartite');
await viewer.setSettings({hulls: true, layoutRunning: true});
await viewer.select({vertices: ['alice'], hyperedges: []});
await viewer.frame();

const unsubscribe = viewer.on('selectionchange', selection => {
  console.log(selection.vertices, selection.hyperedges);
});

// When removing the component:
unsubscribe();
await viewer.dispose();
```

Give the canvas a nonzero CSS size and connect it to the document before
creation. `viewer.canvas` is the current element; fallback or remounting with a
different backend can replace the original element. Always dispose before
unmounting. Use one viewer per canvas; multiple viewers can run simultaneously.

For apps under a subpath, point `manifestUrl` to the deployed asset directory:

```ts
const manifestUrl = new URL(`${import.meta.env.BASE_URL}hyper/renderers.json`, location.origin);
```

For direct ESM usage without bundling, serve the package's `dist` directory
intact and import its `index.mjs`; the default manifest URL then works. Bundlers
do not automatically copy the dynamically loaded renderers, so use the asset
command and explicit `manifestUrl`. Import and create viewers only on the
client in server-rendered applications.

## API

- `loadJson(text)`: parse HIF or native JSON in Rust; invalid input retains the
  previous graph. Keep the input as text to preserve integers above `2^53`.
- `setProjection('bipartite' | 'clique' | 'star')`: reproject the retained source.
- `select({vertices, hyperedges})`, `clearSelection()`, `frame()`.
- `setSettings(patch)`: atomically validate and apply layout, focus, attention,
  lasso, hull, hub, and label controls; see the included TypeScript declarations.
- `getState()`: detached graph, selection, camera, settings, and adapter metadata.
- `on('selectionchange' | 'statechange' | 'error', listener)`: returns unsubscribe.
- `dispose()`: idempotent asynchronous teardown; other operations reject afterward.

Mutation promises resolve after application. Selection events are immediate;
changed state metadata is published at most five times per second. Node
positions stay inside Bevy. Persistence is disabled unless you pass a nonempty
`sessionKey`; use distinct keys for independent saved views.

Forced `webgpu` or `webgl2` selection fails without switching. Later device loss
emits an `error` event; automatic recovery after startup is not provided.

## Hosting and compatibility

Use HTTPS (localhost works during development) and serve WASM with
`Content-Type: application/wasm` and modules as JavaScript. Cross-origin assets
need CORS headers. A restrictive CSP needs `script-src 'self' 'wasm-unsafe-eval'`
and `connect-src 'self'`, including your asset origin when different. No
JavaScript eval, workers, SharedArrayBuffer, or cross-origin isolation is needed.

Hardware verification currently covers Chrome on macOS with both WebGPU and
WebGL2. Safari, Firefox, and mobile remain unverified. Node.js 22+ is needed only
for the asset-copy CLI; rendering runs in the browser. This is an ESM package
with TypeScript declarations; CommonJS is not provided.

[Full browser guide](https://github.com/atomicstrata/hyper/blob/main/docs/BROWSER.md)
· [Source and demo](https://github.com/atomicstrata/hyper/tree/main/web)

Licensed under MIT OR Apache-2.0. See the included licenses and NOTICE.
