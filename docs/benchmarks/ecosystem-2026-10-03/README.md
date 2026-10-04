# Measured ecosystem comparison — October 3, 2026

Exploratory results on an Apple M5 Pro / 24 GiB Mac: 445 measured attempts across 89 configurations. Five attempts per case unless noted. The same graph records and seeded coordinates are used through explicit adapters. Scientific figures, 2D browser views and 3D native rendering have different work and are separate tracks.

Read the [method and limitations](methodology.md), [environment](environment.json), [complete CSV](summary.csv), [structured summary](summary.json) and [raw evidence](evidence/). The [harness guide](../../../scripts/benchmarks/README.md) provides pinned dependencies and rerun commands.

## Findings to use for roadmap decisions

Model fidelity is the clearest differentiator in this run: exact group identity, isolated nodes and empty groups need explicit checks at adapter boundaries. Rendering results are conditional on completion. Hyper had intermittent whole-job stalls, including one on a small fixture; their cause remains unresolved. The source audit found a quadratic hub-status lookup worth profiling, but it has not isolated the cause of those stalls.

Sigma camera measurements use its public cached redraw path. Earlier full-refresh camera measurements were discarded and replaced; the diagnostic evidence is retained separately. Comparisons below keep representations and timing boundaries explicit.

## Scientific figure workflows

Times exclude the separately measured top-level tool import, file I/O and JSON parsing. Setup creates the artists and includes any lazy drawing-module initialization; raster draws the Agg canvas. PNG compression is a separate stage. A 120-second timeout covers the whole worker. HGX uses its stock 12-round smoothing, HNX rubber-band outlines, XGI convex hulls.

| Dataset | Tool | Completed / attempts | Exact model records | Construct ms | Artist setup ms | Raster ms | PNG ms | Worker peak MiB |
|---|---|---|---|---|---|---|---|---|
| moderate | hypergraphx | 0/5 | no / unavailable | — | — | — | — | — |
| moderate | hypernetx | 5/5 | yes | 159.92 | 2,717.87 | 1,089.55 | 28.12 | 242.42 |
| moderate | xgi | 5/5 | yes | 4.71 | 214.57 | 452.82 | 37.70 | 144.00 |
| scientific | hypergraphx | 5/5 | no / unavailable | 1.09 | 13,764.90 | 396.79 | 40.78 | 181.56 |
| scientific | hypernetx | 5/5 | yes | 27.49 | 500.68 | 154.87 | 35.97 | 228.19 |
| scientific | xgi | 5/5 | yes | 1.37 | 23.96 | 28.11 | 33.24 | 132.25 |

XGI preserves all model records in every tested input. HNX omits isolated vertices and empty groups. HGX identifies groups by member tuple and merges parallel groups: Diseasome becomes 481 groups instead of 903, and the arity-1,000 fixture becomes one group instead of 200. Its lossy timings cannot rank equivalent work. Original records stored in opaque metadata do not repair those native identities.

## Browser incidence rendering

All incidences are rendered, forces and labels are off, and the camera moves. Values describe rAF callback cadence; they are not GPU durations or verified presented frames. Average callbacks/s uses total elapsed intervals. The p95 range spans per-run p95 values. “Completed” requires sampling and the shared update to finish within the 90-second job budget.

| Dataset | Tool | Completed / attempts | p50 ms | p95 ms | p95 run range ms | Avg callbacks/s | Update command ms | Failed stage |
|---|---|---|---|---|---|---|---|---|
| high-arity-10 | 3d-force-graph | 5/5 | 8.30 | 8.50 | 8.40–8.70 | 120.00 | 0.80 | — |
| high-arity-10 | cytoscape | 5/5 | 8.30 | 9.10 | 9.00–16.70 | 120.00 | 45.70 | — |
| high-arity-10 | sigma | 5/5 | 16.70 | 17.50 | 17.50–17.50 | 60.00 | 4.10 | — |
| high-arity-100 | 3d-force-graph | 5/5 | 41.70 | 50.00 | 41.80–50.40 | 24.71 | 5.80 | — |
| high-arity-100 | cytoscape | 5/5 | 8.30 | 9.20 | 9.10–9.20 | 120.00 | 2,450.00 | — |
| high-arity-100 | sigma | 5/5 | 16.70 | 17.50 | 17.40–17.50 | 60.00 | 13.90 | — |
| high-arity-1000 | 3d-force-graph | 0/5 | — | — | —–— | — | — | incremental update, timed sampling |
| high-arity-1000 | cytoscape | 0/5 | — | — | —–— | — | — | timed sampling |
| high-arity-1000 | sigma | 5/5 | 16.70 | 50.00 | 50.00–50.10 | 43.16 | 167.80 | — |
| large | 3d-force-graph | 2/5 | 191.65 | 203.62 | 201.33–205.91 | 5.22 | 48.35 | incremental update |
| large | cytoscape | 4/5 | 8.30 | 9.30 | 9.21–9.30 | 120.02 | 51,638.90 | incremental update |
| large | sigma | 5/5 | 16.70 | 17.50 | 17.40–17.50 | 60.00 | 1,079.80 | — |
| mathlib | 3d-force-graph | 5/5 | 133.30 | 150.00 | 134.00–191.32 | 7.37 | 73.30 | — |
| mathlib | cytoscape | 0/5 | — | — | —–— | — | — | incremental update, timed sampling |
| mathlib | sigma | 5/5 | 16.70 | 17.50 | 17.40–17.60 | 60.00 | 13,242.90 | — |
| moderate | 3d-force-graph | 5/5 | 25.10 | 33.40 | 33.40–34.11 | 35.40 | 4.20 | — |
| moderate | cytoscape | 5/5 | 8.30 | 9.21 | 9.20–10.00 | 120.00 | 1,342.60 | — |
| moderate | sigma | 5/5 | 16.70 | 17.50 | 17.40–17.60 | 60.00 | 42.20 | — |
| scientific | 3d-force-graph | 5/5 | 8.30 | 8.40 | 8.40–8.70 | 120.00 | 0.80 | — |
| scientific | cytoscape | 5/5 | 8.30 | 9.20 | 9.10–10.00 | 120.00 | 37.50 | — |
| scientific | sigma | 5/5 | 16.70 | 17.50 | 17.40–17.50 | 60.00 | 7.50 | — |

Sigma uses WebGL 2D, Cytoscape uses Canvas2D, and 3d-force-graph uses perspective spheres and WebGL. The corrected Sigma block and original Cytoscape block recorded different small-case callback cadences (roughly 60 versus 120 callbacks/s); display refresh was not locked, so their rate ratio cannot rank rendering efficiency. No link sampling is applied. Update command timings include different adapter commit strategies and sometimes unchanged-record work; see the method. A failed batch has no completed rendering score; its timeout is not a single-frame measurement.

## Hyper native renderer

Physical 1920 × 1080, AutoVsync, 120 warmup and 300 recorded application frames. HUD and picking remain installed. Basic modes disable labels; `native-interactive` enables the default Capped policy (count threshold 250, with selected/hovered exemptions). Frozen modes use fixed coordinates; live modes perform one layout step per frame. All lines are enabled. Hulls cap input at 24 vertices and therefore approximate large groups.

| Track / dataset | Mode | Completed / attempts | p50 app interval ms | p95 app interval ms | p95 run range ms | Avg app frames/s | Update commit ms | Next app interval ms | Peak MiB |
|---|---|---|---|---|---|---|---|---|---|
| native / high-arity-10 | frozen | 4/5 | 8.34 | 8.99 | 8.61–17.45 | 119.83 | 0.24 | 8.40 | 235.17 |
| native / high-arity-10 | frozen+hulls | 5/5 | 8.67 | 17.07 | 16.94–26.34 | 64.62 | 0.28 | 16.41 | 260.62 |
| native / high-arity-100 | frozen | 5/5 | 8.32 | 9.53 | 8.70–17.02 | 117.86 | 0.87 | 8.28 | 255.80 |
| native / high-arity-100 | frozen+hulls | 5/5 | 16.72 | 33.51 | 25.30–33.73 | 49.45 | 0.58 | 42.72 | 271.91 |
| native / high-arity-1000 | frozen | 5/5 | 12.68 | 17.11 | 11.72–20.55 | 72.00 | 6.16 | 18.40 | 411.09 |
| native / high-arity-1000 | frozen+hulls | 5/5 | 33.35 | 41.78 | 34.80–53.86 | 29.78 | 8.97 | 56.79 | 427.14 |
| native / large | frozen | 4/5 | 61.37 | 68.10 | 66.62–82.65 | 16.35 | 5.77 | 62.77 | 700.15 |
| native / large | frozen+hulls | 4/5 | 270.73 | 375.32 | 347.01–455.02 | 3.52 | 6.91 | 302.92 | 1,006.70 |
| native / mathlib | frozen | 0/5 | — | — | —–— | — | — | — | — |
| native / mathlib | frozen+hulls | 0/5 | — | — | —–— | — | — | — | — |
| native / moderate | frozen | 5/5 | 8.31 | 9.68 | 8.73–17.10 | 117.15 | 0.92 | 8.48 | 281.80 |
| native / moderate | frozen+hulls | 5/5 | 49.98 | 62.08 | 51.96–74.23 | 19.57 | 0.79 | 58.94 | 358.38 |
| native / moderate | live | 5/5 | 16.66 | 17.01 | 9.09–17.23 | 60.00 | 0.91 | 16.76 | 287.14 |
| native / moderate | live+hulls | 3/5 | 21.91 | 37.07 | 34.94–43.49 | 41.35 | 0.87 | 25.38 | 384.80 |
| native / scientific | frozen | 5/5 | 8.41 | 9.08 | 8.66–17.76 | 119.37 | 0.36 | 8.50 | 237.25 |
| native / scientific | frozen+hulls | 5/5 | 8.33 | 9.16 | 8.70–17.22 | 119.60 | 0.39 | 8.54 | 257.50 |
| native-interactive / mathlib | frozen+hulls | 5/5 | 162.18 | 219.91 | 191.63–257.63 | 5.93 | 10.35 | 195.36 | 880.30 |
| native-interactive / mathlib | live+hulls | 5/5 | 97.19 | 171.71 | 166.70–176.77 | 9.08 | 9.24 | 97.10 | 935.11 |
| native-interactive / moderate | frozen+hulls | 5/5 | 50.04 | 64.93 | 53.90–95.84 | 19.05 | 0.37 | 50.90 | 298.53 |
| native-interactive / moderate | live+hulls | 5/5 | 18.00 | 30.84 | 30.10–37.94 | 50.06 | 0.33 | 20.60 | 321.88 |
| native-star / mathlib | frozen | 5/5 | 18.16 | 25.95 | 25.78–33.88 | 47.75 | 9.55 | 31.53 | 674.31 |
| native-star / mathlib | frozen+hulls | 5/5 | 187.35 | 216.60 | 174.74–253.40 | 5.39 | 12.26 | 205.94 | 746.03 |
| native-star / mathlib | live | 5/5 | 21.78 | 24.80 | 22.54–27.75 | 45.10 | 7.62 | 42.01 | 666.34 |
| native-star / mathlib | live+hulls | 4/5 | 100.76 | 179.87 | 173.58–220.51 | 8.85 | 9.59 | 101.56 | 1,057.62 |

`native` is bipartite incidence rendering; `native-star` and `native-interactive` use group/star rendering. Star draws pair lines for dyadic groups and hulls for larger groups; its scene has no spring links. Star without hulls is an ablation that hides larger groups, not a full-group rendering score. Update commit and the next application interval exclude file-watch polling and parse/I/O; they are not input-to-visible latency. Native and browser settings, geometry and vsync differ, so these tables do not establish a native-versus-browser speed ratio.

## Hyper parsing, conversion and geometry

Fresh release processes, one discarded warmup and five measured runs. HIF schema validation is offline. Solver timings use the bipartite scene; hull timings use the production 24-vertex cap. Worker RSS includes multiple retained documents for correctness checks.

| Dataset | Native parse ms | HIF parse / validate ms | HIF conversion ms | Projection ms | All hulls CPU ms | Layout step p50 ms | Peak MiB |
|---|---|---|---|---|---|---|---|
| high-arity-10 | 1.13 | 3.87 | 3.36 | 0.15 | 1.28 | 0.90 | 34.56 |
| high-arity-100 | 1.48 | 13.00 | 16.00 | 0.61 | 37.93 | 1.17 | 91.78 |
| high-arity-1000 | 4.78 | 104.58 | 147.10 | 4.32 | 44.92 | 1.76 | 677.53 |
| large | 14.19 | 60.41 | 85.06 | 4.09 | 78.29 | 31.90 | 423.45 |
| mathlib | 36.26 | 141.10 | 228.00 | 15.37 | 10.77 | 97.66 | 1,020.78 |
| moderate | 2.12 | 11.52 | 12.63 | 0.63 | 15.59 | 3.53 | 80.77 |
| scientific | 0.77 | 3.08 | 2.42 | 0.19 | 0.11 | 2.06 | 30.08 |

## Desktop alternatives

Graphia measures stock headless import/save including process startup, then independently checks saved memberships and node attributes. The adapter retains incidence topology and arbitrary `attrs`, but omits document metadata and top-level native label/kind/status/weight fields; XYZ are attributes, and Graphia saves this graph as directed. These are import workflow timings, not rendering FPS.

| Dataset | Tool | Completed / attempts | Import + save ms | Exact incidence records | Peak MiB |
|---|---|---|---|---|---|
| mathlib | Graphia | 5/5 | 1,490.57 | yes | 656.16 |
| moderate | Graphia | 5/5 | 390.86 | yes | 130.73 |
| scientific | Graphia | 5/5 | 332.84 | yes | 106.38 |

HyperGodot measures its production 2D group drawing with camera motion, fixed XY coordinates and stock labels/UI. It disables vsync and uses different node/hull geometry. Its five repetitions share one process, with a new warmup each. A disposable source copy removes a missing unused autoload. Original IDs are mapped to safe tokens/line-order group IDs; weights and group categories are normalized for this fixture. Stock HiDPI is enabled and the physical render texture is verified at 1920 × 1080; earlier clamped-window timings are excluded.

| Dataset | Tool | Completed / attempts | p50 ms | p95 ms | Avg frames/s | Whole-session peak MiB | Render target pixels |
|---|---|---|---|---|---|---|---|
| moderate | HyperGodot | 5/5 | 8.33 | 10.37 | 118.82 | 336.55 | [1920.0, 1080.0] |

## Coverage and remaining experiments

| Alternative | Measured scope / reason for no comparable score |
|---|---|
| HyperNetX, XGI, Hypergraphx | Model construction and scientific figure workflow; no GUI FPS claim |
| 3d-force-graph, Cytoscape.js, Sigma | Frozen full-incidence camera motion and scripted update |
| HyperGodot | Native 2D group drawing on the moderate dataset |
| Graphia | Headless incidence import/save; common render coordinates not established |
| HNX Widget | Installation and dictionary-input smoke test; public position API does not freeze all hubs; isolated vertices omitted |
| HGPolyVis | Supplied Windows-only executable; requires a Windows run |
| SimpleHypergraphs.jl | Julia runtime verified; package timings not collected |
| PAOHVis | Needs a temporal dataset and temporal exploration task |
| Gephi | Outside this automated render harness; requires a separate plugin/task comparison |
| egui_graphs | Embedding framework reference; no matched application benchmark |

Human task accuracy, dense-diagram readability, layout convergence, GPU completion, input-to-visible latency and other machines remain unmeasured. Metadata retention does not establish that a viewer exposes or filters it.

## Roadmap implications

1. **Interoperability:** keep original HIF documents and stable group identities through scientific workflows. The identity-loss cases demonstrate why an incidence adapter needs correctness gates before timing.
2. **Inspection and filtering:** expose provenance, arbitrary attributes and membership before expanding visual effects. Rendering every relation does not establish readability or a successful scientific task.
3. **Responsiveness:** profile the confirmed quadratic hub-status scan in `animation::node_motion` first; the existing `GraphLayout::hub_status` cache offers a direct replacement. Then use the frozen/live split and update traces to separate solver, hull rebuild/upload and redraw costs. Add a controlled input-to-visible study before advertising latency; keep hull approximation and line budgets explicit.
4. **Installation:** ship verified wheels and a clear separately installed viewer path. The widget and desktop packaging probes show that dependency/version friction can block evaluation before performance matters.

These priorities are interpretations of this exploratory run, not proof of adoption or a universal winner.
