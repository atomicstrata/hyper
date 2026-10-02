//! Quarantined one-way import for a host-specific hypergraph snapshot format.
//!
//! This module maps version strings prefixed with `am-hg-graph.` into the public
//! [`Hypergraph`](crate::schema::Hypergraph) model (`hypergraph.v1`).
//!
//! ## Boundary
//!
//! - **In scope:** parse vertices/hyperedges and produce a domain-agnostic graph.
//! - **Out of scope:** Atomic Memory Core, claim pipelines, retrieval, conversation
//!   engines, or any proprietary runtime. Extra host fields (`event_id`, `fact_id`,
//!   `polarity`, `stats`, …) are ignored on purpose.
//!
//! Prefer `hypergraph.v1` for new data. Keep this adapter only for loading older
//! host exports; do not grow it into an engine API.

use serde::Deserialize;

use crate::schema::{GRAPH_VERSION, GraphMeta, Hyperedge, Hypergraph, Vertex};

/// Version prefix recognized by the quarantined legacy importer.
pub(crate) const LEGACY_EXPORT_VERSION_PREFIX: &str = "am-hg-graph.";

pub(crate) fn is_legacy_export_version(version: &str) -> bool {
    version.starts_with(LEGACY_EXPORT_VERSION_PREFIX)
}

pub(crate) fn from_legacy_export_value(
    value: serde_json::Value,
) -> Result<Hypergraph, serde_json::Error> {
    let export: LegacyHostExport = serde_json::from_value(value)?;
    Ok(export.into_hypergraph())
}

/// Host-specific snapshot shape — never re-exported from the public API.
#[derive(Debug, Deserialize)]
struct LegacyHostExport {
    /// Host graph/conversation identifier; stored as `meta.id` / title only.
    conversation_id: String,
    vertices: Vec<LegacyVertex>,
    hyperedges: Vec<LegacyHyperedge>,
}

#[derive(Debug, Deserialize)]
struct LegacyVertex {
    id: String,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    label: String,
}

#[derive(Debug, Deserialize)]
struct LegacyHyperedge {
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

impl LegacyHostExport {
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
                    status: String::new(),
                    weight: None,
                    attrs: serde_json::Map::new(),
                })
                .collect(),
            hyperedges: self
                .hyperedges
                .into_iter()
                .map(|e| {
                    // Flatten optional host label fragments into a single display string.
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
