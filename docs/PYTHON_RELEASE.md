# Publishing the Python packages

`hypergraph-viz` provides the `hyper_viz` import, HIF bindings, typing, and a
launcher. `hypergraph-viz-viewer` provides the native desktop executable.
This checkout prepares `0.2.0` for both packages; it is not published yet.
The preceding published release is `0.1.2`. `hypergraph-viz[viewer]` pins the
companion to the core version. Installing the core alone does not install Bevy
or a viewer. See the [release notes](../CHANGELOG.md) for the new features and
Rust API migration notes.

PyPI treats hyphens and underscores as equivalent. The original `hyper-viz` /
`hyper_viz` distribution name was rejected as too similar to an existing project.
The distribution name does not change the Python import.

## One-time PyPI setup

Configure a GitHub Trusted Publisher for **each** project. For a new project,
add a pending publisher at <https://pypi.org/manage/account/publishing/>. For an
existing project, use its publishing settings.

| Field | Value |
|---|---|
| PyPI project names | `hypergraph-viz` and `hypergraph-viz-viewer` (separate entries) |
| GitHub owner | `atomicstrata` |
| Repository | `hyper` |
| Workflow filename | `python.yml` |
| Environment name | `pypi` |

Pending publishers do not reserve names; the first successful upload creates the
project. Both publishers have been configured for this release. Create the
`pypi` GitHub environment too. The workflow requests `id-token: write` only in
the publish job; no long-lived API token is needed. Any configured environment
reviewers must approve that job before it runs.

See PyPI's [pending publisher instructions](https://docs.pypi.org/trusted-publishers/creating-a-project-through-oidc/)
and [publishing instructions](https://docs.pypi.org/trusted-publishers/using-a-publisher/).

## Release a version

1. Validate the release commit with `python wheels and release`. It checks stubs,
   Rust/Python tests, clean installs, standalone core source builds, four desktop
   binary wheels, and installation through the viewer extra. Linux also captures
   a real window using software Vulkan and Xvfb. macOS arm64 graphics startup was
   checked locally; Windows and macOS Intel CI verify CLI startup and discovery,
   without claiming graphics coverage on those systems.
2. Keep versions identical in `[workspace.package]` of `Cargo.toml`, `[project]`
   of the root `pyproject.toml`, and `[project]` of
   `crates/hyper-viz-python/pyproject.toml`. Regenerate `Cargo.lock`, update the
   exact `[viewer]` dependency, and update release-tag documentation URLs.
3. Tag the validated commit. Prefer a merged release commit; an explicitly
   selected validated PR commit can also be released. For `0.2.0`:

   ```sh
   git tag python-v0.2.0 <validated-commit>
   git push origin python-v0.2.0
   ```

The tag workflow builds and tests that exact source again. Versions must match
`python-v<version>`. All build and integration jobs gate publication. The
publisher uploads the companion first, then the core, so the extra's exact
viewer dependency is available when the core is released. Pull requests, branch
pushes, and manual runs never publish.

The core ships CPython 3.10+ `abi3` wheels for Linux x86_64, macOS arm64/x86_64,
and Windows x86_64, plus a source archive. The source helper
(`scripts/releases/build_python_sdist.py`) stages only the core and bindings,
excluding the root desktop project and retaining locked dependencies. CI builds
and installs a wheel from this archive.

The companion ships `py3-none` platform wheels containing a native executable;
there is no desktop source archive. Maturin installs it in the environment's
scripts directory and records its installed location. Linux desktop wheels
require glibc 2.28+, macOS requires 11.0+, and Windows targets x86_64. A desktop
session and working graphics driver remain required. Build from source with
`cargo install --path . --locked` on other supported Rust targets. Wheel checks
verify executable format/permissions, package identity, bundled licenses, and
the default 100 MiB PyPI file limit. Windows desktop builds link the C runtime
statically and verify that the executable imports no external Visual C++ runtime
DLL. The unpublished `python-v0.1.1` tag was stopped after this dependency was
found; it remains unchanged, and the corrected release uses `0.1.2`.

## Verify the published packages

After publishing `0.2.0`, use a fresh standard CPython 3.10+ environment and
the normal PyPI index:

```sh
python -m venv /tmp/hyper-release-check
/tmp/hyper-release-check/bin/python -m pip install --only-binary=:all: "hypergraph-viz[viewer]==0.2.0"
/tmp/hyper-release-check/bin/python - <<'PY'
import os
import subprocess
from importlib.metadata import version
from hyper_viz import HifDocument
from hyper_viz._viewer import _resolve_executable

assert version("hypergraph-viz") == version("hypergraph-viz-viewer") == "0.2.0"
document = HifDocument.from_json('{"incidences":[{"node":"a","edge":"e"}]}')
assert 's:a' in document.to_hypergraph_json()
os.environ["PATH"] = ""  # Discovery works without activation.
subprocess.run([_resolve_executable(None), "--version"], check=True)
PY
```

Adapt the environment path for Windows. Confirm both PyPI pages render their
README and list all four wheels; the core also lists its source archive. On a
local desktop, launch `hyper_viz.show(document)` and close its window to check
actual graphics. A remote notebook kernel does not display this window in the
browser; see the [notebook prototype roadmap](NOTEBOOK_ROADMAP.md).

PyPI releases cannot be overwritten. If an uploaded release is unusable, yank
it and publish a higher version. Do not reuse or move its tag. If an upload
partially succeeds, inspect the published files and retry only missing files;
the workflow does not silently skip existing distributions. If only the
companion upload succeeds, finish the matching core upload from the tested
artifacts before announcing the extra.
