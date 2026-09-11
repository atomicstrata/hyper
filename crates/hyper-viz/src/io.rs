use std::path::Path;

use serde::Deserialize;

use crate::error::VizError;
use crate::schema::{GRAPH_VERSION, GraphMeta, Hyperedge, Hypergraph, Vertex};

pub fn load_json(path: impl AsRef<Path>) -> Result<Hypergraph, VizError> {
    let path = path.as_ref();
    let raw = std::fs::read_to_string(path).map_err(|source| VizError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    from_json_str(&raw)
}

pub fn save_json(path: impl AsRef<Path>, graph: &Hypergraph) -> Result<(), VizError> {
    let path = path.as_ref();
    let raw = serde_json::to_string_pretty(graph)?;
    std::fs::write(path, raw).map_err(|source| VizError::Io {
        path: path.to_path_buf(),
        source,
    })
}

pub fn from_json_str(raw: &str) -> Result<Hypergraph, VizError> {
    let value: serde_json::Value = serde_json::from_str(raw)?;
    let version = value
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or(GRAPH_VERSION);

    if version.starts_with("am-hg-graph.") {
        let am: AmGraphExport = serde_json::from_value(value)?;
        return Ok(am.into_hypergraph());
    }

    if version != GRAPH_VERSION && !version.is_empty() {
        return Err(VizError::UnsupportedVersion(version.to_string()));
    }

    let graph: Hypergraph = serde_json::from_value(value)?;
    Ok(graph)
}

/// AtomicMemory `am-hg-graph.v*` snapshot — accepted as an input adapter only.
#[derive(Debug, Deserialize)]
struct AmGraphExport {
    conversation_id: String,
    vertices: Vec<AmVertex>,
    hyperedges: Vec<AmHyperedge>,
}

#[derive(Debug, Deserialize)]
struct AmVertex {
    id: String,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    label: String,
}

#[derive(Debug, Deserialize)]
struct AmHyperedge {
    id: String,
    #[serde(default)]
    subject: String,
    #[serde(default)]
    relation: String,
    #[serde(default)]
    value: String,
    #[serde(default)]
    status: String,
    vertices: Vec<String>,
}

impl AmGraphExport {
    fn into_hypergraph(self) -> Hypergraph {
        let title = self.conversation_id.clone();
        Hypergraph {
            version: GRAPH_VERSION.to_string(),
            meta: GraphMeta {
                id: self.conversation_id,
                title,
                attrs: serde_json::Map::new(),
            },
            vertices: self
                .vertices
                .into_iter()
                .map(|v| Vertex {
                    id: v.id,
                    label: v.label,
                    kind: v.kind,
                    weight: None,
                    attrs: serde_json::Map::new(),
                })
                .collect(),
            hyperedges: self
                .hyperedges
                .into_iter()
                .map(|e| {
                    let label = [e.subject.as_str(), e.relation.as_str(), e.value.as_str()]
                        .iter()
                        .filter(|s| !s.is_empty())
                        .copied()
                        .collect::<Vec<_>>()
                        .join(" ");
                    Hyperedge {
                        id: e.id,
                        vertices: e.vertices,
                        label,
                        kind: String::new(),
                        status: e.status,
                        weight: None,
                        attrs: serde_json::Map::new(),
                    }
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::sample_coauthorship;
    use tempfile::NamedTempFile;

    #[test]
    fn round_trips_json_file() {
        let file = NamedTempFile::new().unwrap();
        let graph = sample_coauthorship();
        save_json(file.path(), &graph).unwrap();
        let loaded = load_json(file.path()).unwrap();
        assert_eq!(loaded, graph);
    }

    #[test]
    fn rejects_unknown_version() {
        let err = from_json_str(r#"{"version":"hypergraph.v99","vertices":[],"hyperedges":[]}"#)
            .unwrap_err();
        assert!(matches!(err, VizError::UnsupportedVersion(_)));
    }

    #[test]
    fn loads_fixture_sample() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/sample.json");
        let graph = load_json(path).unwrap();
        assert_eq!(graph.vertices.len(), 5);
        assert_eq!(graph.hyperedges.len(), 4);
        assert_eq!(graph.meta.id, "demo-coauthorship");
    }

    #[test]
    fn loads_am_hg_export() {
        let raw = r#"{
            "version": "am-hg-graph.v2",
            "conversation_id": "conv-1",
            "stats": {},
            "vertices": [
                {"id": "v:1", "kind": "entity", "label": "Alice"}
            ],
            "hyperedges": [
                {
                    "id": "e:1",
                    "event_id": 1,
                    "fact_id": 1,
                    "subject": "Alice",
                    "relation": "likes",
                    "value": "Rust",
                    "status": "active",
                    "polarity": true,
                    "vertices": ["v:1"]
                }
            ]
        }"#;
        let graph = from_json_str(raw).unwrap();
        assert_eq!(graph.meta.id, "conv-1");
        assert_eq!(graph.vertices[0].label, "Alice");
        assert_eq!(graph.hyperedges[0].label, "Alice likes Rust");
        assert_eq!(graph.hyperedges[0].status, "active");
    }
}
