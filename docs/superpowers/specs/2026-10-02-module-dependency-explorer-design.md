# Module dependency explorer

Date: 2026-10-02

## Goal and approved direction

Help the user understand a Mathlib module's imports and dependents. On selecting
one exact module, show its direct relationships with readable names and arrows,
then let the user expand the scope deliberately. Keep the original hypergraph
available for spatial exploration and optional group inspection.

The user approved the investigation's recommendation on 2026-10-02. This document
makes that recommendation concrete for implementation. The first delivery covers
the module workflow, projection correctness, useful layout controls, and the
performance work needed by that workflow. A global subject aggregation map is
outside this delivery.

## Evidence

The existing Mathlib JSON has 9,384 vertices, 37,356 directed import relations,
and 7,550 dependency groups. Import direction lives in hyperedge attributes;
projection currently discards those attributes.

`deps:Mathlib` contains 8,532 vertices. In the current input order, 4,473 of the
first 5,000 import lines originate at `Mathlib`. The line budget consequently
favors this umbrella module rather than a representative or useful neighborhood.

The existing focus operation expands all incident hyperedges, including this
umbrella group. A headless probe found that focusing `Mathlib.Topology.Basic`
produces 8,534 vertices despite that module having only three direct imports and
four direct dependents. Its direct neighborhood needs eight vertices.

Star projection has no link springs. A headless probe found identical positions
when changing its exposed attraction and ideal-length controls. Its effective
centroid attraction and gravity have no HUD controls. Interactive defaults are
16 and 20 times the corresponding tour settings. Every star hyperedge also uses
the fabricated hub index zero, which corrupts focus, visibility, and hub status.

The screenshot reports approximately 5 FPS. A short headless layout probe took
approximately 12 ms per step; this is not an interactive renderer profile.
Performance acceptance therefore requires new viewer measurements.

## User workflow

### Open and find

Graphs with validated directed import relations expose a "Module explorer"
mode alongside "Spatial graph". The standalone Mathlib CLI opens the module
explorer by default; generic hypergraphs retain the current spatial viewer.
Embedded hosts can choose the initial mode explicitly.

The initial explorer screen explains that a module must be selected. Search
operates across the loaded graph, even when the current view is filtered.
Search shows a bounded result list with full module names. Exact ID matches
rank first; selecting a result or pressing Enter chooses one module. Typing
does not automatically select 32 nodes and hyperedges. The generic spatial
localization workflow and optional host-backed repository search remain available.

Selecting a module pins it as the center, shows its file location, and opens
imports and dependents lists. Clicking a listed neighbor makes it the center.
Back and forward restore the center, expansion settings, and filters by stable
IDs. Missing centers after a reload show an explanatory empty state.

### Relationships and expansion

The default scope is direct imports and direct dependents, one hop each. Controls
can enable either direction independently and set their depths independently
from one to six. "Transitive" traverses the selected direction to closure, but
the display remains subject to a visible-node budget.

Expansion follows directed import relations only. Membership in a dependency
group does not add co-members or the rest of an umbrella group. Every arrow
means "the source module imports the target module".

Lists report complete direct counts and full names. The graph initially displays
at most 200 modules, always including the center. Additional expansion is ordered
by hop distance, then stable module ID. A disclosure reports how many reachable
modules are not displayed. The user can raise the budget to 1,000. Lists remain
scrollable independently of the graph's budget.

An "Expand this module" action adds one hop of that module's enabled relationship
directions to the existing scope without replacing the center. It uses the same
filters and budget. A reset action returns to the center's initial neighborhood.

### Filters

Provide independent switches for umbrella modules, test modules, tactic modules,
and external modules. All start included so the initial counts faithfully match
the source data. Counts for hidden neighbors remain visible when filters change.
The selected center is always retained and visibly identified if it falls into
a filtered category. Traversal stops at filtered vertices; the UI describes this
as traversing the filtered graph.

For Mathlib, classify modules using preserved metadata and module paths. An
umbrella module has a corresponding source directory containing other modules;
derive this from loaded module IDs and paths, without accessing the filesystem
or applying an arbitrary degree threshold. Tests include MathlibTest and
DownstreamTest. Tactics use Mathlib.Tactic. External vertices use their explicit
external/stub metadata. Generic graph consumers can ignore these domain filters.

### Dependency paths and groups

An optional second module picker asks "Does this module depend on that module?"
and finds a shortest directed import path from the center to the chosen target.
Show the ordered full names and highlight the path. Distinguish a zero-hop path,
no path in the loaded graph, and a path blocked by the active filters. A path
can exceed the normal visible budget; show its full textual list and disclose
graph truncation rather than silently omitting steps.

The "Show dependency group" action displays only the chosen module's exported
dependency group. Its member list is authoritative. Membership visualization
does not imply that all imports depend on one another or form a community.
Do not automatically expand every group containing a selected module.

## View and interaction

Use a layered 2D canvas for the module explorer: dependents to the left, the
selected module at the center, and imports to the right. Nodes reached in both
directions appear once, with a deterministic placement and visible relation
types. Cycle-safe traversal is required even though this Mathlib dataset is
acyclic. Local expansion may add connections between layers; arrow direction
remains authoritative.

Render the canvas in the existing native egui UI. Pan, zoom, fit, selection,
hover, tooltips, and short labels with full-name details operate on the bounded
scope. Keep a stable order and deterministic positions when switching filters
or changing the visible budget. A legend explains arrow direction and node
colors. Provide a list view for modules whose relationships are still dense.

Spatial mode retains the Bevy viewer. An action transfers the current module
scope into spatial focus; another returns to the layered view. The layered
canvas does not run the global force simulation, create global hull meshes,
or perform 3D picking behind the UI. Returning to spatial mode restores its
previous running state. Spatial labels use the visible scope size when deciding
whether a neighborhood is small enough to label.

Global hulls start disabled for large directed-import graphs. The chosen module's
dependency group can be inspected explicitly in spatial mode. Generic small
hypergraph hull defaults remain available.

## Data and boundaries

### Projection and scene identity

Replace fabricated hub indices with optional hub identity. Bipartite scenes have
real hubs; star and clique scenes have no hubs. All consumers must distinguish
an actual hub from member vertices. Neighborhood and visibility checks use
explicit hyperedge membership or selected hyperedge indices when no hub exists.

Preserve graph, vertex, and hyperedge attributes in the scene representation.
Derive import direction only from validated source/target attributes identifying
existing members of an arity-two import edge. Invalid direction produces a scene
warning and remains inspectable as a generic hyperedge; never guess orientation
from a label, vector order, or module name.

The wire-format version stays `hypergraph.v1`. Scene deserialization defaults
new optional fields so saved scenes without attributes remain readable. The
Rust scene API changes are documented because adding fields and optional hubs
affects embedding hosts' struct literals. Audit every in-repository constructor
and consumer, including showcase, sessions, attention, and status animation.

### Core exploration

Add a Bevy-free directed dependency index responsible for outgoing and incoming
adjacency, bounded traversal, paths, direct counts, and deterministic scope
construction. Build it once per scene epoch. Deduplicate equivalent directed
pairs while retaining their source hyperedge identities for inspection. Traversal
must terminate on cycles, missing IDs, empty graphs, and self-loops.

Keep module category interpretation separate from the generic directed index.
Filtering returns both displayed relationships and counts excluded by category
or budget. Group membership remains available separately from directed adjacency.

### Viewer state

A dedicated explorer module owns center/target IDs, history, cached search
results, expansion, filters, canvas transform, and rendering. The existing main
HUD remains responsible for spatial layout and visual settings. Cache search
results by query and scene epoch; an unchanged query does not rebuild and rank
the full corpus every frame.

Session fields for the explorer are additive and defaulted under
`hyperviz.session.v1`. Store stable IDs and bounded history, not scene indices.
Validate restored numeric values and budget/depth limits. Session disabling via
`HYPER_VIZ_SESSION=off` continues to work.

## Controls and spatial defaults

Expose centroid attraction and gravity with an explicit zero/off state, numeric
entry, and logarithmic adjustment over positive values. Start with ranges
`0.000001–0.1` for centroid attraction and `0.00001–0.1` for gravity. Extend
repulsion to `1–100000`. Keep dt at `0.01–1` and damping at `0.5–0.99`.

Show spring attraction and ideal length only when the current layout actually
uses link springs. Show centroid attraction only when centroid groups are used.
Allow hull opacity `0–0.6` with precision sufficient for `0.001`, and line
opacity `0–1`. Do not use a slider minimum to silently clamp a valid saved value.

Provide a large-graph spatial preset with gravity `0.001`, centroid attraction
`0.0005`, deterministic subject initialization, hulls disabled, and subdued
background imports. This preset is a starting point for exploration, not a claim
of optimal layout quality. Keep the video tour's settings explicit and stable.

Display the actual layout iterations executed and their measured duration. Use
an 8 ms per-frame simulation budget and a requested maximum step count; perform
at least one step when running, so expensive graphs still progress. The budget
is best-effort because a single CPU step cannot be interrupted. Freeze and resume
remain explicit user actions. Avoid implicit changes to dt.

## Rendering and performance

Skip arity-two hyperedges before hull construction or hull update allocation.
Only update geometry when positions or geometry settings actually change.
Style changes update materials without rebuilding hull topology. Passing an
unchanged setting to egui must not mark geometry dirty.

Restrict picking to the geometry actually displayed under current scope,
visibility, and line budget. Cache static pick geometry and avoid global hull
triangle scans in the module explorer. Apply the same visibility decision to
labels, rendering, and picking.

Use visible import relations to prioritize line rendering in focused spatial
views. When a global line budget omits relations, report the number omitted;
do not imply the displayed lines represent the entire dependency graph.

Measure interactive frame time separately from headless layout steps and tour
capture. Target responsive local exploration with 200 nodes at 30 FPS on the
available machine, and record measured median/p95 rather than treating that
target as a guaranteed cross-platform performance claim.

## Verification and acceptance

1. A tiny graph containing an umbrella group focuses a selected module's direct
   imports/dependents without expanding unrelated group members.
2. Selecting `Mathlib.Topology.Basic` in the actual dataset shows three imports
   and four dependents, with eight total vertices before filters. Search selects
   that single module. All seven arrows agree with the source/target attributes.
3. `Archive`, the first vertex, does not become a fake hub for all star hyperedges.
   Star/clique focus, hull visibility, selection, and vertex status remain correct.
4. Two-hop imports and dependents are independently checked against a hand-built
   directed fixture. Cycle, duplicate-edge, self-loop, missing-center, and empty
   graph fixtures terminate and report sensible scopes.
5. Paths preserve direction. Reverse-only connectivity is not reported as a
   dependency path. Filters and truncation remain explicit.
6. Unrendered lines cannot be picked. Small visible neighborhoods have labels
   even when the loaded graph has thousands of vertices.
7. Changing the effective centroid force changes star-layout motion; inactive
   spring controls are absent. Small hull opacity values survive UI and session
   round trips. Frozen layouts do not rebuild unchanged hull geometry.
8. Existing coauthorship, bipartite, clique, hot-reload, embedding, session, and
   tour tests pass. Direction or path metadata changes invalidate scene caches.
9. Run `cargo fmt --all -- --check`, `cargo test --workspace`, workspace clippy,
   the Python exporter tests, and a release build. Verify the available installed
   Rust toolchain satisfies locked dependencies without changing the declared
   Rust floor as an unrelated fix.
10. Launch and visually inspect the native module view with the actual dataset;
    measure its responsiveness and verify navigation, filters, expansion, path
    tracing, and transfer to spatial mode. Report any inability to exercise the
    window separately from automated verification.

## Delivery

Keep changes in the current managed worktree. Provide the launch command, a
brief description of the implemented behavior, and verification results. Do not
publish, merge, or alter the user's primary checkout as part of this work.
