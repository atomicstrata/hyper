# Browser incidence rendering benchmark

This harness measures pinned browser baselines using canonical native Hyper and bipartite scene inputs produced by `scripts/benchmarks/datasets.py`. It validates every scene against exact native memberships, all IDs (including isolated vertices and empty-edge hubs), fixed coordinates, and each live library's own graph structure before measuring. It validates the shared `*.updated.json`/`*.updated.incidence.json` fixtures after each update. It hashes raw input bytes, lockfile and topology.

Install with `npm ci --prefix scripts/benchmarks/browser`. Node 24.16.0, Playwright 1.63.0 and exact npm dependency versions are recorded in every evidence JSON. The committed lockfile pins transitive dependencies and registry integrity hashes.

Run untimed validation:

```sh
node scripts/benchmarks/browser/run.mjs --validate-only --executable '/path/to/chrome'
```

This cached browser was verified as Chrome for Testing 153.0.8010.12, Chromium revision `971a7443b0c9b0a9b2860529b33331b76077ec62`. Use an explicit installed browser path, or install Playwright's pinned browser with `npx --prefix scripts/benchmarks/browser playwright install chromium` and omit `--executable`. Evidence includes the *actual* CDP browser version/revision. Do not silently substitute another browser and combine its results.

Timed run, after the coordinator has finished other resource-intensive workloads:

```sh
BENCHMARK_CONDITIONS='AC power; thermal condition; background jobs; display refresh rate' \
node scripts/benchmarks/browser/run.mjs --datasets moderate,large,high-arity-10,high-arity-100,high-arity-1000,scientific,mathlib --modes camera-motion --job-timeout-ms 90000 --executable '/path/to/chrome'
```

The main example selects camera motion only across seven datasets (105 runs). Both modes remain the default. A per-job 180-second watchdog (`--job-timeout-ms`) closes failed/stalled pages, saves a failed row with its stage/reason and continues, preserving partial evidence. The default is a headed 1920×1080 viewport with DPR 1, browser default vsync, five repetitions, 120 warmup rAF callbacks and 300 sample callbacks for both static and camera-motion modes. A seeded Fisher–Yates shuffle randomizes tool/dataset/mode order within every repetition. `--datasets`, `--tools`, `--modes static,camera-motion`, `--seed`, `--output`, `--executable` can restrict or place runs. `--headless` is explicit and recorded. `--swiftshader` requests software rendering explicitly and records the true backend; do not publish it as hardware performance.

At a 60 Hz rAF cadence, 300 samples span about five seconds. No artificial sleep enforces a fixed display cadence. Static scenes also request redraw every callback, so Sigma and Cytoscape do not receive idle-scene scores. 3d-force-graph's internal animation is paused and its public Three.js renderer draws directly; force layout is disabled and fx/fy/fz remain canonical. Sigma's public scheduleRender schedules a cached redraw without reprocessing unchanged graph data while Cytoscape's public forceRender *schedules* a redraw. CPU command duration therefore measures different submission paths and must remain distinct from rAF wall intervals. A two-rAF scripted-update feedback surrogate is separately recorded; it is not human input latency or compositor presentation. Browser startup/evaluate load wall time includes fixture transfer and driver roundtrip. GPU timestamps, GPU completion, layout convergence, peak memory and human task accuracy are unmeasured.

All vertices, edge hubs and incidence links are supplied, without sampling or line budgets. Settings are nodeRelSize 2, nodeResolution 8, nodeVal 1, linkWidth 0 (thin native WebGL lines), opacity 0.35 for 3d-force-graph, and basic 2D circles/lines, labels disabled, 1 px line width and opacity 0.35 for Cytoscape/Sigma. Exact settings and framing are saved per run. Cyto uses negated y only for its screen coordinate convention and converts it back for canonical verification. Sigma and Cytoscape show XY projections with y upward; 3D uses perspective and orbit. They share fixed source coordinates but cannot be called equivalent GPU workloads or camera projections. Sigma's native normalization/pixel node sizing and 3D sphere world sizing also differ. Report their tracks separately.

Validation JSON and screenshots go under ignored `target/benchmark/browser`. Screenshot readback happens outside timing and after the canonical update, once per tool/dataset (first static run, or first motion run when static is omitted). Traces contain all individual CPU command durations and rAF callback timestamps/intervals; summaries report p50/p95/p99, mean, max and average throughput calculated as `sample count / sum(wall intervals)`. Do not use inverse median as average FPS. The instrumentation records every actual WebGL renderer, extension availability, and Chromium CDP GPU information. Cytoscape uses Canvas2D: browser GPU device metadata is not evidence that its graph geometry uses WebGL.

## HNX Widget capability probe

Official [HNX Widget README](https://github.com/pnnl/hypernetx-widget) describes notebook installation and a bipartite view. Source revision `58e795d6a362dcd3bbd9ccff462052956a40f4f8`, package version `0.1.1-beta.0` declares React 16. The [view source](https://github.com/pnnl/hypernetx-widget/blob/58e795d6a362dcd3bbd9ccff462052956a40f4f8/src/HypernetxWidgetView.js) accepts `pos={uid:[x,y]}` and assigns those positions to internal vertex parents, but not hyperedge hubs. It runs charge/link/center/collide/bound forces on hubs, and has no public frozen shared-hub position/stop simulation API. This prevents a matched frozen bipartite comparison without modifying its rendering implementation. The Python wrapper derives vertices from the union of memberships, omitting explicit isolated vertices. A standalone npm package availability request timed out; that is recorded as an installation probe failure, not a renderer capacity result. A notebook native group-exploration smoke test may be run separately; this harness marks the matched frozen incidence track unsupported rather than assigning a score.

Primary API references used: [3d-force-graph](https://github.com/vasturiano/3d-force-graph), [Cytoscape.js](https://js.cytoscape.org/), [Sigma lifecycle](https://www.sigmajs.org/docs/advanced/lifecycle/). Local installed API/source was also inspected for render submission and RGBA handling.
