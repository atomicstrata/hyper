# Architecture boundary

This package is a **standalone hypergraph visualization library**. It is not
Atomic Memory Core, and it does not implement private claim pipelines.

## What this repo is

```text
HIF JSON → HifDocument → viewer compatibility check
Native JSON (hypergraph.v1)  or  Rust builder API
        │
        ▼
   project(Bipartite | Clique | Star)
        │
        ▼
   HypergraphScene          # render-agnostic IR
        │
        ▼
   ForceLayout3D            # Barnes-Hut repulsion; Legacy / Normalized / LinLog attraction
        │
        ▼
   hyper-viz-bevy (optional)  # native 3D window
```

| Crate | Public role |
|---|---|
| `hyper-viz` | HIF, native JSON, projections, scene IR, layout, hulls, dependency queries, semantics |
| `hyper-viz-bevy` | Optional Bevy 0.18 + egui native viewer |
| `hyper-viz-python` | PyO3 HIF bindings, generated typing, Python viewer process launcher; no Bevy |
| `hyper` (binary) | CLI demo / file viewer — not a library dependency |

## What this repo is not

- **Not** a memory engine, conversation store, or fact/claim database.
- **Not** a retrieval stack (hosts may plug their own search into the viewer).
- **Not** a WASM browser product (session JSON is ready for `localStorage`, but
  there is no packaged web serve in this repo).
- **Not** published to crates.io until an explicit public-release decision
  (`publish = false` on workspace members).

## Host coupling policy

Hosts should depend on `hyper-viz` (+ optionally `hyper-viz-bevy`) and pass:

1. A `Hypergraph` built in-process, or
2. `hypergraph.v1` JSON on disk / over the wire, or
3. HIF JSON converted through an offline-validated `HifDocument`.

The document owns the scientific representation; viewer conversion creates a
separate native snapshot and rejects unsupported semantics instead of silently
dropping them. Python's launcher validates before starting a subprocess without
a shell. `ViewerHandle` owns its temporary snapshot until the process exits.
The separately installed `hypergraph-viz-viewer` wheel supplies the executable;
the core extension does not link Bevy or download a viewer. See [HIF](HIF.md) and
the [desktop package](DESKTOP_PACKAGE.md) for compatibility and platform details.

A **quarantined one-way importer** accepts a legacy host export version prefix
(`am-hg-graph.*`) and maps only viz fields into `hypergraph.v1`. It lives in
[`crates/hyper-viz/src/io/legacy_export.rs`](../crates/hyper-viz/src/io/legacy_export.rs).
Extra host fields are ignored. That adapter is **not** an engine API and must
not grow claim or memory semantics.

## Scene and layout ownership

`project` preserves stable graph, vertex, and hyperedge IDs and opaque metadata
in a render-independent scene. Bipartite projection creates optional hub nodes;
star and clique projections never fabricate hubs. Rendering caps hull geometry
at 24 sampled members without changing model membership. Retained attributes
are not automatically displayed by the stock inspector.

`ForceLayout3D` owns positions, velocities, and its connectivity cache. Legacy
remains the core default. Structural models weight pair and centroid-set forces;
their sparse initializer derives coordinates from connectivity and stable IDs.
Mathlib categories affect optional filtering and colors, rather than coordinates.
The large star-import interactive preset seeds, refines for 64 steps, and starts
paused. Scripted tours retain their previous Legacy behavior. Algorithm details,
control ranges, and limits are in the [Spatial guide](mathlib-demo.md#full-spatial-overview).

## Viewer replacement and optional exploration

The renderer owns Bevy resources for layout, hulls, line caches, selection,
search, focus, and session state. Live channel updates and watched reloads compare
scenes before replacing them. Changed scenes advance an epoch, retain surviving
positions and UI state by stable ID, and invalidate geometry and query caches.
Explicit structural rebuilds also advance a position revision so paused hulls
resample their geometry immediately. A failed watched load leaves the last valid
scene visible. Hosts can request shutdown through an atomic flag.

Auto view remains Spatial. The optional dependency canvas uses `DependencyIndex`
to validate native arity-two `kind: "import"` edges with explicit source/target
attributes. Direction is independent of unordered hyperedge membership and HIF
directional semantics. Duplicate source edges retain identity; malformed direction
produces warnings and remains available for generic inspection. Separate imports
and dependents traversals use deterministic budgets and caller-supplied filters.
The native canvas adds exact-ID search, paths, expansion, and bounded history;
entering it pauses spatial work and returning restores the prior run state.

`hyperviz.session.v1` persists optional layout/model preferences and per-graph
stable-ID explorer snapshots alongside camera, selection, and focus. Older fields
default on load. Positions are recomputed rather than stored. See the
[API migration notes](API.md#scene-migration) and [session guide](VIEWER.md#session-persistence).

## Documentation and verification

The [API guide](API.md) maps supported surfaces; [CONTRIBUTING](../CONTRIBUTING.md)
lists development checks. The [Python release guide](PYTHON_RELEASE.md) covers
wheel/source packaging and tag-driven publication. Benchmarks and screenshots
are workload evidence with explicit limitations, not capacity guarantees.
The [release checklist](PUBLIC_RELEASE_CHECKLIST.md) records preparation separately
from the maintainer's visibility and publication decisions.

## Secrets and internals

- No service credentials or private API endpoints belong in this tree.
- Optional env vars are local viewer prefs only (`RUST_LOG`, `HYPER_VIZ_SESSION`).
- See [`.env.example`](../.env.example).
