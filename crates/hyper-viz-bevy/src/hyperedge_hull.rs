use std::collections::{HashMap, HashSet};

use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use hyper_viz::{
    Emphasis, NodeRole, StatusMotion, apply_motion_rgba, hull_from_points, hull_style_emphasized,
};

use crate::animation::{StatusBursts, motion_for};
use crate::focus::{AttentionMode, FocusScope, apply_attention_rgba};
use crate::graph::{GraphLayout, GraphSceneEpoch};
use crate::interaction::{PointerTarget, SelectionState};
use crate::render::SceneNodeEntity;

const TOPOLOGY_INTERVAL: u32 = 8;
const WIREFRAME_ALL_LIMIT: usize = 64;

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
pub(crate) struct HullWireCache {
    /// Maps hull vertex index → scene node index.
    pub member_scene_indices: Vec<usize>,
    pub positions: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub edges: Vec<(u32, u32)>,
    pub wire_color: Color,
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
    epoch: Res<GraphSceneEpoch>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    existing: Query<(
        Entity,
        &HyperedgeHullEntity,
        &Mesh3d,
        &MeshMaterial3d<StandardMaterial>,
    )>,
    mut caches: Query<&mut HullWireCache>,
    sel_state: Res<SelectionState>,
    pointer: Option<Res<PointerTarget>>,
    bursts: Option<Res<StatusBursts>>,
    attention: Res<AttentionMode>,
    focus: Res<FocusScope>,
    time: Res<Time>,
    mut frame: Local<u32>,
    mut last_epoch: Local<u64>,
) {
    if !settings.enabled {
        for (entity, _, _, _) in existing.iter() {
            commands.entity(entity).despawn();
        }
        return;
    }

    *frame = frame.wrapping_add(1);
    let rebuild_topo = (layout.running && (*frame % TOPOLOGY_INTERVAL == 1))
        || epoch.0 != *last_epoch
        || settings.is_changed();
    *last_epoch = epoch.0;

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
        if !focus.contains(hyperedge.hub_index) {
            continue;
        }
        let mut all_scene_indices = Vec::new();
        let mut all_positions = Vec::new();
        for idx in &hyperedge.member_indices {
            let Some(pos) = layout.position_at(*idx) else {
                continue;
            };
            all_scene_indices.push(*idx);
            all_positions.push([pos.x, pos.y, pos.z]);
        }

        // Hull the real member positions, then pad the shell a little so nested
        // sets do not z-fight. Do not scale by full arity: 0.02 * n on a 400-member
        // folder explodes the hull into empty space.
        let inflate = nest_inflate(all_positions.len());

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
        let tinted = apply_attention_rgba(
            apply_motion_rgba(style.fill, motion),
            &hyperedge.status,
            attention.on,
        );
        let fill = Color::srgba(tinted.r, tinted.g, tinted.b, tinted.a);
        let wire_tinted = apply_attention_rgba(
            apply_motion_rgba(style.wire, motion),
            &hyperedge.status,
            attention.on,
        );
        let wire = Color::srgba(wire_tinted.r, wire_tinted.g, wire_tinted.b, wire_tinted.a);

        if let Some((entity, mesh_handle, mat_handle)) = live.remove(&he_index) {
            let can_skin = !rebuild_topo
                && caches.get(entity).is_ok_and(|cache| {
                    !cache.member_scene_indices.is_empty() && cache.indices.len() >= 3
                });
            if can_skin {
                if let Ok(mut cache) = caches.get_mut(entity) {
                    cache.wire_color = wire;
                    if layout.running {
                        cache.positions = skin_hull_positions(&cache.member_scene_indices, &layout);
                        inflate_from_centroid(&mut cache.positions, inflate);
                        if let Some(mesh) = meshes.get_mut(&mesh_handle) {
                            mesh.insert_attribute(
                                Mesh::ATTRIBUTE_POSITION,
                                cache.positions.clone(),
                            );
                        }
                    }
                }
                if let Some(current) = materials.get(&mat_handle) {
                    if current.base_color != fill {
                        if let Some(mat) = materials.get_mut(&mat_handle) {
                            mat.base_color = fill;
                        }
                    }
                }
                continue;
            }

            let Some(mut hull_mesh) = hull_from_points(&all_positions) else {
                commands.entity(entity).despawn();
                continue;
            };
            inflate_from_centroid(&mut hull_mesh.positions, inflate);
            let cache = cache_from_hull(&hull_mesh, &all_scene_indices, wire);
            if let Some(mesh) = meshes.get_mut(&mesh_handle) {
                apply_hull_mesh(mesh, &hull_mesh);
            }
            if let Some(mat) = materials.get_mut(&mat_handle) {
                mat.base_color = fill;
            }
            commands.entity(entity).insert(cache);
            continue;
        }

        let Some(mut hull_mesh) = hull_from_points(&all_positions) else {
            continue;
        };
        inflate_from_centroid(&mut hull_mesh.positions, inflate);
        let cache = cache_from_hull(&hull_mesh, &all_scene_indices, wire);
        let mesh_handle = meshes.add(build_mesh(&hull_mesh));
        commands.spawn((
            HyperedgeHullEntity {
                hyperedge_index: he_index,
            },
            Mesh3d(mesh_handle),
            MeshMaterial3d(hull_material(&mut materials, fill)),
            Transform::default(),
            cache,
        ));
    }

    for (entity, _, _) in live.into_values() {
        commands.entity(entity).despawn();
    }
}

fn skin_hull_positions(scene_indices: &[usize], layout: &GraphLayout) -> Vec<[f32; 3]> {
    scene_indices
        .iter()
        .map(|idx| {
            layout
                .position_at(*idx)
                .map(|p| [p.x, p.y, p.z])
                .unwrap_or([0.0, 0.0, 0.0])
        })
        .collect()
}

fn cache_from_hull(
    hull_mesh: &hyper_viz::HullMesh,
    all_scene_indices: &[usize],
    wire: Color,
) -> HullWireCache {
    let member_scene_indices = hull_mesh
        .sources
        .iter()
        .map(|&i| all_scene_indices.get(i).copied().unwrap_or(0))
        .collect();
    HullWireCache {
        member_scene_indices,
        positions: hull_mesh.positions.clone(),
        indices: hull_mesh.indices.clone(),
        edges: hull_mesh.edges.clone(),
        wire_color: wire,
    }
}

fn draw_hull_wireframes(
    hulls: Query<(&HyperedgeHullEntity, &HullWireCache)>,
    sel_state: Res<SelectionState>,
    pointer: Option<Res<PointerTarget>>,
    layout: Res<GraphLayout>,
    focus: Res<FocusScope>,
    mut gizmos: Gizmos,
) {
    let hull_count = hulls.iter().len();
    let hovered = pointer.and_then(|p| match *p {
        PointerTarget::Hyperedge(i) => Some(i),
        _ => None,
    });
    for (entity, cache) in hulls.iter() {
        if layout
            .scene
            .hyperedges
            .get(entity.hyperedge_index)
            .is_some_and(|he| !focus.contains(he.hub_index))
        {
            continue;
        }
        let emphasized = hovered == Some(entity.hyperedge_index)
            || sel_state.hyperedges.contains(&entity.hyperedge_index);
        if hull_count > WIREFRAME_ALL_LIMIT && !emphasized {
            continue;
        }
        for &(a, b) in &cache.edges {
            let Some(p1) = cache.positions.get(a as usize) else {
                continue;
            };
            let Some(p2) = cache.positions.get(b as usize) else {
                continue;
            };
            gizmos.line(
                Vec3::from_array(*p1),
                Vec3::from_array(*p2),
                cache.wire_color,
            );
        }
    }
}

fn update_hub_visibility(
    settings: Res<HyperedgeHullSettings>,
    layout: Res<GraphLayout>,
    focus: Res<FocusScope>,
    mut nodes: Query<(&SceneNodeEntity, &mut Visibility)>,
) {
    let hide = settings.hide_hubs;
    for (node, mut visibility) in nodes.iter_mut() {
        let Some(scene_node) = layout.scene.nodes.get(node.index) else {
            continue;
        };
        let hidden_hub = hide && scene_node.role == NodeRole::HyperedgeHub;
        let out_of_focus = !focus.contains(node.index);
        *visibility = if hidden_hub || out_of_focus {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
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

/// Tiny extra shell so nested hulls do not z-fight. Capped so a 400-member
/// folder still wraps its vertices instead of filling the view.
fn nest_inflate(member_count: usize) -> f32 {
    const ARITY_CAP: usize = 6;
    1.02 + 0.02 * member_count.min(ARITY_CAP) as f32
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nest_inflate_matches_small_arity_and_caps_large_sets() {
        assert!((nest_inflate(3) - 1.08).abs() < 1e-5);
        assert!((nest_inflate(6) - 1.14).abs() < 1e-5);
        assert_eq!(nest_inflate(6), nest_inflate(400));
        assert!(nest_inflate(400) < 1.2);
    }

    #[test]
    fn inflate_does_not_move_centroid() {
        let mut points = [[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]];
        inflate_from_centroid(&mut points, 1.1);
        let c = [
            (points[0][0] + points[1][0] + points[2][0]) / 3.0,
            (points[0][1] + points[1][1] + points[2][1]) / 3.0,
            (points[0][2] + points[1][2] + points[2][2]) / 3.0,
        ];
        assert!((c[0] - 2.0 / 3.0).abs() < 1e-5);
        assert!((c[1] - 2.0 / 3.0).abs() < 1e-5);
        assert!(c[2].abs() < 1e-5);
    }
}
