# Migrate am-next's browser integration

This guide describes a follow-up in `am-next`; this Hyper change does not modify
that repository or move its application responsibilities into Hyper.

## Establish the dependency baseline

The local `am-next/vendor/hyper` symlink resolves to this Hyper checkout.
Its CI checkout script and deploy workflow pin an older Hyper revision
(`9d9afc15353c8b0dbd781f089cf80eedb6cfc797`). Update those pins to the reviewed
Hyper revision containing the browser SDK before removing any patch scripts.
Test both the local sibling checkout and the exact CI-pinned checkout; local
success alone does not prove the pin contains the necessary interfaces.

## Two supported integration paths

**Keep the Rust-composed application:** `am-web` can continue adding
`HyperVisualizerPlugin` to its own Bevy `App` alongside its in-memory engine,
room replication and application plugins. The supported `host` exports include
layout/selection/settings, `ExternalUi` and `SessionConfig`. Browser builds use
one of the viewer's `webgpu`/`webgl2` features; keep native platform features
separate from browser features. This path preserves the existing one-process
Rust composition and its scene channel.

**Use the JavaScript SDK:** mount one or more Hyper viewers using
`createHyperViewer`, feeding native/HIF JSON snapshots from the application.
Keep the memory engine in its own application runtime and perform updates
through `loadJson`. Each viewer owns a separate WASM runtime; this is a different
composition from today's combined engine/viewer module, with additional memory
and serialization cost. Do not move `am-hg`, source import, extraction, retrieval
or replication into Hyper to avoid that boundary.

## Map the existing integration

| am-next integration | Hyper replacement or migration action |
|---|---|
| `workspace/graphics.mjs` | SDK capability detection; WebGPU adapter probing and WebGL2 fallback |
| `workspace/renderer-loader.mjs` | `createHyperViewer({canvas, backend})` plus the generated renderer manifest |
| `scripts/build-web-demo.sh` and packaging | Build the Hyper SDK separately; package its full `dist` directory under a stable asset prefix |
| `amWebRendererReady` | Creation promise resolves after Bevy's first app update |
| `amWebCommand` | Typed instance methods returning application acknowledgements |
| `amWebState` | Instance `statechange`/`selectionchange` events and `getState()` |
| `hyper-host-ui.patch` | Supported `host` exports and `ExternalUi`; hide HUD while retaining labels/lasso |
| `hyper-live-app.patch` | Existing `visualizer_app_live` and `HyperVisualizerPlugin::with_live` for Rust composition |
| `hyper-focus-mode.patch` | Compare behavior before removal: the old pin's focus patch differs from current focus semantics; SDK enabling focus requires a selection |
| `hyper-graph-toggle.patch` | Keep its graph-off scheduling and engine/UI behavior in am-next; layout pause and full viewer disposal are distinct SDK operations |

Keep search scopes, bounded import pipelines, room state, save acknowledgements,
and extraction UX in `am-next`. Hyper's browser bridge intentionally exposes
visualization commands and metadata rather than those application commands.

Remove a local patch only after the corresponding consumer behavior passes
against the new pinned revision. The current Hyper checkout already contains
uncommitted host hooks from the local integration; those are preserved and
extended by the SDK implementation, not evidence that the older CI pin has them.

## Consumer acceptance

Build both `am-web` renderer features against the new pin. Re-run its graphics,
loader, workspace-state, import and native Rust tests. Verify Remember, source
search, room replication, selection, keyboard ownership and graph-off/resume
behavior; these remain application responsibilities. For an SDK migration, add
two-view isolation and disposal/remounting checks and preserve HIF identities as
JSON text. Check the actual backend and downloaded assets in the browser rather
than inferring them from Cargo feature names.
