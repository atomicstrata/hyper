use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use hyper_viz::{Emphasis, apply_motion_rgba, emphasis_radius_scale, emphasize, link_style_for};

use crate::animation::{StatusBursts, motion_for, node_motion};
use crate::focus::{AttentionMode, apply_attention_rgba};
use crate::graph::{GraphLayout, GraphSceneEpoch, LayoutSettings};
use crate::hyperedge_hull::HyperedgeHullSettings;
use crate::interaction::{PointerTarget, SelectionState};
use crate::node_visual::{
    NodeRenderSettings, material_for_node, node_emphasized_rgba, visual_spec_for,
};

#[derive(Component)]
pub struct SceneNodeEntity {
    pub index: usize,
}

#[derive(Component)]
pub struct SceneNodeKey(pub String);

#[derive(Component)]
pub struct Hovered;

#[derive(Component)]
pub struct Selected;

/// Maximum visible lines per frame. Zero draws every line.
#[derive(Resource, Clone)]
pub struct LinkRenderSettings {
    pub max_lines: usize,
    pub opacity: f32,
}

impl Default for LinkRenderSettings {
    fn default() -> Self {
        Self {
            max_lines: 5000,
            opacity: 1.0,
        }
    }
}

impl LinkRenderSettings {
    pub fn budget(&self) -> usize {
        if self.max_lines == 0 {
            usize::MAX
        } else {
            self.max_lines
        }
    }
}

#[derive(Resource)]
pub struct GraphAssets {
    pub node_mesh: Handle<Mesh>,
    pub node_mesh_lod: Handle<Mesh>,
}

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NodeRenderSettings>()
            .init_resource::<LinkRenderSettings>()
            .init_resource::<crate::animation::StatusBursts>()
            .add_systems(Startup, setup_assets)
            // Apply the scene replacement and remap entities before any consumer
            // reads node indices, including the egui label pass later this frame.
            .add_systems(
                PreUpdate,
                sync_graph_nodes
                    .after(crate::explorer_state::sync_explorer)
                    .after(crate::graph::poll_live_scene)
                    .after(crate::graph::poll_graph_watch)
                    .run_if(resource_exists::<GraphLayout>)
                    .run_if(crate::explorer_state::spatial_mode),
            )
            .add_systems(
                Update,
                (
                    update_node_positions
                        .run_if(resource_exists::<GraphLayout>)
                        .run_if(crate::explorer_state::spatial_mode),
                    draw_links
                        .after(crate::spatial_visibility::cache_lines)
                        .run_if(resource_exists::<GraphLayout>)
                        .run_if(crate::explorer_state::spatial_mode),
                    highlight_selected,
                ),
            );
    }
}

fn setup_assets(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    let node_mesh = meshes.add(Sphere::new(1.0).mesh().ico(3).unwrap());
    let node_mesh_lod = meshes.add(Sphere::new(1.0).mesh().ico(1).unwrap());

    commands.insert_resource(GraphAssets {
        node_mesh,
        node_mesh_lod,
    });

    commands.spawn(AmbientLight {
        color: Color::srgb(0.9, 0.92, 1.0),
        brightness: 800.0,
        ..default()
    });

    commands.spawn((
        DirectionalLight {
            illuminance: 3000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.7, 0.4, 0.0)),
    ));
}

#[allow(clippy::too_many_arguments)]
fn sync_graph_nodes(
    mut commands: Commands,
    layout: Res<GraphLayout>,
    settings: Res<LayoutSettings>,
    render_settings: Res<NodeRenderSettings>,
    assets: Res<GraphAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    epoch: Res<GraphSceneEpoch>,
    mut last_epoch: Local<Option<u64>>,
    existing: Query<(Entity, &SceneNodeKey, &SceneNodeEntity)>,
) {
    if *last_epoch == Some(epoch.0) {
        return;
    }
    *last_epoch = Some(epoch.0);

    let mesh = if layout.node_count > 2000 {
        assets.node_mesh_lod.clone()
    } else {
        assets.node_mesh.clone()
    };

    let mut live: HashMap<String, Entity> = existing
        .iter()
        .map(|(entity, key, _)| (key.0.clone(), entity))
        .collect();

    for (i, pos) in layout.bevy_positions().into_iter().enumerate() {
        let node = &layout.scene.nodes[i];
        let spec = visual_spec_for(&layout, &render_settings, settings.node_size, i);

        if let Some(entity) = live.remove(&node.id) {
            commands.entity(entity).insert((
                SceneNodeEntity { index: spec.index },
                Transform::from_translation(pos).with_scale(Vec3::splat(spec.radius)),
            ));
            continue;
        }

        let mat = material_for_node(&mut materials, &layout, i);
        commands.spawn((
            SceneNodeKey(node.id.clone()),
            SceneNodeEntity { index: spec.index },
            Mesh3d(mesh.clone()),
            MeshMaterial3d(mat),
            Transform::from_translation(pos).with_scale(Vec3::splat(spec.radius)),
            Visibility::default(),
        ));
    }

    for entity in live.into_values() {
        commands.entity(entity).despawn();
    }
}

#[allow(clippy::type_complexity)]
fn update_node_positions(
    layout: Res<GraphLayout>,
    settings: Res<crate::graph::LayoutSettings>,
    render_settings: Res<NodeRenderSettings>,
    bursts: Res<StatusBursts>,
    time: Res<Time>,
    mut query: Query<(
        &SceneNodeEntity,
        &mut Transform,
        Option<&Hovered>,
        Option<&Selected>,
    )>,
) {
    let elapsed = time.elapsed_secs();
    for (node, mut transform, hovered, selected) in query.iter_mut() {
        if let Some(pos) = layout.position_at(node.index) {
            transform.translation = pos;
        }
        let spec = visual_spec_for(&layout, &render_settings, settings.node_size, node.index);
        let motion = node_motion(&layout.scene, node.index, elapsed, &bursts);
        let scale = Vec3::splat(
            spec.radius
                * emphasis_radius_scale(hovered.is_some(), selected.is_some())
                * motion.scale,
        );
        if transform.scale != scale {
            transform.scale = scale;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_links(
    layout: Res<GraphLayout>,
    line_settings: Res<LinkRenderSettings>,
    hull_settings: Option<Res<HyperedgeHullSettings>>,
    pointer: Option<Res<PointerTarget>>,
    sel_state: Option<Res<SelectionState>>,
    attention: Option<Res<AttentionMode>>,
    cache: Res<crate::spatial_visibility::LineCache>,
    bursts: Res<StatusBursts>,
    time: Res<Time>,
    mut gizmos: Gizmos,
) {
    let elapsed = time.elapsed_secs();
    let _ = hull_settings;
    let hovered_he = pointer.and_then(|p| match *p {
        PointerTarget::Hyperedge(i) => Some(i),
        _ => None,
    });
    let selected_hes: HashSet<usize> = sel_state
        .as_ref()
        .map(|s| s.hyperedges.iter().copied().collect())
        .unwrap_or_default();
    let attention_on = attention.is_some_and(|mode| mode.on);
    for &(line, p1, p2) in &cache.segments {
        let edge = line.hyperedge.and_then(|i| layout.scene.hyperedges.get(i));
        let mut style =
            link_style_for(edge.map(|e| e.id.as_str()), edge.map(|e| e.status.as_str()));
        apply_link_emphasis(
            &mut style,
            Emphasis::from_flags(
                line.hyperedge.is_some_and(|i| hovered_he == Some(i)),
                line.hyperedge.is_some_and(|i| selected_hes.contains(&i)),
            ),
        );
        apply_link_status_motion(
            &mut style,
            edge.map(|e| e.status.as_str()),
            edge.map(|e| e.id.as_str()),
            elapsed,
            &bursts,
        );
        if let Some(edge) = edge {
            style.color = apply_attention_rgba(style.color, &edge.status, attention_on);
        }
        style.color.a *= line_settings.opacity;
        gizmos.line(
            p1,
            p2,
            Color::srgba(style.color.r, style.color.g, style.color.b, style.color.a),
        );
    }
}

fn apply_link_status_motion(
    style: &mut hyper_viz::LinkVisualStyle,
    status: Option<&str>,
    hyperedge_id: Option<&str>,
    elapsed: f32,
    bursts: &StatusBursts,
) {
    let id = hyperedge_id.unwrap_or("");
    let motion = motion_for(
        status.unwrap_or(""),
        id,
        elapsed,
        bursts,
        &hyper_viz::hyperedge_status_key(id),
    );
    let tinted = apply_motion_rgba(style.color, motion);
    style.color = tinted;
}

fn apply_link_emphasis(style: &mut hyper_viz::LinkVisualStyle, emphasis: Emphasis) {
    match emphasis {
        Emphasis::Rest => {}
        Emphasis::Hover => {
            style.color.a = (style.color.a * 1.45).min(0.72);
        }
        Emphasis::Selected => {
            style.color = emphasize(style.color, emphasis);
            style.color.a = (style.color.a * 2.8).min(0.98);
        }
    }
}

#[allow(clippy::type_complexity)]
fn highlight_selected(
    query: Query<(
        &SceneNodeEntity,
        &MeshMaterial3d<StandardMaterial>,
        Option<&Selected>,
        Option<&Hovered>,
    )>,
    layout: Option<Res<GraphLayout>>,
    attention: Option<Res<AttentionMode>>,
    bursts: Option<Res<StatusBursts>>,
    time: Res<Time>,
    mode: Option<Res<crate::explorer_state::ExplorerState>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if mode.is_some_and(|state| state.mode != crate::ViewMode::Spatial) {
        return;
    }
    let Some(layout) = layout else {
        return;
    };
    let elapsed = time.elapsed_secs();
    let bursts = bursts.as_deref();
    let attention_on = attention.is_some_and(|mode| mode.on);

    for (node, mat, selected, hovered) in query.iter() {
        let emphasis = Emphasis::from_flags(hovered.is_some(), selected.is_some());
        let (base, emissive) = node_emphasized_rgba(&layout, node.index, emphasis, attention_on);
        let new_color = Color::srgba(base.r, base.g, base.b, base.a);
        let new_emissive = LinearRgba::new(emissive.r, emissive.g, emissive.b, emissive.a);
        let mut glow = LinearRgba::NONE;
        if let Some(bursts) = bursts {
            let motion = node_motion(&layout.scene, node.index, elapsed, bursts);
            if motion.glow > 1e-4 {
                glow = LinearRgba::new(motion.glow, motion.glow, motion.glow, 0.0);
            }
        }
        let target_emissive = new_emissive + glow;
        let Some(current) = materials.get(&mat.0) else {
            continue;
        };
        if current.base_color == new_color && current.emissive == target_emissive {
            continue;
        }
        if let Some(material) = materials.get_mut(&mat.0) {
            material.base_color = new_color;
            material.emissive = target_emissive;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{LiveSceneReceiver, poll_live_scene};
    use hyper_viz::{Hypergraph, Projection, project};
    use std::sync::{Mutex, mpsc};

    #[test]
    fn material_updates_stop_while_dependency_entities_await_reload() {
        let scene = Hypergraph::new()
            .vertex("a", "a", "person")
            .project(Projection::StarCentroid);
        let layout = GraphLayout::from_scene(scene, &LayoutSettings::default());
        let mut app = App::new();
        let mut materials = Assets::<StandardMaterial>::default();
        let material = materials.add(StandardMaterial::default());
        app.init_resource::<Time>()
            .insert_resource(layout)
            .insert_resource(materials)
            .insert_resource(crate::explorer_state::ExplorerState::default())
            .add_systems(Update, highlight_selected);
        app.world_mut()
            .resource_mut::<crate::explorer_state::ExplorerState>()
            .mode = crate::ViewMode::Dependencies;
        app.world_mut()
            .spawn((SceneNodeEntity { index: 99 }, MeshMaterial3d(material)));
        app.update();
    }

    #[test]
    fn live_updates_keep_every_node_available_for_labels() {
        let scene = |names: &[&str]| {
            let mut graph = Hypergraph::new().with_id("live-labels");
            for name in names {
                graph = graph.vertex(*name, *name, "person");
            }
            project(&graph, Projection::Bipartite)
        };
        let settings = LayoutSettings::default();
        let (tx, rx) = mpsc::channel();
        let mut app = App::new();
        app.insert_resource(GraphLayout::from_scene(scene(&[]), &settings))
            .insert_resource(settings)
            .insert_resource(LiveSceneReceiver(Mutex::new(rx)))
            .init_resource::<GraphSceneEpoch>()
            .init_resource::<NodeRenderSettings>()
            .init_resource::<Assets<StandardMaterial>>()
            .insert_resource(GraphAssets {
                node_mesh: default(),
                node_mesh_lod: default(),
            })
            .add_systems(PreUpdate, (poll_live_scene, sync_graph_nodes).chain());
        app.update();

        for names in [
            vec!["Alice", "Bob"],
            vec!["Bob", "Carol", "Alice"],
            vec![],
            vec!["Dave"],
        ] {
            tx.send(scene(&names)).unwrap();
            app.update();
            let world = app.world_mut();
            let mut query = world.query::<(&SceneNodeKey, &SceneNodeEntity, &Transform)>();
            let layout = world.resource::<GraphLayout>();
            let mut labels: Vec<_> = query
                .iter(world)
                .map(|(key, node, _)| {
                    assert_eq!(key.0, layout.scene.nodes[node.index].id);
                    layout.scene.nodes[node.index].label.as_str()
                })
                .collect();
            labels.sort_unstable();
            let mut expected = names;
            expected.sort_unstable();
            assert_eq!(labels, expected);
        }
    }
}
