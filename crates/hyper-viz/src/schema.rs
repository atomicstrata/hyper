//! Wire-format hypergraph: vertices + hyperedges, no domain semantics.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Schema version for first-party JSON snapshots.
pub const GRAPH_VERSION: &str = "hypergraph.v1";

/// Optional document-level metadata. Applications may stash extra keys in `attrs`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GraphMeta {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub attrs: Map<String, Value>,
}

/// A vertex in the hypergraph. `kind` is a free-form string used only for coloring.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Vertex {
    pub id: String,
    #[serde(default)]
    pub label: String,
    /// Free-form type, e.g. `"person"`, `"paper"`, `"entity"`. Empty is allowed.
    #[serde(default)]
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<f32>,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub attrs: Map<String, Value>,
}

impl Vertex {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            kind: String::new(),
            weight: None,
            attrs: Map::new(),
        }
    }

    pub fn with_kind(mut self, kind: impl Into<String>) -> Self {
        self.kind = kind.into();
        self
    }
}

/// A hyperedge: an unordered set of vertex ids, plus optional display fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hyperedge {
    pub id: String,
    /// Ids of the vertices this hyperedge connects.
    pub vertices: Vec<String>,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub kind: String,
    /// Visual status: `"active"` (default), `"shadowed"`, `"rejected"`, or custom.
    #[serde(default)]
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<f32>,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub attrs: Map<String, Value>,
}

impl Hyperedge {
    pub fn new(
        id: impl Into<String>,
        vertices: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            id: id.into(),
            vertices: vertices.into_iter().map(Into::into).collect(),
            label: String::new(),
            kind: String::new(),
            status: String::new(),
            weight: None,
            attrs: Map::new(),
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    pub fn with_kind(mut self, kind: impl Into<String>) -> Self {
        self.kind = kind.into();
        self
    }

    pub fn with_status(mut self, status: impl Into<String>) -> Self {
        self.status = status.into();
        self
    }
}

/// Application-agnostic hypergraph snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hypergraph {
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub meta: GraphMeta,
    #[serde(default)]
    pub vertices: Vec<Vertex>,
    #[serde(default)]
    pub hyperedges: Vec<Hyperedge>,
}

fn default_version() -> String {
    GRAPH_VERSION.to_string()
}

impl Default for Hypergraph {
    fn default() -> Self {
        Self {
            version: GRAPH_VERSION.to_string(),
            meta: GraphMeta::default(),
            vertices: Vec::new(),
            hyperedges: Vec::new(),
        }
    }
}

impl Hypergraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.meta.id = id.into();
        self
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.meta.title = title.into();
        self
    }

    pub fn vertex(
        mut self,
        id: impl Into<String>,
        label: impl Into<String>,
        kind: impl Into<String>,
    ) -> Self {
        self.vertices.push(Vertex::new(id, label).with_kind(kind));
        self
    }

    pub fn hyperedge(
        mut self,
        id: impl Into<String>,
        vertices: impl IntoIterator<Item = impl Into<String>>,
        label: impl Into<String>,
    ) -> Self {
        self.hyperedges
            .push(Hyperedge::new(id, vertices).with_label(label));
        self
    }

    pub fn add_vertex(&mut self, vertex: Vertex) -> &mut Self {
        self.vertices.push(vertex);
        self
    }

    pub fn add_hyperedge(&mut self, edge: Hyperedge) -> &mut Self {
        self.hyperedges.push(edge);
        self
    }
}

/// Built-in coauthorship sample used by the CLI when no file is given.
pub fn sample_coauthorship() -> Hypergraph {
    let mut graph = Hypergraph::new()
        .with_id("demo-coauthorship")
        .with_title("Coauthorship hypergraph")
        .vertex("alice", "Alice", "person")
        .vertex("bob", "Bob", "person")
        .vertex("carol", "Carol", "person")
        .vertex("dave", "Dave", "person")
        .vertex("eve", "Eve", "person")
        .hyperedge("paper-a", ["alice", "bob", "carol"], "Paper A · active")
        .hyperedge("paper-b", ["bob", "dave"], "Paper B · shadowed")
        .hyperedge(
            "paper-c",
            ["alice", "eve", "dave", "carol"],
            "Paper C · rejected",
        )
        .hyperedge("lab", ["alice", "bob", "carol", "dave", "eve"], "Lab group");
    graph.hyperedges[0].status = "active".into();
    graph.hyperedges[1].status = "shadowed".into();
    graph.hyperedges[2].status = "rejected".into();
    graph
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_round_trips_counts() {
        let g = sample_coauthorship();
        assert_eq!(g.vertices.len(), 5);
        assert_eq!(g.hyperedges.len(), 4);
        assert_eq!(g.hyperedges[0].vertices.len(), 3);
    }
}
