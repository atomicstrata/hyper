#!/usr/bin/env python3
"""Prepare a disposable copy of pinned HyperGodot, without running it."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

HERE = Path(__file__).resolve().parent

def prepare(source, dataset, destination):
    graph = json.loads(dataset.read_text())
    ids = {v['id'] for v in graph['vertices']}
    members = {v for e in graph['hyperedges'] for v in e['vertices']}
    if ids != members or any(not e['vertices'] for e in graph['hyperedges']):
        raise ValueError('Stock HyperGodot cannot represent isolated vertices or empty hyperedges; use a compatible dataset, not silent omission.')
    if any(len(e['vertices']) != len(set(e['vertices'])) for e in graph['hyperedges']):
        raise ValueError('Duplicate memberships require an explicit normalization policy.')
    if any(set(e['vertices']) - ids for e in graph['hyperedges']):
        raise ValueError('Unknown member ID')
    if destination.exists():
        raise ValueError('Destination already exists; choose a fresh disposable copy.')
    shutil.copytree(source, destination, ignore=shutil.ignore_patterns('.git', '.godot', 'Export', '.import'))
    project = destination / 'project.godot'
    project.write_text(project.read_text().replace('MaterialIconsDB="*res://addons/material-design-icons/icons/icons.gd"\n', '').replace('[display]\n', '[display]\n\nwindow/dpi/allow_hidpi=true\n'))
    mapping = {v['id']: f'n{i}' for i, v in enumerate(graph['vertices'])}
    # Safe token IDs preserve a reversible mapping even for IDs with spaces.
    text = ''.join(' '.join(mapping[v] for v in e['vertices']) + ' #WEIGHT: 1 #GROUP baseline\n' for e in graph['hyperedges'])
    (destination / 'patterns.txt').write_text(text)
    (destination / 'benchmark_input.json').write_text(json.dumps(graph))
    (destination / 'benchmark_idmap.json').write_text(json.dumps(mapping))
    shutil.copy2(HERE / 'hypergodot_scene.gd', destination / 'benchmark_scene.gd')
    shutil.copy2(HERE / 'hypergodot_runner.gd', destination / 'benchmark_runner.gd')
    manifest = {'tool': 'HyperGodot', 'source_commit': subprocess.check_output(['git', '-C', str(source), 'rev-parse', 'HEAD'], text=True).strip(),
                'dataset': str(dataset.resolve()), 'sha256': hashlib.sha256(dataset.read_bytes()).hexdigest(),
                'vertices': len(ids), 'hyperedges': len(graph['hyperedges']), 'memberships': sum(len(e['vertices']) for e in graph['hyperedges']),
                'adaptations': ['remove missing unused MaterialIconsDB autoload', 'reversible safe token IDs and line-order hyperedge IDs', 'unit weights and one baseline group', 'fixed XY projection of canonical 3D positions', 'override startup layout; production read_file/create_nodes/populate_edge_data/drawing', 'scripted camera and wall-clock process-boundary frame intervals'],
                'node_radius': 30, 'edge_width': 1, 'point_count': 4, 'vsync': 'disabled by upstream project', 'metric': 'wall-clock frame intervals, not GPU timestamps'}
    (destination / 'benchmark_manifest.json').write_text(json.dumps(manifest, indent=2))
    print(json.dumps({'project': str(destination.resolve()), 'manifest': manifest}))

if __name__ == '__main__':
    p = argparse.ArgumentParser()
    p.add_argument('--source', type=Path, default=Path('/tmp/hyper-benchmark-competitors/hypergodot'))
    p.add_argument('--dataset', type=Path, required=True)
    p.add_argument('--destination', type=Path, required=True)
    a = p.parse_args()
    prepare(a.source, a.dataset, a.destination)
