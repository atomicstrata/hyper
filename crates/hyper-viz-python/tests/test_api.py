import json
import os
import sys
from pathlib import Path

import pytest
import hyper_viz as hv


def test_rich_hif_is_preserved_but_not_viewed(tmp_path):
    raw = {"network-type":"directed", "incidences":[{"node":1,"edge":"e","direction":"head","attrs":{"role":None}}]}
    doc = hv.HifDocument.from_json(json.dumps(raw))
    path = tmp_path / 'roundtrip.json'
    doc.save(str(path))
    assert json.loads(hv.HifDocument.load(str(path)).to_json()) == raw
    with pytest.raises(hv.HifCompatibilityError) as error:
        doc.to_hypergraph_json()
    assert error.value.location == '/network-type'
    assert error.value.reason


def test_validation_and_native_constructor():
    with pytest.raises(hv.HifValidationError) as error:
        hv.HifDocument.from_json('{"incidences":[{"node":false,"edge":1}]}')
    assert '/incidences/0/node' in error.value.location
    native = {"version":"hypergraph.v1","vertices":[{"id":"a","attrs":{"source":[None,1]}}],"hyperedges":[{"id":"e","vertices":["a"]}]}
    doc = hv.HifDocument.from_hypergraph_json(json.dumps(native))
    assert json.loads(doc.to_json())['nodes'][0]['attrs']['source'] == [None,1]
    assert json.loads(doc.to_hypergraph_json())['hyperedges'][0]['vertices'] == ['s:a']
    with pytest.raises(hv.HifValidationError):
        hv.HifDocument.from_hypergraph_json('{"incidences":[]}')
    with pytest.raises(OSError):
        hv.HifDocument.load('/nonexistent/hyper-hif-test')


@pytest.mark.skipif(sys.platform == 'win32', reason='POSIX executable fixture; cross-platform missing-executable test still runs')
def test_launcher_owns_snapshot_and_cleans_after_exit(tmp_path):
    exe = tmp_path / 'viewer with spaces'
    output = tmp_path / 'observed.json'
    exe.write_text(f'#!{sys.executable}\nimport sys,json,time\nfrom pathlib import Path\nPath({str(output)!r}).write_text(json.dumps({{"args":sys.argv[1:],"graph":json.loads(Path(sys.argv[1]).read_text())}}))\ntime.sleep(0.1)\nsys.exit(7)\n')
    exe.chmod(0o755)
    doc = hv.HifDocument.from_json('{"incidences":[{"node":"a","edge":"e"}]}')
    handle = hv.show(doc, executable=exe, projection='star')
    assert handle.wait(timeout=10) == 7
    observed = json.loads(output.read_text())
    assert observed['args'][1:] == ['--projection','star']
    assert observed['graph']['vertices'][0]['id'] == 's:a'
    assert not Path(observed['args'][0]).exists()
    handle.close()


@pytest.mark.skipif(sys.platform == 'win32', reason='POSIX executable fixture')
def test_close_terminates_and_cleanup_is_automatic(tmp_path):
    exe = tmp_path / 'sleeper'
    output = tmp_path / 'path.txt'
    exe.write_text(f'#!{sys.executable}\nimport sys,time\nfrom pathlib import Path\nPath({str(output)!r}).write_text(sys.argv[1])\ntime.sleep(60)\n')
    exe.chmod(0o755)
    handle = hv.show(hv.HifDocument.from_json('{"incidences":[]}'), executable=exe)
    import time
    deadline = time.monotonic() + 10
    while not output.exists() and time.monotonic() < deadline:
        time.sleep(0.01)
    snapshot = Path(output.read_text())
    assert snapshot.exists()
    handle.close()
    assert handle.poll() is not None
    assert not snapshot.exists()


def test_launcher_failure_cleans_and_validates(monkeypatch, tmp_path):
    doc = hv.HifDocument.from_json('{"incidences":[]}')
    monkeypatch.setenv('PATH', str(tmp_path))
    # Exercise a missing installation even when the test environment has the extra.
    import hyper_viz._viewer as launcher
    from importlib.metadata import PackageNotFoundError
    real_distribution = launcher.distribution
    def without_companion(name):
        if name == 'hypergraph-viz-viewer':
            raise PackageNotFoundError(name)
        return real_distribution(name)
    monkeypatch.setattr(launcher, 'distribution', without_companion)
    with pytest.raises(FileNotFoundError, match='install'):
        hv.show(doc)
    with pytest.raises(FileNotFoundError):
        hv.show(doc, executable=tmp_path/'absent')
    with pytest.raises(ValueError):
        hv.show(doc, projection='wrong')
    with pytest.raises(hv.HifCompatibilityError):
        hv.show(hv.HifDocument.from_json('{"network-type":"directed","incidences":[]}'))


def test_xgi_roundtrip(tmp_path):
    import xgi
    graph = xgi.Hypergraph()
    graph.add_nodes_from(['isolated','a','b'])
    graph.add_edge(['a','b'],idx='edge')
    graph.nodes['a']['nested'] = {'evidence':[None,3]}
    incoming, outgoing = tmp_path/'xgi.json', tmp_path/'hyper.json'
    xgi.write_hif(graph, str(incoming))
    doc = hv.HifDocument.load(str(incoming))
    doc.save(str(outgoing))
    restored = xgi.read_hif(str(outgoing))
    assert set(restored.nodes) == {'isolated','a','b'}
    assert restored.edges.members('edge') == {'a','b'}
    assert restored.nodes['a']['nested'] == {'evidence':[None,3]}
    # XGI may export incidence weights; full interchange must still succeed.


def test_big_integer_ids_and_exact_decimal_metadata():
    raw = '{"incidences":[],"nodes":[{"node":184467440737095516160,"attrs":{"precise":0.12345678901234567890123456789}}]}'
    doc = hv.HifDocument.from_json(raw)
    assert '184467440737095516160' in doc.to_json()
    assert '0.12345678901234567890123456789' in doc.to_json()
    assert json.loads(doc.to_hypergraph_json())['vertices'][0]['id'] == 'i:184467440737095516160'


def test_cross_platform_process_exit_cleanup():
    # Empty native graph JSON is also a valid Python dictionary expression.
    # A real Python child consumes the snapshot and exits on every platform.
    handle = hv.show(hv.HifDocument.from_json('{"incidences":[]}'), executable=sys.executable)
    assert handle.wait(timeout=10) == 0
    handle.close()


@pytest.mark.skipif(sys.platform == 'win32', reason='POSIX executable fixture')
def test_automatic_exit_cleanup_without_wait(tmp_path):
    exe = tmp_path / 'quick-viewer'
    output = tmp_path / 'snapshot.txt'
    exe.write_text(f'#!{sys.executable}\nimport sys,time\nfrom pathlib import Path\nPath({str(output)!r}).write_text(sys.argv[1])\ntime.sleep(0.1)\n')
    exe.chmod(0o755)
    handle = hv.show(hv.HifDocument.from_json('{"incidences":[]}'), executable=exe)
    import time
    deadline = time.monotonic() + 10
    while not output.exists() and time.monotonic() < deadline:
        time.sleep(0.01)
    path = Path(output.read_text())
    while path.exists() and time.monotonic() < deadline:
        time.sleep(0.01)
    assert not path.exists()
    assert handle.poll() == 0


@pytest.mark.skipif(sys.platform == 'win32', reason='POSIX invalid interpreter fixture')
def test_spawn_failure_removes_created_snapshot(monkeypatch, tmp_path):
    import tempfile
    monkeypatch.setattr(tempfile, 'tempdir', str(tmp_path))
    exe = tmp_path / 'broken-viewer'
    exe.write_text('#!/nonexistent/hyper-test-interpreter\n')
    exe.chmod(0o755)
    with pytest.raises(FileNotFoundError):
        hv.show(hv.HifDocument.from_json('{"incidences":[]}'), executable=exe)
    assert list(tmp_path.glob('hyper-viz-*')) == []
