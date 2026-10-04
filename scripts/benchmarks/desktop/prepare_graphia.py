#!/usr/bin/env python3
"""Export a lossless incidence GraphML and stock Graphia headless parameters."""
import argparse
import hashlib
import json
from pathlib import Path
import sys
import xml.etree.ElementTree as ET

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from datasets import incidence

def prepare(dataset, directory):
    graph = json.loads(dataset.read_text())
    projected = incidence(graph)
    ns = 'http://graphml.graphdrawing.org/xmlns'
    ET.register_namespace('', ns)
    root = ET.Element(f'{{{ns}}}graphml')
    for key in ['role', 'original_id', 'attrs', 'x', 'y', 'z']:
        ET.SubElement(root, f'{{{ns}}}key', {'id': key, 'for': 'node', 'attr.name': key, 'attr.type': 'string'})
    g = ET.SubElement(root, f'{{{ns}}}graph', {'id': 'incidence', 'edgedefault': 'undirected'})
    for item in projected['nodes']:
        node = ET.SubElement(g, f'{{{ns}}}node', {'id': item['id']})
        ET.SubElement(node, f'{{{ns}}}desc').text = item['id']
        for key in ['role', 'original_id', 'attrs', 'x', 'y', 'z']:
            ET.SubElement(node, f'{{{ns}}}data', {'key': key}).text = json.dumps(item[key], ensure_ascii=False)
    for link in projected['links']:
        ET.SubElement(g, f'{{{ns}}}edge', {'id': link['id'], 'source': link['source'], 'target': link['target']})
    directory.mkdir(parents=True, exist_ok=True)
    name = dataset.stem
    target = directory / f'{name}.graphml'
    ET.ElementTree(root).write(target, encoding='utf-8', xml_declaration=True)
    params = directory / f'{name}.parameters.json'
    params.write_text(json.dumps({'type': 'GraphML', 'destination': str((directory / f'{name}.graphia').resolve()), 'plugin': {'name': 'Generic', 'parameters': {}}}, indent=2))
    manifest = {'tool': 'Graphia', 'version': '5.5', 'commit': 'b8ad51e820192a6f22ab310ebd84282384327c10', 'dataset_sha256': hashlib.sha256(dataset.read_bytes()).hexdigest(), 'nodes': len(projected['nodes']), 'links': len(projected['links']), 'adaptation': 'incidence projection; isolated vertices and empty hyperedges retained as standalone nodes', 'coordinate_limit': 'XYZ stored as attributes; stock GraphML parser does not establish common-coordinate layout', 'metric': 'headless load/save wall time includes startup and processing, not render frame time'}
    (directory / f'{name}.manifest.json').write_text(json.dumps(manifest, indent=2))
    print(json.dumps({'graphml': str(target.resolve()), 'parameters': str(params.resolve()), 'manifest': manifest}))

if __name__ == '__main__':
    p = argparse.ArgumentParser()
    p.add_argument('--dataset', type=Path, required=True)
    p.add_argument('--output', type=Path, required=True)
    a = p.parse_args()
    prepare(a.dataset, a.output)
