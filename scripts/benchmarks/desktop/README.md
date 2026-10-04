# Desktop benchmark preparation

These adapters prepare inputs and instrumentation; preparation does not start timed workloads.

## HyperGodot

Source: https://github.com/ElsevierSoftwareX/SOFTX-D-25-00370 at
`49d03cd7540d2900cc8624048973ef430ff4e009`. Runtime prepared from Godot
4.7.2 stable universal macOS release. The upstream project requests Godot 4.4
Forward Plus and disables vsync. This comparison is a 2D group viewer using
native hull drawing, distinct from the 3D incidence render track.
The downloaded runtime SHA-256 was verified as
`c58a24e31d720be9d62f60cb5627c4e695fb72f21b0cfe1bc9ccaa9a3b3ba63e`;
`--version` returns `4.7.2.stable.official.ed1daf0bf`. Headless source import and
two untimed initialization frames of the moderate dataset succeeded. No rendered
frame timings were collected during preparation. Direct `--check-only` on the
scene script omits project autoload globals; the actual project initialization
loads those globals and compiles successfully.

```sh
git clone https://github.com/ElsevierSoftwareX/SOFTX-D-25-00370 /tmp/hyper-benchmark-competitors/hypergodot
git -C /tmp/hyper-benchmark-competitors/hypergodot checkout 49d03cd7540d2900cc8624048973ef430ff4e009
python3 scripts/benchmarks/desktop/prepare_hypergodot.py --dataset target/benchmark/datasets/moderate.json --destination /tmp/hyper-benchmark-competitors/hypergodot-moderate
/path/to/Godot --headless --editor --path /tmp/hyper-benchmark-competitors/hypergodot-moderate --import --quit
```

Preparation copies upstream source, removes its missing and unused
`MaterialIconsDB` autoload, exports exact parser-compatible tagged text, and adds
the instrumentation scripts. The format is `n0 n1 #WEIGHT: 1 #GROUP baseline`;
the actual parser requires no colon after GROUP. Safe token IDs have a reversible
map; hyperedge IDs are line numbers in upstream. Isolated vertices, empty edges,
and duplicate memberships are rejected explicitly. Weights/groups are normalized
for this matched unweighted fixture and recorded as adaptations.

After source import and harness validation, the root coordinator can execute:

```sh
/tmp/hyper-benchmark-competitors/Godot.app/Contents/MacOS/Godot --path /tmp/hyper-benchmark-competitors/hypergodot-moderate --script res://benchmark_runner.gd -- --output /tmp/hyper-benchmark-competitors/hypergodot-moderate-results.json --warmup 120 --frames 300 --repetitions 5
```

The harness overrides upstream startup layout while invoking production
`read_file`, `create_nodes`, `populate_edge_data`, sizing and drawing. It sets
canonical XY coordinates, pans/zooms a Camera2D, requests a 1920x1080 window
with stock HiDPI enabled, and records intervals between `_process` callbacks with the monotonic
clock. These include engine scheduling/render/presentation from the previous
frame and are not GPU duration. Upstream nodes, hulls, UI, labels and picking
remain; node radius 30, hull point count 4, edge width 1. Repetitions share one
process and receive a new warmup each. Window and viewport size, screen scale/DPI,
and the HiDPI setting are recorded. The final run verifies the actual render texture as 1920×1080; an earlier HiDPI-disabled run was clamped to 1920×917 and is excluded. reject matched-render comparison if actual
pixel dimensions differ from the requested size.

## Graphia

Source: https://github.com/graphia-app/graphia at version 5.5 commit
`b8ad51e820192a6f22ab310ebd84282384327c10`. Stock headless interface:
https://graphia.app/guide/section5/2_headless.html .
The public macOS release is universal arm64/x86_64, passes deep/strict code
signature verification, and its SHA-256 is
`ed77be142e34f209b6d95e959bb2b3937eca4d9621239a5f0dcc7a892a98ea85`.
Native CLI help and an untimed correctness GraphML import/save succeeded with
exit code 0. This bundle only supplies the Cocoa Qt platform plugin; setting
`QT_QPA_PLATFORM=offscreen` fails. Use its supported stock headless mode with the
native Cocoa platform instead.

```sh
python3 scripts/benchmarks/desktop/prepare_graphia.py --dataset target/benchmark/datasets/moderate.json --output /tmp/hyper-benchmark-competitors/graphia-inputs
/tmp/hyper-benchmark-competitors/Graphia.app/Contents/MacOS/Graphia --parameters /tmp/hyper-benchmark-competitors/graphia-inputs/moderate.parameters.json /tmp/hyper-benchmark-competitors/graphia-inputs/moderate.graphml
```

GraphML retains every canonical vertex and group as explicit incidence nodes.
XYZ values are attributes; the stock parser does not establish a common fixed
layout. Headless import/save time can be measured independently of interactive
rendering. Do not report it as rendering FPS.

## HNX Widget

Source https://github.com/pnnl/hypernetx-widget at
`58e795d6a362dcd3bbd9ccff462052956a40f4f8`; PyPI package `hnxwidget==0.1.1b3`.
Isolated environment is `/tmp/hyper-benchmark-competitors/widget-venv` with
`notebook==6.5.7`, `traitlets<5.10`, and `hypernetx==2.4.0`. Notebook 6.5.x is the
documented compatibility path. Construct `HypernetxWidget(incidence_mapping,
pos={vertex_id: [x, y]}, collapseNodes=False)` in a notebook cell. The wrapper
was smoke tested successfully with three vertices and two groups; installed
`ipywidgets==7.8.5` provides the compatible classic widget frontend.
The separate HNX 2.4.0 constructor fails with the resolved pandas 3.0.6
dependency; direct membership dictionary input avoids that constructor and is
supported by the widget's production API.
The wrapper
derives nodes from memberships, so isolated vertices are not preserved. Source
inspection shows supplied `pos` does not fix all bipartite hyperedge hubs and
forces continue; this belongs in the group exploration track, not the fully
controlled common-coordinate incidence track without further adaptation.

## HGPolyVis

https://github.com/peterdanieloliver/HGPolyVis at
`ba8868b1b002be5c32fb1e83548778fd7feed186` contains Windows 11 executable/Qt5
DLLs, datasets and README, without buildable application source. Upstream states
no other supported OS. It requires Windows for a valid native benchmark, and is
unavailable on the tested macOS. CSV import rows are
`hyperedge_id,member_id,member_id,...`; `.er` supports saved positions/colors.

## Julia

Optional Julia 1.13.1 macOS arm64 runtime was extracted and `--version` verified from official
https://julialang-s3.julialang.org/bin/mac/aarch64/1.13/julia-1.13.1-macaarch64.tar.gz
(expected SHA-256 `a3e0259d4777c2b776c2cba134d5b6c6c124048dcb913f8881517892c49a0f7f`).
The downloaded SHA-256 matched the expected value. The executable is
`/tmp/hyper-benchmark-competitors/julia-1.13.1/bin/julia`.
SimpleHypergraphs.jl dependency installation and measurements remain pending.
