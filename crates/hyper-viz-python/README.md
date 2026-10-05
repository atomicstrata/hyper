# Hyper for Python

`hypergraph-viz` provides Rust-backed [Hypergraph Interchange Format (HIF)](https://github.com/HIF-org/HIF-standard)
validation and interchange, plus a launcher for Hyper's native 3D viewer. Use it
to exchange scientific datasets while retaining metadata and explicit group identities.
The package includes generated extension type stubs, typed Python wrappers, and `py.typed`.

## Installation

With [uv](https://docs.astral.sh/uv/getting-started/installation/), create a project
with a supported Python version and add the package:

```sh
uv init --python 3.12 hyper-example
cd hyper-example
uv add hypergraph-viz
```

uv manages `.venv` and `uv.lock`, and can download Python 3.12 if needed.
Save your Python code as `explore.py` and run it with `uv run explore.py`.
In an existing uv project, run only `uv add hypergraph-viz`.
Alternatively, use `python -m pip install hypergraph-viz` in a Python 3.10+
virtual environment. Upgrading pip does not upgrade Python.

Requires standard CPython 3.10 or newer. Platform wheels cover Linux x86_64,
macOS arm64/x86_64, and Windows x86_64; installing a compatible wheel requires
no Rust compiler. Building from source requires Rust 1.89 or newer and maturin.
The Python extension has no Bevy dependency.

## Exchange datasets

```python
import hyper_viz

document = hyper_viz.HifDocument.load("dataset.hif.json")
document.save("roundtrip.hif.json")
print(document.to_json())

# Explicitly check compatibility and convert to Hyper's native JSON.
native_json = document.to_hypergraph_json()
```

`HifDocument.from_json(raw)` accepts HIF JSON; `from_hypergraph_json(raw)` accepts
native Hyper JSON. Validation uses a pinned schema bundled with the package and
never fetches resources at runtime.

Keep the original document for scientific export. It preserves JSON values,
record order, optional field presence, integer/string IDs, isolated nodes, empty
edges, metadata, and incidence properties. Whitespace and object-key order are
not preserved.

## Launch the native viewer

From your uv project, install the matching prebuilt desktop viewer:

```sh
uv add "hypergraph-viz[viewer]"
uv run hyper
```

`uv run hyper` opens the built-in demo; use
`uv run hyper dataset.hif.json --projection bipartite` for your own dataset.
For pip, use `python -m pip install --upgrade "hypergraph-viz[viewer]"`.

To launch only the desktop viewer without creating a project:

```sh
uvx --python 3.12 --from hypergraph-viz-viewer hyper
# Open your dataset:
uvx --python 3.12 --from hypergraph-viz-viewer hyper dataset.hif.json
```

`--from` selects the distribution that supplies the `hyper` executable.
`hypergraph-viz` supplies the Python API and has no executable of its own;
use `uv add "hypergraph-viz[viewer]"` for Python integration in a project.
See [uv's tool guide](https://docs.astral.sh/uv/guides/tools/).

Wheels cover Linux x86_64, macOS arm64/x86_64, and Windows x86_64. No Rust
compiler or repository checkout is needed on these platforms. A desktop session
and graphics driver are required. Linux viewer wheels require glibc 2.28+;
macOS wheels target 11.0+. Notebook kernels on remote machines cannot open a
window on your laptop; see the
[notebook prototype roadmap](https://github.com/atomicstrata/hyper/blob/python-v0.1.2/docs/NOTEBOOK_ROADMAP.md).

Save the following as `explore.py` alongside `dataset.hif.json`, then run
`uv run explore.py`:

```python
import hyper_viz

document = hyper_viz.HifDocument.load("dataset.hif.json")
viewer = hyper_viz.show(document, projection="bipartite")
exit_code = viewer.wait()
# Or viewer.close() to terminate and clean up.
```

Pass `executable="/path/to/hyper"` to select a binary explicitly; otherwise the
launcher finds the version-matched installed companion, then falls back to
`hyper` on PATH. It locates the companion through installed-package metadata,
so activating a virtual environment is not required. It launches asynchronously without a shell,
retains an owned temporary snapshot, and cleans up after exit. It does not
download a viewer.

## Compatibility and errors

Full-document interchange supports more semantics than viewer conversion.
The viewer supports undirected memberships. Directed and simplicial semantics,
incidence direction/weights/attributes, and duplicate declarations or incidence
pairs produce `HifCompatibilityError`. Node/edge weights round to finite `f32`
for viewing; original values remain in the document. Arbitrary attributes are
retained but not exposed by the stock viewer inspector.

Malformed HIF raises `HifValidationError`. Both exceptions carry `location`
and `reason`; file failures raise `OSError`. Direct scientific object adapters,
notebook embedding, headless layout bindings, and live Python updates are deferred.

See the [integration guide](https://github.com/atomicstrata/hyper/blob/python-v0.1.2/docs/HIF.md)
and [XGI example](https://github.com/atomicstrata/hyper/blob/python-v0.1.2/crates/hyper-viz-python/examples/xgi_interop.py).
Hyper is licensed under MIT OR Apache-2.0; the bundled HIF schema is MIT-licensed.
