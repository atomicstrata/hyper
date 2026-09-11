use bevy::prelude::*;
use hyper_viz::{Emphasis, NodeRole, Rgba, emphasize, node_style, scaled_radius};

use crate::graph::GraphLayout;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeLabelMode {
    Capped,
    SelectionOnly,
    All,
}

#[derive(Resource, Debug, Clone)]
pub struct NodeRenderSettings {
    pub labels_enabled: bool,
    pub hyperedge_labels: bool,
    pub label_mode: NodeLabelMode,
    pub max_labels: usize,
    pub label_scale: f32,
    pub truncate_len: usize,
    pub label_offset: f32,
}

impl Default for NodeRenderSettings {
    fn default() -> Self {
        Self {
            labels_enabled: true,
            hyperedge_labels: true,
            label_mode: NodeLabelMode::Capped,
            max_labels: 250,
            label_scale: 0.8,
            truncate_len: 28,
            label_offset: 2.2,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NodeVisualSpec {
    pub index: usize,
    pub label: String,
    pub role: NodeRole,
    pub radius: f32,
    pub label_visible_by_default: bool,
}

pub fn visual_spec_for(
    layout: &GraphLayout,
    render_settings: &NodeRenderSettings,
    node_size: f32,
    index: usize,
) -> NodeVisualSpec {
    let node = &layout.scene.nodes[index];
    let label = truncate_label(&node.label, render_settings.truncate_len);
    let label_visible_by_default = match render_settings.label_mode {
        NodeLabelMode::All => true,
        NodeLabelMode::Capped => layout.node_count <= render_settings.max_labels,
        NodeLabelMode::SelectionOnly => false,
    };

    let base_radius = scaled_radius(layout.node_count, node_size);
    let style = node_style(
        node,
        layout
            .scene
            .hyperedges
            .iter()
            .find(|he| he.hub_index == index)
            .map(|he| he.status.as_str()),
    );

    NodeVisualSpec {
        index,
        label,
        role: node.role,
        radius: base_radius * style.radius_scale,
        label_visible_by_default,
    }
}

pub fn label_visible_for(
    settings: &NodeRenderSettings,
    node_count: usize,
    is_selected_or_hovered: bool,
) -> bool {
    if !settings.labels_enabled {
        return false;
    }

    is_selected_or_hovered
        || match settings.label_mode {
            NodeLabelMode::All => true,
            NodeLabelMode::Capped => node_count <= settings.max_labels,
            NodeLabelMode::SelectionOnly => false,
        }
}

pub fn material_for_node(
    materials: &mut Assets<StandardMaterial>,
    layout: &GraphLayout,
    index: usize,
) -> Handle<StandardMaterial> {
    material_for_node_emphasized(materials, layout, index, Emphasis::Rest)
}

pub fn material_for_node_emphasized(
    materials: &mut Assets<StandardMaterial>,
    layout: &GraphLayout,
    index: usize,
    emphasis: Emphasis,
) -> Handle<StandardMaterial> {
    let (base, emissive) = node_emphasized_rgba(layout, index, emphasis);
    materials.add(rgba_to_material(base, emissive))
}

pub fn apply_node_emphasis(
    materials: &mut Assets<StandardMaterial>,
    handle: &Handle<StandardMaterial>,
    layout: &GraphLayout,
    index: usize,
    emphasis: Emphasis,
) {
    let (base, emissive) = node_emphasized_rgba(layout, index, emphasis);
    if let Some(mat) = materials.get_mut(handle) {
        mat.base_color = Color::srgba(base.r, base.g, base.b, base.a);
        mat.emissive = LinearRgba::new(emissive.r, emissive.g, emissive.b, emissive.a);
    }
}

fn node_emphasized_rgba(layout: &GraphLayout, index: usize, emphasis: Emphasis) -> (Rgba, Rgba) {
    let node = &layout.scene.nodes[index];
    let status = layout
        .scene
        .hyperedges
        .iter()
        .find(|he| he.hub_index == index)
        .map(|he| he.status.as_str());
    let style = node_style(node, status);
    let base = emphasize(style.base, emphasis);
    let glow = match emphasis {
        Emphasis::Rest => 0.22,
        Emphasis::Hover => 0.34,
        Emphasis::Selected => 1.05,
    };
    let emissive = Rgba::new(base.r * glow, base.g * glow, base.b * glow, 1.0);
    (base, emissive)
}

fn rgba_to_material(base: Rgba, emissive: Rgba) -> StandardMaterial {
    StandardMaterial {
        base_color: Color::srgba(base.r, base.g, base.b, base.a),
        emissive: LinearRgba::new(emissive.r, emissive.g, emissive.b, emissive.a),
        perceptual_roughness: 0.5,
        ..default()
    }
}

pub(crate) fn truncate_label(label: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    let mut chars = label.chars();
    let prefix: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{prefix}...")
    } else {
        prefix
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_long_labels() {
        assert_eq!(truncate_label("abcdefghijkl", 5), "abcde...");
        assert_eq!(truncate_label("abc", 5), "abc");
    }
}
