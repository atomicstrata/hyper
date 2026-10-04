#!/usr/bin/env python3
"""Render comparison tables from measured summaries, retaining failure counts."""
import argparse
import json
from pathlib import Path


def number(value, digits=2):
    return '—' if value is None else f'{value:,.{digits}f}'


def table(lines, headings, rows):
    lines += ['| ' + ' | '.join(headings) + ' |', '|' + '|'.join(['---'] * len(headings)) + '|']
    lines += ['| ' + ' | '.join(str(value) for value in row) + ' |' for row in rows]
    lines.append('')


def render(rows):
    lines = ['# Measured ecosystem comparison — October 3, 2026', '',
             f'Exploratory results on an Apple M5 Pro / 24 GiB Mac: {sum(r["attempts"] for r in rows)} measured attempts across {len(rows)} configurations. Five attempts per case unless noted. '
             'The same graph records and seeded coordinates are used through explicit adapters. '
             'Scientific figures, 2D browser views and 3D native rendering have different work and are separate tracks.', '',
             'Read the [method and limitations](methodology.md), [environment](environment.json), '
             '[complete CSV](summary.csv), [structured summary](summary.json) and [raw evidence](evidence/). '
             'The [harness guide](../../../scripts/benchmarks/README.md) provides pinned dependencies and rerun commands.', '',
             '## Findings to use for roadmap decisions', '',
             'Model fidelity is the clearest differentiator in this run: exact group identity, isolated nodes and empty groups need explicit checks at adapter boundaries. '
             'Rendering results are conditional on completion. Hyper had intermittent whole-job stalls, including one on a small fixture; their cause remains unresolved. '
             'The source audit found a quadratic hub-status lookup worth profiling, but it has not isolated the cause of those stalls.', '',
             'Sigma camera measurements use its public cached redraw path. Earlier full-refresh camera measurements were discarded and replaced; '
             'the diagnostic evidence is retained separately. Comparisons below keep representations and timing boundaries explicit.', '',
             '## Scientific figure workflows', '',
             'Times exclude the separately measured top-level tool import, file I/O and JSON parsing. Setup creates the artists and includes any lazy drawing-module initialization; raster draws the Agg canvas. '
             'PNG compression is a separate stage. A 120-second timeout covers the whole worker. '
             'HGX uses its stock 12-round smoothing, HNX rubber-band outlines, XGI convex hulls.', '']
    scientific = [r for r in rows if r['track'] == 'scientific' and r['mode'] == 'plot' and r['dataset'] != 'correctness']
    table(lines, ['Dataset', 'Tool', 'Completed / attempts', 'Exact model records', 'Construct ms', 'Artist setup ms', 'Raster ms', 'PNG ms', 'Worker peak MiB'],
          [[r['dataset'], r['tool'], f"{r['successful_runs']}/{r['attempts']}", 'yes' if r['exact_model_records'] else 'no / unavailable',
            *[number(r.get(k)) for k in ['adapter_construct_ms','artist_setup_ms','agg_raster_draw_ms','png_export_ms','peak_rss_mib_median']]] for r in scientific])
    lines += ['XGI preserves all model records in every tested input. HNX omits isolated vertices and empty groups. '
              'HGX identifies groups by member tuple and merges parallel groups: Diseasome becomes 481 groups instead of 903, '
              'and the arity-1,000 fixture becomes one group instead of 200. Its lossy timings cannot rank equivalent work. '
              'Original records stored in opaque metadata do not repair those native identities.', '',
              '## Browser incidence rendering', '',
              'All incidences are rendered, forces and labels are off, and the camera moves. Values describe rAF callback cadence; '
              'they are not GPU durations or verified presented frames. Average callbacks/s uses total elapsed intervals. '
              'The p95 range spans per-run p95 values. “Completed” requires sampling and the shared update to finish within the 90-second job budget.', '']
    browser = [r for r in rows if r['track'] == 'browser']
    table(lines, ['Dataset', 'Tool', 'Completed / attempts', 'p50 ms', 'p95 ms', 'p95 run range ms', 'Avg callbacks/s', 'Update command ms', 'Failed stage'],
          [[r['dataset'], r['tool'], f"{r['successful_runs']}/{r['attempts']}", number(r.get('frame_p50_ms')), number(r.get('frame_p95_ms')),
            f"{number(r.get('frame_p95_run_min_ms'))}–{number(r.get('frame_p95_run_max_ms'))}", number(r.get('average_fps')),
            number(r.get('update_commit_ms')), ', '.join(r.get('failure_stages',[])) or '—'] for r in browser])
    lines += ['Sigma uses WebGL 2D, Cytoscape uses Canvas2D, and 3d-force-graph uses perspective spheres and WebGL. '
              'The corrected Sigma block and original Cytoscape block recorded different small-case callback cadences (roughly 60 versus 120 callbacks/s); display refresh was not locked, so their rate ratio cannot rank rendering efficiency. No link sampling is applied. Update command timings include different adapter commit strategies and sometimes unchanged-record work; see the method. A failed batch has no completed rendering score; its timeout is not a single-frame measurement.', '',
              '## Hyper native renderer', '',
              'Physical 1920 × 1080, AutoVsync, 120 warmup and 300 recorded application frames. '
              'HUD and picking remain installed. Basic modes disable labels; `native-interactive` enables the default Capped policy (count threshold 250, with selected/hovered exemptions). '
              'Frozen modes use fixed coordinates; live modes perform one layout step per frame. '
              'All lines are enabled. Hulls cap input at 24 vertices and therefore approximate large groups.', '']
    native = [r for r in rows if r['track'].startswith('native')]
    table(lines, ['Track / dataset', 'Mode', 'Completed / attempts', 'p50 app interval ms', 'p95 app interval ms', 'p95 run range ms', 'Avg app frames/s', 'Update commit ms', 'Next app interval ms', 'Peak MiB'],
          [[r['track'] + ' / ' + r['dataset'], r['mode'], f"{r['successful_runs']}/{r['attempts']}",
            number(r.get('frame_p50_ms')), number(r.get('frame_p95_ms')),
            f"{number(r.get('frame_p95_run_min_ms'))}–{number(r.get('frame_p95_run_max_ms'))}",
            *[number(r.get(k)) for k in ['average_fps','update_commit_ms','update_next_frame_ms','peak_rss_mib_median']]] for r in native])
    lines += ['`native` is bipartite incidence rendering; `native-star` and `native-interactive` use group/star rendering. '
              'Star draws pair lines for dyadic groups and hulls for larger groups; its scene has no spring links. Star without hulls is an ablation that hides larger groups, not a full-group rendering score. '
              'Update commit and the next application interval exclude file-watch polling and parse/I/O; they are not input-to-visible latency. '
              'Native and browser settings, geometry and vsync differ, so these tables do not establish a native-versus-browser speed ratio.', '',
              '## Hyper parsing, conversion and geometry', '',
              'Fresh release processes, one discarded warmup and five measured runs. HIF schema validation is offline. '
              'Solver timings use the bipartite scene; hull timings use the production 24-vertex cap. '
              'Worker RSS includes multiple retained documents for correctness checks.', '']
    core = [r for r in rows if r['track'] == 'core']
    table(lines, ['Dataset', 'Native parse ms', 'HIF parse / validate ms', 'HIF conversion ms', 'Projection ms', 'All hulls CPU ms', 'Layout step p50 ms', 'Peak MiB'],
          [[r['dataset'], *[number(r.get(k)) for k in ['native_parse_ms','hif_parse_validate_ms','hif_viewer_conversion_ms','bipartite_projection_ms','all_hulls_cpu_ms','layout_step_p50_ms','peak_rss_mib_median']]] for r in core])
    lines += ['## Desktop alternatives', '',
              'Graphia measures stock headless import/save including process startup, then independently checks saved memberships and node attributes. '
              'The adapter retains incidence topology and arbitrary `attrs`, but omits document metadata and top-level native label/kind/status/weight fields; XYZ are attributes, and Graphia saves this graph as directed. '
              'These are import workflow timings, not rendering FPS.', '']
    table(lines, ['Dataset', 'Tool', 'Completed / attempts', 'Import + save ms', 'Exact incidence records', 'Peak MiB'],
          [[r['dataset'],r['tool'],f"{r['successful_runs']}/{r['attempts']}",number(r.get('import_save_wall_ms')),
            'yes' if r['exact_incidence_records'] else 'no / unavailable',number(r.get('peak_rss_mib_median'))]
           for r in rows if r['track']=='desktop-import'])
    lines += ['HyperGodot measures its production 2D group drawing with camera motion, fixed XY coordinates and stock labels/UI. '
              'It disables vsync and uses different node/hull geometry. Its five repetitions share one process, with a new warmup each. '
              'A disposable source copy removes a missing unused autoload. Original IDs are mapped to safe tokens/line-order group IDs; '
              'weights and group categories are normalized for this fixture. Stock HiDPI is enabled and the physical render texture is verified at 1920 × 1080; earlier clamped-window timings are excluded.', '']
    table(lines, ['Dataset', 'Tool', 'Completed / attempts', 'p50 ms', 'p95 ms', 'Avg frames/s', 'Whole-session peak MiB', 'Render target pixels'],
          [[r['dataset'],r['tool'],f"{r['successful_runs']}/{r['attempts']}",number(r.get('frame_p50_ms')),number(r.get('frame_p95_ms')),
            number(r.get('average_fps')),number(r.get('whole_session_peak_rss_mib')),r.get('render_target_size')]
           for r in rows if r['track']=='desktop-group'])
    lines += ['## Coverage and remaining experiments', '']
    table(lines, ['Alternative', 'Measured scope / reason for no comparable score'], [
        ['HyperNetX, XGI, Hypergraphx','Model construction and scientific figure workflow; no GUI FPS claim'],
        ['3d-force-graph, Cytoscape.js, Sigma','Frozen full-incidence camera motion and scripted update'],
        ['HyperGodot','Native 2D group drawing on the moderate dataset'],
        ['Graphia','Headless incidence import/save; common render coordinates not established'],
        ['HNX Widget','Installation and dictionary-input smoke test; public position API does not freeze all hubs; isolated vertices omitted'],
        ['HGPolyVis','Supplied Windows-only executable; requires a Windows run'],
        ['SimpleHypergraphs.jl','Julia runtime verified; package timings not collected'],
        ['PAOHVis','Needs a temporal dataset and temporal exploration task'],
        ['Gephi','Outside this automated render harness; requires a separate plugin/task comparison'],
        ['egui_graphs','Embedding framework reference; no matched application benchmark']])
    lines += ['Human task accuracy, dense-diagram readability, layout convergence, GPU completion, input-to-visible latency and '
              'other machines remain unmeasured. Metadata retention does not establish that a viewer exposes or filters it.', '',
              '## Roadmap implications', '',
              '1. **Interoperability:** keep original HIF documents and stable group identities through scientific workflows. '
              'The identity-loss cases demonstrate why an incidence adapter needs correctness gates before timing.',
              '2. **Inspection and filtering:** expose provenance, arbitrary attributes and membership before expanding visual effects. '
              'Rendering every relation does not establish readability or a successful scientific task.',
              '3. **Responsiveness:** profile the confirmed quadratic hub-status scan in `animation::node_motion` first; the existing `GraphLayout::hub_status` cache offers a direct replacement. Then use the frozen/live split and update traces to separate solver, hull rebuild/upload and redraw costs. '
              'Add a controlled input-to-visible study before advertising latency; keep hull approximation and line budgets explicit.',
              '4. **Installation:** ship verified wheels and a clear separately installed viewer path. The widget and desktop packaging probes '
              'show that dependency/version friction can block evaluation before performance matters.', '',
              'These priorities are interpretations of this exploratory run, not proof of adoption or a universal winner.', '']
    return '\n'.join(lines)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--directory', type=Path, default=Path('docs/benchmarks/ecosystem-2026-10-03'))
    args = parser.parse_args()
    (args.directory / 'README.md').write_text(render(json.loads((args.directory / 'summary.json').read_text())))
