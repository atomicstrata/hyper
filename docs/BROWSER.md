# Embed Hyper in a browser

The browser SDK reuses Hyper's Rust core and Bevy renderer. It supports several
independent direct-canvas viewers on one page, with separate WASM instances,
graphs, cameras, settings, input and lifecycle. WebGPU is preferred; a separate
WebGL2 build provides startup fallback. The public npm package is
[`@atomicstrata/hyper`](https://www.npmjs.com/package/@atomicstrata/hyper), first
released as `0.2.0`.

## Use the npm package

The package contains prebuilt renderers, ESM JavaScript and TypeScript
declarations. Consumers do not need Rust:

```sh
npm install @atomicstrata/hyper
npx hyper-copy-assets public/hyper
```

Import `createHyperViewer` from `@atomicstrata/hyper` and pass the hosted
`manifestUrl` (for example `/hyper/renderers.json`). Bundlers do not copy dynamic
renderer assets automatically. Re-run the asset command during your app build
after package upgrades. See the [package README](../web/README.md) for complete
consumer and subpath examples and the [npm release guide](NPM_RELEASE.md) for
tarball verification, CI artifacts and trusted publishing setup. For unreleased
candidates, install the verified local/CI `.tgz` instead.

## Build and try the demo

Requires Rust 1.89+, Node.js 22+, npm, and wasm-bindgen-cli matching the
`wasm-bindgen` version in Cargo.lock (currently 0.2.127), plus cargo-about 0.9.2
for bundled dependency/font notices:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.127 --locked
cargo install cargo-about --version 0.9.2 --locked --features cli
npm ci --prefix web
npm run build --prefix web
npm run serve --prefix web
```

Open [the two-view demo](http://localhost:8080/demo/). Use
`?backend=webgpu` or `?backend=webgl2` to force a renderer. The demo loads native
JSON and a HIF fixture containing distinct integer/string IDs and an empty edge.
The backend build outputs are assembled before replacing the previous `web/dist`.
`WASM_BINDGEN` can point to an existing matching executable.
`CARGO_ABOUT` can similarly point to a cargo-about binary. Generated notices
ship in `dist/licenses/` and are retained by the asset-copy CLI.

## JavaScript and TypeScript

The [package README](../web/README.md) is the consumer reference: a complete
bundled-app quick start, creation options, all methods/settings, HIF ID examples,
multiple viewers, framework lifecycle guidance and troubleshooting. This guide
focuses on building Hyper itself and serving its static demo or ESM output.

Serve the entire `web/dist` directory. Import its `index.mjs`; TypeScript
interfaces are in `index.d.ts`. No framework, backend or CDN is required.

```html
<div style="width: 100%; height: 500px">
  <canvas id="graph" tabindex="0" style="display:block;width:100%;height:100%"></canvas>
</div>
<script type="module">
  import {createHyperViewer} from './hyper/index.mjs';

  const viewer = await createHyperViewer({
    canvas: document.querySelector('#graph'),
    backend: 'auto',
  });
  await viewer.loadJson(await (await fetch('./dataset.hif.json')).text());
  const unsubscribe = viewer.on('selectionchange', selection => {
    console.log(selection.vertices, selection.hyperedges);
  });

  // Before removing this view:
  // unsubscribe();
  // await viewer.dispose();
</script>
```

Keep JSON as text through `loadJson`; parsing large numeric HIF IDs into a
JavaScript Number first can lose precision. HIF IDs are exposed using Hyper's
existing type-prefixed native identities (`i:1` and `s:1`, for example).
Viewer compatibility follows the same rules as [desktop HIF](HIF.md).

Call `createHyperViewer` again with another connected canvas to create another
viewer. Each instance allocates its own WASM memory and renderer; compiled WASM
modules and fetched binding-factory modules are cached. This isolation costs
more memory than a shared multi-view renderer. No per-frame node coordinates
are transferred to JavaScript.

## Controls, events and lifecycle

- `loadJson(text)`: validate and replace the graph. Invalid or incompatible input
  retains the current graph. Empty graphs, isolated vertices and empty edges are
  accepted according to the core's existing semantics.
- `setProjection('bipartite' | 'clique' | 'star')`: project the retained source
  graph; preserve node positions and selection by stable ID where possible.
- `select({vertices, hyperedges})`, `clearSelection()`, `frame()`: manipulate
  the viewer. Unknown IDs reject the entire selection request.
- `setSettings(patch)`: change layout, focus, attention, lasso, hull and label
  controls. Unknown fields and out-of-range values reject the entire patch.
  Enabling focus requires a selection. Numeric bounds are documented in the
  TypeScript declarations.
- `getState()`: obtain a detached metadata snapshot, including graph counts,
  warnings, projection, selection, camera metadata, actual adapter information and
  current settings.
- `on('selectionchange' | 'statechange' | 'error', callback)`: subscribe and
  receive an unsubscribe function. Selection changes are emitted after they
  apply; changed metadata snapshots are published at most five times per second.
  Error events carry a message string. Listener exceptions are logged without
  unwinding into the renderer.
- `dispose()`: stop the app, release its listeners and runtime references, and
  resolve after Bevy exits. Repeated calls return the same promise. Other
  operations reject after disposal. Browser garbage collection is asynchronous;
  disposal does not promise immediate memory reclamation.

Mutation promises resolve after Bevy applies the command. The bounded command
queue holds at most 64 waiting commands; overflow rejects the submission.
Commands and renderer initialization each have a 30-second timeout. A timed-out
command may already have been applied; do not automatically retry non-idempotent
application actions based on that timeout.

The parent element controls canvas size. Give it explicit dimensions; the
renderer follows parent resizing and display pixel ratio. Only the focused
canvas owns graph keyboard shortcuts; HTML inputs keep their keyboard behavior.
The SDK hides the native HUD while retaining graph labels and lasso drawing,
so applications can provide their own HTML controls.

Startup fallback replaces a canvas that has already acquired a WebGPU context.
Remounting an SDK canvas with a different backend also replaces that element,
because browser canvases cannot switch context types.
Use `viewer.canvas` to access the current element and SDK events for selections.
Forced backend selection never silently switches. Later device loss is reported
as an error; automatic recovery after successful startup is not provided.

Persistence is disabled by default. Set `sessionKey` to a unique nonempty string
for opt-in localStorage persistence; use different keys for independent saved
sessions. Storage unavailability does not prevent rendering. Native viewers keep
their existing session location and defaults.

## Hosting

For direct ESM hosting, serve `renderers.json`, both `renderers/` directories,
`licenses/` and `index.mjs` together. For a bundled npm consumer, deploy the
asset-copy output and pass its manifest URL; the application bundle supplies
the SDK's JavaScript entry.
Use `application/wasm` for WASM and a JavaScript MIME type for `.mjs`. Hashed
renderer assets can be cached immutably; revalidate the SDK and manifest together.
Both backend artifacts must be deployed from the same build. The current
optimized WASM files are 49.9 MiB (WebGPU) and 51.3 MiB (WebGL2) before HTTP
compression; enable Brotli/gzip and measure loading on your target connections.
Normal successful startup downloads only the chosen backend.

Asset URLs resolve relative to the manifest. For another asset location, pass
`manifestUrl: new URL('/assets/hyper/renderers.json', location.href)`. Cross-origin
assets require appropriate CORS headers. Use HTTP locally and HTTPS in production;
WebGPU requires a secure context (localhost is accepted).

A restrictive CSP needs `script-src 'self' 'wasm-unsafe-eval'` and
`connect-src 'self'` (add the chosen asset origin if different). The binding
factory uses no JavaScript `eval`, blob URLs, worker or iframe. The SDK does not
require SharedArrayBuffer or cross-origin isolation headers. Hosts own CSP and
asset deployment; Hyper does not publish or deploy this build automatically.

## Verify

```sh
npm test --prefix web
npm run test:package --prefix web
web/node_modules/.bin/tsc --noEmit --strict --lib ES2022,DOM web/src/index.d.ts
cargo test --locked -p hyper-viz-web --lib
cargo check --locked -p hyper-viz-web --target wasm32-unknown-unknown --no-default-features --features webgpu
cargo check --locked -p hyper-viz-web --target wasm32-unknown-unknown --no-default-features --features webgl2
HYPER_BROWSER='/path/to/chromium' npm run test:browser --prefix web
```

The real-browser harness requires hardware-capable WebGPU and WebGL2 and reports
its browser/adapter details; software rendering cannot establish hardware support.
Use `HYPER_HEADLESS=0` for a visible run. A screenshot is written to
`/tmp/hyper-browser-sdk.png` by default (`HYPER_SCREENSHOT` overrides it).
Also run the repository checks in [CONTRIBUTING.md](../CONTRIBUTING.md).

## Verification recorded on 2026-10-13

Both optimized backends built with repository Rust 1.89.0. Hardware verification
passed 20 scenarios in Chromium 153.0.8010.12 on Apple M5 Pro: actual Bevy
backends were BrowserWebGpu and Gl (ANGLE Metal), with no software adapter.
Coverage includes mixed simultaneous viewers, pointer/keyboard/camera/resize
isolation, repeated teardown, rejected and hung device initialization, saved
views, restrictive CSP and scoped device loss. The actual two-view demo's
loading, pause, disposal and remount controls also passed. This records one
browser/platform; it does not establish Safari, Firefox or mobile coverage.

Repository verification passed 192 Rust tests, 24 SDK/package JavaScript tests, 17 Python
API/launcher tests, ten Python utility/packaging tests, both WASM clippy targets, workspace
clippy/docs, formatting, TypeScript declarations, generated Python stubs and
headless examples. The local Python extension needed installed Rust 1.98 to
avoid a macOS LINKEDIT load error from the Rust 1.89 dylib; native Rust checks
and the browser artifacts used the repository toolchain.

The npm tarball also passed a clean consumer install, ESM bundling, TypeScript
NodeNext/Bundler checks, asset-copy checks and rendering under a subpath on both
hardware backends. Software WebGL2 was verified separately with SwiftShader;
the consumer harness checks adapter evidence before labeling a hardware pass.
