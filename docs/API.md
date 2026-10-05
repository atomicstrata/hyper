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
| I/O | `load_json`, `from_json_str`, `InputFormat`, `load_json_with_format`, `from_json_str_with_format`, `save_json` | Auto-detects native, HIF, and legacy input; writes native JSON |
| HIF | `HifDocument`, `HifId`, `parse_hif`, `load_hif`, `serialize_hif`, `save_hif` | Offline validation and faithful scientific interchange; [semantics and Python API](HIF.md) |
| Project | `Projection`, `project`, `Hypergraph::project` | → `HypergraphScene` |
| Layout | `ForceLayout3D`, `LayoutConfig`, `LayoutModel`, `TopologySettings` | Headless; no GPU |
| Hulls | `hull_from_points`, `HullMesh` | For custom renderers |
| Semantics | `kind_color`, `emphasize`, status helpers | Hue-preserving hover/select |
| Query | `scene_hits` | BM25 + Jaro–Winkler over scene labels |
| Dependencies | `DependencyIndex`, `DependencyScope`, `ScopeOptions`, `TraversalDepth` | Validated directed imports, independent scopes, shortest paths |
| Module categories | `module_categories`, `ModuleFilters` | Optional Mathlib path/ID filters, separate from graph semantics |
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

`seed_from_topology` changes positions, clears velocities, and leaves the iteration
counter unchanged. Use the same scene and node order as the constructor; construct
a new layout when replacing a scene. The initializer clamps its diffusion passes
to 8–512 and operates on connectivity and stable IDs, without label-based clusters.
`pair_opacity` and `set_opacity` return display multipliers; they do not remove
memberships or change forces. See [algorithm and control details](mathlib-demo.md#full-spatial-overview).

In star projection, structural forces treat arity-two edges as pairs and larger
edges as weighted centroid sets. `kind: "dependency_group"` sets additionally use
`derived_set_influence`. Bipartite and clique projections use their projected
links as pairs; set strength, arity normalization, and derived-set influence do
not affect those links. The global `repulsion`, `gravity`, `dt`, and `damping`
settings apply to every model. Legacy uses `attraction`, `ideal_length`, and
`centroid_attraction`; Normalized and LinLog use `TopologySettings` instead.

### Directed dependency queries

Direction requires an arity-two native hyperedge with `kind: "import"` and string
`attrs.source` / `attrs.target` equal to its vertex IDs. This is a native attribute
convention, separate from directed HIF, which the viewer rejects. The first
member or the label alone never establishes direction. Invalid imports remain
ordinary hyperedges and appear in `DependencyIndex::warnings()`.

```rust
use hyper_viz::{DependencyIndex, Hyperedge, Hypergraph, Projection, ScopeOptions};

let mut graph = Hypergraph::new()
    .vertex("app", "Application", "module")
    .vertex("base", "Base", "module");
let mut edge = Hyperedge::new("app-imports-base", ["app", "base"]).with_kind("import");
edge.attrs.insert("source".into(), "app".into());
edge.attrs.insert("target".into(), "base".into());
graph.add_hyperedge(edge);
let scene = graph.project(Projection::StarCentroid);
let index = DependencyIndex::new(&scene);
let allowed = vec![true; scene.node_count()];
let app = index.node_index("app").unwrap();
let base = index.node_index("base").unwrap();
let scope = index.scope(app, &ScopeOptions::default(), &allowed);
assert_eq!(scope.nodes.len(), 2);
assert_eq!(index.shortest_path(app, base, &allowed), Some(vec![app, base]));
assert_eq!(index.shortest_path(base, app, &allowed), None);
```

`imports` follows source → target; `dependents` follows the reverse adjacency.
Neighbor order and shortest-path ties use stable IDs. Duplicate relations share
one adjacency entry but retain all original scene hyperedge indices in
`DirectedImport::hyperedge_indices`. Cycles and self-loops terminate.

Call `ScopeOptions::normalize()` to clamp hop depths to 1–6 and the node budget
to 1–1,000. `Off` disables a direction; `Transitive` traverses every allowed
reachable vertex. The budget limits displayed nodes, while depth maps and
`total_reachable` retain counts before truncation. Nodes are chosen by distance,
then ID, with the center always retained. `allowed` is indexed by scene node;
missing entries block traversal and filtered nodes stop traversal. Scopes retain
their center even if filtered; shortest paths require both endpoints allowed.
`expand` adds one enabled-direction hop from a reached node; `include_path`
prioritizes the ordered prefix of a valid source → target path within the budget.

The index contains scene indices: rebuild it after scene replacement or reordering
and resolve saved stable IDs again. `module_categories` recognizes umbrella source
paths, test/tactic namespaces, and external/stub attributes. Generic callers can
supply their own mask instead of using those Mathlib conventions. See the runnable
[dependency example](../crates/hyper-viz/examples/inspect_dependencies.rs).

### Scene migration

`SceneMeta`, `SceneNode`, and `SceneHyperedge` retain opaque source `attrs`. Add
`attrs: Default::default()` to manual struct literals. `SceneHyperedge::hub_index`
is `Some(index)` only for a bipartite hub and `None` for star/clique projections;
older serialized scenes still load. Use optional hub handling instead of treating
the first vertex as a hub. `scenes_equivalent` detects metadata/attribute changes
while ignoring index reorderings. Native input remains `hypergraph.v1`.

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
| `run_visualizer_with(scene, config)` | Standalone window with view mode and initial module |
| `run_visualizer_from_path_with_format(path, watch, projection, config, format)` | Explicit startup and reload input format |
| `run_visualizer_live(scene, rx, shutdown)` | Host thread pushes scenes |
| `run_visualizer_live_with_search`, `run_visualizer_live_with_search_modes` | Host search callbacks run outside the render thread |
| `HyperVisualizerPlugin` | Embed inside an existing Bevy `App` |
| `VisualizerConfig`, `ViewMode` | Window title / size, view mode, initial module |
| `visualizer_app`, `visualizer_app_with_config` | Build an app without running its event loop |

```rust
use hyper_viz_bevy::{run_from_graph, run_visualizer, HyperVisualizerPlugin};

run_from_graph(graph);
run_visualizer(scene);

// Or embed:
// app.add_plugins(HyperVisualizerPlugin::from_scene(scene));
```

`HyperVisualizerPlugin` requires the host to install Bevy's `DefaultPlugins`.
Use `.with_projection(...)` and `.with_input_format(...)` with `from_path` to
configure loading and watched reloads. `.with_view_mode(ViewMode::Dependencies)`
selects the optional canvas; Auto uses Spatial. Standalone hosts can use
`VisualizerConfig::default().with_view_mode(...).with_module("exact.vertex.id")`.
Add `..Default::default()` to existing config literals for the new fields.

Live updates drain the channel to the most recent scene, preserve positions by
stable ID, remap surviving selection/focus, and rebuild geometry and query caches.
Unchanged scenes do not trigger rebuilds. Invalid watched files retain the last
valid scene. `.with_shutdown(Arc<AtomicBool>)` and the standalone live helpers
support a host-requested clean exit. See [viewer state and sessions](VIEWER.md#session-persistence)
and the [architecture](ARCHITECTURE.md) for ownership and refresh rules.

## Non-goals of the API

- No memory-engine traits, claim types, or conversation stores.
- No required network calls.
- Legacy host export import is internal to `io` and not a public module.
