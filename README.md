# hyper

General-purpose **hypergraph visualization** for Rust. Domain-agnostic: vertices
and hyperedges only — no memory-engine, conversation, or fact types.

**Other Rust projects should depend on `hyper-viz`** (headless) and, optionally,
`hyper-viz-bevy` (native 3D window). The root `hyper` package is the CLI.

Any app that already has a hypergraph can feed this crate. AtomicMemory
`am-hg-graph.v1` / `v2` exports still load via an input adapter.

## Use as a library

```toml
# Headless: schema, JSON, projections, 3D layout (no Bevy)
hyper-viz = { git = "https://github.com/atomicstrata/hyper.git" }

# Optional native 3D window
hyper-viz-bevy = { git = "https://github.com/atomicstrata/hyper.git" }
```

Path dep while this repo sits next to the host:

```toml
hyper-viz = { path = "../hyper/crates/hyper-viz" }
hyper-viz-bevy = { path = "../hyper/crates/hyper-viz-bevy" }
```

`hyper-viz` has **no Bevy dependency**. Embed it in a WASM, SVG, or custom renderer.

### Headless

```rust
use hyper_viz::prelude::*;

let graph = Hypergraph::new()
    .with_title("Reactions")
    .vertex("h2o", "H2O", "molecule")
    .vertex("h2", "H2", "molecule")
    .vertex("o2", "O2", "molecule")
    .hyperedge("combust", ["h2", "o2", "h2o"], "2H2 + O2 → 2H2O");

let scene = graph.project(Projection::Bipartite);
let mut layout = ForceLayout3D::from_scene(&scene, LayoutConfig::default());
layout.step();
```

Load JSON (`hypergraph.v1` or `am-hg-graph.v*`):

```rust
use hyper_viz::{from_json_str, load_json};

let graph = load_json("graph.json")?;
let graph = from_json_str(r#"{"version":"hypergraph.v1","vertices":[],"hyperedges":[]}"#)?;
```

### Native viewer

```rust
use hyper_viz_bevy::{run_from_graph, run_visualizer, run_visualizer_live};

run_from_graph(graph);              // bipartite + blocking window
run_visualizer(scene);              // already projected
run_visualizer_live(scene, rx, None); // host thread pushes scenes
```

A new scene on the live channel (or a file-watch reload) rebuilds the layout
and **clears selection**.

### Embed in a Bevy app

```rust
use bevy::prelude::*;
use hyper_viz_bevy::HyperVisualizerPlugin;

App::new()
    .add_plugins(DefaultPlugins)
    .add_plugins(HyperVisualizerPlugin::from_scene(scene))
    .run();
```

`VisualizerConfig` sets window title and size for the standalone helpers.
Pass `Some(shutdown_flag)` to `run_visualizer_live` so a host thread can exit
the Bevy app.

## Architecture

```text
Hypergraph JSON  (or builder API)
  → project(Bipartite | Clique | Star)
  → HypergraphScene     # render-agnostic IR
  → ForceLayout3D       # Barnes-Hut octree, CPU
  → hyper-viz-bevy      # native 3D window (optional)
```

| Crate | Role | Depend on this? |
|---|---|---|
| [`crates/hyper-viz`](crates/hyper-viz) | Scene IR, projections, layout, hulls, JSON I/O | **Yes** — library |
| [`crates/hyper-viz-bevy`](crates/hyper-viz-bevy) | Bevy 0.18 + egui native viewer | Only if you want the window |
| `hyper` (this binary) | CLI: load JSON or open the built-in demo | No — not a library |

## Quick start (CLI)

```bash
# Built-in coauthorship demo
cargo run

# Any hypergraph.v1 JSON
cargo run -- fixtures/sample.json

# Hot-reload while you edit the file
cargo run -- --watch fixtures/sample.json

# Other projections: bipartite (default), clique, star
cargo run -- --projection clique fixtures/sample.json

# Headless library example (no window)
cargo run -p hyper-viz --example project_scene
```

First Bevy compile is slow (~40s cold). Subsequent runs are fast. Native window
only — this is not a web app.

## JSON schema (`hypergraph.v1`)

```json
{
  "version": "hypergraph.v1",
  "meta": { "id": "g1", "title": "Coauthorship" },
  "vertices": [
    { "id": "alice", "label": "Alice", "kind": "person" }
  ],
  "hyperedges": [
    { "id": "paper-a", "label": "Paper A", "vertices": ["alice", "bob", "carol"] }
  ]
}
```

| Field | Required | Notes |
|---|---|---|
| `vertices[].id` | yes | Stable id referenced by hyperedges |
| `vertices[].label` | no | Falls back to `id` |
| `vertices[].kind` | no | Free-form; used only for color |
| `vertices[].status` | no | `active` (default), `shadowed`, `rejected`, `attention` |
| `hyperedges[].vertices` | yes | Member vertex ids |
| `hyperedges[].status` | no | `active` (default), `shadowed`, `rejected`, `attention` |
| `*.attrs` | no | Opaque JSON map for the host app |

## How the graph is drawn

Default projection is **bipartite** (one extra-node hub per hyperedge). The
viewer hides those hubs by default so the hyperedge is read as a **set**:

| Arity | Drawing |
|---|---|
| 2 | Colored line between the two members |
| 3 | Triangle hull |
| 4 | Tetrahedron hull |
| 5+ | Convex hull of members |

Each hyperedge keeps a **stable hue** from its id (hash), so nested sets stay
distinguishable. Nested hulls are inflated slightly to avoid z-fighting.
Vertex color comes from `kind` (fixed palette for well-known names, hash otherwise).

Uncheck **Hide extra-node hubs** in the Hyperedge hulls panel to show the
bipartite hubs as spheres.

## Controls

Left button is dual-purpose. **Drag** orbits; **click** (press + release with
almost no movement) selects. Orbiting does not change selection.

| Input | Action |
|---|---|
| Left drag | Orbit camera (selection is kept) |
| Left click | Select the vertex or hyperedge under the cursor |
| Click empty space | Clear selection |
| Shift / Cmd + click | Add to selection |
| Scroll | Zoom |
| Lasso (UI panel) | Polygon select (disables orbit while on) |
| Space | Toggle force layout |
| A | Toggle attention highlight (dims non-`attention` status) |
| F | Frame camera on focus / selection / attention / live-work |
| Shift+F | Isolate the selected incident neighborhood (toggle) |
| Clear selection | Button in the Selection panel |

Picking hits vertices, hull triangles, and arity-2 segments. Hidden hubs are
not pickable.

## Hover vs selected

Both states keep the object's own hue. They do **not** swap to a highlight
color.

| State | Look |
|---|---|
| Hover | Nodes grow slightly; hull/line wires brighten |
| Selected | Same hue, higher saturation and glow |
| Hover + selected | Selected treatment wins (still a bit larger) |

## Projections

- **Bipartite** (default): one hub node per hyperedge + incidence links (Ouvrard extra-node).
- **CliqueExpansion** (`--projection clique`): all pairs within each hyperedge (no hubs).
- **StarCentroid** (`--projection star`): centroid attraction, no hub nodes.

## Tests

```bash
cargo test -p hyper-viz
cargo test -p hyper-viz-bevy --lib
cargo clippy -p hyper-viz -p hyper-viz-bevy --all-targets -- -D warnings
```

## Observability

JSON-friendly `tracing` logs on scene init, file-watch reload, and live-channel
reload (`nodes`, `hyperedges`, `graph_id`). Set `RUST_LOG=hyper_viz_bevy=info`
(see [`.env.example`](.env.example)).

No environment variables are required. No secrets.

## Directory layout

```text
crates/hyper-viz/          # library other projects depend on
crates/hyper-viz-bevy/    # optional Bevy renderer
fixtures/sample.json      # coauthorship demo
src/main.rs               # CLI
```

## Non-goals

- PAOH timeline view
- WASM / browser serve
- Euler-style set diagrams (convex-hull member shells only)
