# hyper

**A native 3D hypergraph viewer for Python and Rust.**

A hypergraph connects several items through one relationship: coauthors on a
paper, molecules in a reaction, or files in a dependency group. Hyper lets you
explore those groups with search, picking, lasso selection, and neighborhood
isolation.

[![Mathlib dependency graph in Hyper](docs/media/mathlib.gif)](docs/media/mathlib.mp4)

[Mathlib demo in full resolution](docs/media/mathlib.mp4) ·
[How the demo was made](docs/mathlib-demo.md).
The recording uses a settled layout.

## Quick start

### Install with Python

Requires **CPython 3.10+**, a desktop session, and working graphics drivers.

```bash
python -m pip install --upgrade "hypergraph-viz[viewer]"
hyper
```

This installs the Python API and native viewer, then opens the built-in
coauthorship demo. Prebuilt wheels support Linux x86_64, macOS arm64/x86_64,
and Windows x86_64. See [platform requirements](docs/DESKTOP_PACKAGE.md)
for details.

To open your own HIF dataset from Python:

```python
import hyper_viz

document = hyper_viz.HifDocument.load("dataset.hif.json")
viewer = hyper_viz.show(document)
viewer.wait()  # Wait until the window closes.
```

The distribution name is [hypergraph-viz](https://pypi.org/project/hypergraph-viz/);
the import name is `hyper_viz`.
For document loading, validation, and export alone, install
`hypergraph-viz` without the viewer extra.
See the [Python and HIF guide](docs/HIF.md) for the full API and
[XGI example](crates/hyper-viz-python/examples/xgi_interop.py).

### Run from source

Requires **Rust 1.89+** and a desktop graphics environment.

```bash
git clone https://github.com/atomicstrata/hyper.git
cd hyper
cargo run --release
```

The first build can take several minutes. To open the sample dataset:

```bash
cargo run --release -- fixtures/sample.json
```

See the [viewer guide](docs/VIEWER.md) for installation and more commands.

## Open your data

The viewer reads [HIF](docs/HIF.md) (Hypergraph Interchange Format) and native
`hypergraph.v1` JSON. It detects the format automatically.

```bash
hyper dataset.hif.json
hyper --watch graph.json
hyper --projection star graph.json
```

Use `--watch` to reload a file as it changes. Choose `bipartite` (default),
`clique`, or `star` for the projection.

For a small native example, save this as `graph.json` and run
`hyper graph.json`:

```json
{
  "version": "hypergraph.v1",
  "vertices": [
    { "id": "alice", "label": "Alice" },
    { "id": "bob", "label": "Bob" },
    { "id": "carol", "label": "Carol" }
  ],
  "hyperedges": [
    { "id": "paper", "label": "Paper", "vertices": ["alice", "bob", "carol"] }
  ]
}
```

The viewer supports undirected memberships. HIF documents can preserve richer
data, but directed or simplicial semantics and incidence properties are rejected
when converting for viewing. Keep the original `HifDocument` for scientific
export; see [supported semantics](docs/HIF.md#supported-semantics).

Groups appear as colored lines or hulls. Large hulls approximate group shape;
use the membership list to check which nodes belong to a group. Dense datasets
can be expensive to render; see the [measured benchmarks](docs/benchmarks/ecosystem-2026-10-03/README.md).

## Controls

| Input | Action |
|---|---|
| Left drag | Orbit |
| Scroll | Zoom |
| Left click | Select a node or group |
| Shift + click | Add to selection |
| Space | Pause or resume the force layout |
| F | Frame the camera |
| Cmd+F / Ctrl+F | Search |
| Enter in search | Isolate matching neighborhoods and frame them |
| Esc | Clear search or neighborhood focus |

Use the lasso panel to select several nodes.
See the [viewer guide](docs/VIEWER.md#controls) for all controls and display settings.

## Use from Rust

Add the core library to your application's `Cargo.toml`:

```toml
[dependencies]
hyper-viz = { git = "https://github.com/atomicstrata/hyper.git" }

# Add this for a native viewer window:
# hyper-viz-bevy = { git = "https://github.com/atomicstrata/hyper.git" }
```

`hyper-viz` provides JSON I/O, projections, and 3D force layout without Bevy.
`hyper-viz-bevy` adds a standalone viewer and a plugin for existing Bevy apps.

Start with the [Rust API guide](docs/API.md) or the
[headless example](crates/hyper-viz/examples/project_scene.rs):

```bash
cargo run -p hyper-viz --example project_scene
```

## Documentation

| Guide | What you'll find |
|---|---|
| [Viewer](docs/VIEWER.md) | Setup, controls, display settings, and saved sessions |
| [Python and HIF](docs/HIF.md) | Interchange, Python API, supported semantics, and source builds |
| [Rust API](docs/API.md) | Build graphs, run layouts, and embed the viewer |
| [Architecture](docs/ARCHITECTURE.md) | Crate responsibilities and integration boundaries |
| [Benchmarks](docs/benchmarks/ecosystem-2026-10-03/README.md) | Measured results, methods, and limitations |
| [Notebook roadmap](docs/NOTEBOOK_ROADMAP.md) | Plans for a browser frontend; the current viewer opens a native window |
| [Contributing](CONTRIBUTING.md) | Development setup, checks, and pull request guidelines |

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE).
See [NOTICE](NOTICE) for third-party notices.
