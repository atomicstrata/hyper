# HIF interchange and Python integration

Hyper can read [HIF](https://github.com/HIF-org/HIF-standard), the Hypergraph Interchange Format used by scientific hypergraph tools. Full-document interchange and viewer conversion are separate operations: a valid directed HIF document can be saved unchanged, but cannot currently be visualized faithfully.

## Rust

```rust
use hyper_viz::{load_hif, save_hif, HifDocument};

let document = load_hif("dataset.hif.json")?;
save_hif("roundtrip.hif.json", &document)?;
let graph = document.to_hypergraph()?; // checks viewer compatibility
let exported = HifDocument::from_hypergraph(&graph)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

`parse_hif` and `serialize_hif` operate on strings. `load_json` and `from_json_str` automatically detect HIF, native `hypergraph.v1`, and supported legacy exports. `load_json_with_format` and `from_json_str_with_format` accept `InputFormat::{Auto,Hif,Hypergraph}`. Mixed or unidentified envelopes are rejected. Native documents without a version remain accepted when native fields identify them.

```sh
cargo run -- dataset.hif.json --format hif
cargo run -- dataset.hif.json --format hif --watch
```

Both initial loading and watched reloads use the selected format. An invalid watched update leaves the previous scene visible and logs its error.

## Supported semantics

HIF serialization preserves JSON values, record order, optional field presence, metadata, typed string/integer IDs, node/edge weights, and incidence direction, weights, and attributes. Whitespace and object-key ordering are not retained. The pinned upstream schema is bundled and validated offline; network retrieval features are disabled. Upstream provenance and license live in `crates/hyper-viz/schemas`.

Viewer conversion supports undirected documents, including an omitted `network-type` when incidences have no direction. It rejects `directed`, `asc`, and any incidence direction, weight, or attributes (including an explicitly empty attributes object). Errors identify the offending JSON Pointer. Duplicate declarations and incidence pairs are rejected at conversion; they can still be preserved by document interchange.

Nodes/edges referenced only by incidences are inferred. Explicit isolated nodes and empty edges are retained in the graph, although empty edges have no drawable geometry. Integer `1` becomes viewer ID `i:1`; string `"1"` becomes `s:1`. Labels show original IDs. Integral numeric spellings denote the same identity. Scientific attribute keys are not interpreted as viewer coloring or status.

Node/edge weights are rounded to finite `f32` during viewer conversion; overflow is an error. Their original numbers remain in the HIF document. Keep that document for scientific export—exporting a converted native graph does not reconstruct the original IDs or precision.

Native-to-HIF export keeps native string IDs, memberships, weights, and attributes. Native display fields are preserved under the reserved `_hyper_viz` attribute on metadata/nodes/edges. A collision with that key, non-finite weights, duplicate IDs/memberships, or missing member references is an error. HIF-to-viewer conversion treats this reserved content as opaque metadata too.

## Python installation

The distribution is named `hypergraph-viz`; import it as `hyper_viz`. The Python extension depends on the Rust core, not Bevy. Requires standard CPython 3.10 or newer. Install a platform wheel with `python -m pip install hypergraph-viz`; a compatible wheel needs no Rust compiler. For local development, build with Rust 1.89 and maturin:

```sh
python -m venv .venv
. .venv/bin/activate # Windows: .venv\Scripts\activate
python -m pip install maturin==1.9.6
cd crates/hyper-viz-python
cargo run --bin stub_gen
maturin develop --locked
```

For wheel installation, run `maturin build --release --locked` in the same directory and install the resulting platform wheel from `target/wheels`. CI prepares Linux x86_64, macOS arm64/x86_64, and Windows x86_64 wheels. Version tags (`python-v<version>`) publish tested wheels and a verified source archive through PyPI Trusted Publishing; ordinary CI runs do not publish. See the [Python release guide](PYTHON_RELEASE.md) for the one-time publisher setup and release procedure. Wheels use the CPython stable ABI starting at 3.10, not the free-threaded ABI. To build a standalone source archive (maintainer Python 3.11+), run `python scripts/releases/build_python_sdist.py --out dist` from the repository root with maturin on PATH; the helper excludes the root viewer package so the archive can build independently.

```python
import hyper_viz

document = hyper_viz.HifDocument.load("dataset.hif.json")
document.save("roundtrip.hif.json")
print(document.to_json())

# Install hypergraph-viz[viewer], then:
viewer = hyper_viz.show(document, projection="bipartite")
code = viewer.wait() # or viewer.close() to terminate
```

`HifDocument.from_json(raw)` accepts HIF JSON; `from_hypergraph_json(raw)` accepts native JSON. `to_hypergraph_json()` explicitly checks viewer compatibility. File arguments on document methods are strings; the launcher also accepts `pathlib.Path`.

`HifValidationError` and `HifCompatibilityError` expose `location` and `reason`. I/O failures raise `OSError`. The launcher validates before launching and resolves an explicit executable, a version-matched installed `hypergraph-viz-viewer` binary, or `hyper` on PATH, in that order. Companion discovery uses installed RECORD metadata and works without virtual-environment activation. Missing/damaged installations raise `FileNotFoundError` with reinstall guidance; companion/core version mismatches raise `RuntimeError`. It never invokes a shell and inherits stdout/stderr. It creates an owned temporary snapshot, returns immediately, and cleans up automatically when the viewer exits. `wait()` returns nonzero exit codes without hiding diagnostics. A timeout preserves the running process and snapshot. `close()` terminates, waits five seconds, then kills if necessary. A context manager closes on exit. Dropping the handle keeps the viewer open; a background waiter retains ownership until exit.

Install the desktop wheel with `python -m pip install "hypergraph-viz[viewer]"`; no Rust compiler or checkout is needed. The companion is pinned to the core version. Linux desktop wheels require glibc 2.28+, macOS wheels target 11.0+, and Windows wheels target x86_64. A desktop session and a working graphics driver are required; graphics drivers remain system dependencies. For source installation, use `cargo install --path . --locked`. Nothing is downloaded on first launch. Live Python updates, notebook embedding, analysis/layout bindings, and direct scientific-library object adapters are outside this release. See the [notebook prototype roadmap](NOTEBOOK_ROADMAP.md) for the next anywidget canvas and WASM probes.

## XGI and Julia

The runnable [XGI example](../crates/hyper-viz-python/examples/xgi_interop.py) demonstrates scientific export, HIF round-trip, and optional viewer launch. Tests pin XGI 0.10.2. No XGI import is required to use `hyper_viz`.

The [Julia example](../examples/hif_interop.jl) uses SimpleHypergraphs.jl and its HIF export. Run it in a Julia environment containing SimpleHypergraphs, then preserve its output with the Rust `hif_roundtrip` example or Python package. SimpleHypergraphs exports incidence weights, so viewer conversion rejects that output explicitly. Julia integration is an example; Julia is not a Python runtime dependency.

## Development checks

```sh
cargo test -p hyper-viz
cargo run -p hyper-viz-python --bin stub_gen
python -m pytest crates/hyper-viz-python/tests
python -m mypy --strict crates/hyper-viz-python/python/hyper_viz
```

The generated `_core.pyi` declares the native API. A CI regeneration diff catches drift; `py.typed` and annotated Python wrappers provide IDE and type-checker support. The wrappers are handwritten; stubs are generated from Rust binding declarations.
