# Module Dependency Explorer Implementation Plan

> Updated user direction, 2026-10-03: the main workflow is the complete Spatial
> graph with wider, precise controls. Keep the dependency explorer as an explicit
> opt-in, unused by default. This supersedes the original Auto/default and
> hidden-hull choices below. The final full overview enables every line and hull,
> pauses layout, and frames the whole graph; force ranges and validation are
> documented in `docs/mathlib-demo.md` and `docs/benchmarks/module-explorer.md`.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans for native execution, or superpowers:subagent-driven-development if the user selects parallel implementation and review. Steps use checkbox syntax for tracking.

**Goal:** Make Mathlib module imports and dependents understandable through exact selection, directed expansion, a bounded 2D view, useful spatial controls, and a verified pull request.

**Architecture:** Preserve attributes and actual hub identity in the existing scene. A Bevy-free dependency index implements directed queries; dedicated explorer state, UI, and canvas modules consume it. Spatial rendering uses a shared visibility policy and runs only in spatial mode.

**Tech Stack:** Rust 2024, hyper-viz, Bevy 0.18, bevy_egui 0.39, serde, Python standard-library exporter tests.

**Spec:** `docs/superpowers/specs/2026-10-02-module-dependency-explorer-design.md`.

## Global Constraints

- The wire-format version stays `hypergraph.v1`.
- Session fields for the explorer are additive and defaulted under `hyperviz.session.v1`.
- Default scope: one hop in each direction; initial visible budget 200, maximum 1,000; hop controls one through six.
- Every arrow means "the source module imports the target module".
- Filters initially include all categories and never remove the center.
- Traversal stops at filtered vertices; omitted and filtered counts are disclosed.
- Centroid attraction positive range `0.000001–0.1`; gravity positive range `0.00001–0.1`; both support zero/off and numeric entry.
- Repulsion `1–100000`; dt `0.01–1`; damping `0.5–0.99`; hull opacity `0–0.6`; line opacity `0–1`.
- Large-graph spatial preset: gravity `0.001`, centroid attraction `0.0005`, deterministic subject initialization, global hulls disabled.
- Simulation budget 8 ms/frame, with at least one step while running and displayed actual step count/duration.
- Keep the declared Rust floor at 1.89; use an installed compatible toolchain for the lockfile without unrelated dependency changes.
- Work in the current managed worktree. No merge or deployment. The user's latest instruction explicitly authorizes creating and publishing the PR branch and opening a PR.

## Review Focus

- A scene reload reorders vertices or removes the center: restore IDs safely, invalidate direction/search caches, and show a missing-center state (Tasks 1 and 3).
- A malformed direction refers to a hub, missing vertex, non-member, or higher-arity group: warn and retain generic inspection; never infer a directed import (Tasks 1 and 2).
- Filtered intermediate vertices, cycles, self-loops, and duplicate imports: queries terminate, distinguish excluded paths, and count modules once (Task 2).
- A restored session has out-of-range depths/budgets, nonfinite canvas values, or stale history IDs: normalize before rendering (Task 3).
- Mode switches, disabled hulls, line budgets, and frozen geometry: do not leave invisible pick targets, background simulation, or needless topology rebuilds (Tasks 4 and 5).

## Execution and workspace

Recommended execution is native: the implementer works through the six tasks in order, then one independent reviewer examines the complete diff before the PR. The scene, focus, and visibility changes share interfaces; parallel edits would need extra coordination.

Use the current detached managed worktree. Verify it with `git rev-parse --git-dir --git-common-dir` and `git rev-parse --show-superproject-working-tree`. Create `codex/module-dependency-explorer` before commits using the host's supported Git controls. If the branch name exists, inspect it before choosing a new suffix; never reset an existing branch.

Create this plan's ledger through the executing-plans workspace script. Preserve completed tasks and record any spec/plan rulings. Build artifacts stay under ignored `target/` or a task-specific temporary directory. Never write to the primary checkout's target directory to reuse its cache.

Baseline commands:

```sh
cargo +1.96.0 test --workspace --offline
python3 -m unittest discover -s scripts -p 'test_*.py'
```

Expected: the existing suites pass. If dependency/toolchain or runtime restrictions prevent a command, establish the concrete cause, use an installed compatible alternative when available, and preserve the limitation in the ledger.

---

## Task 1: Preserve direction and fix hubless projections

**Files:** `crates/hyper-viz/src/scene.rs`, `project.rs`, `semantics.rs`; all hub consumers in `crates/hyper-viz-bevy/src/{animation,focus,graph,hyperedge_hull,interaction,ui}.rs`; `crates/hyper-viz/README.md`.

**Interfaces:**

```rust
// SceneNode, SceneHyperedge, and SceneMeta gain defaulted attrs maps.
pub attrs: serde_json::Map<String, serde_json::Value>;
// SceneHyperedge gains truthful hub identity.
pub hub_index: Option<SceneIndex>;
// Shared generic visibility helper, used by focus/labels/hulls/picking.
pub fn hyperedge_in_scope(
    scene: &HypergraphScene,
    hyperedge_index: usize,
    nodes: Option<&std::collections::HashSet<SceneIndex>>,
    hyperedges: Option<&std::collections::HashSet<usize>>,
) -> bool;
```

- [ ] Write the regression below in `scene.rs`, then add projection tests for preserved graph/node/edge attributes and legacy scene JSON without attributes. The realistic mutations are fabricating hub zero and dropping direction/path metadata.

```rust
#[test]
fn star_first_vertex_does_not_focus_unrelated_groups() {
    let graph = Hypergraph::new()
        .vertex("a", "A", "module")
        .vertex("b", "B", "module")
        .vertex("c", "C", "module")
        .vertex("d", "D", "module")
        .hyperedge("ab", ["a", "b"], "AB")
        .hyperedge("cd", ["c", "d"], "CD");
    let scene = graph.project(Projection::StarCentroid);
    let actual = neighborhood(&scene, [0]);
    assert_eq!(actual, std::collections::HashSet::from([0, 1]));
}
```

- [ ] Run `cargo +1.96.0 test -p hyper-viz star_first_vertex_does_not_focus_unrelated_groups --offline`. Expected before the fix: four vertices, assertion failure.
- [ ] Copy attributes in every projection. Use `Some(hub_index)` only for bipartite hubs and `None` for star/clique. In `neighborhood`, test/insert the hub only when it exists. Include members through incident membership independently of hub identity.

```rust
let hits_hub = he.hub_index.is_some_and(|hub| seeds.contains(&hub));
let hits_member = he.member_indices.iter().any(|i| seeds.contains(i));
if hits_hub || hits_member {
    out.extend(he.hub_index);
    out.extend(he.member_indices.iter().copied());
}
```

- [ ] Audit all `hub_index` consumers. Only attach hub status to a real hub; only seed/highlight a hub when `Some`; use `hyperedge_in_scope` for hubless visibility. Extend `FocusScope` with an optional explicit hyperedge set, defaulting old scopes to membership-based checks.
- [ ] Make scene equivalence compare retained attributes and directed relation metadata as well as stable member IDs. Add a reload test changing only source/target attributes and one changing only a source path; both must invalidate caches.
- [ ] Run `cargo +1.96.0 test --workspace --offline`. Expected: all tests pass, including existing generic projections and sessions. Document the Rust scene struct-literal changes and legacy JSON defaults. Commit this task.

## Task 2: Implement directed dependency queries and module categories

**Files:** create `crates/hyper-viz/src/dependency.rs` and `module.rs`; modify `lib.rs` and projection warning generation.

**Consumes:** retained scene attributes and optional hubs from Task 1.

**Produces:**

```rust
pub enum TraversalDepth { Off, Hops(u8), Transitive }
pub struct ScopeOptions {
    pub imports: TraversalDepth,
    pub dependents: TraversalDepth,
    pub budget: usize,
}
pub struct DependencyScope {
    pub nodes: Vec<usize>,
    pub edges: Vec<(usize, usize)>,
    pub import_depths: std::collections::HashMap<usize, usize>,
    pub dependent_depths: std::collections::HashMap<usize, usize>,
    pub total_reachable: usize,
    pub omitted: usize,
}
pub struct DirectedImport {
    pub source: usize,
    pub target: usize,
    pub hyperedge_indices: Vec<usize>,
}
pub struct DependencyIndex {
    ids: std::collections::HashMap<String, usize>,
    node_ids: Vec<String>,
    outgoing: Vec<Vec<usize>>,
    incoming: Vec<Vec<usize>>,
    relations: Vec<DirectedImport>,
}
impl DependencyIndex {
    pub fn new(scene: &HypergraphScene) -> Self;
    pub fn node_index(&self, id: &str) -> Option<usize>;
    pub fn imports(&self, node: usize) -> &[usize];
    pub fn dependents(&self, node: usize) -> &[usize];
    pub fn scope(&self, center: usize, options: &ScopeOptions, allowed: &[bool]) -> DependencyScope;
    pub fn expand(&self, center: usize, previous: &DependencyScope, node: usize, options: &ScopeOptions, allowed: &[bool]) -> DependencyScope;
    pub fn shortest_path(&self, source: usize, target: usize, allowed: &[bool]) -> Option<Vec<usize>>;
    pub fn relations(&self) -> &[DirectedImport];
}
pub struct ModuleCategory {
    pub umbrella: bool,
    pub test: bool,
    pub tactic: bool,
    pub external: bool,
}
pub fn module_categories(scene: &HypergraphScene) -> Vec<ModuleCategory>;
```

- [ ] Add a hand-built fixture with directed imports `a→b`, `b→c`, `d→a`, `x→d`, and a group `[a,b,c,d,x,z]`. Construct each import as an arity-two edge with literal source/target attributes. Expected scope at one hop: `{a,b,d}`; two-hop imports: `{a,b,c,d}`; two-hop dependents: `{a,b,d,x}`. The group must never add `z`.
- [ ] Add tests for `a→b→a`, a duplicate `a→b`, self-loop `a→a`, empty scene, invalid center, bad endpoints, filtered `b`, and deterministic budget truncation. For `a→b→c`, the shortest path is `[a,b,c]`; from `c` to `a` it is absent; `a` to itself is `[a]`. Filtering `b` blocks that path.
- [ ] Run `cargo +1.96.0 test -p hyper-viz dependency --offline`. Expected: new API is missing, or desired traversal behavior fails.
- [ ] Validate imports using existing vertex IDs, arity, membership, and `kind == "import"`. Keep malformed edges in the scene and emit warnings. Build deduplicated outgoing/incoming adjacency sorted by stable ID. Retain duplicate source edge identities in `DirectedImport`.
- [ ] Implement BFS with visited sets. Traverse each direction independently; union nodes once, retain the center, order by minimum hop distance then ID, and apply budget after traversal. Return induced directed edges only between displayed vertices. Preserve existing scope and center during one-hop expansion.

```rust
fn traverse(adjacency: &[Vec<usize>], source: usize, depth_limit: Option<usize>, allowed: &[bool]) -> std::collections::HashMap<usize, usize> {
    if source >= adjacency.len() { return std::collections::HashMap::new(); }
    let mut depths = std::collections::HashMap::from([(source, 0usize)]);
    let mut queue = std::collections::VecDeque::from([source]);
    while let Some(node) = queue.pop_front() {
        let depth = depths[&node];
        if depth_limit.is_some_and(|limit| depth >= limit) { continue; }
        for &next in &adjacency[node] {
            if allowed.get(next).copied().unwrap_or(false) && !depths.contains_key(&next) {
                depths.insert(next, depth + 1);
                queue.push_back(next);
            }
        }
    }
    depths
}
```

- [ ] Classify umbrellas from a module's source-path stem having loaded descendants, tests from the specified path roots, tactics from `Mathlib.Tactic`, and external nodes from source/stub attributes. Add category tests with arbitrary high-degree non-umbrella modules to prevent classification by degree alone.
- [ ] Run `cargo +1.96.0 test -p hyper-viz --offline`. Expected: all core tests pass. Commit this task.

## Task 3: Explorer state, exact selection, history, and session compatibility

**Files:** create `crates/hyper-viz-bevy/src/explorer_state.rs`; modify `lib.rs`, `graph.rs`, `session.rs`, and `crates/hyper-viz/src/session.rs`.

**Consumes:** `DependencyIndex`, `ScopeOptions`, categories, and current scene epoch.

**Produces:**

```rust
pub enum ViewMode { Auto, Spatial, Dependencies }
pub struct ModuleFilters {
    pub umbrella: bool, pub tests: bool, pub tactics: bool, pub external: bool,
}
pub struct ExplorerSnapshot {
    pub center: Option<String>, pub target: Option<String>,
    pub options: ScopeOptions, pub filters: ModuleFilters,
    pub expanded_ids: Vec<String>, pub pan: [f32; 2], pub zoom: f32,
}
impl ExplorerState {
    pub fn replace_scene(&mut self, scene: &HypergraphScene, epoch: u64);
    pub fn select_module(&mut self, id: &str);
    pub fn search(&mut self, query: &str) -> &[usize];
    pub fn back(&mut self);
    pub fn forward(&mut self);
    pub fn snapshot(&self) -> ExplorerSnapshot;
    pub fn restore(&mut self, snapshot: ExplorerSnapshot);
}
// Embedding override:
pub fn with_view_mode(self, mode: ViewMode) -> Self;
// Runtime predicates registered with spatial systems:
pub fn spatial_mode(state: Option<Res<ExplorerState>>) -> bool;
```

- [ ] Add tests for exact module selection despite overlapping node/group search names, back/forward restoration of filters and expansion, reordering IDs, disappearing centers, and cached unchanged queries. Add session round trips for old JSON, explorer state, opacity `0.001`, and invalid saved values.
- [ ] Run the targeted state/session tests before implementation. Expected: new API/fields are absent or selection/restore assertions fail.
- [ ] Build the index and category mask once per epoch. Cache a case-folded search corpus; rank exact ID first, then prefix/substring candidates, then fuzzy fallback. Return at most 20 result rows. Search remains global; selecting a row is the only action that changes the center.
- [ ] Add bounded history of 32 snapshots, stable IDs, and restore normalization. Clamp hops to `1..=6`, budget to `1..=1000`, zoom to `0.1..=4.0`; replace nonfinite pan with `[0,0]` and nonfinite zoom with `1`. Preserve a missing center's ID for the empty-state message.

```rust
snapshot.options.budget = snapshot.options.budget.clamp(1, 1000);
snapshot.zoom = if snapshot.zoom.is_finite() {
    snapshot.zoom.clamp(0.1, 4.0)
} else { 1.0 };
if snapshot.pan.iter().any(|value| !value.is_finite()) {
    snapshot.pan = [0.0, 0.0];
}
```

- [ ] Add defaulted per-view explorer session data. Preserve `HYPER_VIZ_SESSION=off`, generic localization, host-backed search, camera pose, and stable-ID selection. Initialize auto mode after the scene is available and before spatial consumers; imported graphs choose Dependencies, generic graphs choose Spatial. Showcase explicitly chooses Spatial.
- [ ] Run `cargo +1.96.0 test --workspace --offline`. Expected: old and new state/session tests pass. Commit this task.

## Task 4: Build the native module canvas and exploration UI

**Files:** create `crates/hyper-viz-bevy/src/explorer_ui.rs` and `explorer_canvas.rs`; modify `ui.rs`, `camera.rs`, `focus.rs`, and `lib.rs`.

**Consumes:** explorer state, core index, scopes, paths, categories, and shared visibility.

**Produces:**

```rust
pub struct CanvasNode { pub index: usize, pub position: [f32; 2] }
pub fn layered_positions(index: &DependencyIndex, scope: &DependencyScope, canonical: &DependencyScope) -> Vec<CanvasNode>;
pub fn explorer_panel(contexts: EguiContexts, state: ResMut<ExplorerState>, layout: ResMut<GraphLayout>, focus: ResMut<FocusScope>);
```

- [ ] Add tests for one node appearing once when reached in both directions, deterministic layout after scene reordering, and retained rows when filters/budget remove other nodes. Test transfer to spatial focus retains only directed displayed edges plus the explicitly chosen group.
- [ ] Run the targeted canvas/focus tests. Expected: missing layout function or incorrect focus assertions.
- [ ] Lay out dependents at negative x and imports at positive x using hop depth. Center the selected module at `[0,0]`; choose dependent placement deterministically for dual-role nodes and label both roles. Order each canonical layer by stable ID. Derive row positions before filtering/truncation to preserve retained node positions.

```rust
let column = if node == center { 0 } else if let Some(depth) = dependent_depth {
    -(depth as i32)
} else { import_depth.unwrap_or(1) as i32 };
let position = [column as f32 * 340.0, canonical_row as f32 * 44.0];
```

- [ ] Render an egui central canvas with directed arrows, short suffix labels, full-name tooltips, category colors/legend, pan/zoom/fit, selected-node details, and an explicit list-view toggle. Use scoped clipped painting. Overlay arrows only for displayed directed relations. No match produces an explanatory empty state.
- [ ] Build search/selection, history, independent imports/dependents depth and transitive controls, category switches, budget disclosure, full direct neighbor lists, and one-hop expansion/reset. Lists show complete names and counts independent of graph truncation; clicking a neighbor recenters the view.
- [ ] Add the second module picker and ordered shortest-path list. Compare filtered and unfiltered queries to distinguish absent versus filter-blocked paths. Include zero-hop paths; keep the full textual path even when the graph is truncated. Highlight displayed path segments.
- [ ] Add explicit group inspection for the center's exported group with the authoritative member list. Transfer the bounded scope to spatial focus, preserving source edge IDs. Switching modes hides spatial entities/hulls and gates force steps, 3D picking, and spatial HUD/camera interactions; restore the previous spatial running state on return.
- [ ] Use visible-node counts for spatial label eligibility. Run `cargo +1.96.0 test --workspace --offline`. Expected: all suites pass. Commit this task.

## Task 5: Correct controls and remove spatial rendering waste

**Files:** `crates/hyper-viz-bevy/src/{ui,graph,render,interaction,hyperedge_hull,node_visual}.rs`; create `spatial_visibility.rs`; `crates/hyper-viz/src/layout.rs` if deterministic initialization needs a reusable helper.

**Consumes:** mode predicate, hubless scope policy, explicit scoped edges, and retained metadata.

**Produces:** shared visible-line enumeration used by drawing/picking/counts, geometry dirty tracking, a spatial preset, and measured `LayoutTiming { steps: usize, milliseconds: f64 }`.

- [ ] Add regression tests proving budget-hidden lines are not pickable, focused imports precede unrelated lines, frozen hull geometry is unchanged, style-only edits do not rebuild topology, and dyadic edges never enter hull creation. Test that the simulation executes at least one step under an exhausted budget and reports actual steps.
- [ ] Run targeted tests before implementation. Expected: visibility or geometry assertions fail, or new helpers are missing.
- [ ] Centralize visible-line enumeration. Respect explicit edge scope, node scope, hidden hubs, and the same budget for rendering and picking. Return omitted-line counts for the HUD. Build static pick caches keyed by scene/position/visibility revisions.
- [ ] Skip arity below three before member-position allocation. Use iteration/revision changes for geometry dirtiness and material-only changes for style. Stage egui settings in local copies and update resources only when the values differ.

```rust
if hyperedge.member_indices.len() < 3 { continue; }
let positions_changed = layout.layout.iterations != last_iterations;
let geometry_dirty = positions_changed || epoch.0 != last_epoch;
if geometry_dirty {
    let Some(hull_mesh) = hull_from_points(&all_positions) else { continue; };
    if let Some(mesh) = meshes.get_mut(&mesh_handle) {
        apply_hull_mesh(mesh, &hull_mesh);
    }
}
if let Some(material) = materials.get_mut(&mat_handle) {
    if material.base_color != fill { material.base_color = fill; }
}
```

- [ ] Apply all numeric ranges from Global Constraints. Expose gravity/centroid off switches, logarithmic positive controls, precise numeric entry, and line opacity. Render spring controls only when `layout.edges` is nonempty and centroid controls only when centroid groups are nonempty. Preserve valid saved values outside the slider track without silent clamp.
- [ ] Add the large directed-import spatial preset with deterministic subject seeds and low background line opacity. Keep generic defaults and tour settings explicit. Track actual CPU steps with `Instant`, stopping after a step when elapsed time reaches 8 ms or the requested maximum is met. Do not change dt to satisfy the budget.
- [ ] Run workspace tests and clippy. Expected: pass, including generic viewer behavior, frozen geometry, and small opacity round trips. Commit this task.

## Task 6: Verify Mathlib, document, review, and open the PR

**Files:** add `crates/hyper-viz/examples/inspect_dependencies.rs`; update `docs/mathlib-demo.md`, `docs/VIEWER.md`, both crate READMEs, and the root README. Record validation in the execution ledger; keep temporary screenshots/measurements under ignored `target/`.

**Consumes:** finished module explorer, directed index, spatial controls, and all previous task checks.

- [ ] Add a headless inspection example taking a graph file and exact module ID. Print direct import/dependent IDs, scope size, and warnings; return a nonzero exit for a missing module. Verify it on a small fixture with hand-derived neighbors first.
- [ ] Run the actual dataset acceptance command, using the existing graph read-only if it is available only in the primary checkout:

```sh
cargo +1.96.0 run --release -p hyper-viz --example inspect_dependencies -- /Users/bregy/Documents/atomicstrata/hyper/target/mathlib/graph.json Mathlib.Topology.Basic
```

Expected: imports `Mathlib.Data.Set.Finite.Range`, `Mathlib.Data.Set.Lattice.Bounded`, and `Mathlib.Topology.Defs.Filter`; dependents `Mathlib`, `Mathlib.Topology.Closure`, `Mathlib.Topology.Defs.Induced`, and `MathlibTest.Tactic.Continuity`; eight vertices with all filters included.

- [ ] Run formatting, workspace tests, workspace clippy, exporter tests, and release build:

```sh
cargo +1.96.0 fmt --all -- --check
cargo +1.96.0 test --workspace --offline
cargo +1.96.0 clippy --workspace --all-targets --offline -- -D warnings
python3 -m unittest discover -s scripts -p 'test_*.py'
cargo +1.96.0 build --release --offline
git diff --check
```

Expected: successful exits. Read every output; identify pre-existing failures by name rather than claiming a clean suite.

- [ ] Launch the native viewer with `HYPER_VIZ_SESSION=off target/release/hyper --projection star /Users/bregy/Documents/atomicstrata/hyper/target/mathlib/graph.json`. Inspect exact selection, neighbor lists, arrows, filters, expansion, path tracing, group inspection, mode transfer, and small coauthorship fallback. Record interactive median/p95 frame times at 200 visible nodes and disclose whether the 30 FPS target is met. Do not substitute tour capture or a headless timing for this measurement.
- [ ] Document the launch command, module workflow, arrow semantics, filters/truncation, optional spatial preset, embedding view override, scene API migration, and verification limitations. Update the ledger's acceptance checklist. Commit documentation and final verified changes.
- [ ] Run one independent whole-branch code review using the requesting-code-review skill. Supply the spec, plan, diff, checks, Review Focus above, and ledger rulings. Reproduce important findings with failing tests, fix them, and rerun affected/full checks. Verify final Git status and commits.
- [ ] Verify repository default branch and authentication through `gh`. Write a PR body to a temporary file with the concrete behavior, scene API changes, validation, and any material limitation. Push the verified branch and open a PR using the user's existing authorization:

```sh
git push --set-upstream origin codex/module-dependency-explorer
gh pr create --title "Add directed module dependency exploration" --body-file /tmp/hyper-module-dependency-pr.md
```

Expected: the pushed branch and a PR URL. Attach every created PR using `mcp__codex_app__attach_artifact` and provide the URL plus launch command in the final response. If authentication or automatic approval review blocks publication, complete unaffected work and report the concrete blocker without claiming the PR exists.
