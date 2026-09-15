use bevy::prelude::*;
use hyper_viz::{Emphasis, NodeRole, Rgba, emphasize, node_style, scaled_radius};

use crate::focus::{apply_attention_rgba, scene_status};
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
    /// How strongly font size follows on-screen node size (0 = constant).
    pub label_variation: f32,
    pub truncate_len: usize,
    #[allow(dead_code)]
    pub label_offset: f32,
}

pub const VERTEX_LABEL_BASE_PT: f32 = 14.0;
pub const HYPEREDGE_LABEL_BASE_PT: f32 = 15.0;
pub const LABEL_REFERENCE_PX: f32 = 16.0;
const LABEL_RATIO_MIN: f32 = 0.08;
const LABEL_RATIO_MAX: f32 = 2.5;
const LABEL_ZOOM_MIN: f32 = 0.1;
const LABEL_PT_MIN: f32 = 6.0;
const LABEL_PT_MAX: f32 = 24.0;
const LABEL_PT_LADDER: [f32; 9] = [6.0, 8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 21.0, 24.0];

impl Default for NodeRenderSettings {
    fn default() -> Self {
        Self {
            labels_enabled: true,
            hyperedge_labels: true,
            label_mode: NodeLabelMode::Capped,
            max_labels: 250,
            label_scale: 1.25,
            label_variation: 0.8,
            truncate_len: 28,
            label_offset: 2.2,
        }
    }
}

/// Mix rest size with projected node radius. Zoomed-out type can shrink to
/// a small floor; close-up type is capped and snapped to a short ladder so
/// egui does not rasterize a new large font on every zoom tick.
pub fn label_font_size(base_pt: f32, scale: f32, variation: f32, screen_radius_px: f32) -> f32 {
    let ratio = (screen_radius_px / LABEL_REFERENCE_PX).clamp(LABEL_RATIO_MIN, LABEL_RATIO_MAX);
    let zoom = (1.0 + variation * (ratio - 1.0)).max(LABEL_ZOOM_MIN);
    let unclamped = base_pt * scale * zoom;
    snap_label_pt(unclamped.clamp(LABEL_PT_MIN, LABEL_PT_MAX))
}

fn snap_label_pt(pt: f32) -> f32 {
    LABEL_PT_LADDER
        .iter()
        .copied()
        .min_by(|a, b| (pt - a).abs().total_cmp(&(pt - b).abs()))
        .unwrap_or(pt)
}

pub fn projected_radius_px(
    camera: &Camera,
    cam_transform: &GlobalTransform,
    world_pos: Vec3,
    world_radius: f32,
) -> Option<f32> {
    let (_, rot, _) = cam_transform.to_scale_rotation_translation();
    let right = rot * Vec3::X;
    let center = camera.world_to_viewport(cam_transform, world_pos).ok()?;
    let edge = camera
        .world_to_viewport(cam_transform, world_pos + right * world_radius.max(0.01))
        .ok()?;
    Some(center.distance(edge))
}

#[derive(Debug, Clone)]
pub struct NodeVisualSpec {
    pub index: usize,
    pub label: String,
    #[allow(dead_code)]
    pub role: NodeRole,
    pub radius: f32,
    #[allow(dead_code)]
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
        layout.hub_status.get(index).and_then(|s| s.as_deref()),
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
    force_show: bool,
) -> bool {
    if !settings.labels_enabled {
        return false;
    }

    force_show
        || is_selected_or_hovered
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
    let (base, emissive) = node_emphasized_rgba(layout, index, emphasis, false);
    materials.add(rgba_to_material(base, emissive))
}

#[allow(dead_code)]
pub fn apply_node_emphasis(
    materials: &mut Assets<StandardMaterial>,
    handle: &Handle<StandardMaterial>,
    layout: &GraphLayout,
    index: usize,
    emphasis: Emphasis,
) {
    let (base, emissive) = node_emphasized_rgba(layout, index, emphasis, false);
    if let Some(mat) = materials.get_mut(handle) {
        mat.base_color = Color::srgba(base.r, base.g, base.b, base.a);
        mat.emissive = LinearRgba::new(emissive.r, emissive.g, emissive.b, emissive.a);
    }
}

pub(crate) fn node_emphasized_rgba(
    layout: &GraphLayout,
    index: usize,
    emphasis: Emphasis,
    attention_on: bool,
) -> (Rgba, Rgba) {
    let node = &layout.scene.nodes[index];
    let status = layout.hub_status.get(index).and_then(|s| s.as_deref());
    let style = node_style(node, status);
    let base = apply_attention_rgba(
        emphasize(style.base, emphasis),
        scene_status(&layout.scene, index),
        attention_on,
    );
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

    #[test]
    fn variation_zero_ignores_screen_radius() {
        let a = label_font_size(14.0, 1.25, 0.0, 4.0);
        let b = label_font_size(14.0, 1.25, 0.0, 64.0);
        assert_eq!(a, b);
        assert_eq!(a, 18.0);
    }

    #[test]
    fn larger_screen_radius_grows_with_variation() {
        let near = label_font_size(14.0, 1.25, 0.8, 32.0);
        let far = label_font_size(14.0, 1.25, 0.8, 8.0);
        assert!(near > far);
    }

    #[test]
    fn higher_variation_exaggerates_near_far_spread() {
        let spread = |variation: f32| {
            label_font_size(14.0, 1.25, variation, 32.0)
                - label_font_size(14.0, 1.25, variation, 8.0)
        };
        assert!(spread(1.5) > spread(0.8));
    }

    #[test]
    fn zoomed_out_shrinks_below_rest_size() {
        let rest = label_font_size(14.0, 1.25, 0.8, 16.0);
        let far = label_font_size(14.0, 1.25, 0.8, 2.0);
        assert!(far < rest);
        assert!(far < 12.0);
        assert!(far >= 6.0);
    }

    #[test]
    fn zoomed_out_stays_small_even_when_size_slider_is_high() {
        let far = label_font_size(14.0, 1.52, 1.98, 2.0);
        assert!(far < 12.0);
        assert!(far >= 6.0);
    }

    #[test]
    fn size_slider_still_moves_font_when_variation_is_zero() {
        let small = label_font_size(14.0, 0.5, 0.0, 16.0);
        let large = label_font_size(14.0, 3.0, 0.0, 16.0);
        assert!(large > small + 10.0);
    }

    #[test]
    fn default_settings_are_larger_and_track_zoom() {
        let settings = NodeRenderSettings::default();
        assert!((settings.label_scale - 1.25).abs() < 1e-4);
        assert!((settings.label_variation - 0.8).abs() < 1e-4);
    }

    #[test]
    fn nearby_projected_radii_share_a_font_size() {
        let a = label_font_size(14.0, 1.25, 0.8, 16.0);
        let b = label_font_size(14.0, 1.25, 0.8, 16.05);
        assert_eq!(a, b);
    }

    #[test]
    fn font_sizes_use_a_small_discrete_set() {
        let mut sizes = std::collections::BTreeSet::new();
        for i in 0..400 {
            sizes.insert(label_font_size(14.0, 1.25, 0.8, i as f32 * 0.25).to_bits());
        }
        assert!(
            sizes.len() <= 9,
            "too many unique font sizes: {}",
            sizes.len()
        );
    }

    #[test]
    fn close_up_font_size_is_capped() {
        let pt = label_font_size(14.0, 3.0, 2.0, 400.0);
        assert!(pt <= 24.0);
    }

    #[test]
    fn close_projected_radii_reuse_the_same_large_size() {
        let a = label_font_size(14.0, 1.52, 1.98, 80.0);
        let b = label_font_size(14.0, 1.52, 1.98, 400.0);
        assert_eq!(a, b);
        assert!(a <= 24.0);
    }

    #[test]
    fn attention_force_shows_label_when_over_cap() {
        let settings = NodeRenderSettings {
            labels_enabled: true,
            label_mode: NodeLabelMode::Capped,
            max_labels: 10,
            ..NodeRenderSettings::default()
        };
        assert!(!label_visible_for(&settings, 100, false, false));
        assert!(label_visible_for(&settings, 100, false, true));
    }
}
