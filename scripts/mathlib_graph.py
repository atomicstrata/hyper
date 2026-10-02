#!/usr/bin/env python3
"""Export a mathlib checkout to hypergraph.v1, without building Lean.

Imports are read from module headers, not inferred from theorem text. External
imports remain explicitly marked stub nodes. All repository .lean files are kept,
including isolated files, tests and tooling. No transitive edges are invented.
"""
import argparse
import hashlib
import json
import re
from pathlib import Path


def imports(source):
    # Lean block comments nest. Replacing them with whitespace avoids joining
    # adjacent tokens, and preserves import continuation lines.
    clean = []
    i = depth = 0
    while i < len(source):
        pair = source[i:i + 2]
        if pair == "/-":
            depth += 1
            clean.append(" ")
            i += 2
        elif depth and pair == "-/":
            depth -= 1
            i += 2
        elif depth:
            i += 1
        elif pair == "--":
            end = source.find("\n", i)
            i = len(source) if end < 0 else end
            clean.append(" ")
        else:
            clean.append(source[i])
            i += 1
    tokens = re.findall(r"[^\s]+", "".join(clean))
    result = []
    i = 0
    while i < len(tokens):
        if tokens[i] in ("module", "prelude"):
            i += 1
            continue
        while i < len(tokens) and tokens[i] in ("public", "private", "meta"):
            i += 1
        if i >= len(tokens) or tokens[i] != "import":
            break
        i += 1
        if i < len(tokens) and tokens[i] == "all":
            i += 1
        # Lean's Module.import grammar has exactly one module identifier.
        # Consuming identifiers until the next keyword would invent imports
        # from custom commands such as `assert_not_exists Field`.
        if i >= len(tokens) or not re.fullmatch(r"[\w][\w'.]*", tokens[i]):
            raise ValueError("Malformed import header")
        result.append(tokens[i])
        i += 1
    return sorted(set(result))


def build_graph(root, release, commit):
    paths = sorted(p for p in root.rglob("*.lean")
                   if not any(part.startswith(".") for part in p.relative_to(root).parts))
    if not paths:
        raise ValueError(f"No Lean files found in {root}")
    sources = {".".join(p.relative_to(root).with_suffix("").parts): p for p in paths}
    vertices, edges, external = [], [], set()
    digest = hashlib.sha256()
    max_arity = 0
    for module, path in sorted(sources.items()):
        raw = path.read_bytes()
        relative = path.relative_to(root).as_posix()
        digest.update(relative.encode() + b"\0" + raw + b"\0")
        deps = [dep for dep in imports(raw.decode("utf-8-sig")) if dep != module]
        external.update(set(deps) - sources.keys())
        parts = module.split(".")
        vertices.append({"id": module, "label": module,
                         "kind": parts[1] if parts[0] == "Mathlib" and len(parts) > 1 else parts[0],
                         "attrs": {"path": relative, "source": "mathlib", "bytes": len(raw)}})
        for dep in deps:
            edges.append({"id": f"import:{module}:{dep}", "label": f"{module} → {dep}",
                          "kind": "import", "vertices": [module, dep],
                          "attrs": {"source": module, "target": dep}})
        if len(deps) >= 2:
            members = [module] + deps
            max_arity = max(max_arity, len(members))
            edges.append({"id": f"deps:{module}", "label": f"Dependencies of {module}",
                          "kind": "dependency_group", "vertices": members})
    for module in sorted(external):
        vertices.append({"id": module, "label": module, "kind": "external",
                         "attrs": {"source": "external", "stub": True}})
    pairs = sum(e["kind"] == "import" for e in edges)
    return {"version": "hypergraph.v1",
            "meta": {"id": f"mathlib-{commit}", "title": f"Mathlib {release} · source dependency atlas",
                     "attrs": {"repository": "https://github.com/leanprover-community/mathlib4",
                               "release": release, "commit": commit, "source_sha256": digest.hexdigest(),
                               "source_files": len(sources), "external_modules": len(external),
                               "imports": pairs, "dependency_groups": len(edges) - pairs,
                               "max_group_arity": max_arity,
                               "semantics": "Undirected import pairs; one source + direct imports set per file with >=2 imports. Direction is retained in attrs. External dependencies are stubs."}},
            "vertices": vertices, "hyperedges": edges}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("checkout", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--release", required=True)
    parser.add_argument("--commit", required=True, help="Exact source commit (recorded as provenance)")
    args = parser.parse_args()
    graph = build_graph(args.checkout, args.release, args.commit)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(graph, ensure_ascii=False, separators=(",", ":")) + "\n")
    print(json.dumps(graph["meta"]["attrs"], indent=2))


if __name__ == "__main__":
    main()
