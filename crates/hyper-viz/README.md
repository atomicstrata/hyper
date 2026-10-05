# hyper-viz

Domain-agnostic hypergraph visualization **library**: schema, scene IR, projections,
Barnes-Hut layout, convex hulls, and visual semantics. **No Bevy.**

This is the crate other Rust projects should depend on. It does **not** include
Atomic Memory Core or claim pipelines — only vertices, hyperedges, and viz IR.

```toml
hyper-viz = { git = "https://github.com/atomicstrata/hyper.git" }
# or
hyper-viz = { path = "../hyper/crates/hyper-viz" }
```

Workspace overview, architecture boundary, and embedding examples:

- [root README](../../README.md)
- [docs/ARCHITECTURE.md](../../docs/ARCHITECTURE.md)
- [docs/API.md](../../docs/API.md)

## Quick start

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
for _ in 0..32 {
    layout.step();
}
```

```bash
cargo run -p hyper-viz --example project_scene
```

## Modules

| Module | What it does |
|---|---|
| `schema` | `Hypergraph`, `Vertex`, `Hyperedge` (`hypergraph.v1`) |
| `io` | Native/HIF auto-detection or explicit format; native save; quarantined legacy import |
| `hif` | Offline-validated scientific document interchange and viewer conversion |
| `project` | `Bipartite`, `CliqueExpansion`, `StarCentroid` → `HypergraphScene` |
| `layout` | Headless 3D force layout (`ForceLayout3D`) |
| `topology` | Normalized/LinLog attraction, sparse connectivity-based initialization |
| `dependency` | Validated directed imports, scopes, expansion, shortest paths |
| `module` | Optional Mathlib category detection and traversal filters |
| `scene` | Render-independent nodes, optional hubs, attributes, scene comparison |
| `query` / `session` / `motion` | Search, portable preferences, status animation |
| `hull` | Convex hull from member positions (arity ≥ 3) |
| `semantics` | Kind/id colors, status, hover/selected **emphasis** |

`prelude` re-exports the types a host typically needs.

## Emphasis

Hover and selection keep the object's hue. `emphasize` / `hull_style_emphasized`
raise saturation for **selected** and only slightly brighten **hover**. Selected
wins if both flags are set (`Emphasis::from_flags`).

```rust
use hyper_viz::{Emphasis, emphasize, hull_style_emphasized, hyperedge_color};

let base = hyperedge_color("paper-a");
let selected = emphasize(base, Emphasis::Selected);
let hull = hull_style_emphasized("paper-a", "active", 0.28, Emphasis::Selected);
```

## Tests

```bash
cargo test -p hyper-viz
```

### Scene API migration

Projected scenes now retain graph, vertex, and hyperedge `attrs`. Include
`attrs: Default::default()` in manual scene struct literals. `SceneHyperedge`
uses `hub_index: Option<usize>`: bipartite hubs are `Some(index)`, while star
and clique projections use `None`. Do not treat a member vertex as a hub.
New attributes and optional hubs default when reading older scene JSON;
`hypergraph.v1` input and `hyperviz.session.v1` sessions keep their versions.

`DependencyIndex` validates explicit arity-two `kind: "import"` source/target
attributes, preserves duplicate source edge IDs, and supports directed scopes and
shortest paths. Set membership never implies direction. Malformed imports remain
available for generic inspection and generate warnings. `ScopeOptions` bounds
visible nodes; `ModuleFilters` can stop traversal through Mathlib categories.
See `examples/inspect_dependencies.rs` for a headless query.
