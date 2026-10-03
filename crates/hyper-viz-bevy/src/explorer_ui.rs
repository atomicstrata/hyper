use crate::{explorer_state::ExplorerState, focus::FocusScope, graph::GraphLayout};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use bevy_panorbit_camera::PanOrbitCamera;
use hyper_viz::{TraversalDepth, session::ViewMode};
use std::collections::HashSet;

fn depth(ui: &mut egui::Ui, label: &str, value: &mut TraversalDepth) {
    egui::ComboBox::from_label(label)
        .selected_text(match value {
            TraversalDepth::Off => "Off".into(),
            TraversalDepth::Hops(n) => format!("{n} hop(s)"),
            TraversalDepth::Transitive => "Transitive".into(),
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(value, TraversalDepth::Off, "Off");
            for n in 1..=6 {
                ui.selectable_value(value, TraversalDepth::Hops(n), format!("{n} hop(s)"));
            }
            ui.selectable_value(value, TraversalDepth::Transitive, "Transitive");
        });
}
pub fn scoped_edges(state: &ExplorerState) -> HashSet<usize> {
    let nodes: HashSet<_> = state.scope.nodes.iter().copied().collect();
    let mut edges: HashSet<_> = state
        .index
        .relations()
        .iter()
        .filter(|r| nodes.contains(&r.source) && nodes.contains(&r.target))
        .flat_map(|r| r.hyperedge_indices.iter().copied())
        .collect();
    edges.extend(state.group);
    edges
}
#[allow(clippy::too_many_arguments)]
pub fn explorer_panel(
    mut contexts: EguiContexts,
    mut state: ResMut<ExplorerState>,
    layout: Res<GraphLayout>,
    mut focus: ResMut<FocusScope>,
    mut search: Local<String>,
    mut target: Local<String>,
    mut list: Local<bool>,
    mut hulls: ResMut<crate::hyperedge_hull::HyperedgeHullSettings>,
    mut frame: ResMut<crate::focus::FrameRequest>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    let previous_mode = state.mode;
    egui::TopBottomPanel::top("explorer_mode").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.strong("Module explorer");
            ui.selectable_value(&mut state.mode, ViewMode::Dependencies, "Dependencies");
            ui.selectable_value(&mut state.mode, ViewMode::Spatial, "Spatial");
        });
    });
    if state.mode != previous_mode {
        state.user_mode = true;
    }
    if state.mode != ViewMode::Dependencies {
        return;
    }
    let before = state.snapshot();
    let mut selected = None;
    let mut target_selected = None;
    egui::SidePanel::left("module_controls")
        .default_width(330.)
        .min_width(280.)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Back").clicked() { state.back(); }
                    if ui.button("Forward").clicked() { state.forward(); }
                });
                selected = module_picker(ui, "Find a module", &mut search, &mut state, &layout.scene);
                ui.separator();
                if let Some(center) = state.current.center.clone() {
                    ui.strong(&center);
                    if let Some(i) = state.index.node_index(&center) {
                        ui.label(format!("{} direct imports · {} direct dependents", state.index.imports(i).len(), state.index.dependents(i).len()));
                        if let Some(path) = layout.scene.nodes[i].attrs.get("path").and_then(|v| v.as_str()) { ui.label(path); }
                    } else { ui.label("This module is absent from the current graph. Choose another module."); }
                } else { ui.label("Search and choose one module to begin."); }
                depth(ui, "Imports", &mut state.current.options.imports);
                depth(ui, "Dependents", &mut state.current.options.dependents);
                ui.horizontal(|ui| {
                    ui.label("Node budget");
                    ui.add(egui::DragValue::new(&mut state.current.options.budget).range(1..=1000));
                });
                ui.collapsing("Include categories", |ui| {
                    ui.checkbox(&mut state.current.filters.umbrella, "Umbrella modules");
                    ui.checkbox(&mut state.current.filters.tests, "Tests");
                    ui.checkbox(&mut state.current.filters.tactics, "Tactics");
                    ui.checkbox(&mut state.current.filters.external, "External modules");
                });
                ui.label(format!("{} shown · {} budget omitted · {} excluded by filters", state.scope.nodes.len(), state.scope.omitted, state.canonical.total_reachable.saturating_sub(state.scope.total_reachable)));
                ui.small("Filters stop traversal; the selected module is always retained.");
                ui.horizontal(|ui| {
                    if ui.button("Reset expansion").clicked() { state.current.expanded_ids.clear(); }
                    if ui.button("Fit").clicked() { state.fit_requested = true; }
                });
                ui.checkbox(&mut list, "List view");
                if ui.button("Inspect displayed scope in Spatial").clicked() {
                    focus.nodes = Some(state.scope.nodes.iter().copied().collect());
                    focus.hyperedges = Some(scoped_edges(&state));
                    if state.group.is_some() { hulls.enabled = true; }
                    frame.pending = true;
                    state.choose_mode(ViewMode::Spatial);
                }
                ui.separator();
                target_selected = module_picker(ui, "Shortest import path to", &mut target, &mut state, &layout.scene);
                if let Some(target) = &state.current.target {
                    ui.label(format!("Target: {target}"));
                    if state.index.node_index(target).is_none() { ui.label("The target is absent from the current graph."); }
                }
                if let Some(path) = &state.path {
                    ui.label(format!("{} import hop(s)", path.len().saturating_sub(1)));
                    let omitted=path.iter().filter(|i|!state.scope.nodes.contains(i)).count();
                    if omitted>0 {ui.label(format!("{omitted} path modules omitted from canvas by node budget; full path below."));}
                    for &i in path { ui.label(&layout.scene.nodes[i].id); }
                } else if state.current.target.is_some() {
                    ui.label(if state.path_blocked { "A path exists, but current filters block it." } else { "No directed import path exists." });
                }
                if let Some(center) = state.current.center.as_deref().and_then(|id| state.index.node_index(id)) {
                    for (label, neighbors) in [("Direct imports", state.index.imports(center).to_vec()), ("Direct dependents", state.index.dependents(center).to_vec())] {
                        ui.collapsing(format!("{label} ({})", neighbors.len()), |ui| {
                            for i in neighbors {
                                ui.horizontal(|ui| {
                                    if ui.selectable_label(false, &layout.scene.nodes[i].id).clicked() { selected = Some(layout.scene.nodes[i].id.clone()); }
                                    if ui.small_button("+1").on_hover_text("Expand this module by one hop").clicked() { state.expand(layout.scene.nodes[i].id.clone()); }
                                });
                            }
                        });
                    }
                    let center_id = &layout.scene.nodes[center].id;
                    let exported_id = format!("deps:{center_id}");
                    let group = layout.scene.hyperedges.iter().enumerate().find(|(_, e)| e.kind == "dependency_group" && (e.attrs.get("source").and_then(|v| v.as_str()) == Some(center_id) || e.id == exported_id));
                    if let Some((i, group)) = group {
                        ui.collapsing(format!("Exported dependency group ({})", group.member_indices.len()), |ui| {
                            let mut include = state.group == Some(i);
                            if ui.checkbox(&mut include, "Include this group in Spatial").changed() { state.group = if include { Some(i) } else { None }; }
                            if group.member_indices.iter().any(|i| !state.scope.nodes.contains(i)) { ui.small("Its hull appears when all members are in the displayed scope. The full member list follows."); }
                            for &member in &group.member_indices { ui.label(&layout.scene.nodes[member].id); }
                        });
                    }
                }
                ui.separator();
                ui.small("Blue: module · Gold: umbrella / center\nGreen: tactic · Pink: test · Purple: external");
                if !state.index.warnings().is_empty() { ui.label(format!("{} malformed imports excluded from directed queries", state.index.warnings().len())); }
            });
        });
    if let Some(id) = selected {
        state.select_module(&id);
    }
    if let Some(id) = target_selected {
        state.current.target = Some(id);
    }
    if before != state.current {
        state.rebuild();
        state.record();
    }
    egui::CentralPanel::default().show(ctx, |ui| {
        if state.scope.nodes.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label("Choose a module to inspect its dependencies and dependents.");
            });
        } else if *list {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for i in state.scope.nodes.clone() {
                    ui.horizontal(|ui| {
                        if ui.button(&layout.scene.nodes[i].id).clicked() {
                            state.select_module(&layout.scene.nodes[i].id);
                        }
                        if ui.button("Expand +1").clicked() {
                            state.expand(layout.scene.nodes[i].id.clone());
                        }
                    });
                }
            });
        } else {
            crate::explorer_canvas::canvas(ui, &mut state, &layout.scene);
        }
    });
}
fn module_picker(
    ui: &mut egui::Ui,
    label: &str,
    query: &mut String,
    state: &mut ExplorerState,
    scene: &hyper_viz::HypergraphScene,
) -> Option<String> {
    ui.label(label);
    let response = ui.add(egui::TextEdit::singleline(query).id(egui::Id::new(label)));
    let results = state.search(query).to_vec();
    let mut chosen = None;
    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        chosen = results.first().map(|i| scene.nodes[*i].id.clone());
    }
    if !query.trim().is_empty() {
        if results.is_empty() {
            ui.label("No matching module.");
        }
        for i in results {
            if ui.selectable_label(false, &scene.nodes[i].id).clicked() {
                chosen = Some(scene.nodes[i].id.clone());
            }
        }
    }
    chosen
}

#[allow(clippy::type_complexity)]
pub fn spatial_visibility(
    state: Res<ExplorerState>,
    mut nodes: Query<
        &mut Visibility,
        Or<(
            With<crate::render::SceneNodeEntity>,
            With<crate::hyperedge_hull::HyperedgeHullEntity>,
        )>,
    >,
    mut cameras: Query<&mut PanOrbitCamera>,
    lasso: Res<crate::interaction::LassoState>,
    mut previous: Local<Option<ViewMode>>,
    mut running: Local<bool>,
    mut layout: Option<ResMut<GraphLayout>>,
) {
    if *previous == Some(state.mode) {
        return;
    }
    let spatial = state.mode == ViewMode::Spatial;
    if let Some(ref mut layout) = layout {
        if !spatial {
            *running = layout.running;
            layout.running = false;
        } else if previous.is_some() {
            layout.running = *running;
        }
    }
    if !spatial {
        for mut visibility in &mut nodes {
            *visibility = Visibility::Hidden;
        }
    } else {
        for mut visibility in &mut nodes {
            if *visibility == Visibility::Hidden {
                *visibility = Visibility::Inherited;
            }
        }
    }
    for mut camera in &mut cameras {
        camera.enabled = spatial && !lasso.enabled;
    }
    *previous = Some(state.mode);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_egui::{EguiContext, EguiUserTextures, PrimaryEguiContext};
    use hyper_viz::{Hyperedge, Hypergraph, Projection};
    fn input_frame(app: &mut App, ctx: &egui::Context, events: Vec<egui::Event>) {
        ctx.begin_pass(egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600., 900.),
            )),
            events,
            ..Default::default()
        });
        app.update();
        let _ = ctx.end_pass();
    }
    fn type_into(app: &mut App, ctx: &egui::Context, label: &str, text: &str) {
        let rect = ctx.read_response(egui::Id::new(label)).unwrap().rect;
        let pos = rect.center();
        input_frame(
            app,
            ctx,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: Default::default(),
                },
                egui::Event::Text(text.into()),
            ],
        );
    }
    fn enter(app: &mut App, ctx: &egui::Context) {
        input_frame(
            app,
            ctx,
            vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Default::default(),
            }],
        );
    }
    #[test]
    fn spatial_transfer_contains_only_displayed_imports_and_chosen_group() {
        let mut graph = Hypergraph::new();
        for id in ["a", "b", "c", "d"] {
            graph = graph.vertex(id, id, "module");
        }
        for (a, b) in [("a", "b"), ("b", "c")] {
            let mut edge = Hyperedge::new(format!("{a}:{b}"), [a, b]).with_kind("import");
            edge.attrs.insert("source".into(), a.into());
            edge.attrs.insert("target".into(), b.into());
            graph.add_hyperedge(edge);
        }
        graph = graph.hyperedge("deps:a", ["a", "b"], "deps").hyperedge(
            "global",
            ["a", "b", "c", "d"],
            "global",
        );
        let mut state = ExplorerState::default();
        state.replace_scene(&graph.project(Projection::StarCentroid), 0);
        state.select_module("a");
        assert_eq!(state.scope.nodes, [0, 1]);
        assert_eq!(scoped_edges(&state), HashSet::from([0]));
        state.group = Some(2);
        assert_eq!(scoped_edges(&state), HashSet::from([0, 2]));
    }
    #[test]
    fn switching_modes_restores_running_state_and_hides_spatial_entities() {
        for running in [true, false] {
            let scene = Hypergraph::new()
                .vertex("a", "a", "module")
                .project(Projection::StarCentroid);
            let mut layout =
                GraphLayout::from_scene(scene, &crate::graph::LayoutSettings::default());
            layout.running = running;
            let mut state = ExplorerState::default();
            state.choose_mode(ViewMode::Dependencies);
            let mut app = App::new();
            app.insert_resource(layout)
                .insert_resource(state)
                .init_resource::<crate::interaction::LassoState>()
                .add_systems(Update, spatial_visibility);
            let node = app
                .world_mut()
                .spawn((
                    crate::render::SceneNodeEntity { index: 0 },
                    Visibility::Inherited,
                ))
                .id();
            app.update();
            assert!(!app.world().resource::<GraphLayout>().running);
            assert_eq!(
                *app.world().get::<Visibility>(node).unwrap(),
                Visibility::Hidden
            );
            app.world_mut()
                .resource_mut::<ExplorerState>()
                .choose_mode(ViewMode::Spatial);
            app.update();
            assert_eq!(app.world().resource::<GraphLayout>().running, running);
            assert_eq!(
                *app.world().get::<Visibility>(node).unwrap(),
                Visibility::Inherited
            );
        }
    }

    #[test]
    fn actual_panel_selects_one_exact_module_and_traces_beyond_depth() {
        let mut graph = Hypergraph::new()
            .vertex("a", "a", "module")
            .vertex("ab", "ab", "module")
            .vertex("c", "c", "module");
        for (a, b) in [("a", "ab"), ("ab", "c")] {
            let mut e = Hyperedge::new(format!("{a}:{b}"), [a, b]).with_kind("import");
            e.attrs.insert("source".into(), a.into());
            e.attrs.insert("target".into(), b.into());
            graph.add_hyperedge(e);
        }
        let scene = graph.project(Projection::StarCentroid);
        let mut state = ExplorerState::default();
        state.replace_scene(&scene, 0);
        let mut app = App::new();
        app.insert_resource(state)
            .insert_resource(GraphLayout::from_scene(
                scene,
                &crate::graph::LayoutSettings::default(),
            ))
            .init_resource::<FocusScope>()
            .init_resource::<crate::focus::FrameRequest>()
            .init_resource::<crate::hyperedge_hull::HyperedgeHullSettings>()
            .init_resource::<EguiUserTextures>()
            .add_systems(Update, explorer_panel);
        let entity = app
            .world_mut()
            .spawn((EguiContext::default(), PrimaryEguiContext))
            .id();
        let ctx = app
            .world_mut()
            .get_mut::<EguiContext>(entity)
            .unwrap()
            .get_mut()
            .clone();
        input_frame(&mut app, &ctx, Vec::new());
        type_into(&mut app, &ctx, "Find a module", "a");
        assert!(
            app.world()
                .resource::<ExplorerState>()
                .current
                .center
                .is_none()
        );
        enter(&mut app, &ctx);
        assert_eq!(
            app.world()
                .resource::<ExplorerState>()
                .current
                .center
                .as_deref(),
            Some("a")
        );
        assert_eq!(app.world().resource::<ExplorerState>().scope.nodes.len(), 2);
        type_into(&mut app, &ctx, "Shortest import path to", "c");
        enter(&mut app, &ctx);
        let state = app.world().resource::<ExplorerState>();
        assert_eq!(state.path, Some(vec![0, 1, 2]));
        assert_eq!(state.scope.nodes.len(), 3);
    }
}
