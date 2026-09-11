use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use hyper_viz::{Emphasis, emphasis_radius_scale, emphasize, link_style_for};

use crate::graph::{GraphLayout, GraphSceneEpoch, LayoutSettings};
use crate::hyperedge_hull::HyperedgeHullSettings;
use crate::interaction::{PointerTarget, SelectionState};
use crate::node_visual::{
    NodeRenderSettings, apply_node_emphasis, material_for_node, visual_spec_for,
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

#[derive(Resource)]
pub struct GraphAssets {
    pub node_mesh: Handle<Mesh>,
    pub node_mesh_lod: Handle<Mesh>,
}

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NodeRenderSettings>()
            .add_systems(Startup, setup_assets)
            .add_systems(
                Update,
                (
                    sync_graph_nodes.run_if(resource_exists::<GraphLayout>),
                    update_node_positions.run_if(resource_exists::<GraphLayout>),
                    draw_links.run_if(resource_exists::<GraphLayout>),
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
    mut query: Query<(
        &SceneNodeEntity,
        &mut Transform,
        Option<&Hovered>,
        Option<&Selected>,
    )>,
) {
    for (node, mut transform, hovered, selected) in query.iter_mut() {
        if let Some(pos) = layout.position_at(node.index) {
            transform.translation = pos;
        }
        let spec = visual_spec_for(&layout, &render_settings, settings.node_size, node.index);
        let scale =
            Vec3::splat(spec.radius * emphasis_radius_scale(hovered.is_some(), selected.is_some()));
        if transform.scale != scale {
            transform.scale = scale;
        }
    }
}

fn draw_links(
    layout: Res<GraphLayout>,
    hull_settings: Option<Res<HyperedgeHullSettings>>,
    pointer: Option<Res<PointerTarget>>,
    sel_state: Option<Res<SelectionState>>,
    mut gizmos: Gizmos,
) {
    if layout.link_count > 5000 {
        return;
    }

    let hide_hubs = hull_settings.as_ref().is_some_and(|s| s.hide_hubs);
    let hovered_he = pointer.and_then(|p| match *p {
        PointerTarget::Hyperedge(i) => Some(i),
        _ => None,
    });
    let selected_hes: HashSet<usize> = sel_state
        .as_ref()
        .map(|s| s.hyperedges.iter().copied().collect())
        .unwrap_or_default();

    if hide_hubs {
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
            let mut style = link_style_for(Some(&he.id), Some(&he.status));
            apply_link_emphasis(
                &mut style,
                Emphasis::from_flags(
                    hovered_he == Some(he_index),
                    selected_hes.contains(&he_index),
                ),
            );
            gizmos.line(
                p1,
                p2,
                Color::srgba(style.color.r, style.color.g, style.color.b, style.color.a),
            );
        }
        return;
    }

    for &(src, tgt) in layout.edges() {
        let (Some(p1), Some(p2)) = (layout.position_at(src), layout.position_at(tgt)) else {
            continue;
        };

        let link = layout
            .scene
            .links
            .iter()
            .find(|l| l.source == src && l.target == tgt);
        let he_idx = link.and_then(|l| {
            l.hyperedge_id
                .as_ref()
                .and_then(|id| layout.scene.hyperedges.iter().position(|he| he.id == *id))
        });
        let mut style = link_style_for(
            link.and_then(|l| l.hyperedge_id.as_deref()),
            link.and_then(|l| l.status.as_deref()),
        );
        apply_link_emphasis(
            &mut style,
            Emphasis::from_flags(
                he_idx.is_some_and(|i| hovered_he == Some(i)),
                he_idx.is_some_and(|i| selected_hes.contains(&i)),
            ),
        );
        let color = Color::srgba(style.color.r, style.color.g, style.color.b, style.color.a);
        gizmos.line(p1, p2, color);
    }
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
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(layout) = layout else {
        return;
    };

    for (node, mat, selected, hovered) in query.iter() {
        let emphasis = Emphasis::from_flags(hovered.is_some(), selected.is_some());
        apply_node_emphasis(&mut materials, &mat.0, &layout, node.index, emphasis);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{LiveSceneReceiver, poll_live_scene};
    use hyper_viz::{Hypergraph, Projection, project};
    use std::sync::{Mutex, mpsc};

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
            .add_systems(
                Update,
                (poll_live_scene, sync_graph_nodes).chain_ignore_deferred(),
            );
        app.update();

        for names in [
            vec!["Alice", "Bob"],
            vec!["Bob", "Carol", "Alice"],
            vec![],
            vec!["Dave"],
        ] {
            tx.send(scene(&names)).unwrap();
            app.update();
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
