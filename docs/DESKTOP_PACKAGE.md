# Hyper desktop viewer

Prebuilt native viewer for [hypergraph-viz](https://pypi.org/project/hypergraph-viz/).
Install the Python API and matching viewer together:

With [uv](https://docs.astral.sh/uv/getting-started/installation/):

```sh
uv init --python 3.12 hyper-example
cd hyper-example
uv add "hypergraph-viz[viewer]"
uv run hyper
```

This opens the built-in demo. In an existing uv project, start with `uv add`.
Use `uv run hyper dataset.hif.json` to open your own dataset, or save the Python
example below as `explore.py` and run `uv run explore.py`. No environment
activation is needed. uv can download a compatible Python when necessary.

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
[notebook prototype roadmap](https://github.com/atomicstrata/hyper/blob/python-v0.1.2/docs/NOTEBOOK_ROADMAP.md)
for the planned browser frontend.

This package contains a native binary, not a Python extension. It does not
include HIF Python bindings by itself; use the `[viewer]` extra shown above.
Windows wheels link the C runtime statically, avoiding a separate Visual C++
Redistributable installation. An explicit `executable="/path/to/hyper"` still overrides the packaged binary.
There are no downloads during `show()`.

MIT OR Apache-2.0. The bundled HIF schema is MIT-licensed. Graphics drivers
remain system dependencies. Linux binary dependencies are audited without bundling external libraries.
Unused audio and gamepad backends are excluded from the desktop build.
