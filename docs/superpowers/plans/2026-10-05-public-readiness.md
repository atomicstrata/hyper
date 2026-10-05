# PR #9 Rebase and Public Readiness Plan

> **For agentic workers:** Use superpowers:executing-plans to carry out the remaining documentation and verification work. Steps use checkbox syntax for tracking.

**Goal:** Rebase PR #9 onto current main, preserve both scientific interchange and native layout behavior, and make the combined implementation reviewable for a public repository.

**Architecture:** Keep the Bevy-free Rust core, Python interchange package, optional native renderer, and CLI boundaries. Resolve conflicts by combining explicit input-format propagation with configurable view modes; retain main's Python onboarding and the PR's structural workflow.

**Tech Stack:** Rust 1.89, Bevy 0.18, PyO3, CPython 3.10+, maturin 1.9.6.

**Spec:** [Maintainer release checklist](../../PUBLIC_RELEASE_CHECKLIST.md), plus the user's request to rebase PR #9 and document its implementations.

## Global Constraints

- Preserve HIF imports, watched input formats, and Python package metadata.
- Auto view remains Spatial; dependency exploration requires explicit selection.
- Core defaults retain Legacy forces; structural initialization is opt-in outside the large star-import viewer preset.
- Do not change repository visibility or publish packages as part of preparation.
- Update only PR #9's remote branch, with a lease against its original head.

## Review Focus

- CLI input formats must reach both startup and watched reloads when selecting a view mode.
- Rebuilt or replaced scenes must refresh hulls, search caches, selection, and direction queries.
- Empty scenes, isolates, malformed import direction, and reordered stable IDs must retain defined behavior.
- Python HIF conversion must retain metadata and reject unsupported viewer semantics.
- Documentation must distinguish current source features from previously published wheels and historical benchmark evidence.

## Tasks

- [x] Fetch current refs; rebase the eleven commits; retain both sides of conflicting APIs and regression tests; compare with `git range-diff`.
- [x] Document HIF, dependency queries, layout algorithms, scene migration, viewer configuration, persistence, and package boundaries in public guides and Rust API comments. Check all local documentation links and run Rust doctests.
- [ ] Run formatting, core and native tests, all-target clippy, Python stub drift, installed-wheel tests, typing, exporter tests, and package-content checks. Audit tracked files and history for accidentally included credentials or private artifacts. Obtain an independent review, fix reproducible regressions, push using `--force-with-lease`, and verify GitHub mergeability and CI.

## Verification Commands

```sh
cargo fmt --all -- --check
cargo test --locked -p hyper-viz
cargo test --locked -p hyper-viz-bevy --lib
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo run --locked -p hyper-viz-python --bin stub_gen
git diff --exit-code -- crates/hyper-viz-python/python/hyper_viz/_core.pyi
python -m pytest crates/hyper-viz-python/tests -q
python -m mypy --strict crates/hyper-viz-python/python/hyper_viz
python crates/hyper-viz-python/examples/xgi_interop.py
PYTHONPATH=scripts:scripts/benchmarks python -m unittest scripts/test_mathlib_graph.py scripts/benchmarks/test_datasets.py scripts/benchmarks/test_summarize.py
git diff --check
```

Expected: zero exit codes. Native window and cross-platform wheel checks require the corresponding desktop or GitHub runners; report their actual status separately.

## Execution record

- Rebase retained all eleven original commits; range-diff differences are the intentional conflict resolutions.
- Local Rust verification: 82 core unit, 9 HIF integration, 79 native, and 5 doctests passed. Workspace/all-target clippy and public docs passed with warnings denied. Python checkout/source wheels passed 17 tests each, strict typing, stub drift, and content checks. Exporter/aggregation tests passed 9/9.
- Independent review identified a crash after scene replacement while Dependencies was active: entity synchronization was gated on Spatial, so same-frame mode return could expose stale label indices. The real RenderPlugin regression failed with Alice/Bob retained after a Bob-only reload. Synchronizing every scene epoch fixed it; a second regression assertion then exposed new nodes appearing in Dependencies, fixed by spawning them hidden. Final native suite passed 79/79.
- Existing public rustdoc linked to the private legacy importer; replaced the intra-doc link with a plain code reference and reran strict documentation successfully.
- Historical benchmark counts/timings retained and explicitly labeled as pre-rebase evidence. No new GPU performance run was performed. Cross-platform verification is delegated to the existing GitHub runners.
- Native HIF smoke produced three PNG frames on the Mac GPU. Python launched the actual star viewer, verified its lifetime, closed it, and removed its temporary snapshot. These are functional smoke checks rather than new performance benchmarks.
- No secrets matched the bounded history scan; local command paths were made portable. Main, repository visibility, package versions, and publication are retained pending maintainer decisions.
