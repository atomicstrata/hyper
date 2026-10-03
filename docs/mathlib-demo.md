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

## Full spatial overview

The interactive default remains the spatial viewer, with the whole graph visible.
Launch a fresh full-graph view without restoring an older session's isolation:

```sh
HYPER_VIZ_SESSION=off target/release/hyper --projection star \
  --view spatial target/mathlib/graph.json
```

Large star-projected import graphs start in the **Normalized** model, with
connectivity-based positions and 64 bounded refinement steps. All 9,384 vertices,
37,356 import lines and 7,550 dependency-group hulls remain in the scene.
Layout is then paused for inspection. Press Space or enable Running to continue.
Generic graphs and the scripted tour retain Legacy forces; the optional dependency
explorer stays unused unless explicitly requested.

The initializer iterates a sparse normalized incidence operator, removes each
component's stationary mode and uses three approximate spectral coordinates.
It never reads subject labels or kinds. Median-based axis scaling, smooth radial
compression and small stable-ID jitter keep localized branches from hiding the
main structure and prevent coincident points. Disconnected components are placed
separately. These coordinates suggest connectivity patterns; they do not establish
mathematical topology or certify communities. The positions are reproducible for
stable IDs, with small floating-point differences possible across platforms.

The Layout panel opens by default. **Full graph overview** clears focus and
attention, restores every line/hull, applies the Normalized preset, pauses and
frames the graph while preserving positions. **Rebuild structural layout**
recomputes connectivity positions using the chosen weights, performs 64 actual
refinement steps, pauses and frames the result. It reports total rebuild time.
**Reinitialize neutral positions** resets to stable-ID positions for force-model
comparisons. Saved preferences restore when sessions are enabled; use the command
above for the new defaults.

| Control | Range | Effect |
|---|---|---|
| Attraction model | Legacy / Normalized / LinLog | Legacy applies the previous forces; Normalized uses linear weighted attraction; LinLog uses logarithmic distance growth |
| Pair / set attraction | 0.000001–10, plus Off | Independent strength for arity-2 relations and larger sets |
| Hub normalization | 0–1 | Pair weight divides by maximum endpoint degree raised to this power; set weight uses maximum member set-incidence count |
| Large-set normalization | 0–1 | Divides set weight by (arity−1) raised to this power |
| Derived-set influence | 0–1 | Additional multiplier for exported `dependency_group` sets; zero disables their force but retains their geometry |
| LinLog scale | 0.01–100,000 | Distance scale for logarithmic attraction |
| Repulsion | 0.01–100,000,000, plus Off | Spreads vertices |
| Gravity | 0.00000001–1, plus Off | Pulls vertices toward the origin |
| Max movement / step | 0.01–1,000 | Limits displacement in Normalized and LinLog models |
| Hub line / large-set hull fading | 0–1 | Divides resting opacity by endpoint degree / set arity raised to this power; zero restores equal opacity |
| Legacy centroid / spring attraction | 0.00000001–1, plus Off | Previous projection-dependent forces |
| Legacy spring length | 0.01–10,000 | Applies when the projection has spring links |
| Time step | 0.001–2 | Integration step |
| Velocity retention | 0–0.9999 | Lower values damp movement more quickly |
| Iterations/frame | 1–100 | Upper limit with a best-effort 8 ms CPU budget |
| Vertex size | 0.05–100 | Scales vertex markers |
| Hull / line opacity | 0–0.6 / 0–1 | Logarithmic sliders expose small opacity changes |

The preset uses pair strength `0.1`, set strength `1`, full hub/arity normalization,
derived-set influence `0.1`, repulsion `500`, gravity `0.00001` and vertex size `8`.
Change weights, then Rebuild to inspect their structural effect; Running applies
force changes to current positions. Lower repulsion or increase pair/set strength
to tighten local groups; reduce gravity to avoid pulling all components together.

Line budget **0** submits every import line. The preset uses line opacity `0.06`
and hull opacity `0.001`, with hub fading `0.5`, arity fading `0.3` and wireframes
disabled. Fading does not remove relationships, and hover/selection bypasses the
extra fading. Set both fading controls to zero for equal visual weight. The huge
umbrella remains in both connectivity and rendering. Hull surfaces still sample
at most 24 members; opacity and occlusion mean every overlapping object cannot
be individually legible from one camera angle. Orbit and zoom to inspect branches.

Compare models headlessly on the same Mathlib graph and stable-ID neutral seeds:

```sh
cargo run --release -p hyper-viz --example compare_layouts -- \
  target/mathlib/graph.json target/layout-comparison 800
```

The example saves all positions and reports internal Mathlib import-distance
and same-subject distance ratios. Subject categories are used only to evaluate
results, never to generate structural coordinates. Ratios are diagnostic, not a
community classification. [Native evidence and limitations](benchmarks/module-explorer.md).

## Explore imports and dependents

The earlier dependency explorer remains available as an explicit optional view.
Start at an exact module:

```sh
HYPER_VIZ_SESSION=off target/release/hyper --projection star \
  --view dependencies --module Mathlib.Topology.Basic target/mathlib/graph.json
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
graph without imports. The default `--view auto` retains Spatial for all graphs.

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
