#!/usr/bin/env python3
"""Seeded canonical fixtures and lossless incidence projections (stdlib only)."""
import copy
import hashlib
import json
import math
from pathlib import Path
import random

SEED = 20261003

def positions(graph):
    rng = random.Random(SEED)
    for node in graph['vertices']:
        node.setdefault('attrs', {})['position'] = [round(rng.uniform(-500, 500), 6) for _ in range(3)]
    graph.setdefault('meta', {}).setdefault('attrs', {})['benchmark_seed'] = SEED
    return graph

def synthetic(name, n, m, arity, fixed=False):
    rng = random.Random(SEED)
    vertices = [{'id': f'n{i}', 'label': f'Node {i}', 'attrs': {'category': i % 5, 'nested': {'source': name}}} for i in range(n)]
    edges = []
    for i in range(m):
        k = min(n, arity if fixed else [2, 3, 4, arity, arity * 2][i % 5])
        members = rng.sample(range(n), k)
        edges.append({'id': f'g{i}', 'vertices': [f'n{j}' for j in members], 'attrs': {'category': i % 5}})
    return positions({'version': 'hypergraph.v1', 'meta': {'id': name, 'title': name, 'attrs': {'license': 'CC0-1.0'}}, 'vertices': vertices, 'hyperedges': edges})

def hif(graph):
    def attributes(record):
        # Retain all native display metadata as explicit attributes.
        return {**record.get('attrs', {}), **{k: record[k] for k in ('label', 'kind', 'status') if k in record}}
    return {'network-type': 'undirected', 'metadata': graph.get('meta', {}),
            'nodes': [{'node': v['id'], 'attrs': attributes(v), **({'weight': v['weight']} if 'weight' in v else {})} for v in graph['vertices']],
            'edges': [{'edge': e['id'], 'attrs': attributes(e), **({'weight': e['weight']} if 'weight' in e else {})} for e in graph['hyperedges']],
            'incidences': [{'node': v, 'edge': e['id']} for e in graph['hyperedges'] for v in e['vertices']]}

def incidence(graph):
    points = {v['id']: v['attrs']['position'] for v in graph['vertices']}
    nodes = [{'id': 'v:' + v['id'], 'role': 'vertex', 'original_id': v['id'], 'x': points[v['id']][0], 'y': points[v['id']][1], 'z': points[v['id']][2], 'attrs': v.get('attrs', {})} for v in graph['vertices']]
    links = []
    for e in graph['hyperedges']:
        members = e['vertices']
        p = [sum(points[v][axis] for v in members) / len(members) if members else 0 for axis in range(3)]
        nodes.append({'id': 'e:' + e['id'], 'role': 'hyperedge', 'original_id': e['id'], 'x': p[0], 'y': p[1], 'z': p[2], 'attrs': e.get('attrs', {})})
        links.extend({'id': e['id'] + ':' + str(i), 'source': 'e:' + e['id'], 'target': 'v:' + v, 'edge': e['id'], 'node': v} for i, v in enumerate(members))
    return {'nodes': nodes, 'links': links, 'meta': graph.get('meta', {})}

def changed(graph):
    result = copy.deepcopy(graph)
    all_ids = [v['id'] for v in result['vertices']]
    for e in result['hyperedges'][:math.ceil(len(result['hyperedges']) * .01)]:
        members = e['vertices']
        replacement = next((v for v in all_ids if v not in members), None)
        if replacement is not None:
            if members:
                members[0] = replacement
            else:
                members.append(replacement)
        else:
            members.pop()
    return result

def from_hif(doc):
    edges = {e['edge']: {'id': 'g:' + str(e['edge']), 'vertices': [], 'attrs': e.get('attrs', {})} for e in doc['edges']}
    for i in doc['incidences']:
        edges[i['edge']]['vertices'].append('n:' + str(i['node']))
    return positions({'version': 'hypergraph.v1', 'meta': {'id': 'diseasome', 'title': 'Diseasome', 'attrs': {'license': 'CC-BY-4.0', 'source': 'https://doi.org/10.5281/zenodo.21909416', 'source_metadata': doc.get('metadata', {})}}, 'vertices': [{'id': 'n:' + str(v['node']), 'attrs': v.get('attrs', {})} for v in doc['nodes']], 'hyperedges': list(edges.values())})

def write_suite(directory, source):
    directory.mkdir(parents=True, exist_ok=True)
    graphs = {name: synthetic(name, n, m, a, fixed) for name, n, m, a, fixed in [('correctness', 30, 15, 4, False), ('moderate', 1000, 2000, 8, False), ('large', 10000, 10000, 8, False), ('high-arity-10', 1000, 200, 10, True), ('high-arity-100', 1000, 200, 100, True), ('high-arity-1000', 1000, 200, 1000, True)]}
    fixture = graphs['correctness']
    fixture['hyperedges'][0]['vertices'] = []
    fixture['hyperedges'][1]['vertices'] = ['n0']
    for e in fixture['hyperedges']:
        e['vertices'] = [v for v in e['vertices'] if v != 'n29']
    if (source / 'diseasome.json').exists():
        graphs['scientific'] = from_hif(json.loads((source / 'diseasome.json').read_text()))
    if (source / 'mathlib.json').exists():
        graphs['mathlib'] = positions(json.loads((source / 'mathlib.json').read_text()))
    manifest = []
    for name, graph in graphs.items():
        artifacts = {'': graph, '.hif': hif(graph), '.incidence': incidence(graph), '.updated': changed(graph), '.updated.incidence': incidence(changed(graph))}
        hashes = {}
        for suffix, data in artifacts.items():
            raw = (json.dumps(data, separators=(',', ':'), ensure_ascii=False) + '\n').encode()
            filename = name + suffix + '.json'
            (directory / filename).write_bytes(raw)
            hashes[filename] = hashlib.sha256(raw).hexdigest()
        manifest.append({'dataset': name, 'vertices': len(graph['vertices']), 'hyperedges': len(graph['hyperedges']), 'memberships': sum(len(e['vertices']) for e in graph['hyperedges']), 'maximum_arity': max(map(lambda e: len(e['vertices']), graph['hyperedges'])), 'empty_edges': sum(not e['vertices'] for e in graph['hyperedges']), 'isolated_nodes': len(set(v['id'] for v in graph['vertices']) - set(v for e in graph['hyperedges'] for v in e['vertices'])), 'provenance': graph.get('meta'), 'sha256': hashes})
    (directory / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')

if __name__ == '__main__':
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', type=Path, default=Path('target/benchmark/datasets'))
    parser.add_argument('--source', type=Path, default=Path('target/benchmark/source'))
    args = parser.parse_args()
    write_suite(args.output, args.source)
