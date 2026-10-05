# Running the native viewer

The Bevy viewer opens a native desktop window. It requires a desktop session
and working graphics drivers.

## Install

With [uv](https://docs.astral.sh/uv/getting-started/installation/), open the
prebuilt viewer without creating a project:

```bash
uvx --python 3.12 --from hypergraph-viz-viewer hyper
uvx --python 3.12 --from hypergraph-viz-viewer hyper dataset.hif.json
```

Omitting the file opens the built-in coauthorship demo. `--from` selects the
distribution containing the `hyper` executable. For Python integration, use
`uv add "hypergraph-viz[viewer]"` in a CPython 3.10+ project and run scripts
with `uv run`; see the [complete Python example](../README.md#use-from-python).

Alternatively, run `python -m pip install --upgrade "hypergraph-viz[viewer]"`
in a CPython 3.10+ virtual environment, then run
`hyper`. See [platform requirements](DESKTOP_PACKAGE.md) for supported wheels.

For a source build, use Rust 1.89+ and run the commands below from this checkout.
The first Bevy build can take several minutes. For regular use, build with
`cargo build --release --bin hyper` or install with
`cargo install --path . --locked`.

## Quick start

From the workspace root:

```bash
# Built-in coauthorship demo
cargo run

# Example scene (fixtures/sample.json)
cargo run -- fixtures/sample.json

# Scientific HIF dataset (also detected without --format)
cargo run -- dataset.hif.json --format hif

# Hot-reload while editing JSON
cargo run -- --watch fixtures/sample.json

# Projection: bipartite (default), clique, star
cargo run -- --projection clique fixtures/sample.json

# Same via the crate example
cargo run -p hyper-viz-bevy --example view -- fixtures/sample.json
```

For prebuilt wheels, replace `cargo run --` with
`uvx --python 3.12 --from hypergraph-viz-viewer hyper`, `uv run hyper` in a
project with the viewer extra, or `hyper` in an activated pip environment.
`--watch` reloads the graph while retaining positions by stable ID and remapping
surviving selection and focus. Invalid input leaves the previous scene visible.

The projections are:

- **Bipartite** (default): one hub per hyperedge, connected to its members.
- **Clique**: links between every pair of members within each hyperedge.
- **Star**: attraction toward each hyperedge's centroid, without hub nodes.

Headless (no window) library smoke:

```bash
cargo run -p hyper-viz --example project_scene
```

Optional logs (see [`.env.example`](../.env.example)):

```bash
RUST_LOG=hyper=info,hyper_viz_bevy=info cargo run -- fixtures/sample.json
```

## Example scene

[`fixtures/sample.json`](../fixtures/sample.json) is a domain-agnostic
coauthorship hypergraph (`hypergraph.v1`): five people, four hyperedges with
mixed status (`active` / `shadowed` / `rejected`). Use it as the canonical
external-demo fixture.

Minimal shape:

```json
{
  "version": "hypergraph.v1",
  "meta": { "id": "g1", "title": "Coauthorship" },
  "vertices": [
    { "id": "alice", "label": "Alice", "kind": "person" },
    { "id": "bob", "label": "Bob", "kind": "person" },
    { "id": "carol", "label": "Carol", "kind": "person" }
  ],
  "hyperedges": [
    { "id": "paper-a", "label": "Paper A", "vertices": ["alice", "bob", "carol"] }
  ]
}
```

| Field | Required | Notes |
|---|---|---|
| `vertices[].id` | yes | Stable ID referenced by hyperedges |
| `vertices[].label` | no | Falls back to `id` |
| `vertices[].kind` | no | Free-form; used for color |
| `vertices[].status` | no | `active` (default), `shadowed`, `rejected`, `attention` |
| `hyperedges[].id` | yes | Stable ID for the group |
| `hyperedges[].vertices` | yes | Member vertex IDs |
| `hyperedges[].status` | no | `active` (default), `shadowed`, `rejected`, `attention` |
| `*.attrs` | no | Opaque JSON map for the host app |

For scientific interchange and viewer compatibility, see the
[HIF guide](HIF.md#supported-semantics).

## Controls

Left drag orbits; a click with almost no movement selects. Orbiting keeps the
current selection. Pointer input over a UI panel does not orbit or zoom the scene.

| Input | Action |
|---|---|
| Left drag | Orbit (keeps selection) |
| Left click | Select vertex / hyperedge |
| Click empty | Clear selection |
| Shift / Cmd + click | Add to selection |
| Scroll | Zoom |
| Lasso (UI panel) | Polygon selection; disables orbit while enabled |
| Space | Toggle force layout |
| A | Isolate `attention` status: hide other hulls, boost remaining fills, label those nodes |
| F | Frame the camera (ignored while typing in search) |
| ⌘F / Ctrl+F | Find bar |
| ⌃⌘F | Toggle isolation of the selected incident neighborhood |
| / | Find bar (when not typing) |
| Enter (in find) | Isolate the match neighborhood and frame the camera |
| Esc | Clear find / focus |
| Clear selection | Button in the Selection window |

The find bar highlights matches as you type. Search covers label, ID, kind, and
status using BM25 and Jaro–Winkler matching. While neighborhood focus is active,
each new query also isolates the matches and reframes the camera.

The **Selection** window lists selected hyperedges and vertices with kind,
label, status, and a location taken from the ID (`repo:…`, `wt:…`, `pr:…`).
Click a row to narrow the selection to that node. Picking hits vertices, hull
triangles, and two-member segments. Hidden hubs are not pickable.

## Display settings

The default bipartite projection adds a hub for each hyperedge. Hubs are hidden
by default so groups appear as sets. Uncheck **Hide extra-node hubs** in the
**Hyperedge hulls** panel to show them as spheres.

| Group size | Drawing |
|---|---|
| 2 | Colored line between members |
| 3 | Triangle hull |
| 4 | Tetrahedron hull |
| 5+ | Convex hull using at most 24 sampled members |

Large-group hulls are approximate. The geometry cap does not remove memberships,
and a hull is not an exact membership boundary. Empty edges have no drawable
geometry.

Each hyperedge has a stable hue derived from its ID. Nested hulls are inflated
slightly to avoid overlapping surfaces flickering. Vertex color comes from
`kind`, using a fixed palette for known names and a hash otherwise.

Hover and selection keep the object's hue:

| State | Appearance |
|---|---|
| Hover | Nodes grow slightly; hull and line wires brighten |
| Selected | Higher saturation and glow |
| Hover + selected | Selection treatment wins; nodes still grow slightly |

## Session persistence

The viewer restores UI preferences, navigation (including lasso), camera pose,
selection, neighborhood focus, and the find query from a `hyperviz.session.v1`
JSON file.

| Platform | Default file |
|---|---|
| macOS | `~/Library/Application Support/hyper-viz/session.json` |
| Windows | `%APPDATA%/hyper-viz/session.json` |
| Linux | `$XDG_CONFIG_HOME/hyper-viz/session.json`, or `~/.config/hyper-viz/session.json` when unset |

Override the file with `HYPER_VIZ_SESSION=/path/to/session.json`, or set
`HYPER_VIZ_SESSION=off` to disable persistence. Preferences are global; camera,
selection, and focus are keyed by graph ID. Layout positions are not stored.
