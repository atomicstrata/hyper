use std::collections::HashSet;

use bevy::prelude::*;
use hyper_viz::{
    EdgeStatus, HypergraphScene, Rgba, attention_dim, is_live_status, neighborhood, parse_status,
};

use crate::graph::GraphLayout;

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AttentionMode {
    pub on: bool,
}

#[derive(Resource, Debug, Clone, Default)]
pub struct FocusScope {
    pub nodes: Option<HashSet<usize>>,
}

impl FocusScope {
    pub fn contains(&self, index: usize) -> bool {
        self.nodes.as_ref().is_none_or(|set| set.contains(&index))
    }

    pub fn is_active(&self) -> bool {
        self.nodes.is_some()
    }

    pub fn clear(&mut self) {
        self.nodes = None;
    }
}

#[derive(Resource, Debug, Default)]
pub struct FrameRequest {
    pub pending: bool,
}

pub fn apply_attention_rgba(color: Rgba, status: &str, attention_on: bool) -> Rgba {
    let dim = attention_dim(parse_status(status), attention_on);
    Rgba::new(color.r, color.g, color.b, (color.a * dim).clamp(0.02, 1.0))
}

pub fn scene_status(scene: &HypergraphScene, index: usize) -> &str {
    scene
        .nodes
        .get(index)
        .map(|node| node.status.as_str())
        .unwrap_or("")
}

pub fn attention_indices(scene: &HypergraphScene) -> Vec<usize> {
    scene
        .nodes
        .iter()
        .filter(|node| parse_status(&node.status) == EdgeStatus::Attention)
        .map(|node| node.index)
        .collect()
}

pub fn live_work_indices(scene: &HypergraphScene) -> Vec<usize> {
    scene
        .nodes
        .iter()
        .filter(|node| is_live_status(&node.status))
        .map(|node| node.index)
        .collect()
}

/// AABB center plus a perspective distance that keeps the box in view.
/// Uses Bevy's default 45° vertical FOV (`tan(fov/2) ≈ 0.414`).
pub fn camera_fit(positions: &[Vec3]) -> Option<(Vec3, f32)> {
    if positions.is_empty() {
        return None;
    }
    let mut min = positions[0];
    let mut max = positions[0];
    for p in &positions[1..] {
        min = min.min(*p);
        max = max.max(*p);
    }
    let center = (min + max) * 0.5;
    let half_extents = (max - min) * 0.5;
    let radius = half_extents.length().max(1.0);
    let tan_half_fov = 0.414_213_56; // tan(22.5°)
    let distance = (radius / tan_half_fov * 1.2).max(8.0);
    Some((center, distance))
}

pub fn positions_for(layout: &GraphLayout, indices: &[usize]) -> Vec<Vec3> {
    indices
        .iter()
        .filter_map(|index| layout.position_at(*index))
        .collect()
}

pub fn frame_indices(
    layout: &GraphLayout,
    attention: &AttentionMode,
    focus: &FocusScope,
) -> Vec<usize> {
    if let Some(nodes) = &focus.nodes {
        return nodes.iter().copied().collect();
    }
    if attention.on {
        let attn = attention_indices(&layout.scene);
        if !attn.is_empty() {
            return attn;
        }
    }
    let live = live_work_indices(&layout.scene);
    if !live.is_empty() {
        return live;
    }
    (0..layout.node_count).collect()
}

pub fn toggle_focus(
    scope: &mut FocusScope,
    layout: &GraphLayout,
    selection: &[usize],
    selected_hyperedges: &[usize],
) {
    if scope.nodes.is_some() {
        scope.clear();
        return;
    }
    let mut seeds = selection.to_vec();
    for he_idx in selected_hyperedges {
        if let Some(he) = layout.scene.hyperedges.get(*he_idx) {
            seeds.push(he.hub_index);
            seeds.extend(he.member_indices.iter().copied());
        }
    }
    if seeds.is_empty() {
        seeds = attention_indices(&layout.scene);
    }
    if seeds.is_empty() {
        return;
    }
    scope.nodes = Some(neighborhood(&layout.scene, seeds));
}

pub struct FocusPlugin;

impl Plugin for FocusPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AttentionMode>()
            .init_resource::<FocusScope>()
            .init_resource::<FrameRequest>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_fit_centers_two_points() {
        let (focus, radius) =
            camera_fit(&[Vec3::new(-2.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 0.0)]).expect("fit");
        assert!((focus.x).abs() < 1e-5);
        assert!((radius - 8.0).abs() < 1e-5);
    }

    #[test]
    fn camera_fit_empty_is_none() {
        assert!(camera_fit(&[]).is_none());
    }

    #[test]
    fn camera_fit_frames_elongated_aabb() {
        let (focus, radius) = camera_fit(&[
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(100.0, 0.0, 0.0),
            Vec3::new(50.0, 20.0, 0.0),
        ])
        .expect("fit");
        assert!((focus.x - 50.0).abs() < 1e-5);
        assert!((focus.y - 10.0).abs() < 1e-5);
        assert!(radius > 80.0, "wide graphs need more than the 8-unit floor");
    }

    #[test]
    fn frame_indices_uses_live_work_not_shadowed() {
        use crate::graph::{GraphLayout, LayoutSettings};
        use hyper_viz::{Hypergraph, Projection, Vertex};

        let mut graph = Hypergraph::new();
        graph.add_vertex(Vertex::new("live", "Live").with_status("attention"));
        graph.add_vertex(Vertex::new("backlog", "Backlog").with_status("shadowed"));
        let layout = GraphLayout::from_scene(
            graph.project(Projection::Bipartite),
            &LayoutSettings::default(),
        );
        let indices = frame_indices(
            &layout,
            &AttentionMode { on: false },
            &FocusScope::default(),
        );
        let ids: Vec<&str> = indices
            .iter()
            .map(|i| layout.scene.nodes[*i].id.as_str())
            .collect();
        assert_eq!(ids, vec!["live"]);
    }

    #[test]
    fn attention_dim_passthrough_when_off() {
        let base = Rgba::new(1.0, 0.0, 0.0, 0.8);
        assert_eq!(apply_attention_rgba(base, "active", false).a, 0.8);
        assert!((apply_attention_rgba(base, "active", true).a - 0.8 * 0.35).abs() < 1e-5);
        assert_eq!(apply_attention_rgba(base, "attention", true).a, 0.8);
    }
}
