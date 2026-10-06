# Hyper

**A native 3D hypergraph viewer for Python workflows and Rust applications.**

<a href="docs/media/mathlib-realtime.mp4">
  <img src="docs/media/mathlib-realtime.gif" alt="Realtime navigation through the Mathlib dependency case study" width="100%">
</a>

A hypergraph represents a group relationship as one edge connecting any number
of nodes: authors on a paper, participants in an interaction, or modules in a
dependency group. Hyper lets you inspect those memberships, find overlapping
groups, and explore neighborhoods with search, picking, and lasso selection.

[Try the viewer](#try-the-viewer) · [Use Python](#use-from-python) ·
[Use Rust](#use-from-rust) · [Documentation](#documentation)

## Try the viewer

<details>
<summary>Ask a coding agent to run it</summary>

Paste this prompt into an agent working on your machine:

```text
Read https://github.com/atomicstrata/hyper/blob/main/AGENTS.md and open Hyper's
built-in demo. Install uv first if it is not on PATH. Do not clone the
repository or install Rust.
```

[AGENTS.md](AGENTS.md) carries the commands, the platform matrix, and the
desktop-session constraint, so the agent does not have to infer them.

</details>

Install [uv](https://docs.astral.sh/uv/getting-started/installation/), then open
the built-in coauthorship demo:

```bash
uvx --python 3.12 --from hypergraph-viz-viewer hyper
```

No project, repository checkout, or Rust compiler is needed. `--from` selects
the package that supplies the `hyper` executable. To open your own dataset:

```bash
uvx --python 3.12 --from hypergraph-viz-viewer hyper dataset.hif.json
```

Requires a **desktop session and working graphics drivers**. Prebuilt viewer
wheels support Linux x86_64 (glibc 2.28+), macOS arm64/x86_64 (11.0+), and
Windows x86_64. The viewer opens on the machine running the command.
See [platform requirements](docs/DESKTOP_PACKAGE.md).

Once the window opens, drag to orbit, scroll to zoom, and click a node or group
to inspect it. Press **Space** to pause the layout, **F** to frame the camera,
or **Cmd+F / Ctrl+F** to search. Press **Enter** in search to isolate matching
neighborhoods. The [viewer guide](docs/VIEWER.md#controls) covers all controls.

## Use from Python

Create a project and install the API with its matching desktop viewer:

```bash
uv init --python 3.12 hyper-example
cd hyper-example
uv add "hypergraph-viz[viewer]"
```

In an existing uv project using CPython 3.10+, run only `uv add`.
uv manages the environment and lockfile; `uv run` needs no activation.

Save this as `explore.py`. It creates two papers with a shared author, saves a
HIF dataset, and opens the viewer:

```python
import json
import hyper_viz

document = hyper_viz.HifDocument.from_json(json.dumps({
    "network-type": "undirected",
    "nodes": [{"node": name} for name in ["alice", "bob", "carol", "dana"]],
    "edges": [{"edge": "paper-a"}, {"edge": "paper-b"}],
    "incidences": [
        {"node": "alice", "edge": "paper-a"},
        {"node": "bob", "edge": "paper-a"},
        {"node": "carol", "edge": "paper-a"},
        {"node": "bob", "edge": "paper-b"},
        {"node": "dana", "edge": "paper-b"},
    ],
}))
document.save("papers.hif.json")

viewer = hyper_viz.show(document)
viewer.wait()  # Keep the script running until the window closes.
```

```bash
uv run explore.py
```

For existing data, replace the constructor with
`document = hyper_viz.HifDocument.load("dataset.hif.json")`.
You can also open the saved example with `uv run hyper papers.hif.json`.

| Name | Purpose |
|---|---|
| `hypergraph-viz` | Python distribution for HIF validation and interchange |
| `hypergraph-viz[viewer]` | Python API plus the matching `hypergraph-viz-viewer` distribution |
| `hyper_viz` | Python import |
| `hyper` | Desktop viewer executable |

For interchange alone, install `hypergraph-viz` and omit the `show()` and
`wait()` calls. The API works without a desktop or Bevy. Use `uv run` for
project scripts; `uvx` runs tools in a separate environment and the API package
has no executable. See [uv's tool guide](https://docs.astral.sh/uv/guides/tools/).

<details>
<summary>Use pip instead of uv</summary>

Requires CPython 3.10+. With Python 3.12 installed, create a virtual environment:

```bash
python3.12 -m venv .venv
source .venv/bin/activate
python -m pip install --upgrade "hypergraph-viz[viewer]"
hyper
```

On Windows, create it with `py -3.12 -m venv .venv` and activate it in
PowerShell with `.venv\Scripts\Activate.ps1`. Run the example with
`python explore.py`. For the API alone, install `hypergraph-viz`.
If installation reports no matching distribution, check `python --version`;
upgrading pip does not upgrade Python.

</details>

## Data and scientific use

The viewer reads [HIF](docs/HIF.md) (Hypergraph Interchange Format) and native
`hypergraph.v1` JSON, detecting the format automatically. See the
[native JSON example](docs/VIEWER.md#example-scene) and
[projection guide](docs/VIEWER.md#quick-start) for `bipartite` (default),
`clique`, and `star` projections, and `--watch` for file reloads.

Hyper complements analysis tools such as [XGI](https://github.com/xgi-org/xgi),
[HyperNetX](https://github.com/pnnl/HyperNetX), and
[Hypergraphx](https://github.com/HGX-Team/hypergraphx). Export supported data to
HIF, explore it in Hyper, and keep your analysis in the original library.
The [XGI example](crates/hyper-viz-python/examples/xgi_interop.py) demonstrates
export and round-trip interchange.

Before using a visualization in research:

- **Check viewer compatibility.** The viewer supports undirected memberships.
  Directed or simplicial semantics and incidence properties are rejected during
  conversion. HIF document interchange preserves richer data; keep the original
  `HifDocument` for export. See [supported semantics](docs/HIF.md#supported-semantics).
- **Read hulls as visual aids.** Large groups use approximate hulls. Confirm
  membership in the selection panel; spatial proximity is a layout result.
- **Measure your workload.** Dense hulls can be expensive. The
  [benchmarks](docs/benchmarks/ecosystem-2026-10-03/README.md) report completion
  counts, methods, and limits; they do not establish an overall speed ranking.

Hyper is **early-stage software**. The current Python API exposes HIF
interchange and viewer launching; Rust exposes projections and layout too.
The viewer opens a native window, including when launched from Python.
Notebook embedding, live Python updates, and direct scientific-library object
adapters are not yet available; see the [notebook roadmap](docs/NOTEBOOK_ROADMAP.md)
for browser plans.

## Layout and exploration

Choose bipartite, clique, or star projection to inspect the same memberships
from different views. Search, picking, lasso selection, and neighborhood focus
help you explore individual nodes and overlapping groups; display settings and
saved sessions keep the view manageable.

The upcoming `0.2.0` release adds connectivity-based initialization and tunable
Legacy, Normalized, and LinLog layouts. An optional dependency canvas explores
graphs with validated import-direction attributes through bounded traversal,
filters, and shortest paths. See the [viewer guide](docs/VIEWER.md),
[Rust layout and query APIs](docs/API.md), and [release notes](CHANGELOG.md).
Build this checkout to use these additions until `0.2.0` is published;
published `0.1.2` wheels predate them.

## Use from Rust

The `hyper-viz` core handles JSON I/O, projections, and 3D force layout without
Bevy or a window. Add it to your application's `Cargo.toml`:

```toml
[dependencies]
hyper-viz = { git = "https://github.com/atomicstrata/hyper.git" }
```

Build and lay out a group:

```rust
use hyper_viz::prelude::*;

fn main() {
    let graph = Hypergraph::new()
        .vertex("alice", "Alice", "person")
        .vertex("bob", "Bob", "person")
        .vertex("carol", "Carol", "person")
        .hyperedge("paper-a", ["alice", "bob", "carol"], "Paper A");

    let scene = graph.project(Projection::Bipartite);
    let mut layout = ForceLayout3D::from_scene(&scene, LayoutConfig::default());
    layout.step();
    println!("{} layout nodes", layout.positions.len());
}
```

Add `hyper-viz-bevy` from the same Git repository for a standalone window or a
plugin in an existing Bevy app. The [Rust API guide](docs/API.md) covers viewer
embedding and live scene updates; [architecture](docs/ARCHITECTURE.md) explains
the crate boundaries. Rust builds require **1.89+**.

<details>
<summary>Run from source</summary>

```bash
git clone https://github.com/atomicstrata/hyper.git
cd hyper
cargo run --locked --release -- fixtures/sample.json
```

The native viewer requires a desktop graphics environment; the first build can
take several minutes. To run the core example without a window:

```bash
cargo run --locked -p hyper-viz --example project_scene
```

See the [viewer guide](docs/VIEWER.md) for more commands, and the
[Python build instructions](docs/HIF.md#python-installation) for extension development.

</details>

## Use cases and benchmarks

Hyper works with group relationships such as coauthorship, biochemical
reactions, collaboration networks, and software dependencies. The Python
quick start above demonstrates overlapping coauthor groups.

One larger case study explores **Mathlib module imports** and their dependency
groups. It provides a reproducible dataset and a workload for evaluating layout
and rendering behavior.


[Case study and reproduction](docs/mathlib-demo.md) ·
[Full-resolution recording](docs/media/mathlib-realtime.mp4) ·
[Layout comparisons and native measurements](docs/benchmarks/module-explorer.md).
The recording follows realtime camera motion through a settled layout, with
capture overhead included; it is not a live-layout performance test.
The [ecosystem benchmarks](docs/benchmarks/ecosystem-2026-10-03/README.md)
cover additional workloads, methods, and limitations.

## Documentation

| Guide | What you'll find |
|---|---|
| [Viewer](docs/VIEWER.md) | Setup, controls, projections, display settings, and saved sessions |
| [Python and HIF](docs/HIF.md) | Interchange, Python API, scientific semantics, and source builds |
| [Rust API](docs/API.md) | Build graphs, run layouts, and embed the viewer |
| [Architecture](docs/ARCHITECTURE.md) | Crate responsibilities and integration boundaries |
| [Benchmarks](docs/benchmarks/ecosystem-2026-10-03/README.md) | Measured results, methods, and limitations |

For questions or bug reports, [open an issue](https://github.com/atomicstrata/hyper/issues).
To contribute, start with [CONTRIBUTING.md](CONTRIBUTING.md) for development
setup, checks, and pull request guidelines.

If you use Hyper in research, record the package version or Git commit,
dataset, projection, and viewer settings so others can reproduce the view.

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE).
See [NOTICE](NOTICE) for third-party notices.
