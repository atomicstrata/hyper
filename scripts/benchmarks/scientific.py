#!/usr/bin/env python3
"""Isolated scientific figure benchmark; canonical native JSON, never layout/FPS.

Install PINNED in a dedicated Python 3.12 environment. --validate performs one
untimed fixture per tool. Batch mode requires --run; parent controls contention.
Each observation is a new process; imports/startup and correctness are excluded
from named stage timings, but included in process high-water RSS. PNG compression
uses an already rasterized RGBA buffer, so export does not redraw the figure.
"""
import argparse
import csv
import hashlib
import importlib.metadata
import json
import os
from pathlib import Path
import platform
import random
import resource
import subprocess
import sys
import time
import traceback

PINNED = {'hypernetx': '2.4.3', 'xgi': '0.10.2', 'hypergraphx': '1.8.0',
          'matplotlib': '3.11.2', 'pillow': '12.3.0'}
TOOLS = ['hypernetx', 'xgi', 'hypergraphx']
SOURCES = {
    'hypernetx': 'https://hypernetx.readthedocs.io/en/latest/classes/classes.html',
    'xgi': 'https://xgi.readthedocs.io/en/stable/api/core/xgi.core.hypergraph.Hypergraph.html',
    'hypergraphx': 'https://hypergraphx.readthedocs.io/en/latest/api/hypergraphx.core.html',
}

def canonical(data):
    nodes = {v['id']: v for v in data['vertices']}
    edges = {e['id']: e for e in data['hyperedges']}
    if len(nodes) != len(data['vertices']) or len(edges) != len(data['hyperedges']):
        raise ValueError('Duplicate record IDs')
    node_ids = set(nodes)
    for e in edges.values():
        if len(set(e['vertices'])) != len(e['vertices']):
            raise ValueError('Duplicate incidence in ' + e['id'])
        if set(e['vertices']) - node_ids:
            raise ValueError('Unknown member ID in ' + e['id'])
    return nodes, edges


def construct(tool, data, records=None):
    nodes, edges = records if records is not None else canonical(data)
    if tool == 'hypernetx':
        import hypernetx as hnx
        graph = hnx.Hypergraph({k: e['vertices'] for k, e in edges.items()},
                             node_properties={k: {'canonical': v} for k, v in nodes.items()},
                             edge_properties={k: {'canonical': e} for k, e in edges.items()})
        graph.benchmark_meta = data.get('meta', {})
    elif tool == 'xgi':
        import xgi
        graph = xgi.Hypergraph()
        graph['canonical_meta'] = data.get('meta', {})
        for k, v in nodes.items():
            graph.add_node(k, canonical=v)
        for k, e in edges.items():
            # Use the public bulk API; actual empty-edge support is checked below.
            graph.add_edges_from([(e['vertices'], k, {'canonical': e})])
    else:
        from hypergraphx import Hypergraph
        graph = Hypergraph(weighted=False, hypergraph_metadata={'canonical_meta': data.get('meta', {})})
        for k, v in nodes.items():
            graph.add_node(k, metadata={'canonical': v})
        for e in edges.values():
            graph.add_edge(tuple(set(e['vertices'])), metadata={'canonical': e})
    return graph


def check(tool, graph, data):
    nodes, edges = canonical(data)
    actual_nodes, actual_edges, node_meta, edge_meta = set(), {}, {}, {}
    if tool == 'hypernetx':
        actual_nodes = set(graph.nodes)
        actual_edges = {k: set(v) for k, v in graph.incidence_dict.items()}
        # Element .properties recursively flattens nested dictionaries; the
        # public DataFrame view retains the original opaque JSON shape.
        node_props, edge_props = graph.nodes.to_dataframe, graph.edges.to_dataframe
        node_meta = {k: node_props.loc[k, 'misc_properties'].get('canonical') for k in actual_nodes}
        edge_meta = {k: edge_props.loc[k, 'misc_properties'].get('canonical') for k in actual_edges}
        meta = graph.benchmark_meta
    elif tool == 'xgi':
        actual_nodes = set(graph.nodes)
        actual_edges = {k: set(graph.edges.members(k)) for k in graph.edges}
        node_meta = {k: graph.nodes[k].get('canonical') for k in actual_nodes}
        edge_meta = {k: graph.edges[k].get('canonical') for k in actual_edges}
        meta = graph['canonical_meta']
    else:
        actual_nodes = set(graph.get_nodes())
        node_meta = {k: graph.get_node_metadata(k).get('canonical') for k in actual_nodes}
        for members in graph.get_edges():
            record = graph.get_edge_metadata(members).get('canonical', {})
            # HGX merges duplicate-edge metadata into lists. Recover opaque
            # records while separately reporting the loss of native multiplicity.
            for item in record if isinstance(record, list) else [record]:
                key = item.get('id', repr(members))
                actual_edges[key] = set(members)
                edge_meta[key] = item
        meta = graph.get_hypergraph_metadata().get('canonical_meta')
    missing_nodes = sorted(nodes.keys() - actual_nodes)
    missing_edges = sorted(edges.keys() - actual_edges.keys())
    wrong_members = [k for k in edges.keys() & actual_edges.keys()
                     if set(edges[k]['vertices']) != actual_edges[k]]
    bad_metadata = [k for k in nodes.keys() & actual_nodes if nodes[k] != node_meta[k]]
    bad_metadata += [k for k in edges.keys() & actual_edges.keys() if edges[k] != edge_meta[k]]
    extra_nodes = sorted(actual_nodes - nodes.keys())
    extra_edges = sorted(actual_edges.keys() - edges.keys())
    opaque_exact = not (missing_nodes or missing_edges or wrong_members or bad_metadata or extra_nodes or extra_edges) and meta == data.get('meta', {})
    native_edge_count = len(graph.get_edges()) if tool == 'hypergraphx' else len(actual_edges)
    exact = opaque_exact and native_edge_count == len(edges)
    return {'exact_canonical_roundtrip': exact, 'missing_nodes': missing_nodes,
            'opaque_record_values_preserved': opaque_exact,
            'native_hyperedge_count': native_edge_count,
            'native_multiplicity_preserved': native_edge_count == len(edges),
            'missing_edges': missing_edges, 'extra_nodes': extra_nodes, 'extra_edges': extra_edges,
            'wrong_memberships': wrong_members, 'wrong_metadata': bad_metadata,
            'graph_metadata_equal': meta == data.get('meta', {}),
            'native_edge_ids': tool != 'hypergraphx',
            'edge_id_storage': 'native IDs' if tool != 'hypergraphx' else 'canonical metadata; HGX uses member tuples',
            'empty_edges_expected': sum(not e['vertices'] for e in edges.values()),
            'isolated_nodes_expected': len(nodes.keys() - {m for e in edges.values() for m in e['vertices']}),
            'duplicate_member_occurrences': sum(len(e['vertices']) - len(set(e['vertices'])) for e in edges.values()),
            'membership_semantics': 'unordered unique-member sets; original record retained as opaque metadata'}


def figure(tool, graph, data):
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    pos = {v['id']: v['attrs']['position'][:2] for v in data['vertices']}
    fig, ax = plt.subplots(figsize=(19.2, 10.8), dpi=100)
    if tool == 'hypernetx':
        import hypernetx as hnx
        hnx.draw(graph, pos=pos, ax=ax, with_node_labels=False, with_edge_labels=False,
                 fill_edges=True, fill_edge_alpha=-0.85)
    elif tool == 'xgi':
        import xgi
        xgi.draw(graph, pos=pos, ax=ax, node_labels=False, hyperedge_labels=False,
                 hull=True, alpha=0.15)
    else:
        from hypergraphx.viz.draw_hypergraph import draw_hypergraph
        draw_hypergraph(graph, pos=pos, ax=ax, with_node_labels=False, hyperedge_alpha=0.15)
    xs, ys = zip(*pos.values())
    xpad = max(max(xs) - min(xs), 1) * 0.05
    ypad = max(max(ys) - min(ys), 1) * 0.05
    ax.set_xlim(min(xs)-xpad, max(xs)+xpad)
    ax.set_ylim(min(ys)-ypad, max(ys)+ypad)
    ax.set_aspect('equal', adjustable='box')
    ax.axis('off')
    return fig


def worker(args):
    import faulthandler
    faulthandler.dump_traceback_later(max(1, args.timeout - 10))
    result = {'tool': args.tool, 'dataset': Path(args.dataset).stem,
              'timed': not args.validate, 'status': 'failed', 'stages_ms': {},
              'versions': {p: importlib.metadata.version(p) for p in PINNED}}
    def stage(name, fn):
        start = time.perf_counter_ns()
        value = fn()
        if not args.validate:
            result['stages_ms'][name] = (time.perf_counter_ns() - start) / 1e6
        return value
    try:
        # Dependency imports are a separate cold-process stage, not construction.
        stage('tool_import', lambda: __import__(args.tool))
        import matplotlib
        matplotlib.use('Agg')
        import matplotlib.pyplot
        from PIL import Image
        raw = stage('file_read', lambda: Path(args.dataset).read_bytes())
        result['sha256'] = hashlib.sha256(raw).hexdigest()
        data = stage('json_parse', lambda: json.loads(raw))
        nodes, edges = stage('canonical_validate', lambda: canonical(data))
        result['seed'] = data.get('meta', {}).get('attrs', {}).get('benchmark_seed')
        result['counts'] = {'vertices': len(nodes), 'hyperedges': len(edges),
                            'memberships': sum(len(set(e['vertices'])) for e in edges.values()),
                            'maximum_arity': max((len(set(e['vertices'])) for e in edges.values()), default=0)}
        graph = stage('adapter_construct', lambda: construct(args.tool, data, (nodes, edges)))
        result['correctness'] = check(args.tool, graph, data)
        result['status'] = 'ok' if result['correctness']['exact_canonical_roundtrip'] else 'unsupported_records'
        result['plot_semantics'] = {'empty_edges_have_no_geometry': True,
                                    'hypergraphx_singleton_groups_have_no_group_geometry': args.tool == 'hypergraphx',
                                    'common_coordinates': 'canonical XY; Z discarded for this 2D track'}
        if args.plot:
            fig = stage('artist_setup', lambda: figure(args.tool, graph, data))
            stage('agg_raster_draw', fig.canvas.draw)
            png = Path(args.output).with_suffix('.png')
            # Pixel buffer retrieval is recorded independently; save has no draw.
            image = stage('pixel_buffer_copy', lambda: Image.frombytes('RGBA', fig.canvas.get_width_height(), fig.canvas.buffer_rgba().tobytes()))
            stage('png_export', lambda: image.save(png, dpi=(100, 100)))
            result['png'] = str(png)
            result['png_dimensions'] = list(image.size)
    except Exception as exc:
        result['status'] = 'failed'
        result['failure'] = {'type': type(exc).__name__, 'message': str(exc), 'traceback': traceback.format_exc()}
    finally:
        rss = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
        result['process_peak_rss_bytes'] = rss if sys.platform == 'darwin' else rss * 1024
        result['rss_scope'] = 'whole isolated worker, includes imports and validation'
        faulthandler.cancel_dump_traceback_later()
        Path(args.output).write_text(json.dumps(result, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--dataset', action='append')
    parser.add_argument('--datasets-dir', default='target/benchmark/datasets')
    parser.add_argument('--output', default='target/benchmark/science/results')
    parser.add_argument('--tool', choices=TOOLS)
    parser.add_argument('--worker', action='store_true')
    parser.add_argument('--plot', action='store_true')
    parser.add_argument('--validate', action='store_true')
    parser.add_argument('--run', action='store_true')
    parser.add_argument('--seed', type=int, default=20261003)
    parser.add_argument('--repetitions', type=int, default=5)
    parser.add_argument('--timeout', type=int, default=120)
    args = parser.parse_args()
    installed = {p: importlib.metadata.version(p) for p in PINNED}
    if installed != PINNED:
        parser.error(f'Pinned environment mismatch: {installed}')
    if args.worker:
        args.dataset = args.dataset[0]
        worker(args)
        return
    if not args.validate and not args.run:
        parser.error('Choose --validate (untimed) or --run (coordinated batch)')
    if args.run and args.repetitions < 5:
        parser.error('Protocol requires at least five repetitions')
    out = Path(args.output)
    out.mkdir(parents=True, exist_ok=True)
    datasets = [Path(p) for p in args.dataset] if args.dataset else sorted(Path(args.datasets_dir).glob('*.json'))
    datasets = [p for p in datasets if p.stem in ['correctness', 'moderate', 'large', 'high-arity-10', 'high-arity-100', 'high-arity-1000', 'scientific', 'mathlib']]
    if not datasets:
        parser.error('No canonical datasets found')
    rng = random.Random(args.seed)
    jobs = [(p, tool, mode) for p in datasets for tool in ([args.tool] if args.tool else TOOLS)
            for mode in (['construct', 'plot'] if p.stem in ['correctness', 'moderate', 'scientific'] else ['construct'])]
    rows = []
    schedule = []
    for repetition in range(-1, 1 if args.validate else args.repetitions):
        if args.validate and repetition == -1:
            continue
        rng.shuffle(jobs)
        for path, tool, mode in jobs:
            name = f'{path.stem}-{tool}-{mode}-{repetition}'
            destination = out / (name + '.json')
            command = [sys.executable, str(Path(__file__).resolve()), '--worker', '--tool', tool,
                       '--dataset', str(path), '--output', str(destination), '--timeout', str(args.timeout)]
            if mode == 'plot': command += ['--plot']
            if args.validate: command += ['--validate']
            env = dict(os.environ, MPLCONFIGDIR=str(out.resolve() / 'mpl-cache'),
                       OPENBLAS_NUM_THREADS='1', OMP_NUM_THREADS='1', MKL_NUM_THREADS='1',
                       XDG_CACHE_HOME=str(out.resolve() / 'xdg-cache'))
            schedule.append({'order': len(schedule), 'repetition': repetition, 'dataset': path.stem, 'tool': tool, 'mode': mode})
            destination.unlink(missing_ok=True)
            try:
                completed = subprocess.run(command, env=env, capture_output=True, text=True, timeout=args.timeout)
                row = json.loads(destination.read_text()) if destination.exists() else {'status': 'process_failed', 'returncode': completed.returncode}
                if completed.returncode != 0:
                    row.update(status='process_failed', returncode=completed.returncode)
                row['stderr'] = completed.stderr[-8000:]
            except subprocess.TimeoutExpired:
                row = {'status': 'watchdog_timeout', 'timeout_seconds': args.timeout}
            row.update(schedule[-1], warmup=repetition == -1)
            rows.append(row)
            (out / 'raw.json').write_text(json.dumps(rows, indent=2))
            print(name, row['status'], flush=True)
    metadata = {'seed': args.seed, 'warmup_per_combination': 0 if args.validate else 1,
                'repetitions': 1 if args.validate else args.repetitions, 'untimed_validation': args.validate,
                'platform': platform.platform(), 'python': sys.version, 'pins': PINNED,
                'installed_packages': {d.metadata['Name']: d.version for d in importlib.metadata.distributions()},
                'documentation': SOURCES, 'schedule': schedule,
                'settings': {'resolution': [1920,1080], 'dpi': 100, 'backend': 'Agg', 'labels': False,
                             'layout': 'none; canonical XYZ projected to XY', 'thread_limit': 1, 'hyperedge_alpha': 0.15,
                             'hypergraphx_smoothing_refinements': 12,
                             'hypergraphx_polygon_expansion_triangle': 2.5,
                             'hypergraphx_polygon_expansion_other': 1.8},
                'limitations': ['Different documented polygon/hull artists; no identical raster workload claim',
                                'No static figure vs interactive FPS ranking', 'Peak RSS includes imports',
                                'power/thermal/background conditions recorded by coordinator',
                                'HGX canonical edge IDs stored as metadata; native IDs differ']}
    (out / 'metadata.json').write_text(json.dumps(metadata, indent=2))
    with (out / 'raw.csv').open('w', newline='') as f:
        writer = csv.DictWriter(f, fieldnames=['dataset','tool','mode','repetition','warmup','status','process_peak_rss_bytes','stages_ms','correctness','failure'])
        writer.writeheader()
        for row in rows:
            writer.writerow({k: json.dumps(row.get(k)) if isinstance(row.get(k),(dict,list)) else row.get(k) for k in writer.fieldnames})

if __name__ == '__main__':
    main()
