# HIF and Python implementation record

Approved design: Rust-owned HIF validation and conversion; PyO3 bindings with generated stubs; separate native viewer process. Preserve full documents; reject unsupported viewer semantics. No publication.

1. Add integration tests for faithful JSON round trips, mixed integer/string identities, isolated nodes/empty edges, rich incidence preservation, duplicate rejection, weight overflow, native export, and format detection. Run `cargo test -p hyper-viz --test hif` red, implement the adapter, rerun green.
2. Vendor schema at HIF-org/HIF-standard b691a3d2ec32100c0229ebe1151e9afad015c356 and upstream license. Use a compiled offline validator; typed IDs and opaque JSON attributes. Implement `parse_hif`, `load_hif`, `serialize_hif`, `save_hif`, and conversions.
3. Add `InputFormat` and explicit loader APIs. Route CLI and file watcher through identical selection; retain native/legacy compatibility. Validate with core and Bevy tests.
4. Add Python contract tests before bindings/launcher implementation. Build the Rust extension with maturin, generate stubs, run Python tests with pinned XGI; check process ownership and cleanup using real helper processes.
5. Add platform wheel CI, clean-install tests, stub drift checks, and Rust/Python/Julia examples. Run fmt/clippy/tests, inspect final diff and docs; keep research files intact.

Implementation decisions: Python CPython >=3.10, abi3; package `hyper_viz`, extension `_core`. Viewer IDs use `s:`/`i:` prefixes. HIF document export is faithful; conversion export is deliberately native IDs. Metadata display conventions apply only to native-to-HIF export, never inferred from scientific attributes. Native export rejects attribute collisions instead of losing values.

## Verification and review

Implemented all five steps. Rust 1.89: 61 core unit tests, 9 HIF integration tests, 52 Bevy unit tests, and 3 doc tests pass (125 total). Core and Bevy tests also run with Cargo offline. All-target Clippy (including CLI and bindings), formatting, and diff whitespace checks pass. Python: 10 installed-wheel tests including XGI 0.10.2 roundtrip and real process lifecycle; strict package and consumer typing pass. Generated stubs regenerate identically.

macOS arm64 and Intel ABI3 wheels built; arm64 clean installation and extreme numeric-ID conversion verified. Wheel checks verify native extension uniqueness, generated stubs, typing marker, licenses, and absence of bytecode caches. Linux/Windows wheel builds and CPython 3.10/3.14 install checks are configured in CI but were not executed locally. Julia example API was checked against upstream source; Julia runtime was unavailable.

Independent read-only review found exponent overflow and equivalent numeric-ID canonicalization bugs; BigInt normalization fixes both, with a red/green regression test. Existing Bevy tests caught invalid-file retry behavior regression; original retry semantics restored. No remaining material review findings. This verification record predates the PR commit. HIF/Python work was subsequently committed and opened as PR #8; the October 3 ecosystem measurements and reproducible harness extend that PR. No PyPI publication was performed.
