# Publishing the Python package

Distribution name: `hypergraph-viz`. Import name: `hyper_viz`. Initial version: `0.1.0`.
The native viewer is installed separately; the Python wheels contain interchange
bindings and the launcher, not Bevy or a viewer executable.

PyPI treats hyphens and underscores as equivalent in project names. The original
`hyper-viz` / `hyper_viz` distribution name was rejected as too similar to an
existing project. Changing the distribution does not change the import.

## One-time PyPI setup

Use a PyPI account with two-factor authentication. For the first release, add a
pending GitHub Trusted Publisher at <https://pypi.org/manage/account/publishing/>:

| Field | Value |
|---|---|
| PyPI project name | `hypergraph-viz` |
| GitHub owner | `atomicstrata` |
| Repository | `hyper` |
| Workflow filename | `python.yml` |
| Environment name | `pypi` |

If the project already exists under your account, add the same publisher under
its project publishing settings instead. Pending publishers do not reserve a
name: PyPI creates the project on the first successful upload.

Create the `pypi` GitHub environment in the repository settings. The workflow
uses that exact environment and requests `id-token: write` only in the publish
job. No long-lived PyPI API token or password is stored in GitHub. Any configured
environment reviewers must approve the publication job before it runs.

See PyPI's official [pending publisher instructions](https://docs.pypi.org/trusted-publishers/creating-a-project-through-oidc/)
and [publishing instructions](https://docs.pypi.org/trusted-publishers/using-a-publisher/).

## Release a version

1. Prefer a merged release commit and verify the `python wheels and release` workflow
   passes on the commit being released. It checks generated stubs, Rust/Python
   tests, clean wheel installs, and source-distribution completeness.
   A release may also tag an explicitly chosen validated commit while its PR
   remains open; the tag workflow builds and tests that exact source again.
2. Keep `[workspace.package].version` in the root `Cargo.toml` and
   `[project].version` in `crates/hyper-viz-python/pyproject.toml` identical.
   Update the package documentation links to the new release tag too.
3. Tag that commit with `python-v` followed by the exact version. For `0.1.0`:

   ```sh
   git switch main
   git pull --ff-only
   git tag python-v0.1.0
   git push origin python-v0.1.0
   ```

The tag triggers `.github/workflows/python.yml`. The version check rejects tags
that disagree with the package version. All four platform wheel jobs and the
source-distribution job must pass before the publishing job downloads their
artifacts and uploads them through PyPI Trusted Publishing. Pull requests,
branch pushes, and manual workflow runs never publish.

Wheels target standard CPython 3.10+ using `abi3`: Linux x86_64, macOS arm64 and
x86_64, and Windows x86_64. A source archive provides a fallback for users with
a Rust toolchain. The source-packaging helper (`scripts/releases/build_python_sdist.py`) creates a
virtual workspace containing only the core and bindings, retaining the locked
dependency versions. CI builds a wheel from that archive to check it contains the
Rust core, pinned HIF schema, Python sources, typing, and licenses.

## Verify the published package

Use a fresh CPython 3.10+ virtual environment and the normal PyPI index:

```sh
python -m venv /tmp/hyper-release-check
. /tmp/hyper-release-check/bin/activate
python -m pip install --only-binary=:all: hypergraph-viz==0.1.0
python - <<'PY'
from hyper_viz import HifDocument

document = HifDocument.from_json('{"incidences":[{"node":"a","edge":"e"}]}')
assert 's:a' in document.to_hypergraph_json()
assert document.to_json()
PY
```

Adapt the virtual-environment path/activation for Windows. Confirm the PyPI
page renders the package README, lists all platform wheels and the source
archive, and shows the correct version. Install the viewer separately to check
`show()`; importing or exchanging HIF requires no viewer.

PyPI releases cannot be overwritten. If an uploaded release is unusable, yank
it through PyPI and publish a corrected higher version; do not reuse the tag or
version. If an upload partially succeeds, inspect the published files before
retrying and upload only missing files. The workflow does not silently skip
existing distributions.
