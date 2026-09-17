//! Load/save [`hyper_viz::ViewerSession`].
//!
//! Desktop writes `~/Library/Application Support/hyper-viz/session.json`
//! (or `$XDG_CONFIG_HOME` / `%APPDATA%`). Override with `HYPER_VIZ_SESSION`.
//! Set that env to `off` to disable. On wasm the same JSON goes to
//! `localStorage` key `hyperviz.session.v1`.

#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_panorbit_camera::PanOrbitCamera;
use hyper_viz::{
    HypergraphScene, ViewerSession, ViewerView, parse_session,
    session::{CameraPrefs, LabelPrefs, LayoutPrefs, NavigationPrefs, ViewerPrefs, WindowPrefs},
    session_to_json, view_key,
};

use crate::focus::{AttentionMode, FocusScope, isolate_selection};
use crate::graph::{GraphLayout, LayoutSettings};
use crate::hyperedge_hull::HyperedgeHullSettings;
use crate::interaction::{LassoState, LocalizeQuery, SelectionState};
use crate::node_visual::{NodeLabelMode, NodeRenderSettings};

const FLUSH_SECS: f32 = if cfg!(target_arch = "wasm32") {
    0.0
} else {
    0.45
};
#[cfg(target_arch = "wasm32")]
const STORAGE_KEY: &str = "hyperviz.session.v1";

#[derive(Resource, Clone, Debug, Default)]
pub struct SessionStore {
    pub session: ViewerSession,
    pub last_json: String,
    pub enabled: bool,
}

#[derive(Resource, Debug, Default)]
struct SessionDirty {
    elapsed: f32,
    pending: bool,
}

#[derive(Resource, Debug, Default)]
pub struct SkipAutoFit(pub bool);

#[derive(Resource, Debug, Default)]
struct SessionApply {
    prefs_done: bool,
    view_done: bool,
}

pub struct SessionPlugin;

impl Plugin for SessionPlugin {
    fn build(&self, app: &mut App) {
        let enabled = session_enabled();
        let session = if enabled {
            load_session().unwrap_or_default()
        } else {
            ViewerSession::default()
        };
        let last_json = session_to_json(&session).unwrap_or_default();
        app.insert_resource(SessionStore {
            session,
            last_json,
            enabled,
        })
        .init_resource::<SessionDirty>()
        .init_resource::<SkipAutoFit>()
        .init_resource::<SessionApply>()
        .add_systems(
            PreUpdate,
            apply_saved_session.run_if(resource_exists::<GraphLayout>),
        )
        .add_systems(
            Update,
            (
                capture_session.run_if(resource_exists::<GraphLayout>),
                flush_session.run_if(resource_exists::<GraphLayout>),
            )
                .chain(),
        )
        .add_systems(Last, flush_on_exit);
    }
}

pub fn session_enabled() -> bool {
    // wasm has no useful process env; an empty Ok("") would disable persistence.
    #[cfg(target_arch = "wasm32")]
    {
        return true;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        session_env_enabled(std::env::var("HYPER_VIZ_SESSION").ok().as_deref())
    }
}

fn session_env_enabled(raw: Option<&str>) -> bool {
    match raw {
        None => true,
        Some(value) => !matches!(value.trim(), "" | "0" | "off" | "false" | "none"),
    }
}

pub fn load_session() -> Option<ViewerSession> {
    parse_session(&load_raw()?)
}

pub fn saved_window_size() -> Option<(u32, u32)> {
    let session = load_session()?;
    let window = session.prefs.window?;
    Some((window.width.max(400), window.height.max(300)))
}

#[allow(clippy::too_many_arguments)]
fn apply_saved_session(
    mut apply: ResMut<SessionApply>,
    mut store: ResMut<SessionStore>,
    mut settings: ResMut<LayoutSettings>,
    mut labels: ResMut<NodeRenderSettings>,
    mut hulls: ResMut<HyperedgeHullSettings>,
    mut attention: ResMut<AttentionMode>,
    mut lasso: ResMut<LassoState>,
    mut localize: ResMut<LocalizeQuery>,
    mut sel_state: ResMut<SelectionState>,
    mut focus: ResMut<FocusScope>,
    mut skip_fit: ResMut<SkipAutoFit>,
    mut layout: ResMut<GraphLayout>,
    mut cameras: Query<(&mut Transform, &mut PanOrbitCamera)>,
) {
    if apply.view_done || !store.enabled {
        return;
    }
    #[cfg(target_arch = "wasm32")]
    if store.session.views.is_empty() {
        if let Some(loaded) = load_session() {
            store.session = loaded;
            if let Ok(json) = session_to_json(&store.session) {
                store.last_json = json;
            }
        }
    }
    if !apply.prefs_done {
        apply_prefs(&store.session.prefs, &mut settings, &mut labels, &mut hulls);
        attention.on = store.session.prefs.attention;
        lasso.enabled = store.session.prefs.navigation.lasso || store.session.prefs.lasso;
        layout.running = store.session.prefs.layout.running;
        layout.iterations_per_frame = settings.iterations_per_frame;
        layout.layout.config = settings.config.clone();
        apply.prefs_done = true;
    }

    let Some(view) = store.session.views.get(&view_key(&layout.scene.meta.id)) else {
        apply.view_done = true;
        return;
    };
    if view.camera.is_some() {
        skip_fit.0 = true;
    }
    restore_view(
        view,
        &mut layout,
        &mut sel_state,
        &mut focus,
        &mut localize,
        &mut cameras,
    );
    apply.view_done = view.camera.is_none() || cameras.iter().next().is_some();
}

fn apply_prefs(
    prefs: &ViewerPrefs,
    settings: &mut LayoutSettings,
    labels: &mut NodeRenderSettings,
    hulls: &mut HyperedgeHullSettings,
) {
    apply_layout_prefs(&prefs.layout, settings);
    apply_label_prefs(&prefs.labels, labels);
    hulls.enabled = prefs.hulls.enabled;
    hulls.hide_hubs = prefs.hulls.hide_hubs;
    hulls.wireframe = prefs.hulls.wireframe;
    hulls.opacity = prefs.hulls.opacity;
}

fn apply_layout_prefs(prefs: &LayoutPrefs, settings: &mut LayoutSettings) {
    settings.iterations_per_frame = prefs.iterations_per_frame.max(1);
    settings.config.dt = prefs.dt;
    settings.config.damping = prefs.damping;
    settings.config.repulsion = prefs.repulsion;
    settings.config.attraction = prefs.attraction;
    settings.config.theta = prefs.theta;
    settings.config.gravity = prefs.gravity;
    settings.config.ideal_length = prefs.ideal_length;
    settings.config.max_tree_depth = prefs.max_tree_depth;
    settings.config.centroid_attraction = prefs.centroid_attraction;
}

fn apply_label_prefs(prefs: &LabelPrefs, labels: &mut NodeRenderSettings) {
    labels.labels_enabled = prefs.vertex_labels;
    labels.hyperedge_labels = prefs.hyperedge_labels;
    labels.label_mode = match prefs.mode.as_str() {
        "selection" => NodeLabelMode::SelectionOnly,
        "all" => NodeLabelMode::All,
        _ => NodeLabelMode::Capped,
    };
    labels.label_scale = prefs.scale;
    labels.label_variation = prefs.variation;
}

fn restore_view(
    view: &ViewerView,
    layout: &mut GraphLayout,
    sel_state: &mut SelectionState,
    focus: &mut FocusScope,
    localize: &mut LocalizeQuery,
    cameras: &mut Query<(&mut Transform, &mut PanOrbitCamera)>,
) {
    let nodes = scene_indices_for_ids(&layout.scene, &view.selected_node_ids);
    let hyperedges = scene_hyperedge_indices(&layout.scene, &view.selected_hyperedge_ids);
    sel_state.set_hits(nodes, hyperedges);
    sel_state.remember_ids(&layout.scene);

    localize.query = view.find_query.clone();
    localize.search_owned = !view.find_query.trim().is_empty();

    let isolated = view.isolated || view.find_isolated;
    if isolated {
        isolate_selection(
            focus,
            layout,
            &sel_state.base_selection,
            &sel_state.hyperedges,
        );
        if !focus.is_active() {
            restore_focus_ids(focus, &layout.scene, view.focus_node_ids.as_deref());
        }
        localize.isolated = focus.is_active();
    } else {
        restore_focus_ids(focus, &layout.scene, view.focus_node_ids.as_deref());
        localize.isolated = focus.is_active();
    }

    if let Some(camera) = &view.camera
        && let Ok((mut transform, mut cam)) = cameras.single_mut()
    {
        apply_camera_prefs(&mut cam, &mut transform, camera);
    }
}

fn restore_focus_ids(focus: &mut FocusScope, scene: &HypergraphScene, ids: Option<&[String]>) {
    let Some(ids) = ids else {
        return;
    };
    let remapped: std::collections::HashSet<usize> =
        scene_indices_for_ids(scene, ids).into_iter().collect();
    if remapped.is_empty() {
        focus.clear();
    } else {
        focus.nodes = Some(remapped);
    }
}

fn apply_camera_prefs(cam: &mut PanOrbitCamera, transform: &mut Transform, prefs: &CameraPrefs) {
    let focus = Vec3::new(prefs.focus[0], prefs.focus[1], prefs.focus[2]);
    cam.target_focus = focus;
    cam.focus = focus;
    cam.target_radius = prefs.radius;
    cam.radius = Some(prefs.radius);
    cam.yaw = Some(prefs.yaw);
    cam.target_yaw = prefs.yaw;
    cam.pitch = Some(prefs.pitch);
    cam.target_pitch = prefs.pitch;
    if let Some(position) = prefs.position {
        transform.translation = Vec3::new(position[0], position[1], position[2]);
    }
    cam.force_update = true;
}

fn scene_indices_for_ids(scene: &HypergraphScene, ids: &[String]) -> Vec<usize> {
    ids.iter()
        .filter_map(|id| {
            scene
                .nodes
                .iter()
                .find(|node| node.id == *id)
                .map(|n| n.index)
        })
        .collect()
}

fn scene_hyperedge_indices(scene: &HypergraphScene, ids: &[String]) -> Vec<usize> {
    ids.iter()
        .filter_map(|id| scene.hyperedges.iter().position(|he| he.id == *id))
        .collect()
}

fn scene_node_ids(scene: &HypergraphScene, indices: &[usize]) -> Vec<String> {
    indices
        .iter()
        .filter_map(|index| {
            scene
                .nodes
                .iter()
                .find(|node| node.index == *index)
                .map(|node| node.id.clone())
        })
        .collect()
}

fn scene_hyperedge_ids(scene: &HypergraphScene, indices: &[usize]) -> Vec<String> {
    indices
        .iter()
        .filter_map(|index| scene.hyperedges.get(*index).map(|he| he.id.clone()))
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn capture_session(
    apply: Res<SessionApply>,
    mut store: ResMut<SessionStore>,
    mut dirty: ResMut<SessionDirty>,
    settings: Res<LayoutSettings>,
    layout: Res<GraphLayout>,
    labels: Res<NodeRenderSettings>,
    hulls: Res<HyperedgeHullSettings>,
    attention: Res<AttentionMode>,
    lasso: Res<LassoState>,
    localize: Res<LocalizeQuery>,
    sel_state: Res<SelectionState>,
    focus: Res<FocusScope>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&PanOrbitCamera, &Transform)>,
) {
    if !store.enabled || !apply.view_done {
        return;
    }
    let next = build_session(
        &store.session,
        &settings,
        &layout,
        &labels,
        &hulls,
        attention.on,
        lasso.enabled,
        &localize,
        &sel_state,
        &focus,
        windows.single().ok(),
        cameras.single().ok(),
    );
    let Ok(json) = session_to_json(&next) else {
        return;
    };
    if json == store.last_json {
        return;
    }
    store.session = next;
    dirty.pending = true;
    dirty.elapsed = 0.0;
    // Browser reload never delivers AppExit; write as soon as the view changes.
    #[cfg(target_arch = "wasm32")]
    persist_store(&mut store, &mut dirty);
}

#[allow(clippy::too_many_arguments)]
fn build_session(
    previous: &ViewerSession,
    settings: &LayoutSettings,
    layout: &GraphLayout,
    labels: &NodeRenderSettings,
    hulls: &HyperedgeHullSettings,
    attention: bool,
    lasso: bool,
    localize: &LocalizeQuery,
    sel_state: &SelectionState,
    focus: &FocusScope,
    window: Option<&Window>,
    camera: Option<(&PanOrbitCamera, &Transform)>,
) -> ViewerSession {
    let mut session = previous.clone();
    session.version = hyper_viz::SESSION_VERSION.into();
    let fallback = &previous.prefs;
    session.prefs = ViewerPrefs {
        layout: LayoutPrefs {
            iterations_per_frame: settings.iterations_per_frame,
            running: layout.running,
            dt: finite_or(settings.config.dt, fallback.layout.dt),
            damping: finite_or(settings.config.damping, fallback.layout.damping),
            repulsion: finite_or(settings.config.repulsion, fallback.layout.repulsion),
            attraction: finite_or(settings.config.attraction, fallback.layout.attraction),
            theta: finite_or(settings.config.theta, fallback.layout.theta),
            gravity: finite_or(settings.config.gravity, fallback.layout.gravity),
            ideal_length: finite_or(settings.config.ideal_length, fallback.layout.ideal_length),
            max_tree_depth: settings.config.max_tree_depth,
            centroid_attraction: finite_or(
                settings.config.centroid_attraction,
                fallback.layout.centroid_attraction,
            ),
        },
        labels: LabelPrefs {
            vertex_labels: labels.labels_enabled,
            hyperedge_labels: labels.hyperedge_labels,
            mode: match labels.label_mode {
                NodeLabelMode::Capped => "capped".into(),
                NodeLabelMode::SelectionOnly => "selection".into(),
                NodeLabelMode::All => "all".into(),
            },
            scale: finite_or(labels.label_scale, fallback.labels.scale),
            variation: finite_or(labels.label_variation, fallback.labels.variation),
        },
        hulls: hyper_viz::session::HullPrefs {
            enabled: hulls.enabled,
            hide_hubs: hulls.hide_hubs,
            wireframe: hulls.wireframe,
            opacity: finite_or(hulls.opacity, fallback.hulls.opacity),
        },
        navigation: NavigationPrefs { lasso },
        attention,
        lasso,
        window: window
            .and_then(window_prefs_from_live)
            .or_else(|| previous.prefs.window.clone()),
    };
    let key = view_key(&layout.scene.meta.id);
    let previous_view = session.views.get(&key).cloned();
    session.views.insert(
        key,
        capture_view(
            previous_view.as_ref(),
            layout,
            localize,
            sel_state,
            focus,
            camera,
        ),
    );
    session
}

fn capture_view(
    previous: Option<&ViewerView>,
    layout: &GraphLayout,
    localize: &LocalizeQuery,
    sel_state: &SelectionState,
    focus: &FocusScope,
    camera: Option<(&PanOrbitCamera, &Transform)>,
) -> ViewerView {
    let camera = camera
        .and_then(|(cam, transform)| camera_prefs_from_live(cam, transform))
        .or_else(|| previous.and_then(|view| view.camera.clone()));
    ViewerView {
        camera,
        focus_node_ids: focus.nodes.as_ref().map(|nodes| {
            let mut ids: Vec<usize> = nodes.iter().copied().collect();
            ids.sort_unstable();
            scene_node_ids(&layout.scene, &ids)
        }),
        selected_node_ids: scene_node_ids(&layout.scene, &sel_state.base_selection),
        selected_hyperedge_ids: scene_hyperedge_ids(&layout.scene, &sel_state.hyperedges),
        find_query: localize.query.clone(),
        find_isolated: localize.isolated,
        isolated: focus.is_active(),
    }
}

fn flush_session(
    time: Res<Time>,
    mut store: ResMut<SessionStore>,
    mut dirty: ResMut<SessionDirty>,
) {
    if !store.enabled || !dirty.pending {
        return;
    }
    dirty.elapsed += time.delta_secs();
    if dirty.elapsed < FLUSH_SECS {
        return;
    }
    persist_store(&mut store, &mut dirty);
}

fn flush_on_exit(
    exits: MessageReader<AppExit>,
    mut store: ResMut<SessionStore>,
    mut dirty: ResMut<SessionDirty>,
) {
    if exits.is_empty() || !store.enabled {
        return;
    }
    persist_store(&mut store, &mut dirty);
}

fn persist_store(store: &mut SessionStore, dirty: &mut SessionDirty) {
    let Ok(json) = session_to_json(&store.session) else {
        tracing::warn!("session serialize failed");
        return;
    };
    if json == store.last_json && !dirty.pending {
        return;
    }
    if save_raw(&json) {
        store.last_json = json;
        dirty.pending = false;
        dirty.elapsed = 0.0;
    } else {
        tracing::warn!("session persist failed");
    }
}

fn finite3(values: [f32; 3]) -> bool {
    values.iter().all(|value| value.is_finite())
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value
    } else if fallback.is_finite() {
        fallback
    } else {
        0.0
    }
}

fn camera_prefs_from_live(cam: &PanOrbitCamera, transform: &Transform) -> Option<CameraPrefs> {
    let focus = [cam.focus.x, cam.focus.y, cam.focus.z];
    let radius = cam.radius.unwrap_or(cam.target_radius);
    let yaw = cam.yaw.unwrap_or(cam.target_yaw);
    let pitch = cam.pitch.unwrap_or(cam.target_pitch);
    let position = [
        transform.translation.x,
        transform.translation.y,
        transform.translation.z,
    ];
    if !finite3(focus) || !radius.is_finite() || !yaw.is_finite() || !pitch.is_finite() {
        return None;
    }
    Some(CameraPrefs {
        focus,
        radius,
        yaw,
        pitch,
        position: finite3(position).then_some(position),
    })
}

fn window_prefs_from_live(window: &Window) -> Option<WindowPrefs> {
    let width = window.resolution.width();
    let height = window.resolution.height();
    if !width.is_finite() || !height.is_finite() {
        return None;
    }
    let width = width as u32;
    let height = height as u32;
    if width < 200 || height < 200 {
        return None;
    }
    Some(WindowPrefs { width, height })
}

fn load_raw() -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        return wasm_get(STORAGE_KEY);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let path = session_path()?;
        std::fs::read_to_string(path).ok()
    }
}

fn save_raw(json: &str) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        return wasm_set(STORAGE_KEY, json);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let Some(path) = session_path() else {
            return false;
        };
        if let Some(parent) = path.parent()
            && let Err(error) = std::fs::create_dir_all(parent)
        {
            tracing::warn!(%error, path = %path.display(), "session dir");
            return false;
        }
        let tmp = path.with_extension("json.tmp");
        if std::fs::write(&tmp, json).is_err() || std::fs::rename(&tmp, &path).is_err() {
            tracing::warn!(path = %path.display(), "session write failed");
            return false;
        }
        true
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn session_path() -> Option<PathBuf> {
    if let Ok(explicit) = std::env::var("HYPER_VIZ_SESSION") {
        let trimmed = explicit.trim();
        if !session_env_enabled(Some(trimmed)) {
            return None;
        }
        return Some(PathBuf::from(trimmed));
    }
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    let mut path = PathBuf::from(home);
    if cfg!(target_os = "macos") {
        path.push("Library/Application Support/hyper-viz");
    } else if cfg!(target_os = "windows") {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            path = PathBuf::from(appdata);
            path.push("hyper-viz");
        } else {
            path.push("AppData/Roaming/hyper-viz");
        }
    } else if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        path = PathBuf::from(xdg);
        path.push("hyper-viz");
    } else {
        path.push(".config/hyper-viz");
    }
    path.push("session.json");
    Some(path)
}

#[cfg(target_arch = "wasm32")]
fn wasm_get(key: &str) -> Option<String> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    storage.get_item(key).ok()?
}

#[cfg(target_arch = "wasm32")]
fn wasm_set(key: &str, value: &str) -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let Ok(Some(storage)) = window.local_storage() else {
        return false;
    };
    storage.set_item(key, value).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper_viz::{Hypergraph, Projection, project};

    #[test]
    fn env_off_disables_persistence() {
        assert!(session_env_enabled(None));
        assert!(session_env_enabled(Some("/tmp/session.json")));
        assert!(!session_env_enabled(Some("off")));
        assert!(!session_env_enabled(Some("0")));
        assert!(!session_env_enabled(Some(" none ")));
    }

    #[test]
    fn scene_ids_round_trip_by_stable_id() {
        let scene = project(&sample_pair(), Projection::Bipartite);
        let ids = scene_node_ids(&scene, &[0, 1]);
        assert_eq!(ids, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(scene_indices_for_ids(&scene, &ids), vec![0, 1]);
    }

    #[test]
    fn non_finite_camera_vectors_are_rejected() {
        assert!(finite3([1.0, 2.0, 3.0]));
        assert!(!finite3([1.0, f32::NAN, 3.0]));
        assert!(!finite3([1.0, f32::INFINITY, 3.0]));
        assert_eq!(finite_or(f32::NAN, 1.5), 1.5);
        assert_eq!(finite_or(2.0, 1.5), 2.0);
    }

    #[test]
    fn keep_previous_camera_when_live_camera_missing() {
        let previous = ViewerView {
            camera: Some(CameraPrefs {
                focus: [1.0, 2.0, 3.0],
                radius: 40.0,
                yaw: 0.2,
                pitch: -0.4,
                position: Some([8.0, 4.0, 12.0]),
            }),
            ..ViewerView::default()
        };
        let kept = None.or_else(|| previous.camera.clone());
        assert_eq!(kept, previous.camera);
    }

    fn sample_pair() -> Hypergraph {
        Hypergraph::new()
            .vertex("a", "A", "person")
            .vertex("b", "B", "person")
            .hyperedge("ab", ["a", "b"], "pair")
    }
}
