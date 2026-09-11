use serde::{Deserialize, Serialize};

use crate::schema::Hypergraph;

/// Dense scene index into [`HypergraphScene::nodes`].
pub type SceneIndex = usize;

/// Stable string id carried from the input graph (vertex or hub).
pub type SceneId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeRole {
    Vertex,
    HyperedgeHub,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneNode {
    pub id: SceneId,
    pub index: SceneIndex,
    pub role: NodeRole,
    /// Free-form kind from the source vertex (or `"hyperedge"` for hubs).
    pub kind: String,
    pub label: String,
    /// When `role == HyperedgeHub`, the source hyperedge id.
    pub hyperedge_id: Option<SceneId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkKind {
    Incidence,
    CliquePair,
    StarSpoke,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneLink {
    pub kind: LinkKind,
    pub source: SceneIndex,
    pub target: SceneIndex,
    pub status: Option<String>,
    pub hyperedge_id: Option<SceneId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneHyperedge {
    pub id: SceneId,
    pub hub_index: SceneIndex,
    pub member_indices: Vec<SceneIndex>,
    pub label: String,
    pub kind: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneMeta {
    pub version: String,
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HypergraphScene {
    pub meta: SceneMeta,
    pub nodes: Vec<SceneNode>,
    pub links: Vec<SceneLink>,
    pub hyperedges: Vec<SceneHyperedge>,
    /// Non-fatal projection issues (e.g. dangling vertex refs).
    pub warnings: Vec<String>,
}

impl HypergraphScene {
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn link_count(&self) -> usize {
        self.links.len()
    }

    pub fn hyperedge_count(&self) -> usize {
        self.hyperedges.len()
    }

    pub fn vertices_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|n| n.role == NodeRole::Vertex)
            .count()
    }

    pub fn from_graph(graph: &Hypergraph) -> Self {
        crate::project::project(graph, crate::project::Projection::Bipartite)
    }

    pub fn dyadic_edges(&self) -> Vec<(SceneIndex, SceneIndex)> {
        self.links
            .iter()
            .map(|link| (link.source, link.target))
            .collect()
    }
}
