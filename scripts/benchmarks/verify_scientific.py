#!/usr/bin/env python3
"""Untimed semantic checks against independently parsed immutable input bytes."""
import hashlib
import json
from pathlib import Path
from scientific import construct, check, TOOLS

rows = []
for path in sorted(Path('target/benchmark/datasets').glob('*.json')):
    if path.stem not in ['correctness','moderate','large','high-arity-10','high-arity-100','high-arity-1000','scientific','mathlib']:
        continue
    raw = path.read_bytes()
    for tool in TOOLS:
        data = json.loads(raw)
        graph = construct(tool, data)
        expected = json.loads(raw)
        unmutated = data == expected
        assert unmutated, f'{tool} mutated source data in {path}'
        result = check(tool, graph, expected)
        rows.append({'dataset': path.stem, 'tool': tool, 'sha256': hashlib.sha256(raw).hexdigest(), 'input_unmutated': unmutated, 'correctness': result})
        print(path.stem, tool, result['exact_canonical_roundtrip'], flush=True)
Path('target/benchmark/science/independent-validation.json').write_text(json.dumps(rows, indent=2) + '\n')
