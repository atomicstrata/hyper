# Notebook prototype: anywidget canvas and a small dataset

Status: planned follow-up after desktop wheels. There is no `[notebook]` extra
or `hyper_viz.widget()` API in the current release. Desktop `show()` opens a
window on the kernel machine; it does not display inside a remote notebook.

## Goal and first dataset

Prove that a pip-installed widget can render a small compatible HIF document in
a notebook cell and exchange selections with Python. Use
[`fixtures/notebook-small.hif.json`](../fixtures/notebook-small.hif.json):

- 7 nodes, including distinct integer `1` and string `"1"`, plus an isolated node.
- 4 edges: a pair, a triple, an overlapping triple, and an empty edge.
- 8 incidences; nested metadata, arbitrary attributes, and a node/edge weight.

This fixture is also used for desktop graphics startup tests. It is synthetic;
it is not a performance benchmark or evidence of notebook compatibility.

## Proposed user experience

The following API is a design target, not runnable today:

```sh
python -m pip install "hypergraph-viz[notebook]"
```

```python
import hyper_viz

document = hyper_viz.HifDocument.load("notebook-small.hif.json")
view = hyper_viz.widget(document)
view  # Inline notebook output; no desktop process or viewer wheel.
```

Keep `show()` explicitly desktop and `widget()` explicitly inline. The Python
kernel retains the original HIF document for export. An anywidget frontend
receives a compatible display snapshot and owns its canvas and interaction.

## Step 1: establish widget transport and lifecycle

Create a disposable prototype using anywidget and traitlets. Pin their actual
versions in the prototype environment and record Python, notebook, and browser
versions. Package the frontend ESM/CSS locally; consumers should need neither
Node.js nor external CDN access.

1. Validate the fixture with `HifDocument` and obtain `to_hypergraph_json()`.
   The existing Python package exposes native JSON, not headless scene/layout
   bindings. Do not assume a projection or layout Python API already exists.
2. Draw a simple Canvas2D diagnostic with fixed, seeded coordinates. Show all
   seven nodes, group membership, and an explicit list for the empty edge.
   This is a transport/identity test, not the final 3D renderer.
3. Click a node or group and synchronize selected IDs back to Python. Highlight
   a Python-driven selection in the frontend. Keep camera and pointer movement
   local; do not send every animation frame through the kernel.
4. Display arbitrary attributes for the selection without interpreting attribute
   names as status or coloring. Fetch original metadata from the retained
   document when needed rather than treating display JSON as a lossless export.
5. Re-render the same widget and create two independent widgets. Dispose event
   listeners, observers, animation loops, and graphics resources when a view is
   removed. Closing one view must not corrupt another view of the same model.

Use the type-prefixed native IDs as browser keys and map selections back to
original HIF identities in Python. Do not use row indexes or plain object keys
that collapse integer/string identities. For future large integer IDs, transmit
integer identity as a tagged decimal string rather than a JavaScript Number.

## Step 2: test Bevy/WASM reuse before choosing the 3D frontend

Run a separate feasibility probe; do not assume the desktop dependency graph
compiles for the browser. The current `hyper-viz-bevy` manifest enables desktop
Bevy defaults and Wayland. The core's dependency features also need inspection.

```sh
rustup target add wasm32-unknown-unknown
cargo check -p hyper-viz --target wasm32-unknown-unknown
```

Record failures, including random-number generation and schema-validation
features. Keep HIF validation in the Python kernel for the first widget; a
browser build need not include the complete schema validator. If required,
separate portable scene/layout code from platform-specific dependencies through
explicit features rather than hiding incompatibility.

Build a small browser entry point with a dedicated feature set and a canvas
owned by the widget. Test WebGL2 first for broad compatibility; assess WebGPU
separately. Verify shader/asset delivery, pointer and keyboard focus, resizing,
multiple canvases, and teardown. Confirm a second view can start after the first
is removed without retaining an old event loop or GPU context.

Compare two concrete probes:

| Probe | Evidence required |
|---|---|
| Bevy renderer compiled to WASM | Successfully render the fixture in the widget; demonstrate picking, resize, and clean disposal; measure cold/warm load and bundle size |
| Browser renderer with selected Rust algorithms in WASM | Same fixture and interaction checks; establish the work required for hulls, picking, and UI parity; identify algorithms that can run in a worker |

Choose based on measured integration cost and behavior, not the assumption that
WASM is automatically faster or that a successful compile proves notebook DX.
WASM runs inside the frontend; it does not replace anywidget's kernel connection.

## Step 3: verify environments and decide whether to productize

Start with JupyterLab and VS Code notebooks, then test Colab and a remote Jupyter
kernel explicitly. Anywidget supports those hosts; Hyper compatibility still
has to be demonstrated in each one. Also test a fresh kernel, rerunning a cell,
multiple views, a kernel restart, and a document rejected by viewer compatibility.

Acceptance criteria for this small prototype:

- All nodes/edges are represented, including isolates and the empty edge list.
- Integer/string identity remains distinct in display and selection round-trips.
- Selecting an object exposes its metadata; scientific export remains unchanged.
- Python-to-browser and browser-to-Python selection updates work independently
  for two widgets, without duplicate events or stale state after reruns.
- Bundled assets load with external networking disabled in the local Jupyter
  test. Document any host-specific asset-delivery or content-security restrictions.
- Capture a screenshot and record cold/warm startup, bundle size, selection
  response, and retained resources after repeated creation/disposal. These are
  prototype observations, not a comparison with native benchmarks.

Only after these checks define and publish the `[notebook]` extra, the widget API,
and supported host versions. Add binary buffers and workers when measured costs
justify them; preserve a bounded state/update protocol with sequence numbers.
Notebook support, standalone browser hosting, browser-side HIF validation, and
full desktop feature parity are separate delivery decisions.

## References

- [anywidget getting started](https://anywidget.dev/en/getting-started/)
- [Widget state, messages, and binary buffers](https://anywidget.dev/en/jupyter-widgets-the-good-parts/)
- [anywidget bundling](https://anywidget.dev/en/bundling/)
- [Bevy browser examples](https://bevy.org/examples/)
