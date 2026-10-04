#!/usr/bin/env python3
"""Run installed desktop competitors sequentially, with bounded subprocesses."""
import argparse
from collections import Counter
import gzip
import hashlib
import json
import math
import os
from pathlib import Path
import random
import re
import signal
import subprocess
import sys
import time

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from datasets import incidence
from prepare_graphia import prepare


def execute(command, timeout):
    started = time.perf_counter()
    process = subprocess.Popen(['/usr/bin/time', '-l', *command], stdout=subprocess.PIPE,
                               stderr=subprocess.PIPE, text=True, start_new_session=True)
    try:
        stdout, stderr = process.communicate(timeout=timeout)
        status = 'ok' if process.returncode == 0 else 'failed'
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
        stdout, stderr = process.communicate()
        status = 'watchdog_timeout'
    rss = re.search(r'(\d+)\s+maximum resident set size', stderr)
    return {'status': status, 'returncode': process.returncode,
            'process_wall_seconds': time.perf_counter() - started,
            'process_peak_rss_bytes': int(rss.group(1)) if rss else None,
            'stdout': stdout[-3000:], 'stderr': stderr[-12000:]}


def validate_graphia(path, graph):
    saved = json.loads(gzip.open(path, 'rb').read())
    assert saved[0]['appVersion'] == '5.5', 'Graphia version differs from pinned run'
    document = saved[1]
    names = {str(n['id']): n['value'] for n in document['nodeNames']}
    expected = incidence(graph)
    assert len(names) == len(expected['nodes'])
    assert len(set(names.values())) == len(names)
    assert set(names.values()) == {n['id'] for n in expected['nodes']}
    pairs = Counter((names[e['source']], names[e['target']]) for e in document['graph']['edges'])
    assert pairs == Counter((e['source'], e['target']) for e in expected['links'])
    data = document['userNodeData']
    columns = {key: value['values'] for vector in data['vectors'] for key, value in vector.items()}
    decoded = {names[str(native_id)]: {key: json.loads(columns[key][i]) for key in ('role', 'original_id', 'attrs')}
               for i, native_id in enumerate(data['ids'])}
    assert decoded == {n['id']: {key: n[key] for key in ('role', 'original_id', 'attrs')} for n in expected['nodes']}
    by_id = {n['id']: n for n in expected['nodes']}
    for i, native_id in enumerate(data['ids']):
        for axis in ('x', 'y', 'z'):
            assert math.isclose(float(columns[axis][i]), by_id[names[str(native_id)]][axis], rel_tol=1e-9, abs_tol=1e-6)
    return {'exact_incidence_and_node_attributes': True,
            'document_metadata_retained': False,
            'typed_native_display_fields_retained': False,
            'saved_as_directed': document['graph']['directed'],
            'coordinates_are_attributes_not_layout': True,
            'coordinate_attributes_retained': True,
            'vertices_and_hubs': len(names), 'memberships': sum(pairs.values())}


def main(args):
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    rows = []
    if args.tool == 'graphia':
        sources = {}
        for dataset in args.datasets.split(','):
            source = Path('target/benchmark/datasets') / (dataset + '.json')
            prepare(source, output / 'inputs')
            sources[dataset] = (source, json.loads(source.read_text()))
        schedule = []
        rng = random.Random(20261003)
        for repetition in range(-1, 5):
            batch = list(sources)
            rng.shuffle(batch)
            schedule.extend((repetition, dataset) for dataset in batch)
        for repetition, dataset in schedule:
            source, graph = sources[dataset]
            saved = output / 'inputs' / (dataset + '.graphia')
            saved.unlink(missing_ok=True)
            row = execute([args.executable, '--parameters', str(output / 'inputs' / (dataset + '.parameters.json')),
                           str(output / 'inputs' / (dataset + '.graphml'))], args.timeout)
            row.update(tool='Graphia', dataset=dataset, repetition=repetition, warmup=repetition == -1,
                       dataset_sha256=hashlib.sha256(source.read_bytes()).hexdigest())
            if row['status'] == 'ok':
                try:
                    row['correctness'] = validate_graphia(saved, graph)
                except Exception as error:
                    row.update(status='validation_failed', validation_error=repr(error))
            rows.append(row)
            (output / 'raw.json').write_text(json.dumps(rows, indent=2) + '\n')
            print('Graphia', dataset, repetition, row['status'], flush=True)
    else:
        saved = output / 'moderate.json'
        saved.unlink(missing_ok=True)
        row = execute([args.executable, '--path', str(args.project.resolve()), '--script', 'res://benchmark_runner.gd',
                       '--', '--output', str(saved), '--warmup', '120', '--frames', '300', '--repetitions', '5'], args.timeout)
        row.update(tool='HyperGodot', dataset='moderate', warmup=False)
        if row['status'] == 'ok' and saved.exists():
            row['measurements'] = json.loads(saved.read_text())
        elif row['status'] == 'ok':
            row.update(status='failed', reason='Process exited without evidence JSON')
        rows.append(row)
        (output / 'raw.json').write_text(json.dumps(rows, indent=2) + '\n')
        print('HyperGodot', row['status'], flush=True)
    metadata = {'tool': args.tool, 'executable': args.executable, 'timeout_seconds': args.timeout,
                'executable_sha256': hashlib.sha256(Path(args.executable).read_bytes()).hexdigest(),
                'conditions': os.environ.get('BENCHMARK_CONDITIONS', 'unrecorded'),
                'peak_memory': 'macOS /usr/bin/time -l whole-process RSS high-water in bytes',
                'fresh_process_per_repetition': args.tool == 'graphia',
                'repetitions': 5, 'seed': 20261003}
    (output / 'metadata.json').write_text(json.dumps(metadata, indent=2) + '\n')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('tool', choices=['graphia', 'hypergodot'])
    parser.add_argument('--executable', required=True)
    parser.add_argument('--project', type=Path)
    parser.add_argument('--datasets', default='moderate,scientific,mathlib')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--timeout', type=int, default=180)
    main(parser.parse_args())
