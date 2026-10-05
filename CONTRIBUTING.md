# Contributing

Thanks for interest in **hyper** — a domain-agnostic hypergraph visualization
stack (scene IR, force layout, optional native 3D Bevy viewer).

## Scope

This repository is **visualization only**:

| In scope | Out of scope |
|---|---|
| Native/HIF interchange, scene IR, projections | Memory engines / claim pipelines |
| Headless layout, dependency queries | Proprietary host business logic |
| Optional Bevy viewer, Python bindings and launcher | Publishing packages without a release decision |

Do not add Atomic Memory Core types, private service URLs, secrets, or claim-
pipeline code here. Host apps own those concerns and should feed this library
plain vertices + hyperedges (or `hypergraph.v1` JSON).

## Development

Requirements: Rust **1.89+** (see `rust-toolchain.toml`; Bevy 0.18 MSRV), a
GPU/windowing stack for the native viewer.
Workspace-wide clippy and documentation also build the Python extension and
require **CPython 3.10+** on PATH. When your system Python is older, activate a
supported virtual environment first or set `PYO3_PYTHON=/path/to/python3.12`.

```bash
cargo test --locked -p hyper-viz
cargo test --locked -p hyper-viz-bevy --lib
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps

# Standard-library exporter and benchmark aggregation checks
PYTHONPATH=scripts:scripts/benchmarks python3 -m unittest \
  scripts/test_mathlib_graph.py scripts/benchmarks/test_datasets.py \
  scripts/benchmarks/test_summarize.py

# Headless example
cargo run -p hyper-viz --example project_scene

# Native viewer (opens a window)
cargo run -- fixtures/sample.json
```

For Python development, use CPython 3.10+ and a virtual environment:

```bash
python -m venv .venv
. .venv/bin/activate # Windows PowerShell: .venv\Scripts\Activate.ps1
python -m pip install maturin==1.9.6 -r crates/hyper-viz-python/requirements-test.txt
maturin develop --locked --manifest-path crates/hyper-viz-python/Cargo.toml
cargo run --locked -p hyper-viz-python --bin stub_gen
git diff --exit-code -- crates/hyper-viz-python/python/hyper_viz/_core.pyi
python -m pytest crates/hyper-viz-python/tests -q
python -m mypy --strict crates/hyper-viz-python/python/hyper_viz
python crates/hyper-viz-python/examples/xgi_interop.py
```

The launcher tests use a substitute executable and can run without a display.
Desktop wheel builds and installed-package integration checks run in
[the Python workflow](.github/workflows/python.yml) on all supported platforms.
See the [release guide](docs/PYTHON_RELEASE.md) for package contents and source
distribution checks. Compile documentation examples when changing public APIs,
and update [API](docs/API.md), [architecture](docs/ARCHITECTURE.md), and the
relevant user guide when behavior changes.

## Pull requests

- Prefer small, focused diffs with a clear Conventional Commit subject.
- Keep the public surface domain-agnostic; put host-specific import quirks in
  quarantined adapters (see `crates/hyper-viz/src/io/legacy_export.rs`).
- Do not publish crates or flip repository visibility from a PR.

## License

By contributing, you agree your contributions are licensed under
**MIT OR Apache-2.0** (see [LICENSE](LICENSE)).
