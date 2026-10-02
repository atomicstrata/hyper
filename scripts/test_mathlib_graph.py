import tempfile
import unittest
from pathlib import Path

from mathlib_graph import build_graph, imports


class MathlibGraphTests(unittest.TestCase):
    def test_import_all_and_lowercase_modules(self):
        self.assertEqual(imports("module\nmeta import all Mathlib.Init\nimport docs.Conv.Guide\nopen Lean\n"),
                         ["Mathlib.Init", "docs.Conv.Guide"])

    def test_custom_commands_are_not_imports(self):
        self.assertEqual(imports("import Mathlib.Init\nassert_not_exists Field\n"), ["Mathlib.Init"])

    def test_header_ignores_nested_comments_and_body_strings(self):
        source = '''/- import Fake /- nested -/ -/
module
public import Mathlib.A
meta import Mathlib.B -- import Fake2
import Mathlib.C
import
  Mathlib.D
def s := "import Fake3"
'''
        self.assertEqual(imports(source), ["Mathlib.A", "Mathlib.B", "Mathlib.C", "Mathlib.D"])

    def test_graph_preserves_isolated_files_and_external_imports(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / "Mathlib").mkdir()
            (root / "Mathlib/A.lean").write_text("import Mathlib.B\nimport Lean\nimport Mathlib.B\n")
            (root / "Mathlib/B.lean").write_text("def b := 1\n")
            (root / "Mathlib/Isolated.lean").write_text("-- no imports\n")
            graph = build_graph(root, "test", "abc123")
            self.assertEqual([v["id"] for v in graph["vertices"]],
                             ["Mathlib.A", "Mathlib.B", "Mathlib.Isolated", "Lean"])
            pairs = [e["vertices"] for e in graph["hyperedges"] if e["kind"] == "import"]
            self.assertEqual(pairs, [["Mathlib.A", "Lean"], ["Mathlib.A", "Mathlib.B"]])
            groups = [e["vertices"] for e in graph["hyperedges"] if e["kind"] == "dependency_group"]
            self.assertEqual(groups, [["Mathlib.A", "Lean", "Mathlib.B"]])
            self.assertEqual(graph["meta"]["attrs"]["source_files"], 3)
            self.assertEqual(graph["meta"]["attrs"]["external_modules"], 1)
            self.assertEqual(graph, build_graph(root, "test", "abc123"))


if __name__ == "__main__":
    unittest.main()
