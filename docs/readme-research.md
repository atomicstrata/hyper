# README communication research

Reviewed October 5, 2026 (America/Lima), while consolidating documentation PRs
[#10](https://github.com/atomicstrata/hyper/pull/10) and
[#11](https://github.com/atomicstrata/hyper/pull/11).

## Purpose and method

The README must help a scientist explore a supported dataset and help a developer
integrate Hyper without first learning the entire implementation. Success means a
reader can identify the tool’s purpose, choose the correct package, run a complete
example, and understand the representation’s limits.

This review examined ten primary-source READMEs: four hypergraph projects, three
graph visualization projects, and three scientific or spatial visualization
projects with native viewers or multiple language interfaces. The latter are
communication references, not claims of feature equivalence. Sources below are
pinned to the most recent commit modifying each README on its default branch at
the time of retrieval. Their linked assets and tutorials were inspected as README
entry points; competitor installation instructions and applications were not run.

The review considered six questions: what problem the introduction communicates;
how a reader chooses a workflow; how installation handles prerequisites and
package names; whether the first example supplies its data and execution context;
how semantics, limitations, and research use are explained; and where API,
contributor, and maintenance details live. Observations describe the documents;
the recommendations are editorial judgments. No user study or conversion-rate
measurement was performed, and this is not a performance comparison.

## Project-by-project findings

| Primary source | Comparison role | Observed communication | Applied lesson for Hyper |
|---|---|---|---|
| [xgi-org/xgi](https://github.com/xgi-org/xgi/blob/b8164bca1ce6b8b5c613223c33365dc61e432f28/README.md) | Scientific hypergraphs | Defines the model and supported operations, links a gallery and datasets, and provides citation and contribution routes. Installation leads to external user guides rather than an inline first example. | Explain group relationships and ecosystem fit; retain visible scientific and contributor routes. Supply a complete first example in Hyper itself. |
| [pnnl/HyperNetX](https://github.com/pnnl/HyperNetX/blob/17534c3806a3666e421881b3fb314e787024a6b5/README.md) | Scientific hypergraphs | Connects HIF to other libraries and provides Colab tutorials. Release history, maintainer credits, environment choices, and developer tooling occupy substantial space before and after installation. | Explain HIF interoperability, but keep release history, build variants, and long analysis outside the landing README. |
| [HGX-Team/hypergraphx](https://github.com/HGX-Team/hypergraphx/blob/a71d3c2c2f97fc8d9e7492a633a87668d6b4ab67/README.md) | Scientific hypergraphs | Introduces group interactions with familiar domains, then installs and constructs a small graph before showing metadata and analysis. Separate tutorials and citation guidance support deeper use. | Use a concrete coauthorship example before API details. Keep advanced semantics in the HIF guide rather than copying HGX’s analysis scope. |
| [pnnl/hypernetx-widget](https://github.com/pnnl/hypernetx-widget/blob/11a9f59551e131928367fee092d8bbbb80bde6b0/README.md) | Interactive hypergraphs | Starts with a browser demo, then a notebook example that defines its own graph and shows the resulting widget. Explains selection and alternate representations with screenshots and interactions. | Offer an immediate demo and explicit first interactions. State that Hyper opens a native window; put the full interaction reference in the viewer guide. |
| [graphia-app/graphia](https://github.com/graphia-app/graphia/blob/d7c339c5cc94575edfad39cdfb49b84ea86e1f10/README.md) | Desktop graph exploration | Pairs a short purpose statement with a video and domain-specific applications. Links a user guide and separates help from building, but gives source-build commands rather than a short packaged launch walkthrough. | Keep the visual demo and recognizable use cases. Make the packaged viewer command directly available, with source builds later. |
| [vasturiano/3d-force-graph](https://github.com/vasturiano/3d-force-graph/blob/2a901f53eb9b427635c74f20bfdf0cfc4b3812bc/README.md) | Browser 3D graph component | Shows a linked visual preview and a large gallery whose demos link to source. Follows a short integration sketch with extensive API tables. The sketch uses placeholder container and data arguments. | Keep a reproducible visual example and concise developer entry point. Use defined inputs and move exhaustive API material into dedicated guides. |
| [jacomyal/sigma.js](https://github.com/jacomyal/sigma.js/blob/a923c1676c4f035a6f1b98a465f0d4bd39df6b34/README.md) | Browser graph component | Names both required packages, then shows imports, nodes, an edge, and renderer creation. Separates project use from local development and links documentation, Storybook, and a demo. | Make package roles explicit and show a small constructed dataset. Separate consuming the library from contributing or building it. |
| [nmwsharp/polyscope](https://github.com/nmwsharp/polyscope/blob/63c09627e8252050e7be50e9de9cdeb40cc42799/README.md) | Scientific native 3D viewer | Explains its structure/quantity model and integration philosophy, then gives parallel C++ and Python examples and citation guidance. Example arrays are supplied by the reader. | Give Python and Rust distinct routes and explain the core/viewer boundary. Improve first-run completeness by constructing all example data. |
| [napari/napari](https://github.com/napari/napari/blob/bb3fdfda8567fc84284b1b721553cf1e6bf1da3c/README.md) | Scientific Python desktop viewer | Offers a uvx trial before environment installation, tells users how to open sample data, and distinguishes interactive-shell use from scripts that need an event loop. Includes roadmap, citation, and help routes. | Lead with a standalone uvx demo and retain a project workflow. Include viewer.wait() in the Python script and state where the window opens. |
| [rerun-io/rerun](https://github.com/rerun-io/rerun/blob/44bb79b9c65357f3ee57c2fdf616281243269dfc/README.md) | Rust and Python visualization SDK | Connects purpose to concrete workflows, separates language onboarding from viewer installation, and documents active development, limitations, and research citation. Its short code taste assumes pre-existing arrays. | Explain SDK/viewer installation and evolving scope. Preserve measured limitations without reproducing a benchmark report or assuming missing example data. |

## What the comparison changes

### One entry point per reader task

A single README has three real entry points here: try the desktop viewer, use
Python with a dataset, and integrate the Rust core. These are tasks rather than
skill levels. A scientist may write Rust and a developer may only need the CLI.
Top-level jump links let either reader choose immediately. The viewer trial comes
first because it demonstrates the project without a checkout or a new Python
project. The documented Python 3.12 command selects a supported interpreter rather
than relying on whichever system Python happens to be installed.

The alternative of leading with Cargo and the architecture diagram makes every
new user perform developer setup. Leading with a generic pip command leaves the
package/command mismatch unresolved. Leading with a comparison or benchmark table
asks the reader to evaluate implementation details before they have seen a group.
Those materials remain available in the existing guides.

### Complete examples and visible results

The Python example defines four authors, two papers, and every incidence. The
shared author makes overlapping membership concrete, while saving HIF connects the
example to a real file workflow. It ends with show() and wait(), explains the
expected window, and supplies uv run explore.py. Readers need no unpublished
fixture or scientific-library installation. Existing data replaces the constructor
with HifDocument.load().

The Rust example is a complete main() with a small group, projection, layout step,
and printed node count. The dependency snippet uses the repository because the
Rust crates are not published. The core has no window dependency; optional Bevy
integration has a separate explanation and API link. Source-build instructions
remain available in an expandable section.

### Scientific trust depends on boundaries

A compelling screenshot cannot establish faithful interchange, exact hull
membership, analysis capabilities, or interactive performance. Hyper’s opening
explains membership exploration and later places three concise boundaries beside
the data workflow: viewer conversion accepts undirected memberships; large hulls
are approximate; and workload-specific benchmark results need their methods and
completion counts. Full scientific values belong to the original HifDocument.
The HIF guide remains authoritative for incidence properties, identity mapping,
precision changes, and metadata interpretation.

The README positions Hyper alongside scientific analysis libraries through
supported HIF exchange. It does not imply that their in-memory objects can be
passed directly to show(). It distinguishes the Python API’s current interchange
and launcher surface from Rust’s layout API, and states that the native window is
not a notebook widget. The package metadata identifies the software as alpha, so
the README calls it early-stage without promising a stability policy.

Scientific projects often provide publication citations or archived software
identifiers. Hyper has no CITATION.cff or verified project DOI in this checkout.
Rather than invent one, the README asks researchers to record the version or
commit, dataset, projection, and settings. A formal citation policy can be added
when maintainers establish one.

### Link depth to a maintained guide

PR #10’s useful change is progressive detail: essential actions on the landing
page, complete controls, schema fields, drawing behavior, and session persistence
in docs/VIEWER.md. PR #11’s useful change is installation reliability: the viewer
distribution for uvx, the matching viewer extra for Python projects, supported
Python, and uv run for scripts. The consolidation keeps both, then aligns the
viewer and package guides so navigation never returns a reader to conflicting
launch instructions.

A compact package-name table resolves distribution, import, extra, and executable
roles. pip remains available for existing Python environments. Contributor checks
stay in CONTRIBUTING.md, and launcher internals stay in the Python/HIF guide.
The README retains one visual demo, with descriptive alternate text and a caption
that identifies the settled layout. Competitor marketing, capacity claims, and
visual assets were not copied.

## Consolidation and verification criteria

| Requirement | Result to review |
|---|---|
| Keep #10’s simpler entry page | Purpose, demo, three reader paths, short explanations, and guide links |
| Preserve #10’s viewer reference | Complete controls, drawing rules, schema example, and sessions in docs/VIEWER.md |
| Keep #11’s correct uv launch | uvx selects hypergraph-viz-viewer; Python projects install hypergraph-viz[viewer] |
| Preserve #11’s complete Python path | Project creation, example data, saved HIF, viewer lifetime, and script command |
| Align companion documentation | Python package and desktop package guides retain uv and pip workflows |
| Support Rust integration | Git dependency, compilable headless example, optional viewer boundary, source path |
| Support scientific interpretation | Interchange/viewing distinction, approximate hulls, benchmark link, reproducibility note |
| Avoid regressions in navigation | Check local paths, section anchors, code fences, and retained documentation |
| Check executable examples | Run published CLI help/version, Python construction/save/reload/conversion, and Rust example |

This document records the editorial evidence. The consolidated PR records the
actual verification results, including any checks that did not exercise a GUI.
