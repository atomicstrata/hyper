"""Installed-package discovery: no activation, downloads, or fake package API."""
import csv
from importlib.metadata import version
import os
from pathlib import Path
import sys

import pytest
from hyper_viz._viewer import _resolve_executable


def companion(tmp_path, monkeypatch, package_version=None, filenames=None):
    site = tmp_path / 'site packages'
    metadata = site / 'hypergraph_viz_viewer.dist-info'
    metadata.mkdir(parents=True)
    metadata.joinpath('METADATA').write_text(
        'Metadata-Version: 2.1\nName: hypergraph-viz-viewer\n'
        f'Version: {package_version or version("hypergraph-viz")}\n'
    )
    binaries = []
    for filename in filenames or ['hyper.exe' if sys.platform == 'win32' else 'hyper']:
        binary = site.parent / 'bin with spaces' / filename
        binary.parent.mkdir(exist_ok=True)
        binary.write_text('#!/bin/sh\nexit 0\n')
        binary.chmod(0o755)
        binaries.append(binary)
    with metadata.joinpath('RECORD').open('w', newline='') as stream:
        writer = csv.writer(stream)
        for binary in binaries:
            writer.writerow([os.path.relpath(binary, site), '', ''])
        writer.writerow(['hypergraph_viz_viewer.dist-info/METADATA', '', ''])
    monkeypatch.syspath_prepend(str(site))
    monkeypatch.setenv('PATH', '')
    return binaries


def test_companion_is_found_without_activating_environment(tmp_path, monkeypatch):
    binaries = companion(tmp_path, monkeypatch)
    assert Path(_resolve_executable(None)) == binaries[0]


def test_explicit_executable_overrides_mismatched_companion(tmp_path, monkeypatch):
    companion(tmp_path, monkeypatch, '0.0.0')
    assert Path(_resolve_executable(sys.executable)) == Path(sys.executable)
    with pytest.raises(FileNotFoundError):
        _resolve_executable(tmp_path / 'missing')


def test_companion_version_mismatch_has_install_guidance(tmp_path, monkeypatch):
    companion(tmp_path, monkeypatch, '0.0.0')
    with pytest.raises(RuntimeError, match=r'hypergraph-viz\[viewer\]'):
        _resolve_executable(None)


def test_damaged_companion_is_reported(tmp_path, monkeypatch):
    binaries = companion(tmp_path, monkeypatch)
    binaries[0].unlink()
    with pytest.raises(FileNotFoundError, match='reinstall'):
        _resolve_executable(None)


def test_ambiguous_companion_is_rejected(tmp_path, monkeypatch):
    companion(tmp_path, monkeypatch, filenames=['hyper', 'hyper.exe'])
    with pytest.raises(RuntimeError, match='multiple'):
        _resolve_executable(None)


@pytest.mark.skipif(sys.platform == 'win32', reason='Windows does not use POSIX executable bits')
def test_non_executable_companion_is_reported(tmp_path, monkeypatch):
    binaries = companion(tmp_path, monkeypatch)
    binaries[0].chmod(0o644)
    with pytest.raises(FileNotFoundError, match='unusable'):
        _resolve_executable(None)


def test_missing_record_is_reported(tmp_path, monkeypatch):
    companion(tmp_path, monkeypatch)
    (tmp_path / 'site packages' / 'hypergraph_viz_viewer.dist-info' / 'RECORD').unlink()
    with pytest.raises(FileNotFoundError, match='reinstall'):
        _resolve_executable(None)
