use hyper_viz::{HypergraphScene, NodeRole};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectNode {
    pub index: usize,
    pub kind: String,
    pub title: String,
    pub location: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectHyperedge {
    pub index: usize,
    pub kind: String,
    pub title: String,
    pub location: String,
    pub status: String,
    pub members: Vec<InspectNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InspectReport {
    pub hyperedges: Vec<InspectHyperedge>,
    pub vertices: Vec<InspectNode>,
}

/// Strip a leading `scheme:` so `wt:/tmp/repo` reads as the path.
pub fn node_location(id: &str) -> String {
    match id.split_once(':') {
        Some((head, rest))
            if !rest.is_empty()
                && !head.is_empty()
                && head.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') =>
        {
            rest.to_string()
        }
        _ => id.to_string(),
    }
}

pub fn inspect_node(scene: &HypergraphScene, index: usize) -> Option<InspectNode> {
    let node = scene.nodes.get(index)?;
    Some(InspectNode {
        index,
        kind: node.kind.clone(),
        title: if node.label.is_empty() {
            node.id.clone()
        } else {
            node.label.clone()
        },
        location: node_location(&node.id),
        status: node.status.clone(),
    })
}

pub fn inspect_selection(
    scene: &HypergraphScene,
    selected_nodes: &[usize],
    selected_hyperedges: &[usize],
) -> InspectReport {
    let mut covered = std::collections::HashSet::new();
    let mut hyperedges = Vec::new();
    for &he_index in selected_hyperedges {
        let Some(he) = scene.hyperedges.get(he_index) else {
            continue;
        };
        let members: Vec<InspectNode> = he
            .member_indices
            .iter()
            .filter_map(|index| {
                covered.insert(*index);
                inspect_node(scene, *index)
            })
            .collect();
        hyperedges.push(InspectHyperedge {
            index: he_index,
            kind: he.kind.clone(),
            title: if he.label.is_empty() {
                he.id.clone()
            } else {
                he.label.clone()
            },
            location: node_location(&he.id),
            status: he.status.clone(),
            members,
        });
    }

    let vertices = selected_nodes
        .iter()
        .copied()
        .filter(|index| {
            !covered.contains(index)
                && scene
                    .nodes
                    .get(*index)
                    .is_some_and(|node| node.role == NodeRole::Vertex)
        })
        .filter_map(|index| inspect_node(scene, index))
        .collect();

    InspectReport {
        hyperedges,
        vertices,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper_viz::{Hyperedge, Hypergraph, Projection, Vertex};

    fn sample_scene() -> hyper_viz::HypergraphScene {
        let mut graph = Hypergraph::new();
        graph.add_vertex(Vertex::new("repo:acme/widgets", "widgets").with_kind("repo"));
        graph.add_vertex(Vertex::new("wt:/tmp/widgets", "main").with_kind("worktree"));
        graph.add_vertex(Vertex::new("issue:acme/widgets#3", "#3").with_kind("issue"));
        graph.add_hyperedge(
            Hyperedge::new(
                "repo-mem:repo:acme/widgets",
                ["repo:acme/widgets", "wt:/tmp/widgets"],
            )
            .with_label("widgets repo")
            .with_kind("repo-membership"),
        );
        graph.project(Projection::Bipartite)
    }

    #[test]
    fn node_location_strips_scheme() {
        assert_eq!(node_location("wt:/tmp/widgets"), "/tmp/widgets");
        assert_eq!(node_location("pr:acme/widgets#3"), "acme/widgets#3");
        assert_eq!(node_location("plain"), "plain");
    }

    #[test]
    fn inspect_selection_lists_hyperedge_members_once() {
        let scene = sample_scene();
        let he_index = scene
            .hyperedges
            .iter()
            .position(|he| he.kind == "repo-membership")
            .expect("he");
        let members = scene.hyperedges[he_index].member_indices.clone();
        let report = inspect_selection(&scene, &members, &[he_index]);
        assert_eq!(report.hyperedges.len(), 1);
        assert_eq!(report.hyperedges[0].title, "widgets repo");
        assert_eq!(report.hyperedges[0].members.len(), 2);
        assert!(
            report.vertices.is_empty(),
            "members already listed under the hyperedge"
        );
        assert_eq!(report.hyperedges[0].members[0].location, "acme/widgets");
        assert_eq!(report.hyperedges[0].members[1].location, "/tmp/widgets");
    }

    #[test]
    fn inspect_selection_lists_loose_vertices() {
        let scene = sample_scene();
        let issue = scene
            .nodes
            .iter()
            .find(|n| n.kind == "issue")
            .expect("issue")
            .index;
        let report = inspect_selection(&scene, &[issue], &[]);
        assert!(report.hyperedges.is_empty());
        assert_eq!(report.vertices.len(), 1);
        assert_eq!(report.vertices[0].location, "acme/widgets#3");
    }
}
