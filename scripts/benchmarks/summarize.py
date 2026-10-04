#!/usr/bin/env python3
"""Summarize raw observations; median of per-run quantiles, never inverse median FPS."""
import argparse
import csv
import json
from pathlib import Path
import statistics
from collections import defaultdict

def quantile(values, q):
    values = sorted(values)
    if not values: return None
    position = (len(values) - 1) * q
    left = int(position)
    right = min(left + 1, len(values) - 1)
    return values[left] + (values[right] - values[left]) * (position - left)

def median(values):
    values = [v for v in values if v is not None]
    return statistics.median(values) if values else None

def load(path):
    return json.loads(path.read_text()) if path.exists() else []

def summarize(root):
    rows = []
    for track in ['native', 'native-star', 'native-interactive', 'core']:
        groups = defaultdict(list)
        for row in load(root / track / 'raw.json'):
            if not row.get('warmup'): groups[(row['dataset'], row['hulls'], row['live'])].append(row)
        for (dataset, hulls, live), group in groups.items():
            ok = [r for r in group if r['status'] == 'ok']
            row = {'track': track, 'tool': 'Hyper', 'dataset': dataset, 'mode': ('live' if live else 'frozen') + ('+hulls' if hulls else ''), 'attempts': len(group), 'successful_runs': len(ok), 'statuses': sorted({r['status'] for r in group}), 'peak_rss_mib_median': median([r['process_peak_rss_bytes'] / 2**20 for r in ok if r.get('process_peak_rss_bytes') is not None])}
            if track != 'core':
                traces = [r['measurements']['frame_interval_ms'] for r in ok]
                row.update(frame_p50_ms=median([quantile(t, .5) for t in traces]), frame_p95_ms=median([quantile(t, .95) for t in traces]), frame_p95_run_min_ms=min([quantile(t,.95) for t in traces], default=None), frame_p95_run_max_ms=max([quantile(t,.95) for t in traces], default=None), average_fps=median([1000 * len(t) / sum(t) for t in traces if sum(t)]), update_commit_ms=median([r['measurements']['update_commit_ms'][0] for r in ok]), update_next_frame_ms=median([r['measurements'].get('update_next_frame_ms') for r in ok]), layout_step_p50_ms=median([quantile(r['measurements']['layout_step_ms'][:r['measurements']['frames']],.5) for r in ok]))
            else:
                for key in ['native_parse_ms', 'hif_parse_validate_ms', 'hif_viewer_conversion_ms', 'bipartite_projection_ms', 'all_hulls_cpu_ms']:
                    row[key] = median([r['measurements'][key] for r in ok])
                row['layout_step_p50_ms'] = median([quantile(r['measurements']['layout_step_ms'], .5) for r in ok])
            rows.append(row)
    browser_path = root / 'browser-final/results.json'
    browser = load(browser_path if browser_path.exists() else root / 'browser/results.json')
    if browser:
        groups = defaultdict(list)
        for r in browser['runs']: groups[(r['dataset'],r['tool'],r['mode'])].append(r)
        for (dataset,tool,mode), group in groups.items():
            ok = [r for r in group if 'trace' in r and r.get('status','ok') in ['ok','completed']]
            traces = [r['trace']['rafIntervalMs'] for r in ok]
            rows.append({'track': 'browser', 'tool': tool, 'dataset': dataset, 'mode': mode, 'attempts': len(group), 'successful_runs': len(ok), 'statuses': sorted({r.get('status','ok') for r in group}), 'frame_p50_ms': median([quantile(t,.5) for t in traces]), 'frame_p95_ms': median([quantile(t,.95) for t in traces]), 'average_fps': median([1000 * len(t) / sum(t) for t in traces if sum(t)]), 'frame_p95_run_min_ms': min([quantile(t,.95) for t in traces],default=None), 'frame_p95_run_max_ms': max([quantile(t,.95) for t in traces],default=None), 'load_wall_ms': median([r.get('loadWallMs') for r in ok]), 'update_commit_ms': median([r.get('incremental',{}).get('commandMs') for r in ok]), 'update_two_raf_ms': median([r.get('incremental',{}).get('twoRafFeedbackSurrogateMs') for r in ok]), 'failure_stages': sorted({r.get('stage','unknown') for r in group if r.get('status')=='failed'})})
    groups = defaultdict(list)
    science_path = root / 'science/final-raw.json'
    for r in load(science_path if science_path.exists() else root / 'science/results/raw.json'):
        if not r.get('warmup'): groups[(r['dataset'],r['tool'],r['mode'])].append(r)
    for (dataset,tool,mode), group in groups.items():
        measured = [r for r in group if r['status'] in ['ok','unsupported_records'] and not r.get('failure')]
        row = {'track': 'scientific', 'tool': tool, 'dataset': dataset, 'mode': mode, 'attempts': len(group), 'successful_runs': len(measured), 'statuses': sorted({r['status'] for r in group}), 'exact_model_records': all(r.get('correctness',{}).get('exact_canonical_roundtrip',False) for r in measured) if measured else False, 'peak_rss_mib_median': median([r['process_peak_rss_bytes'] / 2**20 for r in measured])}
        for key in ['tool_import','file_read','json_parse','canonical_validate','adapter_construct','artist_setup','agg_raster_draw','pixel_buffer_copy','png_export']:
            row[key + '_ms'] = median([r.get('stages_ms',{}).get(key) for r in measured])
        rows.append(row)
    groups = defaultdict(list)
    for row in load(root / 'graphia/raw.json'):
        if not row.get('warmup'): groups[row['dataset']].append(row)
    for dataset, group in groups.items():
        ok = [r for r in group if r['status'] == 'ok']
        rows.append({'track':'desktop-import', 'tool':'Graphia', 'dataset':dataset,
                     'mode':'headless-import-save', 'attempts':len(group), 'successful_runs':len(ok),
                     'statuses':sorted({r['status'] for r in group}),
                     'import_save_wall_ms':median([r['process_wall_seconds'] * 1000 for r in ok]),
                     'peak_rss_mib_median':median([r['process_peak_rss_bytes'] / 2**20 for r in ok if r.get('process_peak_rss_bytes') is not None]),
                     'exact_incidence_records':all(r['correctness']['exact_incidence_and_node_attributes'] for r in ok) if ok else False})
    for run in load(root / 'hypergodot/raw.json'):
        measurements = run.get('measurements',{})
        repetitions = measurements.get('repetitions',[])
        traces = [r['frame_ms'] for r in repetitions]
        rows.append({'track':'desktop-group', 'tool':'HyperGodot', 'dataset':run['dataset'],
                     'mode':'frozen-camera-motion', 'attempts':5, 'successful_runs':len(repetitions),
                     'statuses':[run['status']], 'frame_p50_ms':median([quantile(t,.5) for t in traces]),
                     'frame_p95_ms':median([quantile(t,.95) for t in traces]),
                     'frame_p95_run_min_ms':min([quantile(t,.95) for t in traces],default=None),
                     'frame_p95_run_max_ms':max([quantile(t,.95) for t in traces],default=None),
                     'average_fps':median([1000*len(t)/sum(t) for t in traces if sum(t)]),
                     'whole_session_peak_rss_mib':run.get('process_peak_rss_bytes',0)/2**20 if run.get('process_peak_rss_bytes') else None,
                     'exact_parser_memberships':measurements.get('exact_parser_memberships_and_rendered_group_count',False),
                     'viewport_size':measurements.get('viewport_size'),
                     'render_target_size':measurements.get('render_target_size'),
                     'matched_requested_viewport':measurements.get('matched_requested_viewport',False),
                     'startup_construction_ms':measurements.get('startup_construction_ms')})
    return sorted(rows, key=lambda r: (r['track'],r['dataset'],r['tool'],r['mode']))

if __name__ == '__main__':
    p = argparse.ArgumentParser()
    p.add_argument('--input', type=Path, default=Path('target/benchmark'))
    p.add_argument('--output', type=Path, default=Path('docs/benchmarks/ecosystem-2026-10-03'))
    args = p.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    rows = summarize(args.input)
    (args.output / 'summary.json').write_text(json.dumps(rows, indent=2) + '\n')
    keys = sorted({key for r in rows for key in r})
    with (args.output / 'summary.csv').open('w', newline='') as f:
        writer = csv.DictWriter(f, fieldnames=keys, lineterminator="\n")
        writer.writeheader()
        for row in rows: writer.writerow({k: json.dumps(v) if isinstance(v,list) else v for k,v in row.items()})
