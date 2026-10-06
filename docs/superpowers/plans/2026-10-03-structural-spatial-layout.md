# Structural Spatial Layout Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans for the user's selected Native execution; one independent final review before publishing. The user approved the preceding diagnosis and recommendation.

**Goal:** Reveal connectivity in the complete Mathlib Spatial graph, keeping every vertex and hyperedge and the optional explorer.
**Architecture:** Add normalized and LinLog attraction models and a sparse normalized-incidence spectral initializer to the existing Bevy-free layout. Native controls choose these models and persist their parameters; optional degree/arity fading preserves complete geometry. Compare neutral seeds, legacy forces, and structural initialization on both planted and real graphs before choosing defaults.
**Tech Stack:** Rust 2024, existing Barnes–Hut repulsion, Bevy 0.18 / egui; no new dependencies.
**Spec:** The user's approved recommendation in this chat: normalized forces, separate pair/set influence, connectivity-based initialization, reduced visual weight for global relationships, complete Spatial rendering.

## Constraints and review focus

- Keep Auto Spatial, all original IDs and geometry, unlimited overview lines, and existing JSON/session versions. Generic/tour legacy forces remain available.
- Use graph connectivity and stable IDs for structural positions, never subject labels. Disconnected/isolated components remain finite and visible.
- Pair attraction remains symmetric; degree normalization caps hub dominance. Large sets are weighted by arity and derived dependency groups independently of import pairs.
- Sparse spectral products are O(nodes + memberships), with bounded iteration counts; no dense n×n matrix or clique expansion.
- Model/weight changes rebuild force weights and survive scene reloads and sessions. Frozen layouts can rebuild structural positions without fake simulation iterations.
- Preserve optional explorer switching, picking/drawing scope agreement, and paused geometry caching.
- Report community/separation metrics as diagnostics, not proof of mathematical topology. Native full-graph benchmark is distinct from the earlier bounded canvas benchmark.

## Task 1 — Core forces and structural initialization

Files: new `crates/hyper-viz/src/topology.rs`; `layout.rs`, `lib.rs`.

- [x] Add failing regression tests using two internally connected blocks, one bridge, and a universal umbrella. Assert structural initialization brings within-block vertices substantially closer than across-block vertices; require all nodes retained.
- [x] Add tests that changing node kinds does not change initialized positions; zero-degree / disconnected / empty inputs produce finite, reproducible positions; reordered stable IDs preserve positions within numeric tolerance.
- [x] Pin normalized pair dynamics on a literal two-node case (repulsion/gravity zero, dt=1, damping=1), equal-and-opposite movement and bounded fan-out hub influence. Pin group influence and weight changes while retaining legacy behavior.
- [x] Run `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 cargo +1.96.0 test -p hyper-viz --offline` and observe RED.
- [x] Implement `LayoutModel`, `TopologySettings`, sparse weighted relations, normalized/LinLog attraction, and connectivity-only structural initialization. Keep the Legacy branch intact.
- [x] Compare actual Mathlib layouts from identical neutral seeds / structural initialization with a headless example that writes positions and numeric neighbor/separation diagnostics. Tune bounded defaults based on this evidence.
- [x] Run the core suite and commit the verified core deliverable.

## Task 2 — Native controls, persistence, and rendering

Files: native `graph.rs`, `ui.rs`, `session.rs`, `render.rs`, `hyperedge_hull.rs`; core session defaults.

- [x] Add session roundtrip and legacy-default regressions for topology parameters, and reload regression retaining positions/model.
- [x] Enable the validated structural model and initializer in the large star-import overview. Preserve tours and generic defaults; expose Legacy/Normalized/LinLog, pair/set attraction, hub/arity normalization, derived-set influence, and a Rebuild structural layout action.
- [x] Add optional hub/large-set visual fading to existing line/hull styles; selected/hovered relationships retain emphasis. Keep all geometry and scope memberships.
- [x] Preserve actual iteration counts when rebuilding positions, pause/frame the result, and expose initialization duration.
- [x] Run full Rust suites, clippy all-targets with warnings denied, fmt, and release build.

## Task 3 — Native evidence and PR update

Files: `docs/mathlib-demo.md`, validation report, API migration notes, headless/native examples.

- [x] Inspect the full Mathlib native structural view, including all nodes/edges, useful marker size, controls, and optional explorer. Record median/p95 full-graph frame times and a screenshot.
- [x] Compare real neighbor-distance/separation metrics and the planted fixture; document initializer approximation, remaining occlusion, and performance limits.
- [x] Have the independent reviewer examine the final diff; reproduce/fix important findings with regressions.
- [x] Commit, push, update PR #9 around the final Spatial implementation, and attach it. Keep the managed worktree.
