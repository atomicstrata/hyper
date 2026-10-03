use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiGlobalSettings, PrimaryEguiContext};
use bevy_panorbit_camera::{
    EguiFocusIncludesHover, EguiWantsFocus, PanOrbitCamera, PanOrbitCameraPlugin,
};

use hyper_viz::NodeRole;

use crate::focus::{
    AttentionMode, FocusScope, FrameRequest, camera_fit, frame_indices, positions_for,
};
use crate::graph::{GraphLayout, GraphSceneEpoch};
use crate::hyperedge_hull::HyperedgeHullSettings;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PanOrbitCameraPlugin)
            // Hovering a window must block orbit/zoom; wants_pointer_input alone
            // misses scroll and the first click frame.
            .insert_resource(EguiFocusIncludesHover(true))
            .add_systems(Startup, spawn_camera)
            .add_systems(
                Update,
                (
                    release_text_focus_for_orbit.run_if(crate::explorer_state::spatial_mode),
                    auto_fit_camera
                        .run_if(resource_exists::<GraphLayout>)
                        .run_if(crate::explorer_state::spatial_mode),
                    apply_frame_request
                        .run_if(resource_exists::<GraphLayout>)
                        .run_if(crate::explorer_state::spatial_mode),
                ),
            );
    }
}

/// A focused TextEdit sets `wants_keyboard_input`, and panorbit only starts a
/// drag when *both* this frame and last frame reported no egui focus. Without
/// this, the first scene drag after find is a no-op (it only blurs the box).
fn release_text_focus_for_orbit(
    mouse: Res<ButtonInput<MouseButton>>,
    mut scroll: MessageReader<MouseWheel>,
    mut contexts: EguiContexts,
    mut wants: ResMut<EguiWantsFocus>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    if ctx.is_pointer_over_area() || !ctx.wants_keyboard_input() {
        return;
    }
    let scrolling = scroll.read().next().is_some();
    if !mouse.just_pressed(MouseButton::Left) && !scrolling {
        return;
    }
    ctx.memory_mut(|memory| memory.stop_text_input());
    wants.prev = false;
    wants.curr = false;
}

fn spawn_camera(mut commands: Commands, mut egui_settings: ResMut<EguiGlobalSettings>) {
    egui_settings.auto_create_primary_context = false;

    commands.spawn((
        Camera3d::default(),
        PrimaryEguiContext,
        Transform::from_translation(Vec3::new(0.0, 50.0, 300.0)).looking_at(Vec3::ZERO, Vec3::Y),
        PanOrbitCamera {
            radius: Some(300.0),
            target_radius: 300.0,
            focus: Vec3::ZERO,
            target_focus: Vec3::ZERO,
            yaw: Some(0.0),
            pitch: Some(-0.3),
            ..default()
        },
    ));
}

#[allow(clippy::too_many_arguments)]
fn auto_fit_camera(
    layout: Res<GraphLayout>,
    epoch: Res<GraphSceneEpoch>,
    attention: Res<AttentionMode>,
    focus: Res<FocusScope>,
    hull_settings: Res<HyperedgeHullSettings>,
    mut cam_q: Query<&mut PanOrbitCamera>,
    mut fitted_epoch: Local<Option<u64>>,
    skip_fit: Option<Res<crate::session::SkipAutoFit>>,
) {
    if skip_fit.is_some_and(|skip| skip.0) {
        *fitted_epoch = Some(epoch.0);
        return;
    }
    if layout.iterations() < 20 {
        return;
    }
    // Fit once. Live reloads used to re-frame every epoch and yank the camera.
    if fitted_epoch.is_some() {
        return;
    }
    let indices = frame_indices(&layout, &attention, &focus);
    let positions = frame_positions(&layout, &indices, hull_settings.hide_hubs);
    if apply_camera_fit(&mut cam_q, &positions) {
        *fitted_epoch = Some(epoch.0);
    }
}

fn apply_frame_request(
    layout: Res<GraphLayout>,
    attention: Res<AttentionMode>,
    focus: Res<FocusScope>,
    mut request: ResMut<FrameRequest>,
    hull_settings: Res<HyperedgeHullSettings>,
    mut cam_q: Query<&mut PanOrbitCamera>,
) {
    if !request.pending {
        return;
    }
    request.pending = false;
    let indices = frame_indices(&layout, &attention, &focus);
    let positions = frame_positions(&layout, &indices, hull_settings.hide_hubs);
    apply_camera_fit(&mut cam_q, &positions);
}

fn frame_positions(layout: &GraphLayout, indices: &[usize], hide_hubs: bool) -> Vec<Vec3> {
    let visible = if hide_hubs {
        indices
            .iter()
            .copied()
            .filter(|index| {
                layout
                    .scene
                    .nodes
                    .get(*index)
                    .is_none_or(|node| node.role != NodeRole::HyperedgeHub)
            })
            .collect::<Vec<_>>()
    } else {
        indices.to_vec()
    };
    let positions = positions_for(layout, &visible);
    if !positions.is_empty() {
        return positions;
    }
    // Hub-only scopes still need a frame; fall back to every visible vertex.
    if hide_hubs {
        let all: Vec<usize> = (0..layout.node_count)
            .filter(|index| {
                layout
                    .scene
                    .nodes
                    .get(*index)
                    .is_none_or(|node| node.role != NodeRole::HyperedgeHub)
            })
            .collect();
        let fallback = positions_for(layout, &all);
        if !fallback.is_empty() {
            return fallback;
        }
    }
    positions_for(layout, &(0..layout.node_count).collect::<Vec<_>>())
}

fn apply_camera_fit(cam_q: &mut Query<&mut PanOrbitCamera>, positions: &[Vec3]) -> bool {
    let Some((focus, radius)) = camera_fit(positions) else {
        return false;
    };
    let Ok(mut cam) = cam_q.single_mut() else {
        return false;
    };
    // panorbit eases toward target_*; writing only focus/radius is overwritten next frame.
    cam.target_focus = focus;
    cam.target_radius = radius;
    cam.focus = focus;
    cam.radius = Some(radius);
    cam.force_update = true;
    true
}
