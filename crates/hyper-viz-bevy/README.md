# hyper-viz-bevy

Native 3D interactive hypergraph viewer (Bevy 0.18 + egui). Depends on
`hyper-viz` only.

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

```rust
use hyper_viz_bevy::{VisualizerConfig, run_from_graph, run_visualizer, run_visualizer_live};

run_from_graph(graph);                 // bipartite + window
run_visualizer(scene);                 // already projected
run_visualizer_live(scene, rx, None); // host thread pushes scenes
```

`VisualizerConfig` sets window title and size. Pass `Some(shutdown_flag)` to
`run_visualizer_live` so a host thread can exit the Bevy app.

## Interaction (summary)

- **Left drag:** orbit (does not change selection)
- **Click (no drag):** select vertex, hull, or arity-2 line
- **Click empty:** clear selection
- **Hover:** size / brighter wire, same hue
- **Selected:** higher saturation and glow, same hue

Hubs are hidden by default. Hulls are on for arity ≥ 3.
