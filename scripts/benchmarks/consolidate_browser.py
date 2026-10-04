#!/usr/bin/env python3
"""Replace the discarded Sigma refresh track with its corrected redraw runs."""
import argparse
import hashlib
import json
from pathlib import Path


def key(row):
    return tuple(row[k] for k in ('dataset', 'tool', 'mode', 'repetition'))


def consolidate(original, replacement):
    old = json.loads(original.read_text())
    new = json.loads(replacement.read_text())
    for field in ('os', 'release', 'arch', 'ramBytes', 'node', 'browser', 'headed', 'viewport', 'packageVersions', 'lockfileSha256'):
        assert old['environment'][field] == new['environment'][field], f'Environment changed: {field}'
    for field in ('seed', 'warmup', 'samples', 'repetitions', 'modes', 'jobTimeoutMs'):
        assert old['protocol'][field] == new['protocol'][field], f'Protocol changed: {field}'
    assert old['datasets'] == new['datasets'], 'Input hashes/provenance changed'
    expected = {key(r) for r in old['runs'] if r['tool'] == 'sigma'}
    actual = {key(r) for r in new['runs']}
    assert len(new['runs']) == len(actual) == len(expected) == 35
    assert expected == actual
    assert all(r['tool'] == 'sigma' for r in new['runs'])
    old['runs'] = [r for r in old['runs'] if r['tool'] != 'sigma'] + new['runs']
    assert len(old['runs']) == len({key(r) for r in old['runs']}) == 105
    old['consolidation'] = {'reason': 'Sigma full refresh reprocessed unchanged graph data during camera motion. Replaced with public scheduleRender cached redraw; original Sigma timings excluded.',
                            'original_report_sha256': hashlib.sha256(original.read_bytes()).hexdigest(),
                            'replacement_report_sha256': hashlib.sha256(replacement.read_bytes()).hexdigest(),
                            'replacement_started_utc': new['startedUTC'],
                            'replacement_environment': new['environment'],
                            'replacement_order': new['order'],
                            'order_limit': 'Corrected Sigma ran in a separate block after native work; original mixed-tool order applies only to retained Cytoscape/3D cases.'}
    return old


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--original', type=Path, default=Path('target/benchmark/browser-original/results.json'))
    parser.add_argument('--replacement', type=Path, default=Path('target/benchmark/browser-sigma-final/results.json'))
    parser.add_argument('--output', type=Path, default=Path('target/benchmark/browser-final/results.json'))
    args = parser.parse_args()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(consolidate(args.original, args.replacement), indent=2) + '\n')
