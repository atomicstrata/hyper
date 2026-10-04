# Hyper comparative benchmark protocol

Proposed experiment, October 2, 2026. The executed October 3 subset, its measurements and remaining coverage gaps are recorded in the [measured comparison](benchmarks/ecosystem-2026-10-03/README.md). This document defines future measurements; it does not report an executed comparison. Existing evidence and the ecosystem table are in [Hyper ecosystem comparison and roadmap](ecosystem-comparison.md).

The objective is to determine which users and workloads Hyper serves well enough to justify adoption, and which roadmap changes improve that outcome. Compare responsiveness, exact membership interpretation, and integration effort alongside raw rendering throughput.

## Baselines and comparison tracks

| Track | Baselines | Controlled question |
|---|---|---|
| Native/group exploration | Hyper, HyperGodot, HGPolyVis, HNX Widget | Can users identify memberships and overlapping groups accurately, and inspect their data? |
| Spatial node-link rendering | Hyper with explicit incidence view, 3d-force-graph, Graphia | How responsive is navigation with the same projected nodes and links? |
| Scientific figure generation | HNX, XGI; optionally HGX/Julia workflows | How quickly can the same small/medium hypergraph become an interpretable export? |
| 2D web exploration | Cytoscape.js and Sigma.js with incidence adapters | What does the simpler browser alternative provide at the same membership workload? |

Comparisons across 2D and 3D should evaluate task accuracy, time, and usability. They should not imply identical GPU rendering work. Similarly, static Matplotlib figure generation is not the same metric as continuously rendering a native window.

Use Windows for a baseline involving the current HGPolyVis binary, or mark it unavailable on the tested OS. Keep that separate from a performance failure. Record native vs browser runtime and exact versions.

## Datasets

Keep a canonical membership list, stable IDs, metadata, dataset license, and SHA-256 for every input. Verify each adapter against those records before timing. HIF files and Hyper JSON are different schemas; renaming keys or accepting an empty deserialization is not valid conversion.

| Workload | Proposed size or source | What it tests |
|---|---|---|
| Correctness fixture | 30 vertices / 15 groups, arity 1–8; overlapping, duplicate-member, isolated and empty cases where allowed | Membership, ID stability, unsupported-input behavior, and dual/incidence interpretation |
| Moderate synthetic | 1k vertices / 2k groups; fixed seed; several arity and overlap distributions | Everyday exploration and layout cost |
| Larger synthetic | 10k vertices / 10k groups; control total memberships separately | Rendering/layout growth without changing graph semantics implicitly |
| High arity | Hold vertex count fixed; sweep maximum arity through 10, 100, 1k and near-total membership | Hull accuracy and projection expansion |
| Real scientific data | Pin suitable [XGI-DATA](https://github.com/xgi-org/xgi-data) or [HGX datasets](https://hgx-team.github.io/hypergraphx-data/) with metadata and license | Evidence beyond generated graphs; preserve documented dataset semantics |
| Mathlib atlas | Existing pinned export: 9,384 vertices; 44,906 total hyperedges; 118,066 memberships | Reproduce the current real-world stress case and investigate large-group hull overhead |
| Incremental updates | Add/remove/change 1% of records with stable IDs and fixed timing | Scene replacement latency, position stability, selection and focus preservation |

For size comparisons, report `(vertices, hyperedges, memberships, maximum arity)` and actual scene nodes, rendered segments, hulls, and triangles. Do not use node count alone. Keep scientific simplices and hyperedges distinct when loading datasets.

The Mathlib atlas includes both import pairs and source-plus-import groups. Preserve both across adapters, and report their counts separately. A comparison using only the pairwise import graph is a different workload. In a bipartite projection the existing atlas has 54,290 scene nodes and 118,066 incidence links; the star-centroid tour has 9,384 scene nodes.

Do not run unrestricted clique expansion on the full atlas: its 8,532-member umbrella group alone yields 36,393,246 pairs before accounting for other groups. Test clique projection on bounded fixtures and label that limit. This pair count is `k*(k-1)/2`, not an observed renderer capacity.

## Measurement modes

Separate these modes rather than combining them into one FPS score:

1. Parse and validate input, then project it; record time and peak memory.
2. Compute layout with a fixed seed, positions and parameter record; report iteration time and time to an explicit convergence criterion.
3. Render frozen common coordinates with basic nodes/segments and identical camera framing, resolution and pixel ratio where possible.
4. Add equivalent group geometry where each tool supports it; record sampling/approximation settings.
5. Run live layout with a stated number of steps per frame.
6. Enable picking, labels, UI panels and metadata inspection; exercise orbit/pan, zoom, click and search.
7. Apply incremental updates and record visual continuity and latency.

For representation-specific layouts, also run a separate “best documented settings” comparison. A common-coordinate render test isolates rendering but does not evaluate layout quality. Fix camera views when comparing hulls on/off; the historical tour changes the view between phases.

Run at least five repetitions after a documented warmup. Randomize tool/mode order, record power/thermal conditions, background load and vsync, and report every run as well as the aggregate. Pin commits, builds, OS, CPU, GPU, RAM, graphics backend, browser, device pixel ratio, window size, labels, line budgets, opacity and hull settings. Keep screenshot readback and video capture outside live timing.

Wall-clock frame intervals, GPU timestamps, layout-step duration, and input-to-feedback latency measure different stages. Name each precisely. Report both percentile frame intervals and average throughput; `1000 / median frame ms` is not average FPS. Include startup stalls, memory, failed loads and omitted objects rather than hiding them in a success score.

## Task evaluation

Use identical target records and ground truth across tools. Counterbalance tool order and provide comparable practice. Proposed pilot: 5–8 participants; use findings to refine the study rather than treating that sample as population-level proof.

| Task | Ground truth and measure | Roadmap decision |
|---|---|---|
| Find a named or attribute-matched vertex | Correct record; elapsed time; query/result latency | Search and metadata support |
| Explain one hyperedge | Exact member set; completeness and false-member count | Hull interpretation and inspector accuracy |
| Identify overlap between groups | Exact intersection; accuracy and elapsed time | Focus, linked tables, complementary 2D view |
| Explain provenance or quantitative attributes | Correct attribute values and source records | Preservation of opaque metadata and attribute styling |
| Track a selected record through updates | Correct identity; selection retention; displacement of unchanged vertices | Stable-ID reload behavior and saved positions |
| Export a selected neighborhood | Correct records, IDs, metadata and memberships after reload | Interoperability and sharing |

Approximate surfaces can exclude genuine members or geometrically enclose nonmembers. Evaluate the membership list and selection highlight as authoritative; record visual ambiguity separately. A plausible-looking screenshot is not correctness evidence.

## Results table to populate

The following is a template, not measured data. `Pending` must remain until a compatible run exists; use `Unsupported` or `Unavailable on tested OS` for those conditions.

| Tool and pinned version | Dataset and representation | Load time | Frozen p50 / p95 | Live p50 / p95 | Input latency p95 | Peak memory | Membership task accuracy | Evidence |
|---|---|---|---|---|---|---|---|---|
| Hyper current HEAD | Mathlib star-centroid, interactive controls enabled | Pending | Pending | Pending | Pending | Pending | Pending | Fresh runs required; historical tour differs |
| Hyper | Canonical moderate fixture, incidence | Pending | Pending | Pending | Pending | Pending | Pending | Pin commit and settings |
| 3d-force-graph | Same projected fixture, same coordinates | Pending | Pending | Pending | Pending | Pending | Pending | Pin browser and rendering settings |
| Graphia | Same projected fixture | Pending | Pending | Pending | Pending | Pending | Pending | Record data adapter and runtime |
| HNX Widget | Canonical moderate hypergraph | Pending | Pending | Pending | Pending | Pending | Pending | Verify documented notebook environment |
| HyperGodot | Canonical moderate hypergraph | Pending | Pending | Pending | Pending | Pending | Pending | Representation differs; evaluate tasks |
| HGPolyVis | Canonical moderate hypergraph | Pending | Pending | Pending | Pending | Pending | Pending | Requires compatible OS; evaluate tasks |

For HNX/XGI plotting, add a separate figure-generation table with parse/layout/draw/export time, peak memory, and task accuracy. Fill PAOHVis temporal rows only after choosing a genuine temporal dataset and shared comparison task.

## Initial acceptance criteria

Proposed targets for a representative moderate workload: p95 interactive frame interval at or below 33.3 ms and p95 input-to-visible-feedback below 100 ms, with correct member inspection and preserved metadata. These are project goals, not guarantees, industry standards, or current results. Agree on hardware and scene settings before using them as release gates.

For the Mathlib stress case, publish its envelope rather than forcing every visual mode into the same target. If hulls must be sampled, selectively updated or hidden, display that limitation and benchmark the resulting mode explicitly. Optimize the measured bottleneck and then repeat the same controlled workload.
