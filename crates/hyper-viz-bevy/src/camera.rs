use bevy::prelude::*;
use bevy_egui::{EguiGlobalSettings, PrimaryEguiContext};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};

use crate::focus::{
    AttentionMode, FocusScope, FrameRequest, camera_fit, frame_indices, positions_for,
};
use crate::graph::{GraphLayout, GraphSceneEpoch};
use crate::interaction::SelectionState;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PanOrbitCameraPlugin)
            .add_systems(Startup, spawn_camera)
            .add_systems(
                Update,
                (
                    auto_fit_camera.run_if(resource_exists::<GraphLayout>),
                    apply_frame_request.run_if(resource_exists::<GraphLayout>),
                ),
            );
    }
}

fn spawn_camera(mut commands: Commands, mut egui_settings: ResMut<EguiGlobalSettings>) {
    egui_settings.auto_create_primary_context = false;

    commands.spawn((
        Camera3d::default(),
        PrimaryEguiContext,
        Transform::from_translation(Vec3::new(0.0, 50.0, 300.0)).looking_at(Vec3::ZERO, Vec3::Y),
        PanOrbitCamera {
            radius: Some(300.0),
            focus: Vec3::ZERO,
            yaw: Some(0.0),
            pitch: Some(-0.3),
            ..default()
        },
    ));
}

fn auto_fit_camera(
    layout: Res<GraphLayout>,
    epoch: Res<GraphSceneEpoch>,
    attention: Res<AttentionMode>,
    focus: Res<FocusScope>,
    sel_state: Option<Res<SelectionState>>,
    mut cam_q: Query<&mut PanOrbitCamera>,
    mut fitted_epoch: Local<Option<u64>>,
) {
    if layout.iterations() < 20 {
        return;
    }
    if *fitted_epoch == Some(epoch.0) {
        return;
    }
    let selection = sel_state
        .as_ref()
        .map(|s| s.base_selection.as_slice())
        .unwrap_or(&[]);
    let indices = frame_indices(&layout, &attention, &focus, selection);
    let positions = positions_for(&layout, &indices);
    if apply_camera_fit(&mut cam_q, &positions) {
        *fitted_epoch = Some(epoch.0);
    }
}

fn apply_frame_request(
    layout: Res<GraphLayout>,
    attention: Res<AttentionMode>,
    focus: Res<FocusScope>,
    sel_state: Option<Res<SelectionState>>,
    mut request: ResMut<FrameRequest>,
    mut cam_q: Query<&mut PanOrbitCamera>,
) {
    if !request.pending {
        return;
    }
    request.pending = false;
    let selection = sel_state
        .as_ref()
        .map(|s| s.base_selection.as_slice())
        .unwrap_or(&[]);
    let indices = frame_indices(&layout, &attention, &focus, selection);
    let positions = positions_for(&layout, &indices);
    apply_camera_fit(&mut cam_q, &positions);
}

fn apply_camera_fit(cam_q: &mut Query<&mut PanOrbitCamera>, positions: &[Vec3]) -> bool {
    let Some((focus, radius)) = camera_fit(positions) else {
        return false;
    };
    let Ok(mut cam) = cam_q.single_mut() else {
        return false;
    };
    cam.focus = focus;
    cam.radius = Some(radius);
    true
}
