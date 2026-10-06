# Agent guide

Instructions for coding agents asked to run or integrate Hyper. Human readers
should start with [README.md](README.md).

## Run the viewer

Requires [uv](https://docs.astral.sh/uv/getting-started/installation/) on PATH.
No repository checkout and no Rust toolchain:

```sh
uvx --python 3.12 --from hypergraph-viz-viewer hyper
uvx --python 3.12 --from hypergraph-viz-viewer hyper dataset.hif.json
```

`--from` is required. `hypergraph-viz` supplies the Python API and ships no
executable; `hypergraph-viz-viewer` supplies the `hyper` command.

Accepted inputs are [HIF](docs/HIF.md) and native `hypergraph.v1` JSON, detected
automatically. `--projection bipartite|clique|star` and `--watch` are available;
see the [viewer guide](docs/VIEWER.md).

## Check before you run it

- **The viewer opens a native window on the machine running the command.** It is
  not a notebook widget, and a remote host or container cannot display on the
  user's laptop. Without a desktop session and working graphics drivers, report
  that constraint rather than retrying the launch.
- Prebuilt wheels cover Linux x86_64 (glibc 2.28+), macOS arm64/x86_64 (11.0+),
  and Windows x86_64. Other platforms build from source and need Rust 1.89+.
- `hyper --help` and `hyper --version` parse arguments before opening a window,
  so they are safe to run headless to confirm the install.

## Use the Python API

For interchange and validation, which need no desktop:

```sh
uv add hypergraph-viz
```

Add the viewer only when a window is actually wanted, with
`uv add "hypergraph-viz[viewer]"`. The API alone has no Bevy dependency. See the
[Python and HIF guide](docs/HIF.md).

## Work in this checkout

```sh
cargo run --locked --release -- fixtures/sample.json   # native viewer
cargo run --locked -p hyper-viz --example project_scene # headless, no window
```

The first release build can take several minutes.
[CONTRIBUTING.md](CONTRIBUTING.md) is authoritative for the test, clippy, fmt,
and Python checks a change must pass, and for what belongs in this repository.
