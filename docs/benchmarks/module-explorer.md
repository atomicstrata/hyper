# Module explorer validation — 2026-10-03

Validated in the native managed worktree on Apple M5 Pro, 24 GiB RAM,
macOS 27.0, Metal. No dependency versions or the declared Rust 1.89 floor
were changed. Commands below use the installed Rust 1.96 toolchain compatible
with the existing lockfile. The Rust 1.89 CI configuration was not exercised
locally; CI remains responsible for that configuration.

## Correctness

The final debug workspace suite passes 72 core tests, 68 viewer tests, and
3 doctests. The exporter passes all 4 Python tests. Formatting, clippy for all
workspace targets with warnings denied, and a release workspace build pass.

```sh
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 \
  cargo +1.96.0 test --workspace --offline
CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 \
  cargo +1.96.0 clippy --workspace --all-targets --offline -- -D warnings
cargo +1.96.0 fmt --all -- --check
cargo +1.96.0 build --release --workspace --offline
python3 -m unittest discover -s scripts -p 'test_*.py'
git diff --check
```

Disabling debug information reduced local build storage; test assertions remained
active. The upstream `block` crate emits its existing future compatibility notice.

Coverage includes validated import direction, malformed metadata, duplicate edges,
cycles, self-loops, independent depth traversal, filtering through intermediate
modules, deterministic truncation, explicit path inclusion, stable layout rows,
exact selection through the actual egui panel, session normalization and scene
reordering, mode changes, spatial scope transfer, visible-only picking, and frozen
hull mesh asset events. Regression tests failed before their fixes. Temporarily
restoring unconditional hull rebuilds and filtered-list row assignment each made
the corresponding regression fail again.

Independent final review found no remaining important findings. Review fixes
cover stale spatial selections during dependency-mode reloads, chosen-mode
preservation, filtered expansion seeds, fresh-session presets, displayed path
steps beyond initial depth, path expansion order, and cinematic opacity.

## Actual Mathlib scope

The pinned v4.34.1 graph contains 9,384 vertices, 37,356 directed imports, and
7,550 dependency groups. `Mathlib.Topology.Basic` has exactly three direct imports
and four direct dependents. The default canvas displays eight modules including
the center and twelve induced directed edges. The 8,532-member umbrella group
cannot enlarge this directed scope.

`Mathlib.Init` has 35 direct imports and 214 direct dependents. Including the
center yields 250 reachable modules: the default 200-module budget reports
50 omitted modules with all categories included.

## Native measurement

The normal native dependency UI was measured independently of the scripted tour.
The release example uses a 1920 × 1080 window, scale factor 1, normal presentation,
and continuous updates. Sessions are disabled. It discards 60 startup frames,
records 180 wall-clock frame intervals, and captures one screenshot at frame 90.
The readback is included in those samples. No synthetic input is injected during
the measured frames.

```sh
cargo +1.96.0 build --release -p hyper-viz-bevy \
  --example module_benchmark --offline
HYPER_VIZ_SESSION=off target/release/examples/module_benchmark \
  target/mathlib/graph.json Mathlib.Init target/module-validation/final-200
```

| Scope | Budget omissions | Median frame | p95 frame |
|---|---:|---:|---:|
| Mathlib.Init, 200 displayed modules | 50 | 8.312 ms | 9.635 ms |

This run satisfies the 33.3 ms/frame target under these steady-state conditions.
It does not establish a hardware-independent guarantee or measure interactions
such as searching, changing scope, and zooming. The canvas clips offscreen
geometry and reduces labels when zoomed out; hover details and neighbor lists
preserve access to full module IDs.

Screenshots were inspected for both the eight-module Topology scope and the
200-module Init scope. A separate native run of `fixtures/sample.json` in Auto
mode confirmed that the generic coauthorship graph opens in Spatial mode with
its nodes, labels, lines, and hulls. That Spatial validation wrote its report and
screenshot but required process termination after requesting AppExit; clean
native Spatial shutdown is not established by this check.

Generated PNGs and raw frame CSVs remain under ignored `target/module-validation/`.
The example reproduces them rather than committing machine-specific build output.

## Execution notes

Scene metadata and optional hubs were committed first, followed by the pure
Rust dependency index. Native state, canvas, session restoration, mode gates,
and spatial visibility share interfaces and were delivered together after the
individual regression checks. Final debug verification supplements the earlier
release-profile checks. Public scene struct literals now require the additive
attribute fields and an optional hub index; legacy JSON remains defaulted and
session format version stays `hyperviz.session.v1`.
