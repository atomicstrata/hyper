use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

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
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub attrs: Map<String, Value>,
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
    #[serde(default)]
    pub hub_index: Option<SceneIndex>,
    pub member_indices: Vec<SceneIndex>,
    pub label: String,
    pub kind: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub attrs: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneMeta {
    pub version: String,
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub attrs: Map<String, Value>,
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

/// True when two scenes show the same vertices/hyperedges even if indices moved.
/// Used so a live reload that only reorders agents does not rebuild the view.
pub fn scenes_equivalent(left: &HypergraphScene, right: &HypergraphScene) -> bool {
    if left.meta != right.meta {
        return false;
    }
    if left.nodes.len() != right.nodes.len() || left.hyperedges.len() != right.hyperedges.len() {
        return false;
    }
    let left_nodes: HashMap<&str, &SceneNode> = left
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect();
    for node in &right.nodes {
        let Some(other) = left_nodes.get(node.id.as_str()) else {
            return false;
        };
        if other.role != node.role
            || other.kind != node.kind
            || other.label != node.label
            || other.status != node.status
            || other.hyperedge_id != node.hyperedge_id
            || other.attrs != node.attrs
        {
            return false;
        }
    }

    let left_edges: HashMap<&str, &SceneHyperedge> = left
        .hyperedges
        .iter()
        .map(|he| (he.id.as_str(), he))
        .collect();
    for he in &right.hyperedges {
        let Some(other) = left_edges.get(he.id.as_str()) else {
            return false;
        };
        if other.kind != he.kind
            || other.label != he.label
            || other.status != he.status
            || other.attrs != he.attrs
            || other.hub_index.is_some() != he.hub_index.is_some()
        {
            return false;
        }
        let left_members: HashSet<&str> = other
            .member_indices
            .iter()
            .filter_map(|index| left.nodes.get(*index).map(|node| node.id.as_str()))
            .collect();
        let right_members: HashSet<&str> = he
            .member_indices
            .iter()
            .filter_map(|index| right.nodes.get(*index).map(|node| node.id.as_str()))
            .collect();
        if left_members != right_members {
            return false;
        }
    }
    true
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
        let hits = he.hub_index.is_some_and(|hub| seeds.contains(&hub))
            || he.member_indices.iter().any(|index| seeds.contains(index));
        if hits {
            out.extend(he.hub_index);
            out.extend(he.member_indices.iter().copied());
        }
    }
    out
}

/// Shared visibility for real hubs and hubless member sets. An explicit edge
/// scope prevents unrelated hyperedges among focused vertices from leaking in.
pub fn hyperedge_in_scope(
    scene: &HypergraphScene,
    hyperedge_index: usize,
    nodes: Option<&HashSet<SceneIndex>>,
    hyperedges: Option<&HashSet<usize>>,
) -> bool {
    let Some(he) = scene.hyperedges.get(hyperedge_index) else {
        return false;
    };
    if hyperedges.is_some_and(|edges| !edges.contains(&hyperedge_index)) {
        return false;
    }
    nodes.is_none_or(|nodes| {
        he.hub_index.is_some_and(|hub| nodes.contains(&hub))
            || (!he.member_indices.is_empty()
                && he.member_indices.iter().all(|i| nodes.contains(i)))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{Projection, project};
    use crate::schema::{Hyperedge, Hypergraph, Vertex};

    #[test]
    fn star_first_vertex_does_not_focus_unrelated_groups() {
        let graph = Hypergraph::new()
            .vertex("a", "A", "module")
            .vertex("b", "B", "module")
            .vertex("c", "C", "module")
            .vertex("d", "D", "module")
            .hyperedge("ab", ["a", "b"], "AB")
            .hyperedge("cd", ["c", "d"], "CD");
        for projection in [Projection::StarCentroid, Projection::CliqueExpansion] {
            let scene = graph.project(projection);
            assert_eq!(neighborhood(&scene, [0]), HashSet::from([0, 1]));
        }
    }

    #[test]
    fn projection_retains_attributes_and_metadata_changes_invalidate_scene() {
        let mut graph = Hypergraph::new()
            .vertex("a", "A", "module")
            .vertex("b", "B", "module")
            .hyperedge("ab", ["a", "b"], "AB");
        graph.meta.attrs.insert("commit".into(), "revision".into());
        graph.vertices[0]
            .attrs
            .insert("path".into(), "a.lean".into());
        graph.hyperedges[0]
            .attrs
            .insert("source".into(), "a".into());
        graph.hyperedges[0]
            .attrs
            .insert("target".into(), "b".into());
        for projection in [
            Projection::Bipartite,
            Projection::StarCentroid,
            Projection::CliqueExpansion,
        ] {
            let first = graph.project(projection);
            let value = serde_json::to_value(&first).unwrap();
            assert_eq!(value["meta"]["attrs"]["commit"], "revision");
            assert_eq!(value["nodes"][0]["attrs"]["path"], "a.lean");
            assert_eq!(value["hyperedges"][0]["attrs"]["source"], "a");
            let mut changed = graph.clone();
            changed.vertices[0]
                .attrs
                .insert("path".into(), "moved.lean".into());
            assert!(!scenes_equivalent(&first, &changed.project(projection)));
            changed = graph.clone();
            changed.hyperedges[0]
                .attrs
                .insert("source".into(), "b".into());
            assert!(!scenes_equivalent(&first, &changed.project(projection)));
        }
    }

    #[test]
    fn legacy_scene_json_without_attributes_still_loads() {
        let graph = Hypergraph::new()
            .vertex("a", "A", "module")
            .vertex("b", "B", "module")
            .hyperedge("ab", ["a", "b"], "AB");
        let mut value = serde_json::to_value(graph.project(Projection::Bipartite)).unwrap();
        value["meta"].as_object_mut().unwrap().remove("attrs");
        for node in value["nodes"].as_array_mut().unwrap() {
            node.as_object_mut().unwrap().remove("attrs");
        }
        for edge in value["hyperedges"].as_array_mut().unwrap() {
            edge.as_object_mut().unwrap().remove("attrs");
        }
        let scene: HypergraphScene = serde_json::from_value(value).unwrap();
        assert_eq!(scene.vertices_count(), 2);
        assert_eq!(scene.hyperedges[0].member_indices, [0, 1]);
    }

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

    #[test]
    fn scenes_equivalent_ignores_vertex_order() {
        let mut first = Hypergraph::new().with_id("p").with_title("P");
        first.add_vertex(Vertex::new("a", "A").with_kind("repo"));
        first.add_vertex(Vertex::new("b", "B").with_kind("worktree"));
        first.add_hyperedge(Hyperedge::new("ab", ["a", "b"]).with_kind("repo-membership"));
        let mut second = Hypergraph::new().with_id("p").with_title("P");
        second.add_vertex(Vertex::new("b", "B").with_kind("worktree"));
        second.add_vertex(Vertex::new("a", "A").with_kind("repo"));
        second.add_hyperedge(Hyperedge::new("ab", ["a", "b"]).with_kind("repo-membership"));
        let left = project(&first, Projection::Bipartite);
        let right = project(&second, Projection::Bipartite);
        assert_ne!(left.nodes[0].id, right.nodes[0].id);
        assert!(scenes_equivalent(&left, &right));
    }

    #[test]
    fn scenes_equivalent_detects_status_change() {
        let mut first = Hypergraph::new();
        first.add_vertex(Vertex::new("a", "A").with_status("attention"));
        let mut second = Hypergraph::new();
        second.add_vertex(Vertex::new("a", "A").with_status("shadowed"));
        assert!(!scenes_equivalent(
            &project(&first, Projection::Bipartite),
            &project(&second, Projection::Bipartite)
        ));
    }
}
