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

pub fn camera_fit(positions: &[Vec3]) -> Option<(Vec3, f32)> {
    if positions.is_empty() {
        return None;
    }
    let centroid = positions.iter().copied().sum::<Vec3>() / positions.len() as f32;
    let max_dist = positions
        .iter()
        .map(|p| (*p - centroid).length())
        .fold(0.0f32, f32::max);
    Some((centroid, (max_dist * 2.5).max(8.0)))
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
    selection: &[usize],
) -> Vec<usize> {
    if let Some(nodes) = &focus.nodes {
        return nodes.iter().copied().collect();
    }
    if !selection.is_empty() {
        return selection.to_vec();
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
    fn attention_dim_passthrough_when_off() {
        let base = Rgba::new(1.0, 0.0, 0.0, 0.8);
        assert_eq!(apply_attention_rgba(base, "active", false).a, 0.8);
        assert!((apply_attention_rgba(base, "active", true).a - 0.8 * 0.35).abs() < 1e-5);
        assert_eq!(apply_attention_rgba(base, "attention", true).a, 0.8);
    }
}
