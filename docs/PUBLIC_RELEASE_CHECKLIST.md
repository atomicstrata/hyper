# Ready for public release decision

Checklist for maintainers deciding whether to open-source this repository.
Completing this list supports a **decision**; it does not flip visibility or
publish crates.

## Packaging / docs

- [x] Root README states what the library **is** and **is not**
- [x] Architecture boundary documented ([ARCHITECTURE.md](ARCHITECTURE.md))
- [x] API overview for external readers ([API.md](API.md))
- [x] Viewer run guide + example scene ([VIEWER.md](VIEWER.md), `fixtures/sample.json`)
- [x] CONTRIBUTING (or equivalent) present
- [x] Dual license **MIT OR Apache-2.0** with `LICENSE`, `LICENSE-APACHE`,
      `LICENSE-MIT`, and `NOTICE` copyright lines

## Boundary / hygiene

- [x] No Atomic Memory Core / private claim-pipeline code in the public crates
- [x] Legacy host export importer quarantined
      (`crates/hyper-viz/src/io/legacy_export.rs`)
- [x] No secrets, private service URLs, or credential env vars required
- [x] Library stays domain-agnostic (vertices + hyperedges)
- [x] `publish = false` until an explicit crates.io release

## Verification

- [x] CI green on the release-prep PR
- [x] `cargo test -p hyper-viz` and `cargo test -p hyper-viz-bevy --lib` pass
- [ ] Human decision: make repo public / keep private / extract further

## Explicit non-actions (this ticket)

- Do **not** make the GitHub repository public from automation
- Do **not** publish crates to crates.io
- Do **not** conflate with ATO-1747 / ATO-1751 quality tracks
