"""Build a standalone core/Python workspace before creating the PyPI sdist."""
from pathlib import Path
import argparse
import shutil
import subprocess
import tempfile
import tomllib


def build(output: Path) -> None:
    root = Path(__file__).resolve().parents[2]
    output.mkdir(parents=True, exist_ok=True)
    original_lock = tomllib.loads((root / "Cargo.lock").read_text())
    original_versions = {
        (p["name"], p["version"], p.get("source", ""))
        for p in original_lock["package"]
    }
    with tempfile.TemporaryDirectory(prefix="hyper-python-sdist-") as folder:
        staged = Path(folder)
        for name in ("hyper-viz", "hyper-viz-python"):
            shutil.copytree(
                root / "crates" / name,
                staged / "crates" / name,
                ignore=shutil.ignore_patterns("__pycache__", "*.pyc", "target"),
            )
        for name in ("Cargo.lock", "LICENSE", "LICENSE-MIT", "LICENSE-APACHE", "NOTICE"):
            shutil.copy2(root / name, staged / name)
        # This root is a virtual workspace: no CLI or Bevy source is needed.
        workspace = (root / "Cargo.toml").read_text().split("[package]", 1)[0]
        workspace = workspace.replace(
            'members = ["crates/hyper-viz", "crates/hyper-viz-bevy", "crates/hyper-viz-python"]',
            'members = ["crates/hyper-viz", "crates/hyper-viz-python"]',
        )
        workspace = workspace.replace('hyper-viz-bevy = { path = "crates/hyper-viz-bevy" }\n', "")
        (staged / "Cargo.toml").write_text(workspace)
        # Prune unused viewer packages while retaining the existing locked versions.
        subprocess.run(["cargo", "metadata", "--format-version", "1"],
                       cwd=staged, check=True, stdout=subprocess.DEVNULL)
        lock = tomllib.loads((staged / "Cargo.lock").read_text())
        for package in lock["package"]:
            identity = (package["name"], package["version"], package.get("source", ""))
            if identity not in original_versions:
                raise RuntimeError(f"Source packaging changed a dependency: {identity}")
        subprocess.run([
            "maturin", "sdist", "--manifest-path",
            str(staged / "crates/hyper-viz-python/Cargo.toml"),
            "--out", str(output.resolve()),
        ], cwd=staged, check=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, default=Path("dist"))
    build(parser.parse_args().out)
