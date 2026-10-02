# hyper

**hyper** is a general-purpose **hypergraph visualization** stack for Rust:
scene IR, force-directed 3D layout, and an optional native Bevy viewer.

It is domain-agnostic — vertices and hyperedges only.

[![Animated Mathlib source dependency atlas: 9,384 nodes, 37,356 imports, and 7,550 dependency groups](docs/media/mathlib.gif)](docs/media/mathlib.mp4)

**[Watch the full-resolution Mathlib demo](docs/media/mathlib.mp4)** — the complete source-file
import graph of mathlib v4.34.1, rendered at 1080p. Includes 9,112 source files
and 272 external-module placeholders. The video uses a settled layout;
[reproduction instructions and live benchmarks](docs/mathlib-demo.md) explain
the workload and rendering limits.

## What this is

- A reusable library (`hyper-viz`) any app can feed with a hypergraph
- Optional native 3D window (`hyper-viz-bevy`)
- A small CLI (`hyper`) for demos and JSON file viewing
- Interchange format: **`hypergraph.v1`** JSON

## What this is not

- **Not** Atomic Memory Core or any proprietary memory engine
- **Not** a claim / conversation / fact pipeline
- **Not** a packaged browser/WASM product (session JSON is portable; no web serve here)
- **Not** published to crates.io until an explicit public-release decision

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the hard boundary.

## Docs for external readers

| Doc | Contents |
|---|---|
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Package boundary, host coupling policy |
| [docs/API.md](docs/API.md) | Library API overview |
| [docs/VIEWER.md](docs/VIEWER.md) | How to run the viewer + example scene |
| [docs/PUBLIC_RELEASE_CHECKLIST.md](docs/PUBLIC_RELEASE_CHECKLIST.md) | Maintainer release-decision checklist |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Dev setup and PR norms |

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

Load JSON (`hypergraph.v1`):

```rust
use hyper_viz::{from_json_str, load_json};

let graph = load_json("fixtures/sample.json")?;
let graph = from_json_str(r#"{"version":"hypergraph.v1","vertices":[],"hyperedges":[]}"#)?;
```

A quarantined one-way importer can still load a legacy host export prefix; it is
**not** part of the public domain model (see architecture docs).

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

## Crate map

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

# Example scene
cargo run -- fixtures/sample.json

# Hot-reload while you edit the file
cargo run -- --watch fixtures/sample.json

# Other projections: bipartite (default), clique, star
cargo run -- --projection clique fixtures/sample.json

# Headless library example (no window)
cargo run -p hyper-viz --example project_scene
```

First Bevy compile is slow (~40s cold). Subsequent runs are fast. Native window
only — this is not a web app. More detail: [docs/VIEWER.md](docs/VIEWER.md).

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
| Scroll | Zoom (HUD windows steal scroll when the pointer is over them) |
| Lasso (UI panel) | Polygon select (disables orbit while on) |
| Space | Toggle force layout |
| A | Isolate `attention` status: hide other hulls, boost remaining fills, label those nodes |
| F | Frame camera (ignored while the find bar is focused) |
| ⌘F / Ctrl+F | Focus the find bar |
| ⌃⌘F | Isolate the selected incident neighborhood (toggle) |
| / | Focus the find bar (when not typing) |
| Enter (in find) | Isolate the match neighborhood and frame the camera |
| Esc | Clear the find query, or clear focus |
| Clear selection | Button in the Selection window |

The find bar at the bottom highlights matches as you type (BM25 + Jaro–Winkler
on label, id, kind, and status). Hits use the existing selection glow and the
**Selection** window. While focus is on, each new query also re-isolates the
match neighborhood and reframes the camera.

The **Selection** window lists selected hyperedges and vertices with kind,
label, status, and a location taken from the id (`repo:…`, `wt:…`, `pr:…`).
Click a row to narrow the selection to that node. Pointer over any HUD window
does not orbit or zoom the scene.

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

No secrets. The viewer restores HUD, navigation (lasso), camera pose,
selection, focus neighborhood, and find query from a `hyperviz.session.v1`
JSON blob:

| Host | Where |
|---|---|
| Desktop | `~/Library/Application Support/hyper-viz/session.json` (macOS), `%APPDATA%/hyper-viz/session.json` (Windows), `$XDG_CONFIG_HOME/hyper-viz/session.json` (Linux) |
| Browser (wasm) | `localStorage["hyperviz.session.v1"]` |

Override the file with `HYPER_VIZ_SESSION=/path/to/session.json`. Set
`HYPER_VIZ_SESSION=off` to disable. Prefs (including navigation) are global;
camera / selection / isolate are keyed by graph id. Layout positions are not
stored.

## License

Dual-licensed under **MIT OR Apache-2.0**. See [LICENSE](LICENSE),
[LICENSE-APACHE](LICENSE-APACHE), [LICENSE-MIT](LICENSE-MIT), and
[NOTICE](NOTICE).

## Directory layout

```text
crates/hyper-viz/          # library other projects depend on
crates/hyper-viz-bevy/     # optional Bevy renderer
docs/                      # architecture, API, viewer, release checklist
fixtures/sample.json       # coauthorship demo
src/main.rs                # CLI
```

## Non-goals

- PAOH timeline view
- WASM / browser serve (session JSON is ready for `localStorage`)
- Euler-style set diagrams (convex-hull member shells only)
- Bundling any proprietary memory engine
