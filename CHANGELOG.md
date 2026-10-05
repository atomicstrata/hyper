# Release notes

## 0.2.0 — unreleased

Rust workspace crates, the `hyper` CLI, `hypergraph-viz`, and the
`hypergraph-viz-viewer` companion share version `0.2.0`. The Python `[viewer]`
extra requires the matching companion. This minor version marks new layout and
exploration features and Rust API changes while the project remains pre-1.0.

### Features

- Connectivity-based spatial initialization, Normalized and LinLog force models,
  separate pair/set weights, degree/arity normalization, derived-set influence,
  movement limits, display fading, structural rebuild, and full-graph framing.
- An optional native dependency canvas (`--view dependencies`) with validated
  import direction, independent import/dependent traversal, filters, shortest
  paths, exact module search, expansion, and bounded navigation history.
- Rust dependency-query APIs and source attributes retained in projected scenes.
- Clean camera tours with structural layout and wall-clock navigation capture;
  timestamped video export preserves navigation speed when captures are dropped,
  with adjustable hull opacity and bounded group outlines for dense tours.
- Expanded API, architecture, viewer, migration, and contributor documentation.

### Fixes

- Scene entities refresh when input reloads in dependency mode, preventing stale
  indices on return to Spatial. Newly created nodes stay hidden in Dependencies.
- Explicit HIF input formats continue through CLI startup and watched reloads
  when a view mode is selected.

### Compatibility and migration

- `LayoutConfig` struct literals must provide `topology` or use
  `..Default::default()`. Construct `ForceLayout3D` through its constructors;
  its structural connectivity cache is private.
- Manual `SceneMeta`, `SceneNode`, and `SceneHyperedge` literals require `attrs`.
  `SceneHyperedge::hub_index` is optional: only bipartite projections have hubs.
  See the [Rust API migration guide](docs/API.md#scene-migration).
- Manual `ShowcaseConfig` literals require `clean`, `realtime`, `structural`,
  `hull_opacity`, and `hull_outlines`. Use `false` for these booleans and `None`
  for opacity to retain the original annotated offline tour.
- Headless layout defaults retain Legacy forces, and Auto view remains Spatial.
  Native `hypergraph.v1` and session `hyperviz.session.v1` formats remain supported.
- Python imports, HIF interchange, viewer launching, and CPython 3.10+ support
  retain their existing interfaces. These layout controls are available through
  Rust and the native viewer; this release adds no Python layout API or notebook
  embedding.

The published `0.1.2` wheels predate these features. Build this checkout until
`0.2.0` is published. Package publication requires a separately created
`python-v0.2.0` tag on a validated commit; see the [release guide](docs/PYTHON_RELEASE.md).
Rust crates retain `publish = false` pending a separate crates.io release decision.
