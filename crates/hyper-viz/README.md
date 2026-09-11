# hyper-viz

Domain-agnostic hypergraph visualization core: schema, scene IR, projections,
Barnes-Hut layout, convex hulls, and visual semantics. **No Bevy.**

Workspace overview, JSON schema, and embedding examples live in the
[root README](../../README.md).

## Modules

| Module | What it does |
|---|---|
| `schema` | `Hypergraph`, `Vertex`, `Hyperedge` (`hypergraph.v1`) |
| `io` | JSON load/save; `am-hg-graph.v*` adapter |
| `project` | `Bipartite`, `CliqueExpansion`, `StarCentroid` → `HypergraphScene` |
| `layout` | Headless 3D force layout (`ForceLayout3D`) |
| `hull` | Convex hull from member positions (arity ≥ 3) |
| `semantics` | Kind/id colors, status, hover/selected **emphasis** |

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
