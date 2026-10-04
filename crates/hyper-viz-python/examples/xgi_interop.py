"""Run with XGI 0.10.2 and the built hyper_viz package."""
import argparse
import tempfile
from pathlib import Path

import xgi
import hyper_viz

parser = argparse.ArgumentParser()
parser.add_argument('--viewer', help='Optional path to the installed native Hyper viewer')
args = parser.parse_args()
graph = xgi.Hypergraph()
graph.add_nodes_from(['a', 'b', 'isolated'])
graph.add_edge(['a', 'b'], idx='research')
graph.nodes['a']['source'] = 'experiment'
with tempfile.TemporaryDirectory() as directory:
    incoming = Path(directory) / 'xgi.json'
    outgoing = Path(directory) / 'roundtrip.json'
    xgi.write_hif(graph, str(incoming))
    document = hyper_viz.HifDocument.load(str(incoming))
    document.save(str(outgoing))
    restored = xgi.read_hif(str(outgoing))
    print(f'Preserved {len(restored.nodes)} nodes and {len(restored.edges)} edges')
    if args.viewer:
        # Some exporters include incidence properties; conversion then raises
        # a compatibility error instead of silently dropping scientific data.
        hyper_viz.show(document, executable=args.viewer).wait()
