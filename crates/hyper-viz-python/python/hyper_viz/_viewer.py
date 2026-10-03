"""A separately installed viewer owns its window and event loop."""
from __future__ import annotations

import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import threading
from typing import Literal

from ._core import HifDocument

Projection = Literal["bipartite", "clique", "star"]


class ViewerHandle:
    """Own a viewer process and snapshot; dropping the handle leaves the viewer open.

    A background waiter removes the snapshot after process exit. Use close() or
    a context manager to explicitly terminate the viewer.
    """

    def __init__(self, process: subprocess.Popen[bytes], snapshot: tempfile.TemporaryDirectory[str]) -> None:
        self._process = process
        self._snapshot = snapshot
        self._lock = threading.Lock()
        self._waiter = threading.Thread(target=self._reap, daemon=True)
        self._waiter.start()

    def _cleanup(self) -> None:
        with self._lock:
            self._snapshot.cleanup()

    def _reap(self) -> None:
        self._process.wait()
        self._cleanup()

    def poll(self) -> int | None:
        result = self._process.poll()
        if result is not None:
            self._cleanup()
        return result

    def wait(self, timeout: float | None = None) -> int:
        """Return the exit code; timeout leaves the viewer and its snapshot intact."""
        result = self._process.wait(timeout=timeout)
        self._cleanup()
        return result

    def close(self) -> None:
        """Terminate, then kill after five seconds if necessary; safe to repeat."""
        if self._process.poll() is None:
            try:
                self._process.terminate()
            except ProcessLookupError:
                pass
        try:
            self.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self._process.kill()
            self.wait()

    def __enter__(self) -> ViewerHandle:
        return self

    def __exit__(self, *_: object) -> None:
        self.close()


def show(
    document: HifDocument,
    *,
    executable: str | os.PathLike[str] | None = None,
    projection: Projection = "bipartite",
) -> ViewerHandle:
    """Validate, snapshot, and launch Hyper asynchronously without shell execution.

    Install the native Hyper binary separately, put it on PATH, or pass its path.
    No executable is downloaded automatically. Nonzero exit codes are returned
    by wait(); inherited stdout/stderr retain the viewer's diagnostics.
    """
    if projection not in ("bipartite", "clique", "star"):
        raise ValueError("projection must be bipartite, clique, or star")
    raw = document.to_hypergraph_json()
    candidate = os.fspath(executable) if executable is not None else "hyper"
    resolved = shutil.which(candidate)
    if resolved is None:
        raise FileNotFoundError(
            f"Cannot find executable {candidate!r}; install the native Hyper viewer "
            "separately and pass executable='/path/to/hyper' or put it on PATH."
        )
    snapshot = tempfile.TemporaryDirectory(prefix="hyper-viz-")
    try:
        path = Path(snapshot.name) / "graph.json"
        path.write_text(raw, encoding="utf-8")
        process = subprocess.Popen([resolved, str(path), "--projection", projection])
        return ViewerHandle(process, snapshot)
    except BaseException:
        snapshot.cleanup()
        raise
