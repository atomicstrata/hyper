# Ecosystem measurements

Run each track **sequentially** on an otherwise idle machine. Preparation, compilation,
package installation and screenshots must not overlap timed workloads. These tools
produce measurements; they do not assign a universal performance ranking.

The first recorded run, its comparison and limitations are in
[`docs/benchmarks/ecosystem-2026-10-03`](../../docs/benchmarks/ecosystem-2026-10-03).
Every adapter checks native identities/memberships before its results are usable.
Lossy model conversions are flagged separately from successful timing.

## Prepare canonical inputs

Requires Python 3.10+ for the orchestration scripts. The dataset generator itself
uses only the standard library. Synthetic inputs are seeded with `20261003` and CC0.

Download Diseasome from the pinned CC-BY-4.0 record:

```sh
mkdir -p target/benchmark/source
curl -L --fail https://zenodo.org/records/21909416/files/diseasome.json -o target/benchmark/source/diseasome.json
curl -L --fail https://api.github.com/repos/leanprover-community/mathlib4/tarball/d13f23b723b8a846827a245b89c10fc7d3f11612 -o target/benchmark/source/mathlib.tar.gz
mkdir -p target/benchmark/source/mathlib
tar -xzf target/benchmark/source/mathlib.tar.gz --strip-components=1 -C target/benchmark/source/mathlib
python3 scripts/mathlib_graph.py target/benchmark/source/mathlib target/benchmark/source/mathlib.json --release v4.34.1 --commit d13f23b723b8a846827a245b89c10fc7d3f11612
```

Then generate the common fixtures:

```sh
python3 scripts/benchmarks/datasets.py
python3 -m unittest discover -s scripts/benchmarks -p 'test_datasets.py'
```

The generated manifest contains counts, provenance and SHA-256 hashes of the
native, HIF, incidence and updated inputs. Verify these against the recorded run's
manifest before combining measurements. Missing scientific/Mathlib source inputs
cause those cases to be omitted; they do not silently produce empty fixtures.

The recorded Diseasome download SHA-256 is
`5b85df24ec67ae8f796478f7559ef238fe0f17c6d63c613d9502335444b54ea1`.
Mathlib's `source_sha256` is the script's digest of sorted source paths and file
bytes, not the archive hash; it is
`dde69137f0279fdb1ec0b40e16dd5252c0a13aa8550c9508e71ea811565347f3`.

## Scientific figure workflows

Use an isolated CPython 3.12 environment and the recorded requirements lock
(`evidence/scientific-requirements.txt` in the report). The direct pins are
HyperNetX 2.4.3, XGI 0.10.2, Hypergraphx 1.8.0, Matplotlib 3.11.2 and Pillow 12.3.0.

```sh
python -m pip install -r docs/benchmarks/ecosystem-2026-10-03/evidence/scientific-requirements.txt
python -m unittest discover -s scripts/benchmarks -p 'test_*.py'
python scripts/benchmarks/scientific.py --validate --output target/benchmark/science/validation
python scripts/benchmarks/scientific.py --run --output target/benchmark/science/results
python scripts/benchmarks/verify_scientific.py
```

The runner starts a fresh worker per case, discards one warmup and records five
attempts. It tests model construction on all inputs and plots correctness,
moderate and Diseasome only. Its 120-second watchdog covers the whole worker.
Construction, artist setup, Agg raster, buffer copy and PNG compression are
separate stages. Model checks are opaque-metadata adapter checks, not tests of
each tool's scientific file format. No figure timing is converted to FPS.

## Hyper conversion and native renderer

```sh
cargo build --release -p hyper-viz --example ecosystem_core
cargo build --release -p hyper-viz-bevy --features benchmark --bin ecosystem-benchmark
python3 scripts/benchmarks/run_local.py core --output target/benchmark/core
python3 scripts/benchmarks/run_local.py native --output target/benchmark/native
python3 scripts/benchmarks/run_local.py native --datasets mathlib --projection star --live-datasets mathlib --output target/benchmark/native-star
python3 scripts/benchmarks/run_local.py native --datasets moderate,mathlib --projection star --live-datasets moderate,mathlib --labels --only-hulls --output target/benchmark/native-interactive
```

These commands use macOS `/usr/bin/time -l` for whole-process peak RSS and require
access to the real desktop GPU. Do not combine software-rendered or headless
substitutions with native results. Set `BENCHMARK_CONDITIONS` to power, thermal,
background-work and display-cadence observations. Native jobs use 120 warmup and
300 measured application frames, with five fresh processes per configuration.
Live modes perform one solver step per frame. Frozen coordinates do not measure
layout quality. The production hull cap is 24 vertices, so large hulls are
approximate. The opt-in feature does not change normal viewer startup.

## Browser incidence renderer

See [browser/README.md](browser/README.md) for installation, pinned Chromium,
Metal verification, untimed validation and camera-motion commands. The recorded
run uses `--job-timeout-ms 90000 --warmup 120 --samples 300 --repetitions 5`.
It keeps all incidences, disables forces and labels, and tests 3d-force-graph,
Cytoscape.js and Sigma. A failed batch has no completed rendering score.

## Desktop competitors

See [desktop/README.md](desktop/README.md) for pinned versions, source adapters,
runtime hashes and limitations. After preparing the isolated upstream project:

```sh
python3 scripts/benchmarks/desktop/run.py hypergodot --executable /path/to/Godot --project /path/to/prepared-hypergodot --output target/benchmark/hypergodot
python3 scripts/benchmarks/desktop/run.py graphia --executable /path/to/Graphia --output target/benchmark/graphia
```

HyperGodot measures five warmed intervals in one process, unlike Hyper's fresh
processes. Graphia measures fresh-process headless import/save including startup,
then validates every saved incidence and JSON metadata attribute. Graphia's
adapter does not set render positions or retain document metadata and its stock
saved graph is directed. Neither metric is interchangeable with browser FPS.

## Aggregate

```sh
python3 scripts/benchmarks/summarize.py
python3 scripts/benchmarks/report.py
```

The report includes all attempts and exclusions. Average FPS is sample count
divided by total interval time; summaries use medians of per-run values and
quantiles. Single-machine exploratory observations need other hardware and human
tasks before they support broad product performance claims.

## Recompute the recorded table without new measurements

Use a fresh, empty restoration directory. This uses only the archived observations:

```sh
python3 scripts/benchmarks/restore_evidence.py --output target/benchmark/archived-replay
python3 scripts/benchmarks/summarize.py --input target/benchmark/archived-replay --output target/benchmark/recomputed-report
python3 scripts/benchmarks/report.py --directory target/benchmark/recomputed-report
```

The recorded JSON, CSV and Markdown were regenerated from the archive and matched
byte-for-byte. `checksums.json` records the report, evidence and current harness
source hashes. Excluded diagnostics remain separate from restored score inputs.
