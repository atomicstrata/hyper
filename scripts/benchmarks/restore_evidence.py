#!/usr/bin/env python3
"""Restore archived observations for aggregation, without running benchmarks."""
import argparse
import gzip
from pathlib import Path

FILES = {
    'core/raw.json': 'core-raw.json',
    'native/raw.json': 'native-raw.json.gz',
    'native-star/raw.json': 'native-star-raw.json.gz',
    'native-interactive/raw.json': 'native-interactive-raw.json.gz',
    'science/final-raw.json': 'scientific-raw.json',
    'browser-final/results.json': 'browser-final-results.json.gz',
    'graphia/raw.json': 'graphia-raw.json',
    'hypergodot/raw.json': 'hypergodot-raw.json',
}


def restore(source, destination):
    if destination.exists() and any(destination.iterdir()):
        raise ValueError('Choose an empty destination to avoid replacing local runs.')
    payloads = {}
    for target, name in FILES.items():
        raw = (source / name).read_bytes()
        payloads[target] = gzip.decompress(raw) if name.endswith('.gz') else raw
    for target, payload in payloads.items():
        path = destination / target
        path.parent.mkdir(parents=True,exist_ok=True)
        path.write_bytes(payload)
    print('Restored',len(payloads),'observation files to',destination)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--evidence',type=Path,default=Path('docs/benchmarks/ecosystem-2026-10-03/evidence'))
    parser.add_argument('--output',type=Path,default=Path('target/benchmark/restored-evidence'))
    args = parser.parse_args()
    restore(args.evidence,args.output)
