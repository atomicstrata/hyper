//! Opt-in finite benchmark of the production renderer, with picking and HUD.
use crate::{
    graph::{GraphLayout, GraphSceneEpoch, LayoutSettings},
    hyperedge_hull::HyperedgeHullSettings,
    node_visual::NodeRenderSettings,
    session::{SessionStore, SkipAutoFit},
};
use bevy::prelude::*;
use bevy_panorbit_camera::PanOrbitCamera;
use hyper_viz::{Hypergraph, HypergraphScene, NodeRole, Projection};
use serde_json::{Value, json};
use std::{collections::HashMap, path::PathBuf, time::Instant};

#[derive(Resource)]
struct Run {
    graph: Hypergraph,
    updated: HypergraphScene,
    frame: usize,
    warmup: usize,
    frames: usize,
    hulls: bool,
    live: bool,
    report: PathBuf,
    last: Instant,
    intervals: Vec<f64>,
    updates: Vec<f64>,
    layout_ms: Vec<f64>,
    parse_ms: f64,
    projection_ms: f64,
    completed: bool,
    capture: Option<PathBuf>,
    capture_pending: bool,
    projection: String,
    labels: bool,
    expected_iterations: u64,
}

/// Arguments: native JSON, output JSON, hulls, live, frames, capture, projection, labels.
pub fn run() {
    let args: Vec<String> = std::env::args().collect();
    assert!(
        args.len() >= 5,
        "usage: ecosystem-benchmark INPUT OUTPUT HULLS LIVE [FRAMES] [CAPTURE|-] [PROJECTION] [LABELS]"
    );
    let raw = std::fs::read_to_string(&args[1]).expect("read dataset");
    let start = Instant::now();
    let graph = hyper_viz::from_json_str(&raw).expect("load canonical native dataset");
    let parse_ms = start.elapsed().as_secs_f64() * 1000.;
    let start = Instant::now();
    let projection_name = args.get(7).map(String::as_str).unwrap_or("bipartite");
    let projection = Projection::parse(projection_name).expect("supported projection");
    let scene = graph.project(projection);
    let projection_ms = start.elapsed().as_secs_f64() * 1000.;
    let updated_path = args[1].strip_suffix(".json").unwrap().to_string() + ".updated.json";
    let updated = hyper_viz::load_json(updated_path)
        .expect("shared update fixture")
        .project(projection);
    let mut app = crate::visualizer_app_with(
        crate::HyperVisualizerPlugin::from_scene(scene).with_projection(projection),
        crate::VisualizerConfig::new("Hyper ecosystem benchmark").with_size(1920, 1080),
        false,
    );
    app.world_mut().resource_mut::<SessionStore>().enabled = false;
    app.world_mut().resource_mut::<SkipAutoFit>().0 = true;
    for mut window in app
        .world_mut()
        .query::<&mut Window>()
        .iter_mut(app.world_mut())
    {
        window.resolution =
            bevy::window::WindowResolution::new(1920, 1080).with_scale_factor_override(1.0);
        window.present_mode = bevy::window::PresentMode::AutoVsync;
    }
    app.insert_resource(bevy::winit::WinitSettings::continuous())
        .insert_resource(crate::LinkRenderSettings {
            max_lines: 0,
            opacity: 0.10,
        })
        .insert_resource(Run {
            graph,
            updated,
            frame: 0,
            warmup: 120,
            frames: args.get(5).map(|v| v.parse().unwrap()).unwrap_or(300),
            hulls: args[3] == "1",
            live: args[4] == "1",
            report: (&args[2]).into(),
            last: Instant::now(),
            intervals: vec![],
            updates: vec![],
            layout_ms: vec![],
            parse_ms,
            projection_ms,
            completed: false,
            capture: args.get(6).filter(|p| p.as_str() != "-").map(PathBuf::from),
            capture_pending: false,
            projection: projection_name.into(),
            labels: args.get(8).is_some_and(|v| v == "1"),
            expected_iterations: 0,
        })
        .add_systems(PostStartup, prepare)
        .add_systems(PreUpdate, tick)
        .add_systems(
            Update,
            disable_builtin_steps.before(crate::graph::step_layout),
        )
        .add_systems(Last, finish)
        .run();
}

fn prepare(
    mut state: ResMut<Run>,
    mut layout: ResMut<GraphLayout>,
    mut settings: ResMut<LayoutSettings>,
    mut hulls: ResMut<HyperedgeHullSettings>,
    mut nodes: ResMut<NodeRenderSettings>,
    mut cameras: Query<&mut PanOrbitCamera>,
) {
    let positions: HashMap<_, _> = state
        .graph
        .vertices
        .iter()
        .map(|v| {
            let p = v.attrs["position"].as_array().unwrap();
            (
                v.id.as_str(),
                hyper_viz::Vec3::new(
                    p[0].as_f64().unwrap() as f32,
                    p[1].as_f64().unwrap() as f32,
                    p[2].as_f64().unwrap() as f32,
                ),
            )
        })
        .collect();
    let edges: HashMap<_, _> = state
        .graph
        .hyperedges
        .iter()
        .map(|e| (e.id.as_str(), e))
        .collect();
    for node in layout.scene.nodes.clone() {
        let p = if node.role == NodeRole::Vertex {
            positions[node.id.as_str()]
        } else {
            let e = edges[node.hyperedge_id.as_deref().unwrap()];
            let mut p = hyper_viz::Vec3::ZERO;
            for v in &e.vertices {
                p += positions[v.as_str()];
            }
            if !e.vertices.is_empty() {
                p = p / e.vertices.len() as f32;
            }
            p
        };
        layout.layout.positions[node.index] = p;
    }
    // Keep production live hull skinning/topology refresh enabled, while the
    // timed step replaces the built-in solver calls.
    layout.running = state.live;
    layout.iterations_per_frame = 0;
    settings.iterations_per_frame = 0;
    settings.node_size = 2.;
    hulls.enabled = state.hulls;
    hulls.wireframe = false;
    hulls.hide_hubs = state.projection == "star";
    nodes.labels_enabled = state.labels;
    nodes.hyperedge_labels = state.labels;
    for mut camera in &mut cameras {
        camera.enabled = false;
    }
    state.last = Instant::now();
}

fn disable_builtin_steps(
    state: Res<Run>,
    mut layout: ResMut<GraphLayout>,
    mut settings: ResMut<LayoutSettings>,
) {
    // The HUD slider clamps its displayed zero to one. Reset before the stock
    // stepping system, without disabling the HUD or live geometry systems.
    layout.iterations_per_frame = 0;
    layout.running = state.live;
    settings.iterations_per_frame = 0;
}

fn tick(
    mut state: ResMut<Run>,
    mut layout: ResMut<GraphLayout>,
    settings: Res<LayoutSettings>,
    mut epoch: ResMut<GraphSceneEpoch>,
    mut cameras: Query<&mut Transform, With<Camera3d>>,
) {
    if state.completed {
        return;
    }
    assert_eq!(
        layout.layout.iterations, state.expected_iterations,
        "unexpected untimed solver step"
    );
    let now = Instant::now();
    let elapsed = now.duration_since(state.last).as_secs_f64() * 1000.;
    state.last = now;
    if state.frame > state.warmup {
        state.intervals.push(elapsed);
    }
    if state.live {
        let start = Instant::now();
        layout.layout.step();
        state.expected_iterations += 1;
        if state.frame >= state.warmup {
            state.layout_ms.push(start.elapsed().as_secs_f64() * 1000.);
        }
    }
    // Scripted orbit keeps every frame dirty. This is not hardware input latency.
    let angle = state.frame as f32 * 0.003;
    for mut camera in &mut cameras {
        *camera = Transform::from_xyz(angle.sin() * 1500., 350., angle.cos() * 1500.)
            .looking_at(Vec3::ZERO, Vec3::Y);
    }
    if state.frame == state.warmup + state.frames {
        let start = Instant::now();
        *layout = GraphLayout::from_scene_preserve(&layout, state.updated.clone(), &settings);
        state.expected_iterations = 0;
        // Match the shared updated hub centroids; stable vertex positions survive.
        let hubs: Vec<_> = layout
            .scene
            .hyperedges
            .iter()
            .filter(|e| layout.scene.nodes[e.hub_index].role == NodeRole::HyperedgeHub)
            .map(|e| {
                let mut p = hyper_viz::Vec3::ZERO;
                for &i in &e.member_indices {
                    p += layout.layout.positions[i];
                }
                if !e.member_indices.is_empty() {
                    p = p / e.member_indices.len() as f32;
                }
                (e.hub_index, p)
            })
            .collect();
        for (index, position) in hubs {
            layout.layout.positions[index] = position;
        }
        epoch.0 += 1;
        state.updates.push(start.elapsed().as_secs_f64() * 1000.);
    }
    state.frame += 1;
}

fn finish(
    mut commands: Commands,
    mut state: ResMut<Run>,
    layout: Res<GraphLayout>,
    mut exit: MessageWriter<AppExit>,
) {
    if state.completed || state.frame <= state.warmup + state.frames + 3 {
        return;
    }
    if state.capture_pending {
        return;
    }
    if let Some(path) = state.capture.take() {
        state.capture_pending = true;
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
            .observe(
                move |event: On<bevy::render::view::screenshot::ScreenshotCaptured>,
                      mut run: ResMut<Run>| {
                    event
                        .image
                        .clone()
                        .try_into_dynamic()
                        .unwrap()
                        .to_rgb8()
                        .save(&path)
                        .unwrap();
                    run.capture_pending = false;
                },
            );
        return;
    }
    state.completed = true;
    assert_eq!(
        layout.layout.iterations, state.expected_iterations,
        "unexpected untimed solver step"
    );
    let intervals: Vec<_> = state.intervals.iter().copied().take(state.frames).collect();
    let result: Value = json!({"tool":"hyper", "dataset":state.graph.meta.id, "profile":"release", "projection":state.projection, "window":[1920,1080], "dpr":1, "present_mode":"AutoVsync", "hulls":state.hulls,"live_layout":state.live,"hud":true,"picking":true,"labels":state.labels,"label_policy":"default Capped; count threshold250; hovered/selected exemptions","line_opacity":0.10,"node_size":2,"hull_opacity":0.28,"hull_wireframe":false,"hull_vertex_cap":hyper_viz::MAX_HULL_VERTICES,"hull_topology_interval":8,"live_solver_steps_per_frame":if state.live {1} else {0},"line_budget":0,"warmup_frames":state.warmup,"frames":state.frames,"frame_interval_ms":intervals,"layout_step_ms":state.layout_ms,"update_commit_ms":state.updates,"update_next_frame_ms":state.intervals.get(state.frames),"native_parse_ms":state.parse_ms,"projection_ms":state.projection_ms,"nodes":layout.node_count,"scene_links":layout.link_count,"rendered_lines":if state.projection == "star" { state.graph.hyperedges.iter().filter(|e| e.vertices.len()==2).count() } else { layout.link_count },"measurement":"CPU wall intervals; no GPU timer, hardware input or presentation timestamp"});
    if let Some(parent) = state.report.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&state.report, serde_json::to_vec_pretty(&result).unwrap())
        .expect("write measurements");
    exit.write(AppExit::Success);
}
