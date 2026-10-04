#!/usr/bin/env python3
"""Archive completed local runs and hash the evidence and current harness sources."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import shutil


def archive(root, destination):
    evidence = destination / 'evidence'
    evidence.mkdir(parents=True, exist_ok=True)
    for track, expected in [('core',42),('native',80),('native-star',20),('native-interactive',20),('graphia',18),('hypergodot',1)]:
        source = root / track / 'raw.json'
        rows = json.loads(source.read_text())
        assert len(rows) == expected, (track, len(rows), expected)
        name = track + '-raw.json'
        if track.startswith('native'):
            (evidence / (name + '.gz')).write_bytes(gzip.compress(source.read_bytes(), mtime=0))
        else:
            shutil.copyfile(source, evidence / name)
        shutil.copyfile(root / track / 'metadata.json', evidence / (track + '-metadata.json'))
    for track, expected in [('browser-sigma-final',35),('browser-final',105)]:
        source = root / track / 'results.json'
        report = json.loads(source.read_text())
        assert len(report['runs']) == expected
        assert len({(r['tool'],r['dataset'],r['mode'],r['repetition']) for r in report['runs']}) == expected
        assert all(len(r['trace']['rafIntervalMs']) == 300 for r in report['runs'] if r['status'] == 'completed')
        (evidence / (track + '-results.json.gz')).write_bytes(gzip.compress(source.read_bytes(), mtime=0))
    shutil.copyfile(root / 'science/independent-validation.json', evidence / 'scientific-independent-validation.json')
    diagnostic = root / 'hypergodot-window-clamped-diagnostic'
    (evidence / 'hypergodot-clamped-window-diagnostic.json.gz').write_bytes(gzip.compress((diagnostic / 'raw.json').read_bytes(),mtime=0))
    shutil.copyfile(diagnostic / 'metadata.json',evidence / 'hypergodot-clamped-window-diagnostic-metadata.json')
    notes = json.loads((evidence / 'driver-notes.json').read_text())
    notes['browser'] = 'Final105 unique jobs: original70 Cytoscape/3D cases plus35 corrected Sigma cases. Original browser closed after evidence was saved; its lingering Node/esbuild worker was then terminated. Explicit esbuild shutdown was added before the corrected run.'
    notes['hypergodot'] = 'HiDPI-disabled window was clamped to1920x917 and excluded. Fullscreen smoke did not match. Final five repetitions restore stock HiDPI and verify actual render texture1920x1080 with exact memberships.'
    notes['sigma'] = 'Original full-refresh camera timings are retained only as excluded diagnostics. All35 were replaced by public scheduleRender cached-redraw observations in a separate block. Consolidation verifies input hashes, protocol, browser and package versions.'
    (evidence / 'driver-notes.json').write_text(json.dumps(notes, indent=2) + '\n')
    paths = sorted(p for p in destination.rglob('*') if p.is_file() and p.name != 'checksums.json')
    sources = sorted(p for p in Path('scripts/benchmarks').rglob('*') if p.is_file() and 'node_modules' not in p.parts and '__pycache__' not in p.parts)
    sources += [Path(p) for p in ['crates/hyper-viz/examples/ecosystem_core.rs','crates/hyper-viz-bevy/src/benchmark.rs','crates/hyper-viz-bevy/src/bin/ecosystem_benchmark.rs']]
    checks = {str(p): {'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in paths + sources}
    (destination / 'checksums.json').write_text(json.dumps({'scope':'Archived evidence/report and current harness sources. Post-run orchestration/checker changes are described in driver-notes; source hashes are not executable hashes.', 'files':checks},indent=2)+'\n')
    print('Archived', len(checks), 'files')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--input', type=Path, default=Path('target/benchmark'))
    parser.add_argument('--output', type=Path, default=Path('docs/benchmarks/ecosystem-2026-10-03'))
    args = parser.parse_args()
    archive(args.input,args.output)
