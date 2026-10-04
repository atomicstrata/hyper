# Hyper ecosystem comparison and roadmap

Follow-up: the [October 3 measured comparison](benchmarks/ecosystem-2026-10-03/README.md) runs a documented subset of the proposed benchmark. This research brief retains its original research-date observations; the HIF implementation and measured follow-up supersede the corresponding future-work descriptions.

Research date: October 2, 2026, America/Lima. Project baseline: commit `024f51716406f9e33244638e313ea761e2b13ed8`. Purpose: positioning and roadmap decisions.

**Recommendation:** position Hyper as an embeddable Rust toolkit for exploring group relationships in native 3D, with explicit hyperedge membership and integration into a host application. Prioritize interoperability, useful inspection, and responsiveness before expanding the visual effects. This is a positioning hypothesis supported by the comparison below; adoption and user preference still need validation.

The ecosystem already offers hypergraph analysis, interactive hypergraph viewers, 3D graph components, and research on simplifying dense diagrams. Hyper's opportunity lies in the combination of its Rust API, renderer-independent scene, native viewer, and host integration. Neither hyperedge hulls nor 3D force layouts alone establish differentiation.

## Scope and evidence

This review covers 14 alternatives, plus Hyper: dedicated hypergraph libraries and viewers, and general graph tools users could adopt through an incidence projection. An incidence projection represents each hyperedge as a separate node connected to its members. It can preserve group identity; converting every group into pairwise connections alone can obscure it. The distinction has longstanding research support. [Ouvrard et al., 2017](https://arxiv.org/abs/1707.00115)

Capabilities come from project documentation, repositories, research papers, and a local source audit. Community findings come from public issue reports, maintainer discussions, and Reddit accounts. These are a purposeful sample, not a representative survey. Competitor applications were not installed or benchmarked in this review. “Not documented” means the reviewed sources did not establish the capability, rather than proving its absence. Recommendations and competitive implications are explicitly judgments.

Existing Hyper CSV measurements were independently recalculated, but the viewer was not rerun. Published performance numbers use different workloads and machines; they do not support a speed ranking. The [benchmark protocol](benchmark-protocol.md) defines a fair follow-up experiment.

Follow-up: the [October 3 benchmark run](benchmarks/ecosystem-2026-10-03/README.md) executes the separate scientific, browser and native tracks. Its measured results supersede the performance hypotheses in this research brief; platform and adapter limitations remain explicit.

## What Hyper currently provides

| Area | Observed implementation | Implication for positioning |
|---|---|---|
| Integration | Bevy-free `hyper-viz`; optional Bevy viewer and plugin; JSON and builder APIs | Suitable for embedding in Rust applications with custom renderers |
| Hypergraph model | Stable vertex and hyperedge IDs; unordered memberships; optional weights and opaque attributes in the wire model | Explicit group identity, but no first-class directed incidences or temporal model |
| Layout and drawing | CPU Barnes–Hut 3D layout; bipartite, clique, and centroid projections; lines and member hulls | Multiple ways to interpret the same memberships; projection choice strongly changes workload |
| Exploration | Vertex, segment, and hull picking; lasso; local search; neighborhood isolation; selected member lists | Useful exploration foundation; task completion needs evaluation |
| Host search | Optional `SearchProvider` returns paginated rows and a result scene from outside the current view | Can explore a bounded slice of a larger host dataset; this is an integration hook, not a bundled database |
| Live updates | File watch and scene channel; positions retained by stable ID; selection and focus remapped | Existing continuity mechanisms should be documented and assessed before adding replacements |
| Inspection | Scene and inspector expose ID, kind, label, status, and members; arbitrary `attrs` and weights are not carried into scene records | A host's evidence, dates, provenance, and quantitative attributes are unavailable to the stock inspector |
| Distribution | Git/path dependencies; crates have `publish = false`; no packaged browser product | Current evaluation requires a development environment |

Sources: [README](../README.md), [API](API.md), [schema](../crates/hyper-viz/src/schema.rs), [projection](../crates/hyper-viz/src/project.rs), [scene](../crates/hyper-viz/src/scene.rs), [inspection](../crates/hyper-viz-bevy/src/inspect.rs), [host search](../crates/hyper-viz-bevy/src/search.rs), [reload](../crates/hyper-viz-bevy/src/graph.rs), and [selection remapping](../crates/hyper-viz-bevy/src/interaction.rs).

Two qualifications matter. The hull algorithm caps input at 24 sampled member positions. Consequently, a large rendered shell is an approximation, while the membership list remains authoritative. Convex enclosure can also include unrelated nodes spatially: geometry alone cannot certify membership. See [hull implementation](../crates/hyper-viz/src/hull.rs). Also, the README's statement that reload clears selection is inconsistent with the current remapping implementation. Correcting that documentation belongs in release preparation.

## Dedicated hypergraph comparison

“3D” distinguishes a freely navigable spatial viewer from a layered scientific plot. HIF is the community Hypergraph Interchange Format.

| Project | Environment | Hyperedge representation | 3D support | Exploration and integration | Interchange and installation | Competitive implication for Hyper |
|---|---|---|---|---|---|---|
| **Hyper** | Rust library and native Bevy app | Explicit memberships; member hulls; incidence/clique/centroid projections | Native spatial viewer | Search, lasso, picking, focus; host scene updates and optional host search | `hypergraph.v1`; git/path dependencies | Candidate strength: embeddable native exploration; gaps: interoperability, metadata inspection, distribution |
| [HyperNetX](https://github.com/pnnl/HyperNetX) | Python scientific library | Explicit hypergraphs; Matplotlib visualization | Stock native 3D viewer not established | Analysis and notebook workflows; interactive widget is separate | HIF ecosystem; PyPI | Better treated as an analysis partner and data source than a library to replace |
| [HyperNetX Widget](https://github.com/pnnl/hypernetx-widget) | JavaScript and Python/Jupyter | Euler-style, bipartite, and dual views | 3D not documented | Pin/drag, brush, member expansion, tables, attribute coloring, supernodes | `hnxwidget`; documented notebook compatibility restrictions | Closest baseline for exploration features; demonstrates that selection and hull-like boundaries are existing capabilities |
| [XGI](https://github.com/xgi-org/xgi) | Python and Matplotlib | Hypergraphs, simplicial complexes, directed hypergraphs; polygons and hulls | Layered 3D plotting by edge order | Scientific analysis and configurable figures | HIF read/write and XGI-DATA; PyPI | Strong research workflow; Hyper can complement it with native interaction |
| [Hypergraphx](https://github.com/HGX-Team/hypergraphx/blob/main/README.md) | Python analysis library | Weighted, directed, temporal, multiplex and signed interactions | Native spatial viewer not established in reviewed docs | Measures, filtering, communities, dynamics and representations | HIF read/write; PyPI; dataset catalog | Much broader analysis/model semantics; avoid recreating that breadth |
| [SimpleHypergraphs.jl](https://github.com/pszufe/SimpleHypergraphs.jl) | Julia | Explicit hypergraphs with several plotting options | Comparable spatial viewer not established | Julia scientific workflow; optional HyperNetX plotting bridge | HIF tutorials; Julia package manager | Potential interoperability partner; some plotting paths require Python dependencies |
| [HyperGodot](https://github.com/ElsevierSoftwareX/SOFTX-D-25-00370) | Godot/GDScript desktop viewer | 2D convex hulls, weighted and grouped edges | Reviewed implementation uses a 2D camera | Layout choices, node dragging, incident-edge filtering, group comparison | Tagged text format; source and Windows-tested export | Direct competitor for interactive hull exploration; grouped/temporal comparison already exists elsewhere |
| [HGPolyVis](https://github.com/peterdanieloliver/HGPolyVis/blob/main/README.md) | Windows desktop research application | Polygon representation; simplification and layout methods | Comparable 3D viewer not documented | Research focuses on simplifying dense structures and their duals | CSV and `.er` layout files; Windows 11 executable | Serious baseline for readability and multiscale exploration; narrower platform availability |
| [PAOHVis](https://www.aviz.fr/Research/paohvis) | Browser research tool | Horizontal vertex rows and vertical hyperedge connections across time | 2D temporal view | Ordered temporal exploration; published case studies | CSV with time slots; web demo and local web server | Better reference for temporal interpretation than a force-layout animation |

Additional source details: XGI's [visualization tutorial](https://xgi.readthedocs.io/en/stable/api/tutorials/focus_5.html) documents hull and multilayer drawing. HGX's [read/write API](https://hypergraphx.readthedocs.io/en/latest/api/hypergraphx.readwrite.html) documents HIF. The [HIF project](https://github.com/HIF-org/HIF-standard) specifies incidences, optional incidence weights and attributes, and head/tail direction. HyperGodot's [SoftwareX publication](https://www.sciencedirect.com/science/article/pii/S235271102500281X) identifies the code repository, GPLv3 license, and Windows-tested export; source execution uses Godot 4.3+. The publisher page was partially accessible through search; its full text was unavailable, so no quantitative findings from it are used here.

## General graph substitutes

These tools can display an incidence graph, but their stock node-link data models do not establish overlapping hyperedges as independently inspectable filled sets. An adapter or custom rendering work is needed. This is an architectural comparison, not a claim that such extensions are impossible.

| Project | Environment and view | Established strengths | Work needed for the Hyper use case | Strategic implication |
|---|---|---|---|---|
| [3d-force-graph](https://github.com/vasturiano/3d-force-graph) | JavaScript, Three.js/WebGL, 3D | Custom objects, click/focus examples, node dragging, direction indicators, dynamic data | Incidence conversion; hyperedge surfaces and membership-specific interactions | Strong substitute for web distribution and generic 3D; CPU layout plus GPU rendering is not unique to Hyper |
| [Graphia](https://graphia.app/guide/section2/2_first_start_up.html) | Desktop, interactive 2D/3D | Attribute tables, search, layout export/reload; [clustering and filtering](https://graphia.app/guide/section3/2_transforms.html) | Hypergraph conversion and explicit group semantics | Strong baseline for native analytical usability; substantial inspection capabilities to learn from |
| [Gephi](https://gephi.org/) | Desktop network analysis; Gephi Lite offers a web alternative | Layout algorithms, attribute styling, filtering, metrics, publication exports | Incidence conversion or another hypergraph-specific extension | Competes for analyst attention through a complete workflow |
| [Cytoscape.js](https://js.cytoscape.org/index.html) | Browser interactive graph library | Data-driven styling, selection, graph algorithms and compound nodes | Incidence adapter and hyperedge interaction; compound hierarchy alone cannot represent arbitrary overlapping membership | Strong baseline for integrating visualization into a web application |
| [Sigma.js](https://www.sigmajs.org/) | Browser renderer with Graphology | Graph rendering, customization and examples for neighborhood search | Incidence adapter and custom group drawing; analytical workflow assembled by host | Useful 2D web baseline, particularly for accessible search-driven exploration |
| [egui_graphs](https://github.com/blitzarx1/egui_graphs) | Rust/egui widget; native and WASM | Pluggable layouts, labels, click/select/hover/drag, styling hooks | Incidence representation and custom hyperedge drawing; comparable stock 3D not documented | Rust integration and WASM are already available for ordinary graphs; README explicitly says development is inactive |

## Community evidence and adoption

### Public ecosystem signals

The counts below were observed on public repository pages during this research. Crawled snapshots vary in freshness; these are approximate visibility signals, not synchronized API counts, active-user counts, or maintenance scores.

| Project | Observed stars / forks | Stronger context than popularity alone |
|---|---:|---|
| [HyperNetX](https://github.com/pnnl/HyperNetX) | 717 / 117 | Named PNNL development team, tutorials, scientific use and HIF participation |
| [XGI](https://github.com/xgi-org/xgi) | 255 / 51 | Research package with datasets, mailing list, Zulip and contributions; public issues include requests posted in 2026 |
| [HyperNetX Widget](https://github.com/pnnl/hypernetx-widget) | 23 / 9 | Dedicated smaller viewer project; its compatibility must be assessed separately from HNX core |
| [SimpleHypergraphs.jl](https://github.com/pszufe/SimpleHypergraphs.jl) | 88 / 15 | Julia package, published research and explicit HIF workflow |
| [3d-force-graph](https://github.com/vasturiano/3d-force-graph) | About 6,400 / 1,000 | Broad examples and public troubleshooting; general graphs, not a hypergraph adoption measure |
| [egui_graphs](https://github.com/blitzarx1/egui_graphs) | 705 / 77 | Significant interest, but current README says the project is not in active development |

Hyper's external adoption is **unmeasured here**. The local release checklist and unpublished crate settings establish packaging status, not repository visibility, user demand, or lack of users. Do not compare its unknown adoption with these counts as zero.

### User problems found in public discussion

| Evidence | Observed concern | Interpretation for Hyper | Confidence and limit |
|---|---|---|---|
| [HNX Widget issue 143](https://github.com/pnnl/HyperNetX/issues/143), January 2024; [current widget docs](https://hypernetx.readthedocs.io/en/latest/widget.html) | A reviewer could not render the widget in JupyterLab. Docs still specify Notebook 6.5.x and no JupyterLab support | A reliable “load my data and inspect it” path can matter more than additional drawing styles | Concrete historical report, now closed; current documentation supports the compatibility restriction, not a claim that every installation fails |
| [HNX issue 171](https://github.com/pnnl/HyperNetX/issues/171), October 2025 | Reporter describes import failure without internet because a schema was downloaded | Bundle schemas and test offline import when adding HIF | A reported failure; reproduction and present code status were not verified |
| [React Force Graph issue 483](https://github.com/vasturiano/react-force-graph/issues/483), December 2023 | About 10k nodes / 80k edges lag while moving the camera; filtering a neighborhood helps | Measure frozen-scene navigation independently of force calculation; extend bounded exploration | Specific workload and user observation, not a universal capacity limit |
| [Three Force Graph discussion 44](https://github.com/vasturiano/three-forcegraph/discussions/44), April–May 2026 | Label-heavy visualization causes performance concern; maintainer suggests prewarming/freezing layout and fewer geometries | Benchmark labels, rendering, layout, and picking separately; show detail on demand | Maintainer and reporter differ on perceived performance, reinforcing the need for controlled tests |
| [Reddit analyst thread](https://www.reddit.com/r/datascience/comments/17212yf), October 2023 | User wants clickable node/edge attributes and attribute-based color; later prefers Cytoscape desktop | Arbitrary attribute inspection and styling are plausible evaluation criteria | One self-report; not a current Gephi defect audit |
| [Reddit graph-view thread](https://www.reddit.com/r/Rag/comments/1qblw1k/best_knowledge_graph_graph_view/), January 2026 | Original poster says 10k nodes is sufficient; finding a particular node is the harder problem | Measure find-and-explain tasks alongside FPS; Hyper's existing search and focus are relevant | Anecdotal adjacent use case; disregard unsupported capacity claims in replies |
| [XGI issue listing](https://github.com/xgi-org/xgi/issues), request 707, April 2026 | Listing requests heterogeneous multilayer hypergraphs and directed layer visualization | Model semantics deserve investigation after core exploration is reliable | Only issue title/listing was accessible; body and demand breadth were not verified |

This evidence suggests demand for inspection, compatibility, and manageable views. It does **not** demonstrate that users prefer 3D, that a browser release will create adoption, or that Hyper outperforms the alternatives.

### Communities to validate with

Start with researchers already producing hypergraphs: [HNX discussions](https://github.com/pnnl/HyperNetX/discussions), [XGI's community and Zulip links](https://github.com/xgi-org/xgi), the [HIF maintainers](https://github.com/HIF-org/HIF-standard), and Julia users working through SimpleHypergraphs. For visualization methods, use PAOHVis and HGPolyVis publications and associated research groups as technical references. Rust/egui and Bevy users are a second audience for embedding needs; their integration interest would not itself validate hypergraph analytical value.

Suggested validation, not conducted: recruit 5–8 people across scientific analysis and Rust host development. Have each bring an actual dataset and perform the same membership, search, overlap, and evidence-inspection tasks. Ask which existing tool they use and where they would switch. No community messages were sent in this research.

## Performance evidence

### Existing Hyper measurements

The checked-in Mathlib workload has 9,384 vertices, 37,356 import pairs, 7,550 dependency groups, **44,906 total hyperedge records**, and 118,066 memberships. The distinction avoids comparing “groups” in one tool with all edges in another. Its largest group has 8,532 members.

Hardware: Apple M5 Pro, 24 GB RAM, Metal, release build, 1920×1080, star-centroid projection. Each recorded run contains 300 measured frames, split into 150 without hulls and 150 with hulls. Figures below reproduce the repository's rounded-rank percentile calculation from raw CSV samples.

| Layout | Drawing | Median frame | p95 frame | 1000 / median frame time |
|---|---|---:|---:|---:|
| Frozen | Nodes + all import lines | 28.985 ms | 33.778 ms | 34.50 FPS |
| Frozen | Plus dependency-group hull fills | 41.042 ms | 52.086 ms | 24.37 FPS |
| One force step per frame | Nodes + all import lines | 24.906 ms | 31.751 ms | 40.15 FPS |
| One force step per frame | Plus dependency-group hull fills | 94.452 ms | 165.965 ms | 10.59 FPS |

Sources: [measurement notes](mathlib-demo.md), [manifest](media/mathlib-manifest.json), [frozen CSV](benchmarks/mathlib-frozen.csv), [moving CSV](benchmarks/mathlib-live.csv), [report calculation](../crates/hyper-viz-bevy/src/showcase.rs).

These measurements were recorded on demo changes based on `9ebe6d6`, before newer main changes were integrated. **They are historical measurements, not verification of current HEAD.** Frame intervals include update/render/presentation scheduling. The scripted tour disables picking and the ordinary interactive HUD, the two phases use different camera views, and there is one run per mode. The faster live no-hull median does not establish a simulation speedup. The MP4's encoding rate is independent of live throughput. Current hull code already caches geometry and periodically rebuilds topology; proposals must account for that implementation.

### Published external evidence

| Tool or method | Available evidence | What it supports | What remains unmeasured |
|---|---|---|---|
| Cytoscape.js | January 2025 developer tests on M1 MacBook Pro/Chrome: ~1,200 nodes / 16k edges, ~20 canvas FPS vs >100 WebGL FPS; ~3,200 nodes / 68k edges, ~3 vs ~10 FPS | Renderer choice and workload strongly affect interaction | Same data/hardware/styles as Hyper; present release behavior |
| 3d-force-graph family | Public reports about rendering and label overhead | Useful hypotheses for benchmark design | A controlled equivalent hypergraph comparison |
| HGPolyVis research | Simplification/layout work and structure preservation evaluation | Scalability must include interpretability at reduced detail | Equivalent native 3D frame-time measurements |
| Graphia | Product documentation claims visualization of millions of data points and relationships | A reason to include it as a scale-oriented substitute | A verified FPS envelope under a matched hypergraph workload |
| HNX, XGI, HGX, SimpleHypergraphs, HyperGodot, PAOHVis | Documented features, examples, and/or research use | Credible functional comparison candidates | Comparable end-to-end performance numbers in this review |

Sources: [Cytoscape.js developer measurements](https://blog.js.cytoscape.org/2025/01/13/webgl-preview/), [rendering discussion](https://github.com/vasturiano/three-forcegraph/discussions/44), [Scalable Hypergraph Visualization](https://arxiv.org/abs/2308.05043), [Structure-Aware Simplification](https://arxiv.org/abs/2407.19621), [Graphia product claims](https://graphia.app/). These rows deliberately retain provenance and limitations instead of presenting an unsupported leaderboard.

## Positioning and roadmap priorities

### Recommended position

Proposed description: **“An embeddable Rust hypergraph explorer for native 3D applications, with explicit group membership, search, and live host integration.”**

The best initial audience hypothesis is developers building tools that already own hypergraph data and need a view into it. Researchers using Python or Julia become reachable through adapters. The Mathlib atlas is useful evidence of a real dataset, but import groups are a modeling choice and direction is not currently visible. It should not imply theorem dependencies or directed reasoning support.

### Prioritized roadmap

These priorities are judgments based on observed gaps and the evidence above. Gates are proposed acceptance criteria, not measured results or delivery promises.

| Priority | Proposed work | Evidence and rationale | Suggested acceptance gate |
|---|---|---|---|
| P0 | Carry arbitrary attributes and weights into inspection; add attribute search/filter and selected-subgraph export | Wire model already has these fields; stock scene drops them; attribute inspection appears in community requests and substitutes | User can find a record by an attribute, inspect its evidence and exact members, and export the selected records without losing metadata |
| P0 | Add a scoped HIF adapter and reference Python/Julia export examples | Existing HIF adoption supplies datasets and workflow compatibility | Round-trip a documented supported subset, preserve isolated vertices and edge IDs, report unsupported incidence direction/weights explicitly, operate offline |
| P0 | Rebenchmark current HEAD and profile live hull updates | Historical live hull p95 is ~166 ms; current caching means optimizations need fresh profiles | Publish repeated runs with interaction enabled, CPU/GPU stages separated, settings and commit pinned; meet an agreed responsiveness target |
| P1 | Improve membership legibility and bounded views | Approximate hulls are not exact membership evidence; search/focus and host result scenes already exist | Selected-group member lists and highlight agree; large groups and approximation are visible; preserve context while moving between result slices |
| P1 | Make evaluation easy and docs accurate | Git/path-only installation; README disagrees with current reload behavior; widget community reports show setup friction | A new user loads a sample and their own supported file with documented steps; distribute native binaries if maintainers authorize release |
| P1 | Add a complementary 2D or incidence-focused mode | Existing alternatives and simplification research offer readable non-spatial views | Evaluate against 3D on identical membership and overlap tasks before broad implementation |
| P2 | Investigate directed incidences, temporal comparison and richer semantic models | HGX, HIF, PAOHVis and the XGI request show these needs are real but distinct | Choose a user segment and define meaning for head/tail, time, and incidence properties before changing schema |
| Conditional | Browser or notebook delivery | Most scientific users already work outside Rust; web substitutes are readily embedded | Build a small feasibility prototype only after pilots show distribution is the adoption blocker; evaluate GPU, picking, build size and host search behavior |

For HIF, recognize the semantic mismatch: it supports properties and direction on individual incidences, while Hyper stores only member IDs. A conversion that silently flattens head/tail or incidence attributes would misrepresent data. Also, `from_json_str` uses default-empty fields and has no HIF detection: feeding raw HIF into the existing loader could deserialize to an empty Hyper graph. Input-format detection and clear errors should be part of the adapter design. This is a source-based inference from the [loader](../crates/hyper-viz/src/io/mod.rs) and [schema](../crates/hyper-viz/src/schema.rs), not a reproduced test result.

### Competitive risks and choices

The most plausible threat is an established scientific library improving interactive web exploration while retaining its datasets and user workflows. A second threat is a host team adapting a mature ordinary-graph component because installation and customization are easier. Hyper needs to win a particular workflow, not the broadest feature count.

Keep its domain-agnostic library boundary. Integrate metrics calculated elsewhere instead of adding a large analysis engine. Defer more visual effects until task studies show better interpretation. Avoid “first hypergraph viewer,” “only Rust/3D option,” “exact set boundaries,” and performance superiority claims: the reviewed evidence does not establish them. An early HG-DB [fork](https://github.com/mahsaabdolahnezhad/hypergraph-visualization) also advertises a Rust normalization core with layered browser 3D, although its implementation and adoption were not independently verified; this is a watch item rather than a mature benchmark baseline.

## Research gaps and refresh triggers

No cross-tool benchmark, user interview, download audit, or comprehensive contributor/issue-response analysis was performed. Repository counts and open/closed statuses may lag current state. Some sources were only accessible as issue listings or publisher search excerpts; those limitations are attached to their claims. “Current ecosystem” here means the sources available on the research date, not an exhaustive inventory.

Refresh this analysis when Hyper changes distribution or schema, HIF evolves, viewer compatibility changes, or a pilot user evaluates another tool. For roadmap decisions, revisit the same task-oriented benchmark after a material performance or exploration change. The foundational [2021 visualization survey](https://arxiv.org/abs/2107.13936) is useful taxonomy and evaluation context, but should not be treated as a complete 2026 market survey.
