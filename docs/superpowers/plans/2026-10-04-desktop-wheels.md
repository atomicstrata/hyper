# Desktop wheels and notebook prototype follow-up

User-authorized scope: ship the desktop companion wheels and document the next
anywidget prototype. Continue in the existing PR #8 worktree.

## Delivery contract

- `hypergraph-viz[viewer]` installs a version-matched `hypergraph-viz-viewer` wheel.
- Resolution: explicit executable, installed companion's RECORD, then PATH.
  It must work without activating the Python environment. No runtime downloads.
- Keep core wheels and source archives free of Bevy; companion wheels contain
  the existing native CLI, MIT/Apache licenses, and the bundled HIF schema license.
- Release 0.1.2 for both distributions on Linux x86_64, macOS arm64/Intel, and
  Windows x86_64. PyPI's companion publisher requires user account setup.
- Validate executable identity, installation, real process startup, temporary
  snapshot cleanup, and Linux software-rendered window creation.
- Document the notebook prototype; do not add a notebook API in this release.

## Tasks

1. Add failing executable-discovery tests using real temporary distribution
   metadata. Implement installed-binary resolution, compatibility errors,
   explicit override, and missing-install guidance. Run the full Python suite.
2. Package the root CLI through maturin's binary bindings. Version all packages
   0.1.2, pin the optional dependency, check wheel contents, and clean-install
   the locally built companion. Preserve the independently built core sdist.
3. Extend the existing release matrix with desktop build/install/startup checks
   and gate publishing on core and desktop jobs. Verify actual graphics startup
   on local macOS and Linux software Vulkan/Xvfb in CI. Obtain an independent
   final review and fix important findings before tagging.
4. Update installation and release docs and add a small-dataset anywidget plan.
   Push PR #8, tag after verification and publisher setup, monitor all jobs,
   then clean-install the extra from public PyPI and launch its binary.

## Review focus

- Missing executable despite an installed companion; damaged or ambiguous RECORD.
- Core/companion mismatch and override priority.
- Inactive virtual environment and paths containing spaces.
- Native platform tags, binary permissions, libraries, licenses, and wheel size.
- No claim that a CLI help check proves graphics startup or notebook support.
