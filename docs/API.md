# API overview

Depend on **`hyper-viz`** for headless work. Add **`hyper-viz-bevy`** only when
you want a native 3D window.

```toml
hyper-viz = { git = "https://github.com/atomicstrata/hyper.git" }
# optional:
hyper-viz-bevy = { git = "https://github.com/atomicstrata/hyper.git" }
```

Crates are `publish = false` until a public-release decision; use git or path
deps.

## `hyper-viz` surface

| Area | Entry points | Notes |
|---|---|---|
| Schema | `Hypergraph`, `Vertex`, `Hyperedge`, `GRAPH_VERSION` | Public interchange: `hypergraph.v1` |
| Builder | `Hypergraph::new().vertex(..).hyperedge(..)` | Fluent in-process construction |
| I/O | `load_json`, `from_json_str`, `save_json` | Writes always emit `hypergraph.v1` |
| Project | `Projection`, `project`, `Hypergraph::project` | → `HypergraphScene` |
| Layout | `ForceLayout3D`, `LayoutConfig`, `LayoutModel`, `TopologySettings` | Headless; no GPU |
| Hulls | `hull_from_points`, `HullMesh` | For custom renderers |
| Semantics | `kind_color`, `emphasize`, status helpers | Hue-preserving hover/select |
| Query | `scene_hits` | BM25 + Jaro–Winkler over scene labels |
| Session | `ViewerSession`, `parse_session` | Portable prefs blob (`hyperviz.session.v1`) |
| Prelude | `hyper_viz::prelude::*` | Typical host imports |

```rust
use hyper_viz::prelude::*;

let graph = Hypergraph::new()
    .with_title("Reactions")
    .vertex("h2", "H2", "molecule")
    .vertex("o2", "O2", "molecule")
    .vertex("h2o", "H2O", "molecule")
    .hyperedge("combust", ["h2", "o2", "h2o"], "2H2 + O2 → 2H2O");

let scene = graph.project(Projection::Bipartite);
let mut layout = ForceLayout3D::from_scene(&scene, LayoutConfig::default());
layout.step();
```

Structural layout is opt-in for headless callers:

```rust
use hyper_viz::{LayoutModel, TopologySettings};
let config = LayoutConfig {
    topology: TopologySettings { model: LayoutModel::Normalized, ..Default::default() },
    ..Default::default()
};
let mut layout = ForceLayout3D::from_scene(&scene, config);
layout.seed_from_topology(&scene, 256); // use the scene that constructed this layout
layout.step();
```

`LayoutConfig::default()` keeps Legacy forces. Full `LayoutConfig` struct literals
must add `topology` or use `..Default::default()`. Construct `ForceLayout3D` through
its constructors; its weighted connectivity cache is private. New session fields
are optional: older `hyperviz.session.v1` preferences default to Legacy, and
`hypergraph.v1` is unchanged. Rebuild via a constructor when connectivity changes;
directly editing public legacy `edges` does not update the structural cache.

Load JSON:

```rust
use hyper_viz::{from_json_str, load_json};

let graph = load_json("fixtures/sample.json")?;
let graph = from_json_str(r#"{"version":"hypergraph.v1","vertices":[],"hyperedges":[]}"#)?;
```

## `hyper-viz-bevy` surface

| Entry | Role |
|---|---|
| `run_from_graph(graph)` | Bipartite project + blocking window |
| `run_visualizer(scene)` | Already-projected scene |
| `run_visualizer_live(scene, rx, shutdown)` | Host thread pushes scenes |
| `HyperVisualizerPlugin` | Embed inside an existing Bevy `App` |
| `VisualizerConfig` | Window title / size for standalone helpers |

```rust
use hyper_viz_bevy::{run_from_graph, run_visualizer, HyperVisualizerPlugin};

run_from_graph(graph);
run_visualizer(scene);

// Or embed:
// app.add_plugins(HyperVisualizerPlugin::from_scene(scene));
```

## Non-goals of the API

- No memory-engine traits, claim types, or conversation stores.
- No required network calls.
- Legacy host export import is internal to `io` and not a public module.
