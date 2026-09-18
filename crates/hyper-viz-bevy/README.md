# hyper-viz-bevy

Native 3D interactive hypergraph viewer (Bevy 0.18 + egui). Depends on
`hyper-viz` only.

Optional: other Rust projects that only need schema / layout should depend on
`hyper-viz` and skip this crate.

```toml
hyper-viz-bevy = { git = "https://github.com/atomicstrata/hyper.git" }
```

Workspace overview, JSON schema, and interaction notes live in the
[root README](../../README.md).

## Run

From the workspace root:

```bash
cargo run
cargo run -- fixtures/sample.json
cargo run -p hyper-viz-bevy --example view -- fixtures/sample.json
```

## Embed

Standalone (blocking window):

```rust
use hyper_viz_bevy::{VisualizerConfig, run_from_graph, run_visualizer, run_visualizer_live};

run_from_graph(graph);                 // bipartite + window
run_visualizer(scene);                 // already projected
run_visualizer_live(scene, rx, None); // host thread pushes scenes
```

Inside an existing Bevy `App` (add after `DefaultPlugins`):

```rust
use hyper_viz_bevy::HyperVisualizerPlugin;

app.add_plugins(HyperVisualizerPlugin::from_scene(scene));
```

`VisualizerConfig` sets window title and size for the standalone helpers.
Pass `Some(shutdown_flag)` to `run_visualizer_live` / `.with_shutdown(...)`
so a host thread can exit the Bevy app.

`visualizer_app(scene)` returns an `App` without running it, if you need to
insert extra plugins first.

## Interaction (summary)

- **Left drag:** orbit (does not change selection)
- **Click (no drag):** select vertex, hull, or arity-2 line
- **Click empty:** clear selection
- **Hover:** size / brighter wire, same hue
- **Selected:** higher saturation and glow, same hue
- **⌘F / Ctrl+F**: focus the bottom find bar (BM25 + Jaro–Winkler)
- **⌃⌘F**: isolate the selected neighborhood (toggle)
- **Enter** (in find): isolate match neighborhood and frame
- While focus is on, each new query re-isolates and reframes
- **Esc**: clear find query, or clear focus

Hubs are hidden by default. Hulls are on for arity ≥ 3.

## Session

On quit (and ~450ms after the last change) the viewer writes
`hyperviz.session.v1` JSON: layout sliders, labels, hulls, attention, navigation
(lasso), window size, plus per-graph camera pose (orbit + world position),
selection, focus neighborhood, and find. Desktop path is
the OS config dir (`hyper-viz/session.json`); wasm uses `localStorage`.
`HYPER_VIZ_SESSION` overrides the file or `off` disables it.
