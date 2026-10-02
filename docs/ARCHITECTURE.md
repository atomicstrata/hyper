# Architecture boundary

This package is a **standalone hypergraph visualization library**. It is not
Atomic Memory Core, and it does not implement private claim pipelines.

## What this repo is

```text
Hypergraph JSON (hypergraph.v1)  or  builder API
        │
        ▼
   project(Bipartite | Clique | Star)
        │
        ▼
   HypergraphScene          # render-agnostic IR
        │
        ▼
   ForceLayout3D            # Barnes-Hut octree, CPU
        │
        ▼
   hyper-viz-bevy (optional)  # native 3D window
```

| Crate | Public role |
|---|---|
| `hyper-viz` | Schema, JSON I/O, projections, scene IR, layout, hulls, semantics |
| `hyper-viz-bevy` | Optional Bevy 0.18 + egui native viewer |
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
2. `hypergraph.v1` JSON on disk / over the wire.

A **quarantined one-way importer** accepts a legacy host export version prefix
(`am-hg-graph.*`) and maps only viz fields into `hypergraph.v1`. It lives in
[`crates/hyper-viz/src/io/legacy_export.rs`](../crates/hyper-viz/src/io/legacy_export.rs).
Extra host fields are ignored. That adapter is **not** an engine API and must
not grow claim or memory semantics.

## Secrets and internals

- No service credentials or private API endpoints belong in this tree.
- Optional env vars are local viewer prefs only (`RUST_LOG`, `HYPER_VIZ_SESSION`).
- See [`.env.example`](../.env.example).
