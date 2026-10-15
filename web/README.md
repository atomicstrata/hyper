# @atomicstrata/hyper

Embed interactive 3D hypergraphs in JavaScript and TypeScript applications.
WebGPU is preferred, with WebGL2 startup fallback. Each canvas owns an
independent graph, camera, selection, settings, WASM memory, and lifecycle.
The package includes both prebuilt renderers and TypeScript declarations,
with no runtime npm dependencies or consumer Rust requirement.

Install the public [npm package](https://www.npmjs.com/package/@atomicstrata/hyper)
using the quick start below. Unreleased candidates are available as verified
`.tgz` files in the
[Browser SDK workflow](https://github.com/atomicstrata/hyper/actions/workflows/browser.yml)'s
`hyper-browser-npm` artifact or from a local build; install a candidate with
`npm install /absolute/path/to/atomicstrata-hyper-<version>.tgz`.

[Quick start](#quick-start) · [Asset hosting](#asset-hosting-and-bundlers) ·
[Data and IDs](#load-data-and-select-by-id) · [Multiple viewers](#multiple-viewers) ·
[API](#api-reference) · [Settings](#settings-reference) · [Troubleshooting](#troubleshooting)

## Quick start

In an existing ESM application, install the package and copy its renderer assets
into the app's static directory:

```sh
npm install @atomicstrata/hyper
npx hyper-copy-assets public/hyper
```

Node.js 22+ is required for the asset-copy command. Rendering runs in the
browser. The following example works in a bundled app serving `public/` at the
site root, such as a default Vite app.

Add a connected, sized canvas and HTML controls to `index.html`:

```html
<div style="width:100%;height:500px">
  <canvas id="graph" tabindex="0" style="display:block;width:100%;height:100%"></canvas>
</div>
<pre id="selection"></pre>
<button id="dispose" type="button" disabled>Dispose viewer</button>
<script type="module" src="./src/main.js"></script>
```

Save this as `src/main.js`:

```js
import {createHyperViewer} from '@atomicstrata/hyper';

const jsonText = `{
  "version": "hypergraph.v1",
  "vertices": [
    {"id": "alice", "label": "Alice"},
    {"id": "bob", "label": "Bob"},
    {"id": "carol", "label": "Carol"}
  ],
  "hyperedges": [{"id": "team", "vertices": ["alice", "bob", "carol"]}]
}`;

const viewer = await createHyperViewer({
  canvas: document.querySelector('#graph'),
  backend: 'auto',
  manifestUrl: new URL('/hyper/renderers.json', location.href),
});

const output = document.querySelector('#selection');
const unsubscribe = viewer.on('selectionchange', selection => {
  output.textContent = JSON.stringify(selection, null, 2);
});
const unsubscribeError = viewer.on('error', message => console.error(message));

try {
  await viewer.loadJson(jsonText);
  await viewer.select({vertices: ['alice'], hyperedges: []});
  await viewer.frame();
} catch (error) {
  unsubscribe();
  unsubscribeError();
  await viewer.dispose();
  throw error;
}

const disposeButton = document.querySelector('#dispose');
disposeButton.disabled = false;
disposeButton.addEventListener('click', async () => {
  disposeButton.disabled = true;
  unsubscribe();
  unsubscribeError();
  await viewer.dispose();
  viewer.canvas.remove();
}, {once: true});
```

Run your app's dev server and open its HTTP localhost URL. Drag to orbit,
scroll to zoom, and click to select. With the canvas focused, **Space** toggles
layout and **F** frames the view. HTML inputs retain their keyboard behavior.
The app supplies HTML controls; the native viewer HUD is hidden.

TypeScript uses the same runtime import and includes named types:

```ts
import {createHyperViewer, type HyperViewer, type ViewerSettings} from '@atomicstrata/hyper';

async function mount(canvas: HTMLCanvasElement): Promise<HyperViewer> {
  const viewer = await createHyperViewer({canvas, manifestUrl: '/hyper/renderers.json'});
  const settings: Partial<ViewerSettings> = {layoutRunning: false, hulls: true};
  await viewer.setSettings(settings);
  return viewer;
}
```

This is an ESM package; CommonJS is not provided. NodeNext and Bundler TypeScript
module resolution are verified. Create viewers on the client after mounting
the DOM in server-rendered applications.

## Asset hosting and bundlers

The JavaScript import does **not** copy dynamically loaded renderer files.
Re-run `hyper-copy-assets` after package upgrades and before the application
build. For an app that already uses Vite, merge these scripts into its
`package.json`:

```json
{
  "scripts": {
    "hyper:assets": "hyper-copy-assets public/hyper",
    "dev": "npm run hyper:assets && vite",
    "build": "npm run hyper:assets && vite build"
  }
}
```

The destination contains:

```text
public/hyper/
  renderers.json
  renderers/webgpu/   # hashed binding module and WASM
  renderers/webgl2/   # hashed binding module and WASM
  licenses/          # Hyper, dependency, and original font notices
```

Deploy the complete copied directory, including `licenses/`. Assets resolve
relative to `renderers.json`; both backends must come from the same build.
Successful startup fetches only the chosen backend. Current WASM files are
about 50 MiB each before HTTP compression; enable gzip/Brotli. Each viewer has
its own runtime memory, while compiled WASM modules are cached for reuse.

For an app deployed at `/my-app/`, host the copied files at
`/my-app/hyper/` and set `manifestUrl: '/my-app/hyper/renderers.json'`. In Vite,
use the configured base path:

```js
const manifestUrl = new URL(
  `${import.meta.env.BASE_URL}hyper/renderers.json`,
  document.baseURI,
);
```

See [Vite's public base-path documentation](https://vite.dev/guide/build.html#public-base-path).
For client-side routed apps, use the configured deployment base so asset URLs
do not change with the current route. Other frameworks should copy into their
static/public directory and pass its deployed manifest URL explicitly.

For direct ESM usage without a bundler, serve the package's entire `dist/`
directory intact and import its `index.mjs`. The default manifest URL then
works because it sits beside the module. Serve modules as JavaScript and WASM
as `application/wasm`. HTTPS is required for WebGPU; HTTP localhost is accepted.
Cross-origin assets require CORS headers.

Hashed renderer assets can use immutable caching. Revalidate the JavaScript SDK
and manifest together. For upgrades, a versioned asset directory avoids mixing
old pages with a newer manifest. The copy command preserves unrelated files and
older hashed assets; retain old assets while cached pages still need them.

A restrictive CSP needs `script-src 'self' 'wasm-unsafe-eval'` and
`connect-src 'self'`, including the asset origin if different. Configure styles
and other app content separately. The SDK needs no JavaScript eval, workers,
SharedArrayBuffer, or cross-origin isolation headers.

## Load data and select by ID

`loadJson` accepts HIF and native `hypergraph.v1` JSON through Hyper's Rust
parser. Invalid or viewer-incompatible input rejects and retains the previous
graph. The viewer supports undirected memberships; see the
[HIF compatibility guide](https://github.com/atomicstrata/hyper/blob/main/docs/HIF.md#supported-semantics).

Fetch JSON as text, checking the HTTP response:

```js
const response = await fetch('/data/graph.hif.json');
if (!response.ok) throw new Error(`Dataset request failed: ${response.status}`);
await viewer.loadJson(await response.text());
```

For a browser file input, use `await viewer.loadJson(await file.text())`.
Keep imported JSON as text: `JSON.parse` followed by `JSON.stringify` can
irreversibly round numeric IDs above `2^53` before Rust sees them.

Native IDs are preserved directly (`alice`, `team` in the quick start). HIF IDs
use type prefixes: integer `1` becomes `i:1`, string `"1"` becomes `s:1`.
The same rule applies to hyperedge IDs. For example:

```js
await viewer.loadJson(`{
  "network-type": "undirected",
  "nodes": [{"node": 1}, {"node": "1"}, {"node": 9007199254740993}],
  "edges": [{"edge": "group"}],
  "incidences": [
    {"node": 1, "edge": "group"},
    {"node": "1", "edge": "group"},
    {"node": 9007199254740993, "edge": "group"}
  ]
}`);
await viewer.select({
  vertices: ['i:1', 's:1', 'i:9007199254740993'],
  hyperedges: ['s:group'],
});
```

Selection events contain these same native IDs, not array offsets. Unknown IDs
reject the entire selection request. Scene replacement and projection changes
preserve positions and selection by stable ID where possible. The source graph
is retained so switching among all three projections does not lose memberships.

## Multiple viewers

Give each viewer its own connected, sized canvas. For two existing canvas
elements, `#left` and `#right`:

```js
const [left, right] = await Promise.all([
  createHyperViewer({canvas: document.querySelector('#left'), manifestUrl}),
  createHyperViewer({canvas: document.querySelector('#right'), manifestUrl}),
]);
await Promise.all([left.loadJson(jsonText), right.loadJson(jsonText)]);
await left.setProjection('clique');
await right.setSettings({layoutRunning: false});
await left.dispose();
await right.frame(); // The second viewer continues independently.
await right.dispose();
```

Backend, graph, selection, settings, camera, input, resizing, events, and
disposal are independent. Different viewers can use different forced backends.
One canvas cannot belong to two active viewers.

## API reference

### Creation options

`createHyperViewer(options): Promise<HyperViewer>` resolves after Bevy's first
completed app update; it does not imply a graph was loaded or the layout settled.

| Option | Type / default | Behavior |
|---|---|---|
| `canvas` | `HTMLCanvasElement`, required | Must be connected with a nonzero parent size |
| `backend` | `'auto'` (default), `'webgpu'`, `'webgl2'` | Auto probes WebGPU, then falls back to WebGL2 on unavailability or startup failure |
| `manifestUrl` | `string \| URL`; `renderers.json` beside the SDK module | Set explicitly when bundling or serving assets elsewhere |
| `sessionKey` | `string`, omitted by default | Nonempty localStorage key enabling saved views; use distinct keys for independent sessions |

The WebGPU capability probe is bounded to three seconds. Each renderer startup
attempt is bounded to 30 seconds. Forced backend selection rejects without
switching. Later device loss emits an `error` event; it does not trigger recovery.

### Viewer methods and properties

| Member | Result | Behavior |
|---|---|---|
| `canvas` | `HTMLCanvasElement` | Current canvas, which can replace the supplied element during fallback/backend remount |
| `backend` | `'webgpu' \| 'webgl2'` | Backend that actually started |
| `loadJson(text)` | `Promise<void>` | Validate and replace the graph |
| `setProjection(value)` | `Promise<void>` | `'bipartite'` (initial default), `'clique'`, or `'star'` |
| `select({vertices, hyperedges})` | `Promise<void>` | Replace selection using native string IDs; provide both arrays |
| `clearSelection()` | `Promise<void>` | Clear vertex and hyperedge selections |
| `frame()` | `Promise<void>` | Request camera framing; the promise does not wait for camera interpolation to settle |
| `setSettings(patch)` | `Promise<void>` | Validate and apply a partial settings object atomically |
| `getState()` | `ViewerState` | Detached snapshot of the latest acknowledged/published metadata |
| `on(type, listener)` | `() => void` | Subscribe to an event; call the returned function to unsubscribe |
| `dispose()` | `Promise<void>` | Idempotent teardown through Bevy's exit path |

Mutation promises resolve when applied. Commands have a 30-second timeout and
the queue accepts at most 64 waiting commands. A timed-out command may already
have applied, so inspect state before deciding to retry. Normal validation
errors reject the method promise; handle those with `try`/`catch`.

`getState()` includes `projection`, `selection`, `settings`, graph counts/title/
warnings/iteration count, and optional camera and actual renderer metadata.
Node positions remain inside Bevy. Mutating a returned snapshot does not change
the viewer; use the methods above.

### Events

| Event | Payload | Timing |
|---|---|---|
| `selectionchange` | `{vertices: string[], hyperedges: string[]}` | Immediate after a changed selection applies, including pointer selection |
| `statechange` | `ViewerState` | Changed metadata at most five times per second |
| `error` | Message `string` | Runtime errors, including later device loss |

```js
const off = viewer.on('statechange', state => {
  console.log(state.graph.vertices, state.settings.layoutRunning);
});
await viewer.setSettings({layoutRunning: false});
console.log(viewer.getState().settings.layoutRunning); // false after acknowledgement
off();
```

## Settings reference

Use `setSettings` with only the fields you want to change. Unknown fields,
invalid types, `null`, nonfinite numbers, and out-of-range values reject the
entire patch. Defaults follow the existing viewer; read
`viewer.getState().settings` for the current values.

| Setting | Accepted value / meaning |
|---|---|
| `layoutRunning` | Boolean; pause or resume layout |
| `iterationsPerFrame` | Integer, 1–16 |
| `focus` | Boolean; enabling requires a nonempty selection |
| `attention` | Boolean; enable attention mode |
| `lasso` | Boolean; enable lasso selection |
| `hulls` | Boolean; draw hyperedge hulls |
| `hideHubs` | Boolean; hide hyperedge hubs |
| `wireframe` | Boolean; draw hull wireframes |
| `opacity` | Number, 0–1; hull opacity |
| `vertexLabels`, `hyperedgeLabels` | Booleans |
| `labelMode` | `'capped'`, `'selection'`, or `'all'` |
| `labelScale` | Number, 0.1–10 |
| `labelVariation` | Number, 0–1 |
| `dt` | Number, 0.001–1; layout timestep |
| `damping` | Number, 0–1 |
| `repulsion` | Number, 0–1,000,000 |
| `attraction`, `gravity` | Numbers, 0–100 |
| `idealLength` | Number, 0.01–100,000 |
| `theta` | Number, 0.01–2 |

## Lifecycle and frameworks

Create viewers after the canvas is mounted; await `dispose()` before reusing a
canvas. Dispose before removing a view. Repeated disposal returns the same
promise. After disposal, mutation methods reject and `getState()`/`on()` throw.
Browser garbage collection is asynchronous; disposal does not promise immediate
memory reclamation.

Fallback and switching backends on remount can replace a canvas because browser
canvases cannot switch context types. Access the current element through
`viewer.canvas`; element-specific listeners and references on the original
canvas do not transfer. In React/Vue/Svelte or another DOM-managed framework,
prefer a host container with an imperatively created canvas so reconciliation
does not restore a stale element. Own viewer creation and disposal in the
component lifecycle, and dispose a viewer that finishes starting after its
component has been cancelled. Route/component controls should catch rejected
mutation promises and unsubscribe listeners during cleanup.

Persistence is disabled by default. `sessionKey` opts into namespaced
localStorage; unavailable storage does not prevent rendering. Use different
keys for different saved views, and omit the option for temporary viewers.

## Troubleshooting

| Symptom | Check |
|---|---|
| Blank or zero-size canvas | Connect it before creation and give its parent explicit dimensions; inspect rejected startup promises and error events |
| `renderers.json` or WASM returns 404 | Run the asset-copy command, deploy its complete output, and verify `manifestUrl` against the app's deployment base |
| WASM fetch returns HTML | Exclude renderer paths from SPA fallback rewrites; check response status, body and MIME type |
| CSP or cross-origin rejection | Allow the module/WASM asset origin, `wasm-unsafe-eval`, and the required CORS responses |
| WebGPU unavailable | Use HTTPS/localhost and working browser graphics; `auto` can fall back, forced `webgpu` cannot |
| Unknown selection ID | Use native IDs, including HIF's `i:`/`s:` prefixes; get IDs from data and selection events |
| Focus patch rejects | Select a vertex or hyperedge before setting `focus: true` |
| Canvas already belongs to a viewer | Reuse the existing viewer, or await its disposal before creating another |
| Startup timeout or device loss | Handle the rejected creation promise/runtime error; remount explicitly if desired and report adapter metadata from `getState().renderer` |
| Viewer API rejects after navigation | Avoid stale component references; dispose/unsubscribe during unmount and create a new instance on remount |

Hardware verification currently covers Chrome on macOS with WebGPU and WebGL2.
Software WebGL2 is tested separately in CI. Safari, Firefox, and mobile remain
unverified; test your target environments before relying on them.

## More documentation

- [Browser build and demo guide](https://github.com/atomicstrata/hyper/blob/main/docs/BROWSER.md)
- [npm release and CI artifacts](https://github.com/atomicstrata/hyper/blob/main/docs/NPM_RELEASE.md)
- [HIF data compatibility](https://github.com/atomicstrata/hyper/blob/main/docs/HIF.md)
- [am-next migration](https://github.com/atomicstrata/hyper/blob/main/docs/AM_NEXT_BROWSER_MIGRATION.md)
- [Issues](https://github.com/atomicstrata/hyper/issues)

Licensed under MIT OR Apache-2.0. Keep the included licenses and notices with
the package and hosted renderer assets.
