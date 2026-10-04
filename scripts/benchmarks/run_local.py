#!/usr/bin/env python3
"""Sequential fresh-process Hyper benchmarks; never overlap with other tracks."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import random
import re
import signal
import subprocess
import time

def run(args):
    output = Path(args.output)
    output.mkdir(parents=True, exist_ok=True)
    datasets = args.datasets.split(',')
    rng = random.Random(20261003)
    if args.track == 'core':
        specs = [(name, False, False) for name in datasets]
    else:
        specs = [(name, hulls, False) for name in datasets for hulls in (False, True)]
        specs += [(name, hulls, True) for name in args.live_datasets.split(',') if name in datasets for hulls in (False, True)]
        if args.only_hulls: specs = [spec for spec in specs if spec[1]]
    schedule = []
    for repetition in range(-1 if args.track == 'core' else 0, 5):
        batch = list(specs)
        rng.shuffle(batch)
        schedule += [(repetition, *item) for item in batch]
    rows = []
    for order, (repetition, dataset, hulls, live) in enumerate(schedule):
        raw = output / f'{dataset}-hulls{int(hulls)}-live{int(live)}-{repetition}.json'
        source = Path('target/benchmark/datasets') / (dataset + '.json')
        executable = 'target/release/examples/ecosystem_core' if args.track == 'core' else 'target/release/ecosystem-benchmark'
        command = ['/usr/bin/time', '-l', executable, str(source), str(raw)]
        if args.track == 'native':
            command += [str(int(hulls)), str(int(live)), '300', '-', args.projection, str(int(args.labels))]
        started = time.time()
        row = {'dataset': dataset, 'hulls': hulls, 'live': live, 'repetition': repetition, 'warmup': repetition == -1, 'order': order, 'started_unix': started, 'sha256': hashlib.sha256(source.read_bytes()).hexdigest()}
        raw.unlink(missing_ok=True)
        process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, start_new_session=True, env={**os.environ, 'HYPER_VIZ_SESSION': 'off'})
        try:
            stdout, stderr = process.communicate(timeout=180)
            row.update(status='ok' if process.returncode == 0 and raw.exists() else 'failed', returncode=process.returncode)
            row['stderr'] = stderr[-12000:]
            rss = re.search(r'(\d+)\s+maximum resident set size', stderr)
            row['process_peak_rss_bytes'] = int(rss.group(1)) if rss else None
            if raw.exists():
                row['measurements'] = json.loads(raw.read_text())
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            stdout, stderr = process.communicate()
            row.update(status='watchdog_timeout', timeout_seconds=180)
            row['stderr'] = stderr[-12000:]
        row['process_wall_seconds'] = time.time() - started
        rows.append(row)
        (output / 'raw.json').write_text(json.dumps(rows, indent=2) + '\n')
        print(args.track, order + 1, '/', len(schedule), dataset, 'hulls', int(hulls), 'live', int(live), repetition, row['status'], flush=True)
    metadata = {'track': args.track, 'seed': 20261003, 'repetitions': 5, 'fresh_process_per_run': True, 'warmup': '5 solver steps plus1 discarded run per dataset' if args.track == 'core' else '120 render frames per run', 'schedule': schedule, 'commit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(), 'binary_sha256': hashlib.sha256(Path(executable).read_bytes()).hexdigest(), 'conditions': os.environ.get('BENCHMARK_CONDITIONS', 'unrecorded'), 'peak_memory': '/usr/bin/time -l process RSS high-water; bytes on macOS; includes startup', 'build_profile': 'release'}
    metadata.update(projection=args.projection, labels=args.labels, only_hulls=args.only_hulls,
                    live_datasets=args.live_datasets.split(','))
    (output / 'metadata.json').write_text(json.dumps(metadata, indent=2) + '\n')

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('track', choices=['core', 'native'])
    parser.add_argument('--datasets', default='moderate,large,high-arity-10,high-arity-100,high-arity-1000,scientific,mathlib')
    parser.add_argument('--output', required=True)
    parser.add_argument('--projection', choices=['bipartite','star'], default='bipartite')
    parser.add_argument('--live-datasets', default='moderate')
    parser.add_argument('--labels', action='store_true')
    parser.add_argument('--only-hulls', action='store_true')
    run(parser.parse_args())
