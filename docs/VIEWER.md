# Running the native viewer

The Bevy viewer is **native window only** (not a web app). First compile is
slow (~40s cold); subsequent runs are fast.

## Quick start

From the workspace root:

```bash
# Built-in coauthorship demo
cargo run

# Example scene (fixtures/sample.json)
cargo run -- fixtures/sample.json

# Hot-reload while editing JSON
cargo run -- --watch fixtures/sample.json

# Projection: bipartite (default), clique, star
cargo run -- --projection clique fixtures/sample.json

# Same via the crate example
cargo run -p hyper-viz-bevy --example view -- fixtures/sample.json
```

Headless (no window) library smoke:

```bash
cargo run -p hyper-viz --example project_scene
```

Optional logs (see [`.env.example`](../.env.example)):

```bash
RUST_LOG=hyper=info,hyper_viz_bevy=info cargo run -- fixtures/sample.json
```

## Example scene

[`fixtures/sample.json`](../fixtures/sample.json) is a domain-agnostic
coauthorship hypergraph (`hypergraph.v1`): five people, four hyperedges with
mixed status (`active` / `shadowed` / `rejected`). Use it as the canonical
external-demo fixture.

Minimal shape:

```json
{
  "version": "hypergraph.v1",
  "meta": { "id": "g1", "title": "Coauthorship" },
  "vertices": [
    { "id": "alice", "label": "Alice", "kind": "person" }
  ],
  "hyperedges": [
    { "id": "paper-a", "label": "Paper A", "vertices": ["alice", "bob", "carol"] }
  ]
}
```

## Controls (summary)

| Input | Action |
|---|---|
| Left drag | Orbit (keeps selection) |
| Left click | Select vertex / hyperedge |
| Click empty | Clear selection |
| Scroll | Zoom |
| Space | Toggle force layout |
| ⌘F / Ctrl+F | Find bar |
| Esc | Clear find / focus |

Full interaction notes live in the [root README](../README.md).

## Session persistence

Desktop writes `hyperviz.session.v1` under the OS config dir
(`hyper-viz/session.json`). Override with `HYPER_VIZ_SESSION=/path` or disable
with `HYPER_VIZ_SESSION=off`.
