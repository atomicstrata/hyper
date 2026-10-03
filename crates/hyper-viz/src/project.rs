use std::collections::HashMap;

use crate::scene::{
    HypergraphScene, LinkKind, NodeRole, SceneHyperedge, SceneId, SceneIndex, SceneLink, SceneMeta,
    SceneNode,
};
use crate::schema::{Hyperedge, Hypergraph, Vertex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Projection {
    /// Extra-node bipartite: one hub per hyperedge + incidence links (default).
    #[default]
    Bipartite,
    /// All pairs within each hyperedge (layout-only; no hub nodes).
    CliqueExpansion,
    /// Layout forces toward hyperedge centroid without rendering hub nodes.
    StarCentroid,
}

impl Projection {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "bipartite" => Some(Self::Bipartite),
            "clique" | "clique-expansion" => Some(Self::CliqueExpansion),
            "star" | "star-centroid" => Some(Self::StarCentroid),
            _ => None,
        }
    }
}

pub fn project(graph: &Hypergraph, projection: Projection) -> HypergraphScene {
    let mut scene = project_inner(graph, projection);
    scene.warnings.extend(crate::dependency::import_warnings(&scene));
    scene
}

impl Hypergraph {
    /// Project this hypergraph into a render-agnostic [`HypergraphScene`].
    pub fn project(&self, projection: Projection) -> HypergraphScene {
        project(self, projection)
    }
}

fn project_inner(graph: &Hypergraph, projection: Projection) -> HypergraphScene {
    match projection {
        Projection::Bipartite => project_bipartite(graph),
        Projection::CliqueExpansion => project_clique(graph),
        Projection::StarCentroid => project_star_centroid(graph),
    }
}

fn scene_meta(graph: &Hypergraph) -> SceneMeta {
    SceneMeta {
        version: graph.version.clone(),
        id: graph.meta.id.clone(),
        title: if graph.meta.title.is_empty() {
            graph.meta.id.clone()
        } else {
            graph.meta.title.clone()
        },
        attrs: graph.meta.attrs.clone(),
    }
}

fn vertex_nodes(graph: &Hypergraph) -> (Vec<SceneNode>, HashMap<SceneId, SceneIndex>) {
    let mut id_to_index = HashMap::new();
    let nodes = graph
        .vertices
        .iter()
        .enumerate()
        .map(|(index, vertex)| {
            id_to_index.insert(vertex.id.clone(), index);
            scene_vertex_node(index, vertex)
        })
        .collect();
    (nodes, id_to_index)
}

fn scene_vertex_node(index: SceneIndex, vertex: &Vertex) -> SceneNode {
    SceneNode {
        id: vertex.id.clone(),
        index,
        role: NodeRole::Vertex,
        kind: vertex.kind.clone(),
        label: if vertex.label.is_empty() {
            vertex.id.clone()
        } else {
            vertex.label.clone()
        },
        hyperedge_id: None,
        status: vertex.status.clone(),
        attrs: vertex.attrs.clone(),
    }
}

fn hub_label(edge: &Hyperedge) -> String {
    if !edge.label.is_empty() {
        return edge.label.clone();
    }
    edge.id.clone()
}

fn scene_hyperedge(
    edge: &Hyperedge,
    hub_index: Option<SceneIndex>,
    member_indices: Vec<SceneIndex>,
) -> SceneHyperedge {
    SceneHyperedge {
        id: edge.id.clone(),
        hub_index,
        member_indices,
        label: hub_label(edge),
        kind: edge.kind.clone(),
        status: edge.status.clone(),
        attrs: edge.attrs.clone(),
    }
}

fn project_bipartite(graph: &Hypergraph) -> HypergraphScene {
    let (mut nodes, mut id_to_index) = vertex_nodes(graph);
    let vertex_count = nodes.len();
    let mut links = Vec::new();
    let mut hyperedges = Vec::new();
    let mut warnings = Vec::new();

    for (offset, edge) in graph.hyperedges.iter().enumerate() {
        let hub_index = vertex_count + offset;
        id_to_index.insert(edge.id.clone(), hub_index);

        nodes.push(SceneNode {
            id: edge.id.clone(),
            index: hub_index,
            role: NodeRole::HyperedgeHub,
            kind: if edge.kind.is_empty() {
                "hyperedge".into()
            } else {
                edge.kind.clone()
            },
            label: hub_label(edge),
            hyperedge_id: Some(edge.id.clone()),
            status: edge.status.clone(),
            attrs: edge.attrs.clone(),
        });

        let mut member_indices = Vec::new();
        for vid in &edge.vertices {
            let Some(&member_index) = id_to_index.get(vid) else {
                warnings.push(format!(
                    "hyperedge {} references unknown vertex {vid}; link skipped",
                    edge.id
                ));
                continue;
            };
            member_indices.push(member_index);
            links.push(SceneLink {
                kind: LinkKind::Incidence,
                source: hub_index,
                target: member_index,
                status: Some(edge.status.clone()),
                hyperedge_id: Some(edge.id.clone()),
            });
        }

        hyperedges.push(scene_hyperedge(edge, Some(hub_index), member_indices));
    }

    HypergraphScene {
        meta: scene_meta(graph),
        nodes,
        links,
        hyperedges,
        warnings,
    }
}

fn project_clique(graph: &Hypergraph) -> HypergraphScene {
    let (nodes, id_to_index) = vertex_nodes(graph);
    let mut links = Vec::new();
    let mut hyperedges = Vec::new();
    let mut warnings = Vec::new();

    for edge in &graph.hyperedges {
        let mut member_indices = Vec::new();
        for vid in &edge.vertices {
            let Some(&member_index) = id_to_index.get(vid) else {
                warnings.push(format!(
                    "hyperedge {} references unknown vertex {vid}; member skipped",
                    edge.id
                ));
                continue;
            };
            member_indices.push(member_index);
        }

        for i in 0..member_indices.len() {
            for j in (i + 1)..member_indices.len() {
                links.push(SceneLink {
                    kind: LinkKind::CliquePair,
                    source: member_indices[i],
                    target: member_indices[j],
                    status: Some(edge.status.clone()),
                    hyperedge_id: Some(edge.id.clone()),
                });
            }
        }

        hyperedges.push(scene_hyperedge(edge, None, member_indices));
    }

    HypergraphScene {
        meta: scene_meta(graph),
        nodes,
        links,
        hyperedges,
        warnings,
    }
}

fn project_star_centroid(graph: &Hypergraph) -> HypergraphScene {
    let (nodes, id_to_index) = vertex_nodes(graph);
    let mut hyperedges = Vec::new();
    let mut warnings = Vec::new();

    for edge in &graph.hyperedges {
        let mut member_indices = Vec::new();
        for vid in &edge.vertices {
            let Some(&member_index) = id_to_index.get(vid) else {
                warnings.push(format!(
                    "hyperedge {} references unknown vertex {vid}; member skipped",
                    edge.id
                ));
                continue;
            };
            member_indices.push(member_index);
        }

        hyperedges.push(scene_hyperedge(edge, None, member_indices));
    }

    HypergraphScene {
        meta: scene_meta(graph),
        nodes,
        links: Vec::new(),
        hyperedges,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::sample_coauthorship;

    fn sample_graph() -> Hypergraph {
        Hypergraph::new()
            .with_id("g1")
            .vertex("v:1", "user", "entity")
            .vertex("v:2", "prefers", "predicate")
            .vertex("v:3", "Rust", "value")
            .hyperedge("e:1", ["v:1", "v:2", "v:3"], "user prefers Rust")
    }

    #[test]
    fn bipartite_adds_hub_and_incidence_links() {
        let scene = project(&sample_graph(), Projection::Bipartite);
        assert_eq!(scene.node_count(), 4);
        assert_eq!(scene.hyperedge_count(), 1);
        assert_eq!(scene.link_count(), 3);
        assert!(scene.warnings.is_empty());

        let hub = scene
            .nodes
            .iter()
            .find(|n| n.role == NodeRole::HyperedgeHub)
            .expect("hub");
        assert_eq!(hub.id, "e:1");
        assert!(scene.links.iter().all(|l| l.kind == LinkKind::Incidence));
    }

    #[test]
    fn unknown_vertex_is_skipped_with_warning() {
        let mut graph = sample_graph();
        graph.hyperedges[0].vertices.push("v:missing".into());
        let scene = project(&graph, Projection::Bipartite);
        assert_eq!(scene.link_count(), 3);
        assert_eq!(scene.warnings.len(), 1);
        assert!(scene.warnings[0].contains("v:missing"));
    }

    #[test]
    fn coauthorship_sample_projects() {
        let scene = project(&sample_coauthorship(), Projection::Bipartite);
        assert_eq!(scene.vertices_count(), 5);
        assert_eq!(scene.hyperedge_count(), 4);
        assert_eq!(scene.link_count(), 3 + 2 + 4 + 5);
    }

    #[test]
    fn sample_has_three_hullable_hyperedges() {
        let scene = project(&sample_coauthorship(), Projection::Bipartite);
        let hullable = scene
            .hyperedges
            .iter()
            .filter(|he| he.member_indices.len() >= 3)
            .count();
        let dyadic = scene
            .hyperedges
            .iter()
            .filter(|he| he.member_indices.len() == 2)
            .count();
        assert_eq!(hullable, 3);
        assert_eq!(dyadic, 1);
    }

    #[test]
    fn projects_vertex_and_hub_status() {
        let mut graph = sample_graph();
        graph.vertices[0].status = "attention".into();
        graph.hyperedges[0].status = "shadowed".into();
        let scene = project(&graph, Projection::Bipartite);
        let vertex = scene.nodes.iter().find(|n| n.id == "v:1").unwrap();
        let hub = scene
            .nodes
            .iter()
            .find(|n| n.role == NodeRole::HyperedgeHub)
            .unwrap();
        assert_eq!(vertex.status, "attention");
        assert_eq!(hub.status, "shadowed");
    }
}
