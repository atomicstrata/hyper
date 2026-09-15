use std::collections::HashSet;

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
    /// Copied from the source vertex or hyperedge. Empty means default/active.
    #[serde(default)]
    pub status: String,
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

/// One-hop incident neighborhood: seeds plus every hyperedge that contains a
/// seed (hub + all members). Not a transitive closure.
pub fn neighborhood(
    scene: &HypergraphScene,
    seed_node_indices: impl IntoIterator<Item = SceneIndex>,
) -> HashSet<SceneIndex> {
    let seeds: HashSet<SceneIndex> = seed_node_indices.into_iter().collect();
    let mut out = seeds.clone();
    for he in &scene.hyperedges {
        let hits = seeds.contains(&he.hub_index)
            || he.member_indices.iter().any(|index| seeds.contains(index));
        if hits {
            out.insert(he.hub_index);
            out.extend(he.member_indices.iter().copied());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{Projection, project};
    use crate::schema::{Hyperedge, Hypergraph, Vertex};

    fn backlog_scene() -> HypergraphScene {
        let mut graph = Hypergraph::new();
        graph.add_vertex(Vertex::new("repo", "mind").with_kind("repo"));
        graph.add_vertex(
            Vertex::new("issue", "#3")
                .with_kind("issue")
                .with_status("shadowed"),
        );
        graph.add_vertex(Vertex::new("wt", "main").with_kind("worktree"));
        graph.add_hyperedge(
            Hyperedge::new("github-backlog", ["repo", "issue"])
                .with_kind("github-backlog")
                .with_status("shadowed"),
        );
        graph
            .add_hyperedge(Hyperedge::new("repo-mem", ["repo", "wt"]).with_kind("repo-membership"));
        project(&graph, Projection::Bipartite)
    }

    #[test]
    fn issue_seed_pulls_backlog_and_repo_not_worktree() {
        let scene = backlog_scene();
        let issue = scene
            .nodes
            .iter()
            .find(|n| n.id == "issue")
            .expect("issue")
            .index;
        let scoped = neighborhood(&scene, [issue]);
        let ids: HashSet<_> = scoped
            .iter()
            .filter_map(|i| scene.nodes.get(*i).map(|n| n.id.as_str()))
            .collect();
        assert!(ids.contains("issue"));
        assert!(ids.contains("repo"));
        assert!(ids.contains("github-backlog"));
        assert!(!ids.contains("wt"));
        assert!(!ids.contains("repo-mem"));
    }

    #[test]
    fn empty_seeds_yield_empty_neighborhood() {
        let scene = backlog_scene();
        assert!(neighborhood(&scene, []).is_empty());
    }
}
