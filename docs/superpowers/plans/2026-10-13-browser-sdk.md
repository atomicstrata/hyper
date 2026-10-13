# Hyper browser SDK implementation plan

Approved by the user on 2026-10-13. Implement in this checkout, preserving the
existing host integration used through am-next/vendor/hyper's symlink.

1. Add an explicit hyper-viz-web crate, portable dependency features, and tests
   for graph loading, stable selection, projection changes, atomic settings,
   and independent command queues.
2. Add per-instance wasm-bindgen runtimes, explicit startup, callback bridges,
   canvas-scoped input, optional namespaced sessions, and clean Bevy exit.
3. Add an ESM/TypeScript SDK with cached compiled modules and fresh binding
   factories, WebGPU/WebGL2 capability selection, bounded startup, fallback,
   promise acknowledgements, events, and idempotent disposal.
4. Package both optimized backend builds using esbuild factories. Add a two-view
   static demo, tests of the actual browser artifacts, documentation and an
   am-next migration guide. Do not publish or modify am-next.
5. Run both WASM build gates, browser scenarios, and CONTRIBUTING.md checks;
   review the final diff and report verification limitations honestly.

Acceptance: two simultaneous direct-canvas viewers have independent data,
selection, settings, cameras and lifecycle; forced backends never silently
switch; auto downloads only the selected backend on successful startup; bad
inputs retain the prior graph; replacement/projection preserve stable IDs;
HIF integer/string and large integer identities remain distinct. Selection
notifications are immediate after application, metadata updates are capped at
5 Hz, and no per-frame coordinates cross into JavaScript.
