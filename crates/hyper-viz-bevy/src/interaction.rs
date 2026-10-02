use std::collections::HashSet;

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use hyper_viz::{NodeRole, scaled_radius};

use crate::focus::{AttentionMode, FocusScope, FrameRequest, toggle_focus};
use crate::graph::{GraphLayout, GraphSceneEpoch, LayoutSettings};
use crate::hyperedge_hull::{HullWireCache, HyperedgeHullEntity, HyperedgeHullSettings};
use crate::pick::{
    PointerHit, Ray, best_hyperedge_hit, ray_hits_sphere, ray_hits_triangle, ray_segment_hit,
    resolve_hit,
};
use crate::render::{Hovered, SceneNodeEntity, Selected};

#[derive(Resource, Default)]
pub struct LassoState {
    pub enabled: bool,
    pub is_drawing: bool,
    pub points: Vec<Vec2>,
}

#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerTarget {
    #[default]
    None,
    Vertex(usize),
    Hyperedge(usize),
}

#[derive(Resource, Default)]
pub struct SelectionState {
    pub base_selection: Vec<usize>,
    pub hyperedges: Vec<usize>,
    pub generation: u64,
    last_applied_generation: u64,
    node_ids: Vec<String>,
    hyperedge_ids: Vec<String>,
}

impl SelectionState {
    #[allow(dead_code)]
    pub fn has_selection(&self) -> bool {
        !self.base_selection.is_empty() || !self.hyperedges.is_empty()
    }

    pub fn clear(&mut self) {
        self.base_selection.clear();
        self.hyperedges.clear();
        self.node_ids.clear();
        self.hyperedge_ids.clear();
        self.generation += 1;
    }

    pub fn remember_ids(&mut self, scene: &hyper_viz::HypergraphScene) {
        self.node_ids = self
            .base_selection
            .iter()
            .filter_map(|index| scene.nodes.get(*index).map(|node| node.id.clone()))
            .collect();
        self.hyperedge_ids = self
            .hyperedges
            .iter()
            .filter_map(|index| scene.hyperedges.get(*index).map(|he| he.id.clone()))
            .collect();
    }

    pub fn remap_ids(&mut self, scene: &hyper_viz::HypergraphScene) {
        self.base_selection = self
            .node_ids
            .iter()
            .filter_map(|id| {
                scene
                    .nodes
                    .iter()
                    .find(|node| node.id == *id)
                    .map(|node| node.index)
            })
            .collect();
        self.hyperedges = self
            .hyperedge_ids
            .iter()
            .filter_map(|id| scene.hyperedges.iter().position(|he| he.id == *id))
            .collect();
        self.generation += 1;
    }

    pub fn set_selection(&mut self, nodes: Vec<usize>) {
        self.base_selection = nodes;
        self.hyperedges.clear();
        self.generation += 1;
    }

    pub fn set_hyperedge(&mut self, he_index: usize, members: Vec<usize>) {
        self.hyperedges = vec![he_index];
        self.base_selection = members;
        self.generation += 1;
    }

    pub fn set_hits(&mut self, nodes: Vec<usize>, hyperedges: Vec<usize>) {
        self.base_selection = nodes;
        self.hyperedges = hyperedges;
        self.generation += 1;
    }

    pub fn bump(&mut self) {
        self.generation += 1;
    }

    fn needs_apply(&self) -> bool {
        self.generation != self.last_applied_generation
    }

    fn mark_applied(&mut self) {
        self.last_applied_generation = self.generation;
    }
}

/// Navigation-box query. `search_owned` is true while the current selection
/// came from typing, so clearing the box does not wipe a prior click.
#[derive(Resource, Debug, Clone, Default)]
pub struct LocalizeQuery {
    pub query: String,
    pub focus_box: bool,
    pub input_focused: bool,
    pub search_owned: bool,
    pub isolated: bool,
}

impl LocalizeQuery {
    pub fn release_ownership(&mut self) {
        self.search_owned = false;
        self.isolated = false;
    }
}

pub struct InteractionPlugin;

impl Plugin for InteractionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LassoState>()
            .init_resource::<SelectionState>()
            .init_resource::<LocalizeQuery>()
            .init_resource::<PointerTarget>()
            .add_systems(
                Update,
                (
                    remap_selection_on_reload,
                    keyboard_controls.run_if(resource_exists::<GraphLayout>),
                    pointer_hover.run_if(resource_exists::<GraphLayout>),
                    click_selection.run_if(resource_exists::<GraphLayout>),
                    lasso_interaction.run_if(resource_exists::<GraphLayout>),
                    disable_orbit_on_lasso,
                    apply_selection_state.run_if(resource_exists::<GraphLayout>),
                ),
            );
    }
}

fn remap_selection_on_reload(
    epoch: Res<GraphSceneEpoch>,
    layout: Option<Res<GraphLayout>>,
    mut last_epoch: Local<Option<u64>>,
    mut sel_state: ResMut<SelectionState>,
    mut focus: ResMut<FocusScope>,
    mut focus_ids: Local<Option<HashSet<String>>>,
) {
    let Some(layout) = layout else {
        return;
    };
    if *last_epoch == Some(epoch.0) {
        sel_state.remember_ids(&layout.scene);
        *focus_ids = focus.nodes.as_ref().map(|nodes| {
            nodes
                .iter()
                .filter_map(|index| layout.scene.nodes.get(*index).map(|node| node.id.clone()))
                .collect()
        });
        return;
    }
    *last_epoch = Some(epoch.0);
    if epoch.0 == 0 {
        return;
    }
    sel_state.remap_ids(&layout.scene);
    if let Some(ids) = focus_ids.as_ref() {
        let remapped: HashSet<usize> = ids
            .iter()
            .filter_map(|id| {
                layout
                    .scene
                    .nodes
                    .iter()
                    .find(|node| node.id == *id)
                    .map(|node| node.index)
            })
            .collect();
        if remapped.is_empty() {
            focus.clear();
        } else {
            focus.nodes = Some(remapped);
        }
    }
}

fn keyboard_wants_text(contexts: &mut bevy_egui::EguiContexts) -> bool {
    contexts
        .ctx_mut()
        .map(|ctx| ctx.wants_keyboard_input())
        .unwrap_or(false)
}

fn modifier_down(keys: &ButtonInput<KeyCode>, left: KeyCode, right: KeyCode) -> bool {
    keys.pressed(left) || keys.pressed(right)
}

#[allow(clippy::too_many_arguments)]
fn keyboard_controls(
    keys: Res<ButtonInput<KeyCode>>,
    mut layout: ResMut<GraphLayout>,
    mut attention: ResMut<AttentionMode>,
    mut focus: ResMut<FocusScope>,
    mut frame: ResMut<FrameRequest>,
    sel_state: Res<SelectionState>,
    mut localize: ResMut<LocalizeQuery>,
    mut contexts: bevy_egui::EguiContexts,
) {
    let cmd = modifier_down(&keys, KeyCode::SuperLeft, KeyCode::SuperRight);
    let ctrl = modifier_down(&keys, KeyCode::ControlLeft, KeyCode::ControlRight);
    let shift = modifier_down(&keys, KeyCode::ShiftLeft, KeyCode::ShiftRight);
    let in_find = localize.input_focused || keyboard_wants_text(&mut contexts);

    if keys.just_pressed(KeyCode::KeyF) && cmd && ctrl {
        toggle_focus(
            &mut focus,
            &layout,
            &sel_state.base_selection,
            &sel_state.hyperedges,
        );
        tracing::info!(active = focus.is_active(), "Focus neighborhood");
        return;
    }
    if keys.just_pressed(KeyCode::KeyF) && cmd && !ctrl && !shift {
        localize.focus_box = true;
        return;
    }
    if keys.just_pressed(KeyCode::KeyF) && ctrl && !cmd && !shift && !in_find {
        localize.focus_box = true;
        return;
    }

    if in_find {
        return;
    }
    if keys.just_pressed(KeyCode::Slash) && !shift {
        localize.focus_box = true;
        return;
    }
    if keys.just_pressed(KeyCode::Space) {
        layout.running = !layout.running;
        tracing::info!(running = layout.running, "Layout toggled");
    }
    if keys.just_pressed(KeyCode::KeyA) {
        attention.on = !attention.on;
        tracing::info!(on = attention.on, "Attention mode");
    }
    if keys.just_pressed(KeyCode::KeyF) && !cmd && !ctrl && !shift {
        frame.pending = true;
    }
    if keys.just_pressed(KeyCode::Escape) && focus.is_active() {
        focus.clear();
        tracing::info!("Focus cleared");
    }
}

fn pointer_over_ui(contexts: &mut bevy_egui::EguiContexts) -> bool {
    contexts
        .ctx_mut()
        .map(|ctx| ctx.is_pointer_over_area() || ctx.wants_pointer_input())
        .unwrap_or(false)
}

#[allow(clippy::too_many_arguments)]
fn pointer_hover(
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    layout: Res<GraphLayout>,
    settings: Res<LayoutSettings>,
    hull_settings: Option<Res<HyperedgeHullSettings>>,
    mut hovered_q: Query<(Entity, &SceneNodeEntity), With<Hovered>>,
    node_q: Query<(Entity, &SceneNodeEntity, &Transform, &Visibility)>,
    hulls: Query<(&HyperedgeHullEntity, &HullWireCache)>,
    mut commands: Commands,
    mut contexts: bevy_egui::EguiContexts,
    lasso: Res<LassoState>,
    mut target: ResMut<PointerTarget>,
    focus: Res<FocusScope>,
) {
    if lasso.enabled || pointer_over_ui(&mut contexts) {
        *target = PointerTarget::None;
        clear_hovered(&mut commands, &hovered_q);
        return;
    }

    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        *target = PointerTarget::None;
        clear_hovered(&mut commands, &hovered_q);
        return;
    };
    let Ok((camera, cam_transform)) = cameras.single() else {
        return;
    };
    let Ok(ray3) = camera.viewport_to_world(cam_transform, cursor) else {
        return;
    };
    let ray = Ray {
        origin: ray3.origin,
        dir: *ray3.direction,
    };

    let hide_hubs = hull_settings.as_ref().is_some_and(|s| s.hide_hubs);
    let pick_radius = scaled_radius(layout.node_count, settings.node_size) * 1.8;

    let mut best_vertex: Option<(usize, f32)> = None;
    let mut best_vertex_entity: Option<Entity> = None;
    let mut entity_by_index: Vec<Option<Entity>> = vec![None; layout.node_count];

    for (entity, node, transform, visibility) in node_q.iter() {
        if node.index < entity_by_index.len() {
            entity_by_index[node.index] = Some(entity);
        }
        if *visibility == Visibility::Hidden || !focus.contains(node.index) {
            continue;
        }
        let Some(scene_node) = layout.scene.nodes.get(node.index) else {
            continue;
        };
        if hide_hubs && scene_node.role == NodeRole::HyperedgeHub {
            continue;
        }
        let Some(t) = ray_hits_sphere(ray, transform.translation, pick_radius) else {
            continue;
        };
        if best_vertex.is_none_or(|(_, best_t)| t < best_t) {
            best_vertex = Some((node.index, t));
            best_vertex_entity = Some(entity);
        }
    }

    let hulls_on = hull_settings.as_ref().is_none_or(|s| s.enabled);
    let mut hull_hits: Vec<(usize, usize, f32)> = Vec::new();
    if hulls_on {
        for (entity, cache) in hulls.iter() {
            if cache.indices.len() < 3 {
                continue;
            }
            let mut best_t: Option<f32> = None;
            for tri in cache.indices.chunks_exact(3) {
                let a = Vec3::from_array(cache.positions[tri[0] as usize]);
                let b = Vec3::from_array(cache.positions[tri[1] as usize]);
                let c = Vec3::from_array(cache.positions[tri[2] as usize]);
                if let Some(t) = ray_hits_triangle(ray, a, b, c)
                    && best_t.is_none_or(|cur| t < cur)
                {
                    best_t = Some(t);
                }
            }
            if let Some(t) = best_t
                && layout
                    .scene
                    .hyperedges
                    .get(entity.hyperedge_index)
                    .is_some_and(|he| focus.contains(he.hub_index))
            {
                hull_hits.push((entity.hyperedge_index, cache.member_scene_indices.len(), t));
            }
        }
    }
    for (he_index, he) in layout.scene.hyperedges.iter().enumerate() {
        if he.member_indices.len() != 2 {
            continue;
        }
        let (Some(p1), Some(p2)) = (
            layout.position_at(he.member_indices[0]),
            layout.position_at(he.member_indices[1]),
        ) else {
            continue;
        };
        if !focus.contains(he.hub_index)
            && !he.member_indices.iter().all(|idx| focus.contains(*idx))
        {
            continue;
        }
        if let Some(t) = ray_segment_hit(ray, p1, p2, pick_radius) {
            hull_hits.push((he_index, 2, t));
        }
    }

    let hyperedge = best_hyperedge_hit(&hull_hits);
    let hit = resolve_hit(best_vertex, hyperedge);

    *target = match hit {
        Some(PointerHit::Vertex(i)) => PointerTarget::Vertex(i),
        Some(PointerHit::Hyperedge(i)) => PointerTarget::Hyperedge(i),
        None => PointerTarget::None,
    };

    let mut keep: HashSet<Entity> = HashSet::new();
    match *target {
        PointerTarget::Vertex(idx) => {
            if let Some(entity) = best_vertex_entity {
                keep.insert(entity);
            } else if let Some(entity) = entity_by_index.get(idx).copied().flatten() {
                keep.insert(entity);
            }
        }
        PointerTarget::Hyperedge(he_idx) => {
            if let Some(he) = layout.scene.hyperedges.get(he_idx) {
                for &idx in &he.member_indices {
                    if let Some(entity) = entity_by_index.get(idx).copied().flatten() {
                        keep.insert(entity);
                    }
                }
                if !hide_hubs
                    && let Some(entity) = entity_by_index.get(he.hub_index).copied().flatten()
                {
                    keep.insert(entity);
                }
            }
        }
        PointerTarget::None => {}
    }

    for (entity, _) in hovered_q.iter_mut() {
        if !keep.contains(&entity) {
            commands.entity(entity).remove::<Hovered>();
        }
    }
    for entity in keep {
        commands.entity(entity).insert(Hovered);
    }
}

fn clear_hovered(
    commands: &mut Commands,
    hovered_q: &Query<(Entity, &SceneNodeEntity), With<Hovered>>,
) {
    for (entity, _) in hovered_q.iter() {
        commands.entity(entity).remove::<Hovered>();
    }
}

#[derive(Clone, Copy)]
struct PendingPointerClick {
    screen: Vec2,
    target: PointerTarget,
}

/// Movement beyond this is an orbit drag, not a selection click.
pub(crate) const CLICK_MAX_DRAG_PX: f32 = 6.0;

pub(crate) fn is_selection_click(press: Vec2, release: Vec2) -> bool {
    press.distance(release) <= CLICK_MAX_DRAG_PX
}

#[allow(clippy::too_many_arguments)]
fn click_selection(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    selected_q: Query<(Entity, &SceneNodeEntity), With<Selected>>,
    mut commands: Commands,
    mut contexts: bevy_egui::EguiContexts,
    keys: Res<ButtonInput<KeyCode>>,
    lasso: Res<LassoState>,
    mut sel_state: ResMut<SelectionState>,
    mut localize: ResMut<LocalizeQuery>,
    layout: Res<GraphLayout>,
    target: Res<PointerTarget>,
    mut pending: Local<Option<PendingPointerClick>>,
) {
    if lasso.enabled {
        pending.take();
        return;
    }
    if pointer_over_ui(&mut contexts) {
        if mouse.just_pressed(MouseButton::Left) {
            pending.take();
        }
        return;
    }

    if mouse.just_pressed(MouseButton::Left) {
        let Ok(window) = windows.single() else {
            return;
        };
        if let Some(screen) = window.cursor_position() {
            *pending = Some(PendingPointerClick {
                screen,
                target: *target,
            });
        }
        return;
    }

    if !mouse.just_released(MouseButton::Left) {
        return;
    }
    let Some(start) = pending.take() else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(release) = window.cursor_position() else {
        return;
    };
    if !is_selection_click(start.screen, release) {
        return;
    }

    let multi = keys.pressed(KeyCode::ShiftLeft)
        || keys.pressed(KeyCode::ShiftRight)
        || keys.pressed(KeyCode::SuperLeft)
        || keys.pressed(KeyCode::SuperRight);

    match start.target {
        PointerTarget::Vertex(idx) => {
            if multi {
                if !sel_state.base_selection.contains(&idx) {
                    sel_state.base_selection.push(idx);
                }
                sel_state.bump();
            } else {
                sel_state.set_selection(vec![idx]);
            }
        }
        PointerTarget::Hyperedge(he_idx) => {
            let Some(he) = layout.scene.hyperedges.get(he_idx) else {
                return;
            };
            let members = he.member_indices.clone();
            if multi {
                for idx in &members {
                    if !sel_state.base_selection.contains(idx) {
                        sel_state.base_selection.push(*idx);
                    }
                }
                if !sel_state.hyperedges.contains(&he_idx) {
                    sel_state.hyperedges.push(he_idx);
                }
                sel_state.bump();
            } else {
                sel_state.set_hyperedge(he_idx, members);
            }
        }
        PointerTarget::None if !multi => {
            sel_state.clear();
        }
        PointerTarget::None => {}
    }
    localize.release_ownership();

    let effective: HashSet<usize> = sel_state.base_selection.iter().copied().collect();
    for (entity, node) in selected_q.iter() {
        if !effective.contains(&node.index) {
            commands.entity(entity).remove::<Selected>();
        }
    }
}

fn disable_orbit_on_lasso(
    mut cam_q: Query<&mut bevy_panorbit_camera::PanOrbitCamera>,
    lasso: Res<LassoState>,
) {
    if lasso.is_changed() {
        for mut cam in cam_q.iter_mut() {
            cam.enabled = !lasso.enabled;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn lasso_interaction(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut lasso: ResMut<LassoState>,
    mut sel_state: ResMut<SelectionState>,
    mut localize: ResMut<LocalizeQuery>,
    layout: Res<GraphLayout>,
    node_q: Query<(&SceneNodeEntity, &Transform, &Visibility)>,
    mut contexts: bevy_egui::EguiContexts,
) {
    if !lasso.enabled {
        return;
    }
    if pointer_over_ui(&mut contexts) {
        return;
    }

    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };

    if mouse.just_pressed(MouseButton::Left) {
        lasso.is_drawing = true;
        lasso.points.clear();
        lasso.points.push(cursor);
    }

    if lasso.is_drawing
        && mouse.pressed(MouseButton::Left)
        && lasso
            .points
            .last()
            .is_none_or(|p| (*p - cursor).length() > 4.0)
    {
        lasso.points.push(cursor);
    }

    if lasso.is_drawing && mouse.just_released(MouseButton::Left) {
        lasso.is_drawing = false;
        if lasso.points.len() < 3 {
            lasso.points.clear();
            return;
        }

        let Ok((camera, cam_transform)) = cameras.single() else {
            return;
        };

        let mut selected = Vec::new();
        for (node, transform, visibility) in node_q.iter() {
            if *visibility == Visibility::Hidden {
                continue;
            }
            if layout
                .scene
                .nodes
                .get(node.index)
                .is_some_and(|n| n.role == NodeRole::HyperedgeHub)
            {
                continue;
            }
            if let Ok(screen) = camera.world_to_viewport(cam_transform, transform.translation)
                && crate::pick::point_in_polygon(screen, &lasso.points)
            {
                selected.push(node.index);
            }
        }

        selected.sort_unstable();
        selected.dedup();
        sel_state.set_selection(selected);
        localize.release_ownership();
        lasso.points.clear();
    }
}

fn apply_selection_state(
    mut sel_state: ResMut<SelectionState>,
    layout: Res<GraphLayout>,
    mut node_q: Query<(Entity, &SceneNodeEntity), With<Selected>>,
    all_nodes: Query<(Entity, &SceneNodeEntity)>,
    mut commands: Commands,
) {
    if !sel_state.needs_apply() {
        return;
    }

    let effective: HashSet<usize> = sel_state.base_selection.iter().copied().collect();

    for (entity, node) in node_q.iter_mut() {
        if !effective.contains(&node.index) {
            commands.entity(entity).remove::<Selected>();
        }
    }

    for (entity, node) in all_nodes.iter() {
        if effective.contains(&node.index) {
            commands.entity(entity).insert(Selected);
        }
    }

    sel_state.remember_ids(&layout.scene);
    sel_state.mark_applied();
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper_viz::{Hypergraph, Projection, project};

    #[test]
    fn drag_is_not_a_selection_click() {
        let press = Vec2::new(100.0, 80.0);
        assert!(is_selection_click(press, Vec2::new(104.0, 82.0)));
        assert!(!is_selection_click(press, Vec2::new(140.0, 80.0)));
    }

    #[test]
    fn set_hyperedge_selects_members() {
        let scene = project(&sample(), Projection::Bipartite);
        let he = scene
            .hyperedges
            .iter()
            .find(|h| he_label(h) == "Paper A")
            .expect("paper a");
        let mut state = SelectionState::default();
        state.set_hyperedge(0, he.member_indices.clone());
        assert_eq!(state.hyperedges, vec![0]);
        assert_eq!(state.base_selection.len(), 3);
    }

    #[test]
    fn set_hits_keeps_nodes_and_hyperedges() {
        let mut state = SelectionState::default();
        state.set_hits(vec![1, 2], vec![0]);
        assert_eq!(state.base_selection, vec![1, 2]);
        assert_eq!(state.hyperedges, vec![0]);
        assert!(state.has_selection());
    }

    #[test]
    fn remap_keeps_selection_when_indices_shift() {
        let mut first = Hypergraph::new();
        first.add_vertex(hyper_viz::Vertex::new("keep", "Keep"));
        first.add_vertex(hyper_viz::Vertex::new("other", "Other"));
        let left = project(&first, Projection::Bipartite);
        let keep = left
            .nodes
            .iter()
            .find(|n| n.id == "keep")
            .expect("keep")
            .index;
        let mut state = SelectionState::default();
        state.set_selection(vec![keep]);
        state.remember_ids(&left);

        let mut second = Hypergraph::new();
        second.add_vertex(hyper_viz::Vertex::new("other", "Other"));
        second.add_vertex(hyper_viz::Vertex::new("keep", "Keep"));
        let right = project(&second, Projection::Bipartite);
        state.remap_ids(&right);
        assert_eq!(state.base_selection.len(), 1);
        assert_eq!(right.nodes[state.base_selection[0]].id, "keep");
    }

    fn sample() -> Hypergraph {
        Hypergraph::new()
            .vertex("a", "Alice", "person")
            .vertex("b", "Bob", "person")
            .vertex("c", "Carol", "person")
            .hyperedge("paper-a", ["a", "b", "c"], "Paper A")
    }

    fn he_label(he: &hyper_viz::SceneHyperedge) -> &str {
        &he.label
    }
}
