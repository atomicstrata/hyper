# Mathlib source dependency atlas

This demo renders the actual mathlib source import graph with the native Bevy
viewer. The MP4 is a deterministic frame capture, not evidence of 30 FPS live
performance. Live measurements use a separate run with no screenshot readbacks.

## Dataset

Pinned source: [mathlib v4.34.1](https://github.com/leanprover-community/mathlib4/releases/tag/v4.34.1),
commit `d13f23b723b8a846827a245b89c10fc7d3f11612` (latest stable release checked
on 2026-10-02). Source license: Apache-2.0; authors: the mathlib community.

| Element | Count | Meaning |
|---|---:|---|
| Source files | 9,112 | Every `.lean` file in the repository, including tests and tooling |
| External modules | 272 | Explicit placeholders for imports whose source is outside this checkout |
| Visible nodes | 9,384 | Source files plus external placeholders |
| Import edges | 37,356 | One source → imported module relation; direction preserved in JSON attributes |
| Dependency groups | 7,550 | A source file plus its direct imports, when it has at least two |
| Largest group | 8,532 | Includes the umbrella `Mathlib.lean` import set |
| Total memberships | 118,066 | Both import pairs and dependency groups |

The spatial viewer draws undirected lines and unordered hyperedges; the native
dependency canvas draws directed import arrows. These groups describe
module dependencies, not theorem/proof dependencies. The exporter reads Lean's
module header (including `public`, `meta`, `import all`, and nested comments);
it does not infer dependencies from names appearing in theorem text. Duplicate
imports within one file are deduplicated. There are no synthetic replicated nodes.

## Reproduce

Run from the repository root. Python 3 uses only the standard library. The source
archive and generated data stay under ignored `target/`.

```bash
mkdir -p target/mathlib/source
curl -L --fail \
  https://codeload.github.com/leanprover-community/mathlib4/tar.gz/d13f23b723b8a846827a245b89c10fc7d3f11612 \
  -o target/mathlib/source.tar.gz
tar -xzf target/mathlib/source.tar.gz -C target/mathlib/source --strip-components=1
python3 scripts/mathlib_graph.py target/mathlib/source target/mathlib/graph.json \
  --release v4.34.1 --commit d13f23b723b8a846827a245b89c10fc7d3f11612
cargo build --release

# Explore interactively. Use Spatial for the full force-layout view.
HYPER_VIZ_SESSION=off target/release/hyper --projection star target/mathlib/graph.json

# Benchmark separately from video capture.
HYPER_VIZ_SESSION=off target/release/hyper target/mathlib/graph.json \
  --projection star --tour --frames 300 --warmup 120 \
  --report target/mathlib/benchmark-frozen.csv
HYPER_VIZ_SESSION=off target/release/hyper target/mathlib/graph.json \
  --projection star --tour --frames 300 --warmup 120 --live-layout \
  --report target/mathlib/benchmark-live.csv

# Capture 24 seconds at 1920×1080. The frames directory must not exist yet.
HYPER_VIZ_SESSION=off target/release/hyper target/mathlib/graph.json \
  --projection star --tour --frames 720 --fps 30 --warmup 120 \
  --capture target/mathlib/frames --report target/mathlib/capture.csv

# macOS system H.264 encoder (requires Xcode command-line tools).
mkdir -p docs/media
swift scripts/encode_video.swift target/mathlib/frames docs/media/mathlib.mp4 30

# Looping README GIF, preserving the full 24-second tour at 640×360 / 10 fps.
swift scripts/encode_gif.swift docs/media/mathlib.mp4 docs/media/mathlib.gif 640 10

# Alternative with FFmpeg on any platform:
ffmpeg -framerate 30 -i target/mathlib/frames/%06d.png \
  -c:v libx264 -crf 20 -pix_fmt yuv420p -movflags +faststart docs/media/mathlib.mp4
```

The README uses the looping GIF as its inline preview and links it to the 1080p
MP4. The GIF encoder also uses only macOS system frameworks; its output path must
not already exist.

The tour uses star-centroid projection: all 9,384 real/stub vertices are rendered,
without adding invisible bipartite hubs. The same dataset in bipartite projection
has 54,290 scene nodes and 118,066 layout links; those extra nodes are renderer
hubs, not additional source files. The measurements below apply only to the
star-centroid tour, not that larger bipartite scene. Do not use clique expansion for this dataset:
the umbrella import group alone would expand into tens of millions of pairs.

## Visual and measurement settings

The tour seeds positions deterministically from module IDs and subject kinds,
then runs 120 force-layout steps. It uses gravity `0.001` and centroid attraction
`0.0005` to retain subject structure. The video freezes layout after warmup; the
live-layout benchmark instead continues one force step per frame. Camera paths
are frame-indexed. Floating-point results can vary across platforms.

All 37,356 import lines are submitted, at 10% opacity. Nodes use the existing
low-detail sphere mesh. In the second half, all dependency groups are submitted
as hull fills at 0.1% opacity. Hull wireframes are disabled. The existing hull
algorithm samples at most 24 member positions for large groups, so these are
approximate surfaces; every member remains in the graph and layout. Labels are
replaced by a small title/count overlay for legibility.

The scripted tour disables picking, interactive HUD panels, and keyboard/mouse
controls, and renders continuously even when unfocused. Interactive viewer
performance can therefore differ.

CSV frame times measure wall-clock intervals between updates, including CPU,
render scheduling and presentation; they are not isolated GPU timings. The first
30 frames are discarded for pipeline startup. Hull creation at the phase
transition is retained in the raw samples. The two phases have different camera
positions, so their figures describe the tour workload rather than a controlled
same-view A/B test. Other desktop activity and thermal state affect results.

This is one large real-world workload, not a proven universal renderer maximum.
Use the same commands on larger datasets to explore the next limit.

## Measured run (2026-10-02)

Apple M5 Pro, 24 GB RAM; release build, Metal, 1920×1080, star-centroid
projection, 300 measured frames per run. These are single-run desktop
measurements, not hardware-independent guarantees.

These recordings were made with the demo changes on base commit `9ebe6d6`,
before integrating the newer `main` changes for the PR. They have not been
remeasured after that integration; the commands above let you measure the
current checkout.

| Layout | Drawing | Median frame | p95 frame | 1000 / median ms |
|---|---|---:|---:|---:|
| Frozen | Nodes + all imports | 28.99 ms | 33.78 ms | 34.50 FPS |
| Frozen | Plus all hull fills | 41.04 ms | 52.09 ms | 24.37 FPS |
| One step/frame | Nodes + all imports | 24.91 ms | 31.75 ms | 40.15 FPS |
| One step/frame | Plus all hull fills | 94.45 ms | 165.96 ms | 10.59 FPS |

Raw samples: [frozen](benchmarks/mathlib-frozen.csv),
[moving](benchmarks/mathlib-live.csv). The moving geometry and system load differ,
so the faster no-hull median in the moving run is not evidence that adding
simulation improves performance. Prewarm force steps took about 11 ms each.
The clearest limit here is updating thousands of hulls while the layout moves.

Two existing behaviors also surfaced at this scale and were corrected for this
demo: exceeding 5,000 projected links used to hide every line in the default
projection; animation opacity also imposed a 2% minimum that obscured dense hull
overlaps. Line rendering now has an explicit budget (0 means all), and low
configured opacity is preserved.

## Validation

```bash
python3 -m unittest discover -s scripts -p 'test_*.py'
cargo test --workspace
```

The exporter records a SHA-256 over sorted source paths and file bytes in graph
metadata, in addition to the release and commit. The source digest for this
dataset is `dde69137f0279fdb1ec0b40e16dd5252c0a13aa8550c9508e71ea811565347f3`.

## Explore imports and dependents

Graphs with validated directed imports open the native dependency canvas automatically.
Start at an exact module:

```sh
HYPER_VIZ_SESSION=off target/release/hyper --projection star \
  --module Mathlib.Topology.Basic target/mathlib/graph.json
```

Choose a search result or press Enter to select one module. Imports appear to the
right and dependents to the left; every arrow points from importer to imported.
The default scope is one hop in each direction, with all categories included.
`Mathlib.Topology.Basic` therefore shows three imports, four dependents, and
itself: eight modules. The umbrella dependency group does not expand this scope.

Set each direction independently to Off, 1–6 hops, or Transitive. The node budget
starts at 200 and can reach 1,000. Counts distinguish budget omissions and modules
excluded by filters; filters stop traversal and always retain the center. Full
direct-neighbor lists and the full shortest-path list remain available when the
canvas is truncated. Use `+1` to expand a neighbor, Reset expansion to undo those
expansions, and Back/Forward to revisit views. Drag to pan, scroll to zoom, Fit to
frame the scope, or use List view for dense neighborhoods.

The second picker traces a shortest directed import path from the center. A
filtered path is distinguished from an absent path; selecting the center as the
target yields zero hops. Path tracing adds path steps beyond the selected hop
depths, within the node budget, and reports any path steps omitted from the canvas.
Group inspection shows only the center's exported
`deps:<module>` group and its authoritative member list. Including it in Spatial
does not add members beyond the current scope; its hull needs all members present.

Inspect displayed scope in Spatial transfers exactly the displayed modules and
imports, plus the explicitly included group. Switch back to Dependencies to pause
spatial simulation and geometry work. `--view spatial` forces the spatial viewer;
`--view dependencies` forces the canvas, including its empty-state guidance for a
graph without imports. The default `--view auto` retains Spatial for generic graphs.

Large import graphs use deterministic subject seeds, gravity `0.001`, centroid
attraction `0.0005`, subdued lines, and disabled global hulls. The Spatial Layout
panel exposes the forces active for the projection, including precise gravity and
centroid controls with Off, repulsion up to 100,000, and zero opacity. Simulation
reports actual steps and CPU duration against a best-effort 8 ms frame budget; one
step can exceed that budget. The Large graph preset reapplies these settings.

Inspect directed neighbors without a window:

```sh
cargo run --release -p hyper-viz --example inspect_dependencies -- \
  target/mathlib/graph.json Mathlib.Topology.Basic
```

Measure the ordinary native canvas independently of the camera tour:

```sh
HYPER_VIZ_SESSION=off cargo run --release -p hyper-viz-bevy \
  --example module_benchmark -- target/mathlib/graph.json Mathlib.Init \
  target/module-benchmark
```

This opens the normal dependency UI with a 200-module scope, saves a canvas PNG,
and measures 180 frames after 60 startup frames. `frames.csv` records each wall
frame duration; the terminal prints median and p95. Rendering clips nodes outside
the viewport. Results depend on the machine, window resolution, and other work.

Current implementation checks and native measurements are recorded in
[the module explorer validation report](benchmarks/module-explorer.md).
