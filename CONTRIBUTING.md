# Contributing

Thanks for interest in **hyper** — a domain-agnostic hypergraph visualization
stack (scene IR, force layout, optional native 3D Bevy viewer).

## Scope

This repository is **visualization only**:

| In scope | Out of scope |
|---|---|
| `hypergraph.v1` schema, scene IR, projections | Memory engines / claim pipelines |
| Headless 3D force layout | Proprietary host business logic |
| Optional Bevy + egui viewer | Publishing crates without a release decision |

Do not add Atomic Memory Core types, private service URLs, secrets, or claim-
pipeline code here. Host apps own those concerns and should feed this library
plain vertices + hyperedges (or `hypergraph.v1` JSON).

## Development

Requirements: Rust **1.89+** (see `rust-toolchain.toml`; Bevy 0.18 MSRV), a
GPU/windowing stack for the native viewer.

```bash
cargo test -p hyper-viz
cargo test -p hyper-viz-bevy --lib
cargo clippy -p hyper-viz -p hyper-viz-bevy --all-targets -- -D warnings
cargo fmt --all -- --check

# Headless example
cargo run -p hyper-viz --example project_scene

# Native viewer (opens a window)
cargo run -- fixtures/sample.json
```

## Pull requests

- Prefer small, focused diffs with a clear Conventional Commit subject.
- Keep the public surface domain-agnostic; put host-specific import quirks in
  quarantined adapters (see `crates/hyper-viz/src/io/legacy_export.rs`).
- Do not publish crates or flip repository visibility from a PR.

## License

By contributing, you agree your contributions are licensed under
**MIT OR Apache-2.0** (see [LICENSE](LICENSE)).
