"""Regression coverage for the standalone Python source workspace."""
import tempfile
import tomllib
import unittest
from pathlib import Path
from unittest.mock import patch

from scripts.releases.build_python_sdist import build


class SourceWorkspaceTests(unittest.TestCase):
    def test_workspace_references_only_shipped_crates(self):
        # Run the real staging code, stopping at the external Cargo/maturin boundary.
        def check_workspace(command, *, cwd, **kwargs):
            workspace = tomllib.loads((Path(cwd) / "Cargo.toml").read_text())
            self.assertEqual(workspace["workspace"]["members"],
                             ["crates/hyper-viz", "crates/hyper-viz-python"])
            for member in workspace["workspace"]["members"]:
                self.assertTrue((Path(cwd) / member / "Cargo.toml").is_file())
            self.assertNotIn("hyper-viz-bevy", workspace["workspace"]["dependencies"])
            self.assertTrue((Path(cwd) / "Cargo.lock").is_file())

        with tempfile.TemporaryDirectory() as folder:
            with patch("scripts.releases.build_python_sdist.subprocess.run", side_effect=check_workspace):
                build(Path(folder))


if __name__ == "__main__":
    unittest.main()
