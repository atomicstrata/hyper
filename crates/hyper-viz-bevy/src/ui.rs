use std::collections::HashSet;

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};
use bevy_panorbit_camera::PanOrbitCamera;
use hyper_viz::{
    EdgeStatus, HypergraphScene, NodeRole, SceneHits, hyperedge_color, kind_color, parse_status,
    scaled_radius, scene_hits,
};

use crate::focus::{AttentionMode, FocusScope, FrameRequest, isolate_selection, toggle_focus};
use crate::graph::{GraphLayout, LayoutSettings};
use crate::hyperedge_hull::HyperedgeHullSettings;
use crate::inspect::{InspectNode, inspect_selection};
use crate::interaction::{LassoState, LocalizeQuery, PointerTarget, SelectionState};
use crate::node_visual::{
    HYPEREDGE_LABEL_BASE_PT, LABEL_REFERENCE_PX, NodeLabelMode, NodeRenderSettings,
    VERTEX_LABEL_BASE_PT, label_font_size, label_visible_for, projected_radius_px, truncate_label,
    visual_spec_for,
};
use crate::render::{Hovered, LinkRenderSettings, SceneNodeEntity, Selected};

const LOCALIZE_APPLY: usize = 32;
const LOCALIZE_EDIT_ID: &str = "localize_query";
const LOCALIZE_BAR_WIDTH: f32 = 420.0;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin::default())
            .add_plugins(FrameTimeDiagnosticsPlugin::default())
            .add_systems(
                EguiPrimaryContextPass,
                ui_panels
                    .run_if(resource_exists::<GraphLayout>)
                    .run_if(crate::explorer_state::spatial_mode)
                    .run_if(not(resource_exists::<crate::showcase::ShowcaseConfig>)),
            );
    }
}

#[allow(clippy::too_many_arguments)]
fn ui_panels(
    mut contexts: EguiContexts,
    mut layout: ResMut<GraphLayout>,
    mut settings: ResMut<LayoutSettings>,
    diagnostics: (
        Res<DiagnosticsStore>,
        Res<crate::spatial_visibility::LineCache>,
    ),
    mut frame_count: Local<u32>,
    mut lasso: ResMut<LassoState>,
    mut sel_state: ResMut<SelectionState>,
    render_options: (ResMut<NodeRenderSettings>, ResMut<LinkRenderSettings>),
    mut hull_resource: ResMut<HyperedgeHullSettings>,
    mut attention: ResMut<AttentionMode>,
    mut focus: ResMut<FocusScope>,
    mut frame: ResMut<FrameRequest>,
    mut localize: ResMut<LocalizeQuery>,
    pointer: Res<PointerTarget>,
    label_q: Query<(
        &SceneNodeEntity,
        &Transform,
        Option<&Selected>,
        Option<&Hovered>,
    )>,
    camera_q: Query<(&Camera, &GlobalTransform, &PanOrbitCamera), With<Camera3d>>,
) {
    let (mut render_settings, mut lines) = render_options;
    let (diagnostics, line_cache) = diagnostics;
    let mut hull_settings = hull_resource.clone();
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

    let (vertex_font_pt, hyperedge_font_pt) =
        shared_label_font_sizes(&layout, &render_settings, settings.node_size, &camera_q);

    draw_labels(
        ctx,
        &layout,
        &render_settings,
        settings.node_size,
        hull_settings.hide_hubs || render_settings.hyperedge_labels,
        vertex_font_pt,
        &camera_q,
        &label_q,
        &focus,
        &attention,
    );
    draw_hyperedge_labels(
        ctx,
        &layout,
        &render_settings,
        &sel_state,
        hyperedge_font_pt,
        &camera_q,
        &label_q,
        &focus,
        &attention,
        &line_cache,
        &hull_settings,
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
    let panel_pos = egui::pos2((viewport.right() - 336.0).max(16.0), 48.0);

    egui::Window::new("Hypergraph")
        .default_pos(panel_pos)
        .default_size(egui::vec2(320.0, 740.0))
        .resizable(true)
        .max_height((viewport.height() - 64.0).max(200.0))
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.label(format!("{} vertices · {} hyperedges · {:.0} FPS", layout.scene.vertices_count(), layout.scene.hyperedge_count(), fps));
                    egui::CollapsingHeader::new("Graph")
                        .default_open(layout.node_count < 1000)
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
                            ui.horizontal(|ui| {
                                ui.label("Line budget (0 = all)");
                                ui.add(egui::DragValue::new(&mut lines.max_lines).speed(100));
                            });
                            if lines.max_lines != 0 {
                                ui.label(format!(
                                    "Draws at most {} visible lines",
                                    lines.max_lines
                                ));
                            }
                            ui.label(format!("Iterations: {}", layout.iterations()));
                            ui.label(format!(
                                "Last frame: {} steps · {:.2} ms (8 ms budget)",
                                layout.last_steps, layout.last_step_ms
                            ));
                            ui.label(format!("{} lines omitted by budget", line_cache.omitted));
                            ui.add(
                                egui::Slider::new(&mut lines.opacity, 0.0..=1.0)
                                    .logarithmic(true)
                                    .smallest_positive(0.000001)
                                    .max_decimals(6)
                                    .text("Line opacity")
                                    .clamping(egui::SliderClamping::Edits),
                            );
                            ui.label(format!("FPS: {:.0}", fps));
                            ui.checkbox(&mut attention.on, "Attention (A)");
                        });

                    egui::CollapsingHeader::new("Navigation")
                        .default_open(layout.node_count < 1000)
                        .show(ui, |ui| {
                            ui.checkbox(&mut lasso.enabled, "Lasso select (disables orbit)");
                            let mut focused = focus.is_active();
                            if ui
                                .checkbox(&mut focused, "Focus neighborhood (⌃⌘F)")
                                .changed()
                            {
                                if focused && !focus.is_active() {
                                    toggle_focus(
                                        &mut focus,
                                        &layout,
                                        &sel_state.base_selection,
                                        &sel_state.hyperedges,
                                    );
                                    if focus.is_active() {
                                        frame.pending = true;
                                    }
                                } else if !focused {
                                    focus.clear();
                                    localize.isolated = false;
                                }
                            }
                            if ui.button("Frame (F)").clicked() {
                                frame.pending = true;
                            }
                            ui.label(
                                "Left drag: orbit (keeps selection) · Click: select · Scroll: zoom",
                            );
                            ui.label("A: attention · F: frame · ⌘F: find · ⌃⌘F: focus");
                        });

                    egui::CollapsingHeader::new("Layout")
                        .default_open(layout.node_count >= 1000)
                        .show(ui, |ui| {
                            ui.checkbox(&mut layout.running, "Running (Space)");
                            ui.small("Run to apply forces. Pause to inspect stable geometry.");
                            egui::ComboBox::from_label("Attraction model")
                                .selected_text(format!("{:?}", settings.config.topology.model))
                                .show_ui(ui, |ui| {
                                    for model in [hyper_viz::LayoutModel::Legacy, hyper_viz::LayoutModel::Normalized, hyper_viz::LayoutModel::LinLog] {
                                        ui.selectable_value(&mut settings.config.topology.model, model, format!("{model:?}"));
                                    }
                                });
                            ui.small("Normalized balances hubs and sets. Rebuild uses connectivity, then pauses.");
                            ui.add(
                                egui::Slider::new(&mut settings.iterations_per_frame, 1..=100)
                                    .text("iters/frame"),
                            );
                            layout.iterations_per_frame = settings.iterations_per_frame;

                            ui.add(
                                egui::Slider::new(&mut settings.config.dt, 0.001..=2.0)
                                    .clamping(egui::SliderClamping::Edits)
                                    .logarithmic(true)
                                    .text("Time step"),
                            );
                            layout.layout.config.dt = settings.config.dt;

                            ui.add(
                                egui::Slider::new(&mut settings.config.damping, 0.0..=0.9999)
                                    .clamping(egui::SliderClamping::Edits)
                                    .max_decimals(4)
                                    .text("Velocity retention"),
                            );
                            layout.layout.config.damping = settings.config.damping;

                            force_control(ui, "Repulsion", &mut settings.config.repulsion, 0.01, 100000000.);
                            layout.layout.config.repulsion = settings.config.repulsion;

                            if settings.config.topology.model == hyper_viz::LayoutModel::Legacy && !layout.layout.edges.is_empty() {
                                force_control(ui, "Spring attraction", &mut settings.config.attraction, 0.00000001, 1.);
                                ui.add(
                                    egui::Slider::new(&mut settings.config.ideal_length, 0.01..=10000.)
                                        .logarithmic(true)
                                        .clamping(egui::SliderClamping::Edits)
                                        .text("Spring length"),
                                );
                            }
                            force_control(ui, "Gravity", &mut settings.config.gravity, 0.00000001, 1.);
                            if settings.config.topology.model == hyper_viz::LayoutModel::Legacy {
                                if !layout.layout.centroid_groups.is_empty() {
                                    force_control(ui, "Centroid attraction", &mut settings.config.centroid_attraction, 0.00000001, 1.);
                                }
                            } else {
                                let topology = &mut settings.config.topology;
                                force_control(ui, "Pair attraction", &mut topology.pair_attraction, 0.000001, 10.);
                                force_control(ui, "Set attraction", &mut topology.set_attraction, 0.000001, 10.);
                                ui.add(egui::Slider::new(&mut topology.hub_normalization, 0.0..=1.).text("Hub normalization"));
                                ui.add(egui::Slider::new(&mut topology.size_normalization, 0.0..=1.).text("Large-set normalization"));
                                ui.add(egui::Slider::new(&mut topology.derived_set_influence, 0.0..=1.).text("Derived-set influence"));
                                if topology.model == hyper_viz::LayoutModel::LinLog {
                                    ui.add(egui::Slider::new(&mut topology.linlog_scale, 0.01..=100000.).logarithmic(true).text("LinLog scale"));
                                }
                                ui.add(egui::Slider::new(&mut topology.max_displacement, 0.01..=1000.).logarithmic(true).text("Max movement / step"));
                                ui.add(egui::Slider::new(&mut topology.hub_fading, 0.0..=1.).text("Hub line fading"));
                                ui.add(egui::Slider::new(&mut topology.size_fading, 0.0..=1.).text("Large-set hull fading"));
                                topology.normalize();
                            }
                            ui.add(egui::Slider::new(&mut settings.node_size, 0.05..=100.)
                                .logarithmic(true).text("Vertex size"));
                            layout.layout.config = settings.config.clone();
                            if ui.button("Full graph overview").clicked() {
                                layout.full_graph_overview(&mut settings, &mut hull_settings, &mut lines);
                                focus.clear();
                                localize.isolated = false;
                                attention.on = false;
                                frame.pending = true;
                            }
                            ui.small("Overview shows all lines and hulls at low opacity and pauses layout.");
                            if ui.button("Rebuild structural layout").clicked() {
                                layout.rebuild_structural();
                                frame.pending = true;
                            }
                            if layout.initialization_ms > 0. {
                                ui.small(format!("Last rebuild: {:.0} ms (64 refinement steps)", layout.initialization_ms));
                            }
                            if ui.button("Reinitialize neutral positions").clicked() {
                                crate::graph::seed_neutral(&mut layout);
                                frame.pending = true;
                            }
                        });

                    egui::CollapsingHeader::new("Selection")
                        .default_open(layout.node_count < 1000)
                        .show(ui, |ui| {
                            let count = sel_state.base_selection.len();
                            ui.label(format!("Selected vertices: {count}"));
                            ui.label(format!(
                                "Selected hyperedges: {}",
                                sel_state.hyperedges.len()
                            ));
                            ui.label("Names and locations are in the Selection window.");
                            if ui.button("Clear selection").clicked() {
                                sel_state.clear();
                                localize.release_ownership();
                            }
                        });

                    egui::CollapsingHeader::new("Hyperedge hulls")
                        .default_open(layout.node_count >= 1000)
                        .show(ui, |ui| {
                            ui.checkbox(&mut hull_settings.enabled, "Show hulls");
                            ui.add_enabled(
                                layout.scene.nodes.iter().any(|n| n.role == NodeRole::HyperedgeHub),
                                egui::Checkbox::new(&mut hull_settings.hide_hubs, "Hide extra-node hubs"),
                            );
                            ui.label("Each hyperedge is a set: hull if arity ≥ 3, line if 2.");
                            ui.label(
                                "Uncheck “Hide extra-node hubs” to see one node per hyperedge.",
                            );
                            ui.label("Sets larger than 24 members use extreme-point sampling.");
                            if hull_settings.enabled {
                                ui.add(
                                    egui::Slider::new(&mut hull_settings.opacity, 0.0..=0.6)
                                        .clamping(egui::SliderClamping::Edits)
                                        .logarithmic(true)
                                        .smallest_positive(0.000001)
                                        .max_decimals(6)
                                        .text("opacity"),
                                );
                                ui.checkbox(&mut hull_settings.wireframe, "Wireframe edges");
                            }
                        });

                    egui::CollapsingHeader::new("Labels")
                        .default_open(false)
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
                            ui.add(
                                egui::Slider::new(&mut render_settings.label_scale, 0.5..=3.0)
                                    .text("size"),
                            );
                            ui.add(
                                egui::Slider::new(&mut render_settings.label_variation, 0.0..=2.0)
                                    .text("variation"),
                            );
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
        });

    draw_selection_window(ctx, &layout, &mut sel_state, &mut localize);
    draw_localize_window(
        ctx,
        &layout.scene,
        &mut sel_state,
        &mut localize,
        &mut focus,
        &mut frame,
        &layout,
    );
    if *hull_resource != hull_settings {
        *hull_resource = hull_settings;
    }
}

fn draw_localize_window(
    ctx: &egui::Context,
    scene: &HypergraphScene,
    sel_state: &mut SelectionState,
    localize: &mut LocalizeQuery,
    focus: &mut FocusScope,
    frame: &mut FrameRequest,
    layout: &GraphLayout,
) {
    egui::Window::new("Find")
        .id(egui::Id::new("localize_window"))
        .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -24.0))
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .movable(false)
        .default_width(LOCALIZE_BAR_WIDTH)
        .show(ctx, |ui| {
            ui.set_min_width(LOCALIZE_BAR_WIDTH);
            ui.horizontal(|ui| {
                let edit_id = ui.make_persistent_id(LOCALIZE_EDIT_ID);
                let edit = ui.add(
                    egui::TextEdit::singleline(&mut localize.query)
                        .id(edit_id)
                        .hint_text("Find in visible graph")
                        .desired_width(LOCALIZE_BAR_WIDTH - 88.0),
                );
                localize.input_focused = edit.has_focus();
                if localize.focus_box {
                    edit.request_focus();
                    localize.focus_box = false;
                    localize.input_focused = true;
                }

                let hits = scene_hits(scene, &localize.query);
                if edit.changed() {
                    apply_localize_hits(scene, sel_state, localize, &hits, LOCALIZE_APPLY);
                    if focus.is_active() {
                        refocus_matches(focus, frame, localize, layout, sel_state);
                    }
                }

                if edit.has_focus()
                    && ui.input(|input| input.key_pressed(egui::Key::Enter))
                    && !hits.is_empty()
                {
                    apply_localize_hits(scene, sel_state, localize, &hits, LOCALIZE_APPLY);
                    refocus_matches(focus, frame, localize, layout, sel_state);
                }

                if edit.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                    if !localize.query.trim().is_empty() {
                        localize.query.clear();
                        clear_search_selection(sel_state, localize, focus);
                    } else {
                        edit.surrender_focus();
                        if localize.isolated {
                            focus.clear();
                            localize.isolated = false;
                        }
                    }
                }

                if !localize.query.trim().is_empty() {
                    let total = hits.nodes.len() + hits.hyperedges.len();
                    ui.weak(format!("{total}"));
                }
            });
        });
}

fn refocus_matches(
    focus: &mut FocusScope,
    frame: &mut FrameRequest,
    localize: &mut LocalizeQuery,
    layout: &GraphLayout,
    sel_state: &SelectionState,
) {
    if sel_state.base_selection.is_empty() && sel_state.hyperedges.is_empty() {
        return;
    }
    isolate_selection(
        focus,
        layout,
        &sel_state.base_selection,
        &sel_state.hyperedges,
    );
    if focus.is_active() {
        localize.isolated = true;
        frame.pending = true;
    }
}

fn apply_localize_hits(
    scene: &HypergraphScene,
    sel_state: &mut SelectionState,
    localize: &mut LocalizeQuery,
    hits: &SceneHits,
    cap: usize,
) {
    if localize.query.trim().is_empty() {
        if localize.search_owned {
            sel_state.clear();
        }
        localize.search_owned = false;
        return;
    }
    if hits.is_empty() {
        if localize.search_owned {
            sel_state.clear();
        }
        localize.search_owned = true;
        return;
    }
    let (nodes, hyperedges) = flatten_hits(scene, hits, cap);
    sel_state.set_hits(nodes, hyperedges);
    localize.search_owned = true;
}

fn clear_search_selection(
    sel_state: &mut SelectionState,
    localize: &mut LocalizeQuery,
    focus: &mut FocusScope,
) {
    if localize.search_owned {
        sel_state.clear();
    }
    if localize.isolated {
        focus.clear();
    }
    localize.search_owned = false;
    localize.isolated = false;
}

#[derive(Clone, Copy)]
enum LocalizeRow {
    Node(usize),
    Hyperedge(usize),
}

fn ranked_rows(hits: &SceneHits, cap: usize) -> Vec<LocalizeRow> {
    let mut ranked: Vec<(f32, LocalizeRow)> = hits
        .nodes
        .iter()
        .map(|hit| (hit.score, LocalizeRow::Node(hit.index)))
        .chain(
            hits.hyperedges
                .iter()
                .map(|hit| (hit.score, LocalizeRow::Hyperedge(hit.index))),
        )
        .collect();
    ranked.sort_by(|a, b| {
        b.0.total_cmp(&a.0).then_with(|| match (&a.1, &b.1) {
            (LocalizeRow::Node(i), LocalizeRow::Node(j)) => i.cmp(j),
            (LocalizeRow::Hyperedge(i), LocalizeRow::Hyperedge(j)) => i.cmp(j),
            (LocalizeRow::Node(_), LocalizeRow::Hyperedge(_)) => std::cmp::Ordering::Less,
            (LocalizeRow::Hyperedge(_), LocalizeRow::Node(_)) => std::cmp::Ordering::Greater,
        })
    });
    ranked.truncate(cap);
    ranked.into_iter().map(|(_, row)| row).collect()
}

fn flatten_hits(scene: &HypergraphScene, hits: &SceneHits, cap: usize) -> (Vec<usize>, Vec<usize>) {
    let mut nodes = Vec::new();
    let mut hyperedges = Vec::new();
    for row in ranked_rows(hits, cap) {
        match row {
            LocalizeRow::Node(index) => nodes.push(index),
            LocalizeRow::Hyperedge(index) => {
                hyperedges.push(index);
                if let Some(he) = scene.hyperedges.get(index) {
                    nodes.extend(he.hub_index);
                    nodes.extend(he.member_indices.iter().copied());
                }
            }
        }
    }
    nodes.sort_unstable();
    nodes.dedup();
    (nodes, hyperedges)
}

fn draw_selection_window(
    ctx: &egui::Context,
    layout: &GraphLayout,
    sel_state: &mut SelectionState,
    localize: &mut LocalizeQuery,
) {
    let report = inspect_selection(
        &layout.scene,
        &sel_state.base_selection,
        &sel_state.hyperedges,
    );
    egui::Window::new("Selection")
        .default_pos(egui::pos2(16.0, 48.0))
        .default_size(egui::vec2(320.0, 420.0))
        .resizable(true)
        .show(ctx, |ui| {
            ui.label(format!(
                "{} hyperedges · {} other vertices",
                report.hyperedges.len(),
                report.vertices.len()
            ));
            if ui.button("Clear").clicked() {
                sel_state.clear();
                localize.release_ownership();
            }
            ui.separator();
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if report.hyperedges.is_empty() && report.vertices.is_empty() {
                        ui.weak("Click a node or hull. Shift/Cmd-click adds.");
                        return;
                    }
                    let mut pick = None;
                    for he in &report.hyperedges {
                        ui.strong(format!("{} · {}", he.kind, he.title));
                        ui.label(egui::RichText::new(&he.location).small().weak());
                        if !he.status.is_empty() {
                            ui.label(format!("status: {}", he.status));
                        }
                        ui.label(format!("{} members", he.members.len()));
                        for member in he.members.iter().take(80) {
                            if inspect_node_row(ui, member).clicked() {
                                pick = Some(member.index);
                            }
                        }
                        if he.members.len() > 80 {
                            ui.weak(format!("…and {} more", he.members.len() - 80));
                        }
                        ui.separator();
                    }
                    for node in &report.vertices {
                        if inspect_node_row(ui, node).clicked() {
                            pick = Some(node.index);
                        }
                    }
                    if let Some(index) = pick {
                        sel_state.set_selection(vec![index]);
                        localize.release_ownership();
                    }
                });
        });
}

fn inspect_node_row(ui: &mut egui::Ui, node: &InspectNode) -> egui::Response {
    let resp = ui.selectable_label(false, format!("{}  {}", node.kind, node.title));
    ui.label(egui::RichText::new(&node.location).small().weak());
    if !node.status.is_empty() {
        ui.label(format!("status: {}", node.status));
    }
    resp
}

fn shared_label_font_sizes(
    layout: &GraphLayout,
    render_settings: &NodeRenderSettings,
    node_size: f32,
    camera_q: &Query<(&Camera, &GlobalTransform, &PanOrbitCamera), With<Camera3d>>,
) -> (f32, f32) {
    let screen_radius = camera_q
        .single()
        .ok()
        .and_then(|(camera, cam_transform, orbit)| {
            let world_radius = scaled_radius(layout.node_count, node_size);
            projected_radius_px(camera, cam_transform, orbit.focus, world_radius)
        })
        .unwrap_or(LABEL_REFERENCE_PX);
    (
        label_font_size(
            VERTEX_LABEL_BASE_PT,
            render_settings.label_scale,
            render_settings.label_variation,
            screen_radius,
        ),
        label_font_size(
            HYPEREDGE_LABEL_BASE_PT,
            render_settings.label_scale,
            render_settings.label_variation,
            screen_radius,
        ),
    )
}

#[allow(clippy::too_many_arguments)]
fn draw_labels(
    ctx: &egui::Context,
    layout: &GraphLayout,
    render_settings: &NodeRenderSettings,
    node_size: f32,
    hide_hubs: bool,
    font_pt: f32,
    camera_q: &Query<(&Camera, &GlobalTransform, &PanOrbitCamera), With<Camera3d>>,
    label_q: &Query<(
        &SceneNodeEntity,
        &Transform,
        Option<&Selected>,
        Option<&Hovered>,
    )>,
    focus: &FocusScope,
    attention: &AttentionMode,
) {
    let Ok((camera, cam_transform, _)) = camera_q.single() else {
        return;
    };

    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Background,
        egui::Id::new("graph_labels"),
    ));

    let visible_count = layout
        .scene
        .nodes
        .iter()
        .enumerate()
        .filter(|(i, n)| focus.contains(*i) && (!hide_hubs || n.role != NodeRole::HyperedgeHub))
        .count();
    for (node, transform, selected, hovered) in label_q.iter() {
        if !focus.contains(node.index) {
            continue;
        }
        let scene_node = &layout.scene.nodes[node.index];
        if !label_visible_for(
            render_settings,
            visible_count,
            selected.is_some() || hovered.is_some(),
            attention.on && parse_status(&scene_node.status) == EdgeStatus::Attention,
        ) {
            continue;
        }

        let Ok(screen) = camera.world_to_viewport(cam_transform, transform.translation) else {
            continue;
        };

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
            egui::pos2(screen.x, screen.y - font_pt.max(12.0)),
            egui::Align2::CENTER_BOTTOM,
            spec.label,
            egui::FontId::proportional(font_pt),
            color,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_hyperedge_labels(
    ctx: &egui::Context,
    layout: &GraphLayout,
    render_settings: &NodeRenderSettings,
    sel_state: &SelectionState,
    font_pt: f32,
    camera_q: &Query<(&Camera, &GlobalTransform, &PanOrbitCamera), With<Camera3d>>,
    label_q: &Query<(
        &SceneNodeEntity,
        &Transform,
        Option<&Selected>,
        Option<&Hovered>,
    )>,
    focus: &FocusScope,
    attention: &AttentionMode,
    line_cache: &crate::spatial_visibility::LineCache,
    hulls: &HyperedgeHullSettings,
) {
    if !render_settings.hyperedge_labels {
        return;
    }

    let Ok((camera, cam_transform, _)) = camera_q.single() else {
        return;
    };

    let hovered: HashSet<usize> = label_q
        .iter()
        .filter(|(_, _, _, hovered)| hovered.is_some())
        .map(|(node, _, _, _)| node.index)
        .collect();
    let selected: HashSet<usize> = sel_state.base_selection.iter().copied().collect();

    let line_edges: HashSet<_> = line_cache
        .segments
        .iter()
        .filter_map(|(line, _, _)| line.hyperedge)
        .collect();
    let drawable = |i: usize| {
        focus.contains_hyperedge(&layout.scene, i)
            && (line_edges.contains(&i)
                || (hulls.enabled
                    && hulls.opacity > 0.
                    && layout.scene.hyperedges[i].member_indices.len() >= 3))
    };
    let visible_count = layout
        .scene
        .hyperedges
        .iter()
        .enumerate()
        .filter(|(i, _)| drawable(*i))
        .count();
    let show_all = match render_settings.label_mode {
        NodeLabelMode::All => true,
        NodeLabelMode::Capped => visible_count <= render_settings.max_labels,
        NodeLabelMode::SelectionOnly => false,
    };

    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Background,
        egui::Id::new("hyperedge_labels"),
    ));

    for (he_index, he) in layout.scene.hyperedges.iter().enumerate() {
        if !drawable(he_index) {
            continue;
        }
        if he.member_indices.is_empty() {
            continue;
        }
        let member_hot = he
            .member_indices
            .iter()
            .any(|idx| selected.contains(idx) || hovered.contains(idx));
        let attention_hot = attention.on && parse_status(&he.status) == EdgeStatus::Attention;
        if !show_all && !member_hot && !attention_hot {
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
            egui::FontId::proportional(font_pt),
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

fn force_control(ui: &mut egui::Ui, label: &str, value: &mut f32, min: f32, max: f32) {
    let mut enabled = *value > 0.;
    if ui.checkbox(&mut enabled, label).changed() {
        *value = if enabled { min } else { 0. };
    }
    if enabled {
        ui.add(
            egui::Slider::new(value, min..=max)
                .logarithmic(true)
                .clamping(egui::SliderClamping::Edits)
                .show_value(false),
        );
        let speed = (*value as f64 * 0.01).max(min as f64);
        ui.add(
            egui::DragValue::new(value)
                .speed(speed)
                .range(0.0..=f32::MAX)
                .max_decimals(8),
        );
    }
}
