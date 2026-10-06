# Hyper desktop viewer

Prebuilt native viewer for [hypergraph-viz](https://pypi.org/project/hypergraph-viz/).
Launch the built-in demo with [uv](https://docs.astral.sh/uv/getting-started/installation/):

```sh
uvx --python 3.12 --from hypergraph-viz-viewer hyper
```

Open your own dataset:

```sh
uvx --python 3.12 --from hypergraph-viz-viewer hyper dataset.hif.json
```

`--from` selects this package, which supplies the `hyper` executable.
See [uv's tool guide](https://docs.astral.sh/uv/guides/tools/).

For Python integration, install the API and matching viewer in a project:

```sh
uv init --python 3.12 hyper-example
cd hyper-example
uv add "hypergraph-viz[viewer]"
```

In an existing uv project using Python 3.10+, run only `uv add`.
Save the Python example below as `explore.py` alongside `dataset.hif.json`,
then run `uv run explore.py`. No environment activation is needed.
uv can download Python 3.12 when necessary.
`hypergraph-viz` provides the Python API and has no executable of its own;
`uvx hypergraph-viz` cannot launch the viewer.

For pip, use `python -m pip install --upgrade "hypergraph-viz[viewer]"` in a
Python 3.10+ virtual environment.

```python
import hyper_viz

document = hyper_viz.HifDocument.load("dataset.hif.json")
viewer = hyper_viz.show(document, projection="bipartite")
viewer.wait()  # Or viewer.close() to terminate.
```

No repository checkout or Rust compiler is needed on supported platforms.
The viewer executable is also available as `hyper` in your Python environment.
The launcher locates it through installed-package metadata, so activating the
virtual environment is optional when running that environment's Python.

Wheels cover Linux x86_64 (glibc 2.28+), macOS arm64/x86_64 (11.0+), and
Windows x86_64. A desktop session and working graphics driver are required. The viewer runs on the
machine executing Python; it does not embed into a notebook or send a window
from a remote kernel to your browser. Use the
[notebook prototype roadmap](https://github.com/atomicstrata/hyper/blob/python-v0.2.0/docs/NOTEBOOK_ROADMAP.md)
for the planned browser frontend.

This package contains a native binary, not a Python extension. It does not
include HIF Python bindings by itself; use the `[viewer]` extra shown above.
Windows wheels link the C runtime statically, avoiding a separate Visual C++
Redistributable installation. An explicit `executable="/path/to/hyper"` still overrides the packaged binary.
There are no downloads during `show()`.

MIT OR Apache-2.0. The bundled HIF schema is MIT-licensed. Graphics drivers
remain system dependencies. Linux binary dependencies are audited without bundling external libraries.
Unused audio and gamepad backends are excluded from the desktop build.
