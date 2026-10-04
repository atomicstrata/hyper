"""Verify installed desktop identity; optionally render a finite real window."""
import argparse
from importlib.metadata import distribution, version
import os
from pathlib import Path
import subprocess
import tempfile
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--render', action='store_true')
parser.add_argument('--viewer-only', action='store_true')
parser.add_argument('--output', type=Path, default=Path('target/desktop-smoke'))
args = parser.parse_args()

companion = distribution('hypergraph-viz-viewer')
if args.viewer_only:
    assert not args.render, '--render requires the core package'
    core = companion.version
else:
    import hyper_viz
    from hyper_viz._viewer import _resolve_executable
    core = version('hypergraph-viz')
    assert companion.version == core
# Empty PATH deliberately proves that activation is unnecessary.
os.environ['PATH'] = ''
os.environ['HYPER_VIZ_SESSION'] = 'off'
if args.viewer_only:
    binaries = [Path(str(companion.locate_file(f))).resolve() for f in companion.files or [] if f.name in ('hyper', 'hyper.exe')]
    assert len(binaries) == 1, binaries
    executable = str(binaries[0])
else:
    executable = _resolve_executable(None)
result = subprocess.run([executable, '--version'], check=True, capture_output=True, text=True, timeout=30)
assert result.stdout.strip() == f'hyper {core}', result.stdout
subprocess.run([executable, '--help'], check=True, stdout=subprocess.DEVNULL, timeout=30)
print(f'Installed desktop executable found without PATH: {executable}')

if args.render:
    args.output.mkdir(parents=True, exist_ok=True)
    fixture = Path(__file__).resolve().parents[2] / 'fixtures/notebook-small.hif.json'
    doc = hyper_viz.HifDocument.load(str(fixture))
    doc.to_hypergraph_json()
    with tempfile.TemporaryDirectory(dir=args.output, prefix='render-') as directory:
        folder = Path(directory)
        subprocess.run([
            executable, str(fixture), '--format', 'hif', '--tour', '--frames', '3',
            '--warmup', '0', '--capture', str(folder / 'frames'), '--report', str(folder / 'frames.csv'),
        ], check=True, timeout=180)
        frames = list((folder / 'frames').glob('*.png'))
        assert len(frames) == 3, frames
        assert all(frame.read_bytes().startswith(b'\x89PNG\r\n\x1a\n') for frame in frames)
        assert (folder / 'frames.csv').is_file()
        # Preserve the final frame for CI/manual visual inspection.
        (args.output / 'render.png').write_bytes(sorted(frames)[-1].read_bytes())
    with hyper_viz.show(doc) as viewer:
        snapshot = Path(viewer._snapshot.name)
        time.sleep(2)
        assert viewer.poll() is None, 'Viewer exited before explicit close'
    assert viewer.poll() is not None
    assert not snapshot.exists(), 'Temporary snapshot was not cleaned up'
    print('Native graphics capture and Python launch/close/cleanup passed.')
