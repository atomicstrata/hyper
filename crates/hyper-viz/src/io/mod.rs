//! JSON load/save for the public `hypergraph.v1` interchange format.
//!
//! A quarantined one-way importer for a legacy host export prefix lives in
//! [`legacy_export`] and is not part of the public domain model.

mod legacy_export;

use std::path::Path;

use crate::error::VizError;
use crate::schema::{GRAPH_VERSION, Hypergraph};

/// Select an interchange format. Auto rejects mixed HIF/native envelopes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputFormat {
    #[default]
    Auto,
    Hypergraph,
    Hif,
}
impl InputFormat {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "auto" => Some(Self::Auto),
            "hypergraph" => Some(Self::Hypergraph),
            "hif" => Some(Self::Hif),
            _ => None,
        }
    }
}
pub fn load_json(path: impl AsRef<Path>) -> Result<Hypergraph, VizError> {
    load_json_with_format(path, InputFormat::Auto)
}
pub fn load_json_with_format(
    path: impl AsRef<Path>,
    format: InputFormat,
) -> Result<Hypergraph, VizError> {
    let path = path.as_ref();
    let raw = std::fs::read_to_string(path).map_err(|source| VizError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    from_json_str_with_format(&raw, format)
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
    from_json_str_with_format(raw, InputFormat::Auto)
}
pub fn from_json_str_with_format(raw: &str, format: InputFormat) -> Result<Hypergraph, VizError> {
    let value: serde_json::Value = serde_json::from_str(raw)?;
    let hif = ["incidences", "nodes", "edges", "network-type", "metadata"]
        .iter()
        .any(|key| value.get(key).is_some());
    let native = ["version", "vertices", "hyperedges", "meta"]
        .iter()
        .any(|key| value.get(key).is_some());
    if format == InputFormat::Hif || (format == InputFormat::Auto && hif && !native) {
        return Ok(crate::parse_hif(raw)?.to_hypergraph()?);
    }
    if hif || !native || !value.is_object() {
        return Err(VizError::InputFormat(
            "unrecognized or ambiguous envelope; select a format and supply its fields".into(),
        ));
    }
    let version = value
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or(GRAPH_VERSION);

    if legacy_export::is_legacy_export_version(version) {
        return Ok(legacy_export::from_legacy_export_value(value)?);
    }

    if version != GRAPH_VERSION && !version.is_empty() {
        return Err(VizError::UnsupportedVersion(version.to_string()));
    }

    let graph: Hypergraph = serde_json::from_value(value)?;
    Ok(graph)
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
    fn loads_quarantined_legacy_host_export() {
        // Synthetic fixture: proves the one-way importer only keeps viz fields.
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
        assert_eq!(graph.version, GRAPH_VERSION);
    }
}
