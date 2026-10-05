# Public repository readiness

This checklist prepares a maintainer's decision to expose the repository. It
covers source and documentation readiness separately from package publication.
Opening the repository, merging a PR, and publishing a release are separate
maintainer actions.

## Source and documentation

- [x] README provides complete viewer, Python, and Rust onboarding workflows.
- [x] [Architecture](ARCHITECTURE.md) documents the Bevy-free core, Python process
      launcher, optional renderer, scene ownership, reloads, and dependency canvas.
- [x] [API guide](API.md) and Rust API comments document HIF, projections,
      structural forces, directed queries, migration, and embedding.
- [x] [Viewer guide](VIEWER.md) and [Mathlib workflow](mathlib-demo.md) describe
      controls, initialization, persistence, and known geometry/performance limits.
- [x] [Python release guide](PYTHON_RELEASE.md) and
      [desktop packaging guide](DESKTOP_PACKAGE.md) cover supported platforms,
      companion versions, wheel/source packaging, and release procedures.
- [x] Source-only layout features are distinguished from published `0.1.2` wheels.
- [x] Rust workspace, Python core, and desktop companion versions are aligned
      at `0.2.0`, with an exact viewer dependency and [release notes](../CHANGELOG.md).
- [x] [CONTRIBUTING](../CONTRIBUTING.md) documents Rust/Python development checks.
- [x] MIT OR Apache-2.0 licenses, NOTICE, bundled HIF schema/license, and Python
      wheel license files are present.

## Repository boundary and hygiene

- [x] Public crates retain generic vertices, hyperedges, and visualization APIs;
      host memory engines and private claim pipelines remain outside this repository.
- [x] Legacy host import is quarantined in `crates/hyper-viz/src/io/legacy_export.rs`.
- [x] No credential files, local environments, caches, or private service
      configuration are required or tracked.
- [x] The October 5 scan of all 429 blobs reachable from the rebased PR found no
      common private-key, AWS/GitHub/Slack/OpenAI token, or credential-URL signatures.
      This is a bounded signature scan, not proof that every kind of sensitive data
      is absent. Local machine paths were removed from runnable documentation;
      original benchmark evidence is retained.
- [x] Rust crates remain `publish = false`; preparation does not publish artifacts.

## October 5, 2026 integration verification

[PR #9](https://github.com/atomicstrata/hyper/pull/9) is rebased onto main's
`ed07a12` revision. Its conflict resolutions preserve explicit HIF input formats,
watched reloads, view-mode configuration, and main's scientific onboarding.

| Check | Local result |
|---|---|
| Core Rust tests | 82 unit + 9 HIF integration tests passed |
| Native Rust tests | 79 passed, including HIF watch and dependency-mode reload/remapping |
| Rust doctests | 3 core + 2 native passed |
| Formatting / whitespace | `cargo fmt --all -- --check` and `git diff --check` passed |
| Clippy | Entire workspace, all targets, warnings denied |
| Public Rust docs | Entire workspace, dependencies excluded, warnings denied |
| Python typing / generated stubs | Strict mypy passed; regenerated stubs have no drift |
| Installed Python package | 17 tests passed for checkout wheel and source-distribution wheel |
| Scientific interoperability | XGI example preserved its 3 nodes and 1 edge |
| Exporter / benchmark aggregation | 9 standard-library tests passed |
| Packaging | macOS arm64 abi3 wheel, standalone source distribution, and wheel built from that source passed content checks |
| CLI / core example | Help, version, and headless example passed |
| Native HIF smoke | Three GPU-rendered PNG frames and timing report produced on macOS arm64 |
| Python/native process smoke | Real viewer launch, lifetime, explicit close, and temporary snapshot cleanup passed |
| Documentation links | Local Markdown file targets resolve |
| Independent review | Reload/return crash reproduced and fixed; regression observed failing before the fix, then passing |

Current cross-platform checks are authoritative on the
[PR checks page](https://github.com/atomicstrata/hyper/pull/9/checks), rather than
an evergreen checked box. The Rust workflow checks documentation, the full
workspace, exporter tests, and CLI smoke commands. The Python workflow builds
and tests supported Linux, macOS, and Windows core/desktop wheels and source
packaging. Require successful checks for the exact commit being merged.

The October 3 [native measurements](benchmarks/module-explorer.md) remain
historical evidence. This rebase does not establish new GPU performance figures
or guarantee native appearance or responsiveness on every supported desktop.

## Maintainer decisions

- [ ] Merge the reviewed PR after its current GitHub checks succeed.
- [ ] Decide whether to make the repository public, keep it private, or extract further.
- [ ] If publishing the prepared `0.2.0` Python/viewer packages, follow the
      tag-based release guide after validating the release commit. Existing
      `0.1.2` artifacts do not include the new layout controls.
- [ ] If publishing Rust crates, explicitly remove `publish = false` as part of
      a separately reviewed release and confirm crate metadata/dependency versions.

No visibility changes, package uploads, or release tags are part of this
preparation. Known approximate hulls, overlapping dense graphs, and deferred
notebook/live Python capabilities remain documented product limitations.
