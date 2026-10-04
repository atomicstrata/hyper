# Hyper for Python

`hypergraph-viz` provides Rust-backed [Hypergraph Interchange Format (HIF)](https://github.com/HIF-org/HIF-standard)
validation and interchange, plus a launcher for Hyper's native 3D viewer. Use it
to exchange scientific datasets while retaining metadata and explicit group identities.
The package includes generated extension type stubs, typed Python wrappers, and `py.typed`.

## Installation

```sh
python -m pip install hypergraph-viz
```

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

Install the matching prebuilt desktop viewer with the optional extra:

```sh
python -m pip install --upgrade "hypergraph-viz[viewer]"
```

Wheels cover Linux x86_64, macOS arm64/x86_64, and Windows x86_64. No Rust
compiler or repository checkout is needed on these platforms. A desktop session
and graphics driver are required. Linux viewer wheels require glibc 2.28+;
macOS wheels target 11.0+. Notebook kernels on remote machines cannot open a
window on your laptop; see the
[notebook prototype roadmap](https://github.com/atomicstrata/hyper/blob/python-v0.1.2/docs/NOTEBOOK_ROADMAP.md).

Then launch it from Python:

```python
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
