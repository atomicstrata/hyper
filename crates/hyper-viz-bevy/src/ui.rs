use std::collections::HashSet;

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};
use hyper_viz::{NodeRole, hyperedge_color, kind_color};

use crate::graph::{GraphLayout, LayoutSettings};
use crate::hyperedge_hull::HyperedgeHullSettings;
use crate::interaction::{LassoState, PointerTarget, SelectionState};
use crate::node_visual::{
    NodeLabelMode, NodeRenderSettings, label_visible_for, truncate_label, visual_spec_for,
};
use crate::render::{Hovered, SceneNodeEntity, Selected};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin::default())
            .add_plugins(FrameTimeDiagnosticsPlugin::default())
            .add_systems(
                EguiPrimaryContextPass,
                ui_panels.run_if(resource_exists::<GraphLayout>),
            );
    }
}

#[allow(clippy::too_many_arguments)]
fn ui_panels(
    mut contexts: EguiContexts,
    mut layout: ResMut<GraphLayout>,
    mut settings: ResMut<LayoutSettings>,
    diagnostics: Res<DiagnosticsStore>,
    mut frame_count: Local<u32>,
    mut lasso: ResMut<LassoState>,
    mut sel_state: ResMut<SelectionState>,
    mut render_settings: ResMut<NodeRenderSettings>,
    mut hull_settings: ResMut<HyperedgeHullSettings>,
    pointer: Res<PointerTarget>,
    _selected_q: Query<&SceneNodeEntity, With<Selected>>,
    label_q: Query<(
        &SceneNodeEntity,
        &Transform,
        Option<&Selected>,
        Option<&Hovered>,
    )>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
) {
    *frame_count += 1;
    if *frame_count < 3 {
        return;
    }

    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    draw_labels(
        ctx,
        &layout,
        &render_settings,
        settings.node_size,
        hull_settings.hide_hubs || render_settings.hyperedge_labels,
        &camera_q,
        &label_q,
    );
    draw_hyperedge_labels(
        ctx,
        &layout,
        &render_settings,
        &sel_state,
        &camera_q,
        &label_q,
    );

    if lasso.is_drawing && lasso.points.len() > 1 {
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("lasso_layer"),
        ));
        let points: Vec<egui::Pos2> = lasso.points.iter().map(|p| egui::pos2(p.x, p.y)).collect();
        painter.add(egui::Shape::line(
            points.clone(),
            egui::Stroke::new(2.0_f32, egui::Color32::WHITE),
        ));
        if points.len() > 2 {
            painter.line_segment(
                [points[0], *points.last().unwrap()],
                egui::Stroke::new(2.0_f32, egui::Color32::WHITE),
            );
        }
    }

    let viewport = ctx.viewport_rect();
    let panel_pos = egui::pos2((viewport.right() - 256.0).max(16.0), 48.0);

    egui::Window::new("Hypergraph")
        .default_pos(panel_pos)
        .default_size(egui::vec2(240.0, 520.0))
        .resizable(true)
        .show(ctx, |ui| {
            egui::CollapsingHeader::new("Graph")
                .default_open(true)
                .show(ui, |ui| {
                    if !layout.scene.meta.title.is_empty() {
                        ui.label(format!("Title: {}", layout.scene.meta.title));
                    }
                    if !layout.scene.meta.id.is_empty() {
                        ui.label(format!("Id: {}", layout.scene.meta.id));
                    }
                    ui.label(format!("Vertices: {}", layout.scene.vertices_count()));
                    ui.label(format!("Hyperedges: {}", layout.scene.hyperedge_count()));
                    let hullable = layout
                        .scene
                        .hyperedges
                        .iter()
                        .filter(|he| he.member_indices.len() >= 3)
                        .count();
                    let dyadic = layout
                        .scene
                        .hyperedges
                        .iter()
                        .filter(|he| he.member_indices.len() == 2)
                        .count();
                    ui.label(format!("Hulls / dyadic: {hullable} / {dyadic}"));
                    ui.label(format!("Hover: {}", pointer_label(&layout, *pointer)));
                    ui.label(format!("Scene nodes: {}", layout.node_count));
                    ui.label(format!("Links: {}", layout.link_count));
                    ui.label(format!("Iterations: {}", layout.iterations()));
                    ui.label(format!("FPS: {:.0}", fps));
                });

            egui::CollapsingHeader::new("Navigation")
                .default_open(true)
                .show(ui, |ui| {
                    ui.checkbox(&mut lasso.enabled, "Lasso select (disables orbit)");
                    ui.label("Left drag: orbit (keeps selection) · Click: select · Scroll: zoom");
                });

            egui::CollapsingHeader::new("Layout")
                .default_open(true)
                .show(ui, |ui| {
                    ui.checkbox(&mut layout.running, "Running (Space)");
                    ui.add(
                        egui::Slider::new(&mut settings.iterations_per_frame, 1..=50)
                            .text("iters/frame"),
                    );
                    layout.iterations_per_frame = settings.iterations_per_frame;

                    ui.add(egui::Slider::new(&mut settings.config.dt, 0.01..=1.0).text("dt"));
                    layout.layout.config.dt = settings.config.dt;

                    ui.add(
                        egui::Slider::new(&mut settings.config.damping, 0.5..=0.99).text("damping"),
                    );
                    layout.layout.config.damping = settings.config.damping;

                    ui.add(
                        egui::Slider::new(&mut settings.config.repulsion, 10.0..=5000.0)
                            .logarithmic(true)
                            .text("repulsion"),
                    );
                    layout.layout.config.repulsion = settings.config.repulsion;

                    ui.add(
                        egui::Slider::new(&mut settings.config.attraction, 0.0001..=0.1)
                            .logarithmic(true)
                            .text("attraction"),
                    );
                    layout.layout.config.attraction = settings.config.attraction;

                    ui.add(
                        egui::Slider::new(&mut settings.config.ideal_length, 5.0..=200.0)
                            .text("ideal_length"),
                    );
                    layout.layout.config.ideal_length = settings.config.ideal_length;
                });

            egui::CollapsingHeader::new("Selection")
                .default_open(true)
                .show(ui, |ui| {
                    let count = sel_state.base_selection.len();
                    ui.label(format!("Selected vertices: {count}"));
                    ui.label(format!(
                        "Selected hyperedges: {}",
                        sel_state.hyperedges.len()
                    ));
                    if ui.button("Clear selection").clicked() {
                        sel_state.clear();
                    }
                });

            egui::CollapsingHeader::new("Hyperedge hulls")
                .default_open(true)
                .show(ui, |ui| {
                    ui.checkbox(&mut hull_settings.enabled, "Show hulls");
                    ui.checkbox(&mut hull_settings.hide_hubs, "Hide extra-node hubs");
                    ui.label("Each hyperedge is a set: hull if arity ≥ 3, line if 2.");
                    ui.label("Uncheck “Hide extra-node hubs” to see one node per hyperedge.");
                    if hull_settings.enabled {
                        ui.add(
                            egui::Slider::new(&mut hull_settings.opacity, 0.05..=0.6)
                                .text("opacity"),
                        );
                        ui.checkbox(&mut hull_settings.wireframe, "Wireframe edges");
                    }
                });

            egui::CollapsingHeader::new("Labels")
                .default_open(true)
                .show(ui, |ui| {
                    ui.checkbox(&mut render_settings.labels_enabled, "Vertex labels");
                    ui.checkbox(&mut render_settings.hyperedge_labels, "Hyperedge labels");
                    if ui
                        .button(format!("Mode: {:?}", render_settings.label_mode))
                        .clicked()
                    {
                        render_settings.label_mode = match render_settings.label_mode {
                            NodeLabelMode::Capped => NodeLabelMode::SelectionOnly,
                            NodeLabelMode::SelectionOnly => NodeLabelMode::All,
                            NodeLabelMode::All => NodeLabelMode::Capped,
                        };
                    }
                });

            if !layout.scene.warnings.is_empty() {
                egui::CollapsingHeader::new("Warnings")
                    .default_open(false)
                    .show(ui, |ui| {
                        for warning in &layout.scene.warnings {
                            ui.colored_label(egui::Color32::YELLOW, warning);
                        }
                    });
            }
        });
}

fn draw_labels(
    ctx: &egui::Context,
    layout: &GraphLayout,
    render_settings: &NodeRenderSettings,
    node_size: f32,
    hide_hubs: bool,
    camera_q: &Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    label_q: &Query<(
        &SceneNodeEntity,
        &Transform,
        Option<&Selected>,
        Option<&Hovered>,
    )>,
) {
    let Ok((camera, cam_transform)) = camera_q.single() else {
        return;
    };

    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("graph_labels"),
    ));

    for (node, transform, selected, hovered) in label_q.iter() {
        let show = label_visible_for(
            render_settings,
            layout.node_count,
            selected.is_some() || hovered.is_some(),
        ) || visual_spec_for(layout, render_settings, node_size, node.index)
            .label_visible_by_default;

        if !show {
            continue;
        }

        let Ok(screen) = camera.world_to_viewport(cam_transform, transform.translation) else {
            continue;
        };

        let scene_node = &layout.scene.nodes[node.index];
        if hide_hubs && scene_node.role == NodeRole::HyperedgeHub {
            continue;
        }
        let rgba = if scene_node.role == NodeRole::HyperedgeHub {
            hyperedge_color(&scene_node.id)
        } else {
            kind_color(&scene_node.kind)
        };
        let color = egui::Color32::from_rgb(
            (rgba.r * 255.0) as u8,
            (rgba.g * 255.0) as u8,
            (rgba.b * 255.0) as u8,
        );

        let spec = visual_spec_for(layout, render_settings, node_size, node.index);
        painter.text(
            egui::pos2(screen.x, screen.y - 12.0),
            egui::Align2::CENTER_BOTTOM,
            spec.label,
            egui::FontId::proportional(11.0 * render_settings.label_scale),
            color,
        );
    }
}

fn draw_hyperedge_labels(
    ctx: &egui::Context,
    layout: &GraphLayout,
    render_settings: &NodeRenderSettings,
    sel_state: &SelectionState,
    camera_q: &Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    label_q: &Query<(
        &SceneNodeEntity,
        &Transform,
        Option<&Selected>,
        Option<&Hovered>,
    )>,
) {
    if !render_settings.hyperedge_labels {
        return;
    }

    let Ok((camera, cam_transform)) = camera_q.single() else {
        return;
    };

    let hovered: HashSet<usize> = label_q
        .iter()
        .filter(|(_, _, _, hovered)| hovered.is_some())
        .map(|(node, _, _, _)| node.index)
        .collect();
    let selected: HashSet<usize> = sel_state.base_selection.iter().copied().collect();

    let show_all = match render_settings.label_mode {
        NodeLabelMode::All => true,
        NodeLabelMode::Capped => layout.scene.hyperedge_count() <= render_settings.max_labels,
        NodeLabelMode::SelectionOnly => false,
    };

    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("hyperedge_labels"),
    ));

    for he in &layout.scene.hyperedges {
        if he.member_indices.is_empty() {
            continue;
        }
        let member_hot = he
            .member_indices
            .iter()
            .any(|idx| selected.contains(idx) || hovered.contains(idx));
        if !show_all && !member_hot {
            continue;
        }

        let Some(anchor) = member_centroid(layout, &he.member_indices) else {
            continue;
        };
        let Ok(screen) = camera.world_to_viewport(cam_transform, anchor) else {
            continue;
        };

        let text = truncate_label(
            if he.label.is_empty() {
                &he.id
            } else {
                &he.label
            },
            render_settings.truncate_len,
        );
        let rgba = hyperedge_color(&he.id);
        let color = egui::Color32::from_rgb(
            (rgba.r * 255.0) as u8,
            (rgba.g * 255.0) as u8,
            (rgba.b * 255.0) as u8,
        );

        painter.text(
            egui::pos2(screen.x, screen.y - 4.0),
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::proportional(12.0 * render_settings.label_scale),
            color,
        );
    }
}

fn member_centroid(layout: &GraphLayout, members: &[usize]) -> Option<Vec3> {
    let mut sum = Vec3::ZERO;
    let mut n = 0.0f32;
    for &idx in members {
        let Some(pos) = layout.position_at(idx) else {
            continue;
        };
        sum += pos;
        n += 1.0;
    }
    if n == 0.0 { None } else { Some(sum / n) }
}

fn pointer_label(layout: &GraphLayout, target: PointerTarget) -> String {
    match target {
        PointerTarget::None => "—".into(),
        PointerTarget::Vertex(i) => layout
            .scene
            .nodes
            .get(i)
            .map(|n| n.label.clone())
            .unwrap_or_else(|| format!("vertex {i}")),
        PointerTarget::Hyperedge(i) => layout
            .scene
            .hyperedges
            .get(i)
            .map(|he| {
                if he.label.is_empty() {
                    he.id.clone()
                } else {
                    he.label.clone()
                }
            })
            .unwrap_or_else(|| format!("hyperedge {i}")),
    }
}
