use std::collections::{HashMap, HashSet};

use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use hyper_viz::{
    Emphasis, NodeRole, StatusMotion, apply_motion_rgba, hull_from_points, hull_style_emphasized,
};

use crate::animation::{StatusBursts, motion_for};
use crate::graph::GraphLayout;
use crate::interaction::{PointerTarget, SelectionState};
use crate::render::SceneNodeEntity;

#[derive(Resource, Debug, Clone)]
pub struct HyperedgeHullSettings {
    pub enabled: bool,
    pub opacity: f32,
    pub wireframe: bool,
    pub hide_hubs: bool,
}

impl Default for HyperedgeHullSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            opacity: 0.28,
            wireframe: true,
            hide_hubs: true,
        }
    }
}

#[derive(Component)]
pub struct HyperedgeHullEntity {
    pub hyperedge_index: usize,
}

#[derive(Component, Clone)]
struct HullWireCache {
    /// Maps hull vertex index → scene node index.
    member_scene_indices: Vec<usize>,
    edges: Vec<(u32, u32)>,
    wire_color: Color,
}

pub struct HyperedgeHullPlugin;

impl Plugin for HyperedgeHullPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HyperedgeHullSettings>().add_systems(
            Update,
            (
                sync_hyperedge_hulls.run_if(resource_exists::<GraphLayout>),
                draw_hull_wireframes
                    .run_if(resource_exists::<GraphLayout>)
                    .run_if(|settings: Res<HyperedgeHullSettings>| {
                        settings.enabled && settings.wireframe
                    }),
                update_hub_visibility.run_if(resource_exists::<GraphLayout>),
            ),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn sync_hyperedge_hulls(
    mut commands: Commands,
    layout: Res<GraphLayout>,
    settings: Res<HyperedgeHullSettings>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    existing: Query<(
        Entity,
        &HyperedgeHullEntity,
        &Mesh3d,
        &MeshMaterial3d<StandardMaterial>,
    )>,
    sel_state: Res<SelectionState>,
    pointer: Option<Res<PointerTarget>>,
    bursts: Option<Res<StatusBursts>>,
    time: Res<Time>,
) {
    if !settings.enabled {
        for (entity, _, _, _) in existing.iter() {
            commands.entity(entity).despawn();
        }
        return;
    }

    let selected_hubs: HashSet<usize> = sel_state
        .base_selection
        .iter()
        .copied()
        .filter(|idx| {
            layout
                .scene
                .nodes
                .get(*idx)
                .is_some_and(|n| n.role == NodeRole::HyperedgeHub)
        })
        .collect();
    let hovered_he = pointer.and_then(|p| match *p {
        PointerTarget::Hyperedge(i) => Some(i),
        _ => None,
    });

    let mut live: HashMap<usize, (Entity, Handle<Mesh>, Handle<StandardMaterial>)> = HashMap::new();
    for (entity, hull, mesh3d, mat) in existing.iter() {
        live.insert(
            hull.hyperedge_index,
            (entity, mesh3d.0.clone(), mat.0.clone()),
        );
    }

    for (he_index, hyperedge) in layout.scene.hyperedges.iter().enumerate() {
        let mut member_scene_indices = Vec::new();
        let mut member_positions = Vec::new();
        for idx in &hyperedge.member_indices {
            let Some(pos) = layout.position_at(*idx) else {
                continue;
            };
            member_scene_indices.push(*idx);
            member_positions.push([pos.x, pos.y, pos.z]);
        }

        // Slight per-arity inflate so nested sets (e.g. Paper C ⊂ Lab) don't z-fight.
        let inflate = 1.02 + 0.02 * member_positions.len() as f32;
        inflate_from_centroid(&mut member_positions, inflate);

        let Some(hull_mesh) = hull_from_points(&member_positions) else {
            if let Some((entity, _, _)) = live.remove(&he_index) {
                commands.entity(entity).despawn();
            }
            continue;
        };

        let emphasis = Emphasis::from_flags(
            hovered_he == Some(he_index),
            selected_hubs.contains(&hyperedge.hub_index)
                || sel_state.hyperedges.contains(&he_index),
        );
        let style =
            hull_style_emphasized(&hyperedge.id, &hyperedge.status, settings.opacity, emphasis);
        let motion = bursts.as_deref().map_or(StatusMotion::default(), |bursts| {
            motion_for(
                &hyperedge.status,
                &hyperedge.id,
                time.elapsed_secs(),
                bursts,
                &hyper_viz::hyperedge_status_key(&hyperedge.id),
            )
        });
        let tinted = apply_motion_rgba(style.fill, motion);
        let fill = Color::srgba(tinted.r, tinted.g, tinted.b, tinted.a);
        let wire_tinted = apply_motion_rgba(style.wire, motion);
        let wire = Color::srgba(wire_tinted.r, wire_tinted.g, wire_tinted.b, wire_tinted.a);

        let wire_cache = HullWireCache {
            member_scene_indices,
            edges: hull_mesh.edges.clone(),
            wire_color: wire,
        };

        if let Some((entity, mesh_handle, mat_handle)) = live.remove(&he_index) {
            if let Some(mesh) = meshes.get_mut(&mesh_handle) {
                apply_hull_mesh(mesh, &hull_mesh);
            }
            if let Some(mat) = materials.get_mut(&mat_handle) {
                mat.base_color = fill;
            }
            commands.entity(entity).insert(wire_cache);
            continue;
        }

        let mesh_handle = meshes.add(build_mesh(&hull_mesh));
        commands.spawn((
            HyperedgeHullEntity {
                hyperedge_index: he_index,
            },
            Mesh3d(mesh_handle),
            MeshMaterial3d(hull_material(&mut materials, fill)),
            Transform::default(),
            wire_cache,
        ));
    }

    for (entity, _, _) in live.into_values() {
        commands.entity(entity).despawn();
    }
}

fn draw_hull_wireframes(
    layout: Res<GraphLayout>,
    hulls: Query<(&HullWireCache,)>,
    mut gizmos: Gizmos,
) {
    for (cache,) in hulls.iter() {
        for &(a, b) in &cache.edges {
            let Some(a_idx) = cache.member_scene_indices.get(a as usize) else {
                continue;
            };
            let Some(b_idx) = cache.member_scene_indices.get(b as usize) else {
                continue;
            };
            let (Some(p1), Some(p2)) = (layout.position_at(*a_idx), layout.position_at(*b_idx))
            else {
                continue;
            };
            gizmos.line(p1, p2, cache.wire_color);
        }
    }
}

fn update_hub_visibility(
    settings: Res<HyperedgeHullSettings>,
    layout: Res<GraphLayout>,
    mut nodes: Query<(&SceneNodeEntity, &mut Visibility)>,
) {
    let hide = settings.hide_hubs;
    for (node, mut visibility) in nodes.iter_mut() {
        if layout.scene.nodes[node.index].role == NodeRole::HyperedgeHub {
            *visibility = if hide {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            };
        }
    }
}

fn hull_material(
    materials: &mut Assets<StandardMaterial>,
    color: Color,
) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: color,
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        cull_mode: None,
        ..default()
    })
}

fn build_mesh(hull: &hyper_viz::HullMesh) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    apply_hull_mesh(&mut mesh, hull);
    mesh
}

fn apply_hull_mesh(mesh: &mut Mesh, hull: &hyper_viz::HullMesh) {
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, hull.positions.clone());
    mesh.insert_indices(Indices::U32(hull.indices.clone()));
}

fn inflate_from_centroid(points: &mut [[f32; 3]], scale: f32) {
    if points.is_empty() {
        return;
    }
    let n = points.len() as f32;
    let mut c = [0.0f32; 3];
    for p in points.iter() {
        c[0] += p[0];
        c[1] += p[1];
        c[2] += p[2];
    }
    c[0] /= n;
    c[1] /= n;
    c[2] /= n;
    for p in points {
        p[0] = c[0] + (p[0] - c[0]) * scale;
        p[1] = c[1] + (p[1] - c[1]) * scale;
        p[2] = c[2] + (p[2] - c[2]) * scale;
    }
}
