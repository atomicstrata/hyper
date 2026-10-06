//! Reproducible camera tour using the real viewer renderer. PNG capture is
//! deliberately separate from live frame-time measurements. Realtime captures
//! use wall-clock camera motion and timestamped BMP frames for video encoding.
use std::{
    collections::{BTreeMap, HashSet},
    io::Write,
    path::PathBuf,
    time::Instant,
};

use bevy::{
    prelude::*,
    render::view::screenshot::{Screenshot, ScreenshotCaptured},
};
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use bevy_panorbit_camera::PanOrbitCamera;
use hyper_viz::{HypergraphScene, NodeRole};

use crate::{
    VisualizerConfig,
    graph::{GraphLayout, LayoutSettings},
    hyperedge_hull::HyperedgeHullSettings,
    render::LinkRenderSettings,
    session::{SessionStore, SkipAutoFit},
};

#[derive(Resource, Clone)]
pub struct ShowcaseConfig {
    pub frames: u32,
    pub fps: u32,
    pub warmup: u32,
    pub capture: Option<PathBuf>,
    pub report: PathBuf,
    /// Keep the force simulation running during the live benchmark.
    pub live_layout: bool,
    /// Hide the tour overlay; normal HUD and graph labels are already disabled.
    pub clean: bool,
    /// Advance the camera by elapsed time, for `frames / fps` seconds.
    pub realtime: bool,
    /// Use the structural preset and retain all hulls for the entire tour.
    pub structural: bool,
    /// Optional hull fill opacity override, applied after the layout preset.
    pub hull_opacity: Option<f32>,
    /// Draw a bounded, reproducible sample of small-group outlines in dense tours.
    pub hull_outlines: bool,
}

/// Extra boundaries for a dense capture; every hull surface remains enabled.
/// A stable ID hash selects at most 64 groups with 3–24 members, keeping both
/// geometry and line counts bounded without preferring one Mathlib subject.
#[derive(Resource)]
pub(crate) struct TourHullOutlines(pub HashSet<usize>);

fn outline_sample(scene: &HypergraphScene) -> HashSet<usize> {
    let mut candidates: Vec<_> = scene
        .hyperedges
        .iter()
        .enumerate()
        .filter(|(_, edge)| (3..=24).contains(&edge.member_indices.len()))
        .collect();
    candidates.sort_by_key(|(_, edge)| (hash(&edge.id), &edge.id));
    candidates
        .into_iter()
        .take(64)
        .map(|(index, _)| index)
        .collect()
}

#[derive(Resource)]
struct Tour {
    frame: u32,
    settle: u32,
    pending: u32,
    center: Vec3,
    radius: f32,
    started: Instant,
    last: Instant,
    samples: Vec<(u32, bool, f64)>,
    layout_ms: Vec<f64>,
    finished: bool,
    motion_started: Option<Instant>,
    capture_times: Vec<f64>,
}

/// Run a finite tour. Capture directory must be new to prevent mixing sequences.
pub fn run_showcase(scene: HypergraphScene, config: ShowcaseConfig) {
    assert!(
        config.frames > 0 && config.fps > 0,
        "frames and fps must be positive"
    );
    if let Some(path) = &config.capture {
        std::fs::create_dir(path)
            .expect("capture directory must not already exist (create its parent first)");
    }
    if let Some(parent) = config.report.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).expect("create report directory");
    }
    let mut app = crate::visualizer_app_with(
        crate::HyperVisualizerPlugin::from_scene(scene).with_view_mode(crate::ViewMode::Spatial),
        VisualizerConfig::new("hyper · source dependency atlas")
            .with_size(1920, 1080)
            .with_view_mode(crate::ViewMode::Spatial),
        true,
    );
    app.world_mut().resource_mut::<SessionStore>().enabled = false;
    app.world_mut().resource_mut::<SkipAutoFit>().0 = true;
    // Use physical 1080p even on Retina displays, independent of saved sessions.
    for mut window in app
        .world_mut()
        .query::<&mut Window>()
        .iter_mut(app.world_mut())
    {
        window.resolution =
            bevy::window::WindowResolution::new(1920, 1080).with_scale_factor_override(1.0);
        window.present_mode = bevy::window::PresentMode::AutoNoVsync;
    }
    app.insert_resource(config)
        .insert_resource(bevy::winit::WinitSettings::continuous())
        .insert_resource(LinkRenderSettings {
            max_lines: 0,
            opacity: 0.10,
        })
        .insert_resource(ClearColor(Color::srgb(0.012, 0.019, 0.035)))
        .add_systems(PostStartup, prepare)
        .add_systems(PreUpdate, advance)
        .add_systems(EguiPrimaryContextPass, overlay)
        .add_systems(Last, capture)
        .run();
}

fn hash(s: &str) -> u64 {
    s.bytes().fold(0xcbf29ce484222325, |h, b| {
        (h ^ b as u64).wrapping_mul(0x100000001b3)
    })
}

fn seed(s: &str) -> Vec3 {
    let h = hash(s);
    let component = |shift: u32| (((h >> shift) & 0xffff_u64) as f32 / 32767.5) - 1.0;
    Vec3::new(component(0), component(16), component(32))
}

fn prepare(
    mut commands: Commands,
    config: Res<ShowcaseConfig>,
    mut layout: ResMut<GraphLayout>,
    mut settings: ResMut<LayoutSettings>,
    mut hulls: ResMut<HyperedgeHullSettings>,
    mut lines: ResMut<LinkRenderSettings>,
    mut cameras: Query<&mut PanOrbitCamera>,
) {
    settings.node_size = 6.0;
    hulls.enabled = false;
    hulls.opacity = 0.001;
    hulls.wireframe = false;
    hulls.hide_hubs = true;
    for mut camera in &mut cameras {
        camera.enabled = false;
    }
    if config.structural {
        layout.full_graph_overview(&mut settings, &mut hulls, &mut lines);
        layout.rebuild_structural();
    } else {
        for i in 0..layout.node_count {
            let node = &layout.scene.nodes[i];
            let p = seed(&node.kind) * 650.0 + seed(&node.id) * 90.0;
            layout.layout.positions[i] = hyper_viz::Vec3::new(p.x, p.y, p.z);
        }
        // Preserve readable subject clusters while letting dependency forces settle
        // local structure. These settings are part of the reproducible tour.
        layout.layout.config.topology.model = hyper_viz::LayoutModel::Legacy;
        layout.layout.config.gravity = 0.001;
        layout.layout.config.centroid_attraction = 0.0005;
    }
    if let Some(opacity) = config.hull_opacity {
        assert!(
            opacity.is_finite() && (0.0..=1.0).contains(&opacity),
            "hull opacity must be finite and between 0 and 1"
        );
        hulls.opacity = opacity;
    }
    hulls.wireframe = config.hull_outlines;
    if config.hull_outlines {
        commands.insert_resource(TourHullOutlines(outline_sample(&layout.scene)));
    }
    let mut layout_ms = Vec::new();
    for step in 0..config.warmup {
        let start = Instant::now();
        layout.layout.step();
        layout_ms.push(start.elapsed().as_secs_f64() * 1000.0);
        if step % 20 == 0 {
            tracing::info!(step, total = config.warmup, "settling layout");
        }
    }
    layout.running = config.live_layout;
    layout.iterations_per_frame = 1;
    let points: Vec<Vec3> = layout
        .scene
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.role == NodeRole::Vertex)
        .map(|(i, _)| layout.position_at(i).unwrap())
        .collect();
    let (center, radius) = crate::focus::camera_fit(&points).unwrap_or((Vec3::ZERO, 300.0));
    tracing::info!(
        vertices = points.len(),
        nodes = layout.node_count,
        hyperedges = layout.scene.hyperedge_count(),
        radius,
        "tour ready; all import lines enabled"
    );
    commands.insert_resource(Tour {
        frame: 0,
        settle: 0,
        pending: 0,
        center,
        radius,
        started: Instant::now(),
        last: Instant::now(),
        samples: vec![],
        layout_ms,
        finished: false,
        motion_started: None,
        capture_times: vec![],
    });
}

fn advance(
    config: Res<ShowcaseConfig>,
    mut tour: ResMut<Tour>,
    mut cameras: Query<&mut Transform, With<Camera3d>>,
    mut hulls: ResMut<HyperedgeHullSettings>,
) {
    if tour.finished {
        return;
    }
    let now = Instant::now();
    if tour.motion_started.is_none() && tour.settle >= 30 {
        tour.motion_started = Some(now);
    }
    let elapsed_ms = now.duration_since(tour.last).as_secs_f64() * 1000.0;
    tour.last = now;
    if (config.capture.is_none() || config.realtime) && tour.settle >= 30 {
        let f = tour.samples.len() as u32;
        tour.samples.push((f, hulls.enabled, elapsed_ms));
        if config.capture.is_none() {
            tour.frame += 1;
        }
    }
    let t = tour_progress(
        tour.frame,
        config.frames,
        config.fps,
        config.realtime,
        tour.motion_started
            .map_or(0.0, |start| now.duration_since(start).as_secs_f64()),
    );
    let show_hulls = config.structural || t >= 0.5;
    if hulls.enabled != show_hulls {
        hulls.enabled = show_hulls;
    }
    // Clean tours make a complete loop and approach the graph more closely.
    // Realtime motion follows the wall clock even when readbacks drop frames.
    let zoom_depth = if config.clean { 0.50 } else { 0.22 };
    let turns = if config.clean { 1.0 } else { 0.7 };
    let zoom = 0.95 - zoom_depth * (std::f32::consts::PI * t).sin().powi(2);
    let yaw = 0.4 + t * std::f32::consts::TAU * turns;
    let pitch = 0.22 + 0.16 * (t * std::f32::consts::TAU).sin();
    let offset = Vec3::new(
        yaw.sin() * pitch.cos(),
        pitch.sin(),
        yaw.cos() * pitch.cos(),
    );
    for mut transform in &mut cameras {
        *transform = Transform::from_translation(tour.center + offset * tour.radius * zoom)
            .looking_at(tour.center, Vec3::Y);
    }
    tour.settle += 1;
}

fn overlay(
    mut contexts: EguiContexts,
    config: Res<ShowcaseConfig>,
    tour: Res<Tour>,
    layout: Res<GraphLayout>,
) {
    if config.clean {
        return;
    }
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    let rect = ctx.viewport_rect();
    let p = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("tour"),
    ));
    let white = egui::Color32::from_rgb(228, 238, 246);
    let muted = egui::Color32::from_rgb(136, 162, 184);
    let cyan = egui::Color32::from_rgb(91, 221, 225);
    let text = |x, y, size, color, s: &str| {
        p.text(
            egui::pos2(x, y),
            egui::Align2::LEFT_TOP,
            s,
            egui::FontId::proportional(size),
            color,
        );
    };
    text(
        64.0,
        46.0,
        18.0,
        cyan,
        "H Y P E R   /   SOURCE DEPENDENCY ATLAS",
    );
    text(62.0, 78.0, 40.0, white, &layout.scene.meta.title);
    let imports = layout
        .scene
        .hyperedges
        .iter()
        .filter(|h| h.member_indices.len() == 2)
        .count();
    let groups = layout
        .scene
        .hyperedges
        .iter()
        .filter(|h| h.member_indices.len() >= 3)
        .count();
    text(
        64.0,
        134.0,
        22.0,
        muted,
        &format!(
            "{} nodes     /     {} imports     /     {} dependency groups",
            layout.scene.vertices_count(),
            imports,
            groups
        ),
    );
    let progress = tour_progress(
        tour.frame,
        config.frames,
        config.fps,
        config.realtime,
        tour.motion_started
            .map_or(0.0, |start| start.elapsed().as_secs_f64()),
    );
    let hulls = config.structural || progress >= 0.5;
    text(
        64.0,
        rect.bottom() - 100.0,
        25.0,
        white,
        if hulls {
            "02   /   Shared dependency sets"
        } else {
            "01   /   Every source file. Every direct import."
        },
    );
    let mode = if config.capture.is_some() {
        "FRAME CAPTURE"
    } else {
        "LIVE BENCHMARK"
    };
    text(
        64.0,
        rect.bottom() - 61.0,
        16.0,
        muted,
        &format!(
            "{mode}   ·   All import lines   ·   {}   ·   External imports included as placeholders",
            if config.live_layout {
                "Live force layout"
            } else {
                "Settled force layout"
            }
        ),
    );
    p.line_segment(
        [
            egui::pos2(64.0, rect.bottom() - 25.0),
            egui::pos2(rect.right() - 64.0, rect.bottom() - 25.0),
        ],
        egui::Stroke::new(2.0, muted.gamma_multiply(0.3)),
    );
    p.line_segment(
        [
            egui::pos2(64.0, rect.bottom() - 25.0),
            egui::pos2(
                64.0 + (rect.width() - 128.0) * progress,
                rect.bottom() - 25.0,
            ),
        ],
        egui::Stroke::new(2.0, cyan),
    );
}

fn capture(
    mut commands: Commands,
    config: Res<ShowcaseConfig>,
    mut tour: ResMut<Tour>,
    layout: Res<GraphLayout>,
    mut exit: MessageWriter<AppExit>,
) {
    if tour.finished {
        return;
    }
    let seconds = tour
        .motion_started
        .map_or(0.0, |start| start.elapsed().as_secs_f64());
    if tour_progress(
        tour.frame,
        config.frames,
        config.fps,
        config.realtime,
        seconds,
    ) >= 1.0
    {
        if tour.pending > 0 {
            return;
        }
        write_report(&config, &tour, &layout);
        tour.finished = true;
        exit.write(AppExit::Success);
        return;
    }
    let Some(dir) = &config.capture else {
        return;
    };
    let realtime = config.realtime;
    if tour.pending >= if realtime { 3 } else { 1 } {
        return;
    }
    // Let extraction, pipelines and the camera catch up before every readback.
    if tour.settle < if tour.frame == 0 { 30 } else { 3 } {
        return;
    }
    let extension = if realtime { "bmp" } else { "png" };
    let frame = tour.frame;
    let path = dir.join(format!("{frame:06}.{extension}"));
    tour.pending += 1;
    if realtime {
        // Reserve indices and timestamps when requesting, since GPU callbacks
        // may arrive out of order. Bound memory to three outstanding images.
        tour.frame += 1;
        tour.capture_times.push(seconds);
    }
    commands.spawn(Screenshot::primary_window()).observe(
        move |event: On<ScreenshotCaptured>, mut tour: ResMut<Tour>| {
            event
                .image
                .clone()
                .try_into_dynamic()
                .expect("screenshot format")
                .to_rgb8()
                .save(&path)
                .expect("save captured frame");
            tour.pending -= 1;
            if !realtime {
                tour.frame += 1;
                tour.settle = 0;
            }
            if (frame + 1).is_multiple_of(30) {
                tracing::info!(frame = frame + 1, "captured");
            }
        },
    );
}

fn tour_progress(frame: u32, frames: u32, fps: u32, realtime: bool, seconds: f64) -> f32 {
    if realtime {
        (seconds * f64::from(fps) / f64::from(frames)).clamp(0.0, 1.0) as f32
    } else {
        (frame as f32 / frames as f32).clamp(0.0, 1.0)
    }
}

fn percentile(values: &[f64], q: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    sorted[((sorted.len() - 1) as f64 * q).round() as usize]
}

fn write_report(config: &ShowcaseConfig, tour: &Tour, layout: &GraphLayout) {
    if config.realtime
        && let Some(directory) = &config.capture
    {
        let mut timing =
            std::fs::File::create(directory.join("timing.csv")).expect("create capture timing");
        writeln!(
            timing,
            "# duration_seconds={:.6}",
            f64::from(config.frames) / f64::from(config.fps)
        )
        .unwrap();
        writeln!(timing, "frame,time_seconds").unwrap();
        for (frame, seconds) in tour.capture_times.iter().enumerate() {
            writeln!(timing, "{frame},{seconds:.6}").unwrap();
        }
    }
    let mut report = std::fs::File::create(&config.report).expect("create report");
    writeln!(report, "# realtime_clock={}, clean={}, structural={}, captured_frames={}, requested_duration_seconds={:.3}",
        config.realtime, config.clean, config.structural, if config.capture.is_some() { tour.frame } else { 0 },
        f64::from(config.frames) / f64::from(config.fps)).unwrap();
    writeln!(
        report,
        "# hull_opacity_override={:?}, sampled_hull_outlines={}",
        config.hull_opacity, config.hull_outlines
    )
    .unwrap();
    writeln!(report, "# {}\n# mode={}, layout_running={}, vertices={}, scene_nodes={}, hyperedges={}, all_import_lines=true, resolution=1920x1080, warmup={}, frames={}, output_fps={}, wall_seconds={:.3}",
        layout.scene.meta.id, if config.capture.is_some() { "capture (includes GPU readback and disk overhead)" } else { "live" },
        config.live_layout, layout.scene.vertices_count(), layout.node_count, layout.scene.hyperedge_count(),
        config.warmup, config.frames, config.fps, tour.started.elapsed().as_secs_f64()).unwrap();
    writeln!(
        report,
        "# layout_step_ms_p50={:.3}, layout_step_ms_p95={:.3}",
        percentile(&tour.layout_ms, 0.5),
        percentile(&tour.layout_ms, 0.95)
    )
    .unwrap();
    let mut stages: BTreeMap<bool, Vec<f64>> = BTreeMap::new();
    for &(_, hulls, ms) in &tour.samples {
        stages.entry(hulls).or_default().push(ms);
    }
    for (hulls, values) in stages {
        let p50 = percentile(&values, 0.5);
        let p95 = percentile(&values, 0.95);
        writeln!(
            report,
            "# hulls={hulls}, frame_ms_p50={p50:.3}, frame_ms_p95={p95:.3}, median_fps={:.2}",
            1000.0 / p50
        )
        .unwrap();
        tracing::info!(hulls, p50, p95, median_fps = 1000.0 / p50, "live benchmark");
    }
    writeln!(report, "frame,hulls,frame_ms").unwrap();
    for &(frame, hulls, ms) in &tour.samples {
        writeln!(report, "{frame},{hulls},{ms:.4}").unwrap();
    }
    tracing::info!(path = %config.report.display(), "report saved");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn outline_budget_is_stable_and_does_not_drop_group_surfaces() {
        let mut graph = hyper_viz::Hypergraph::new();
        for n in 0..30 {
            let id = n.to_string();
            graph = graph.vertex(&id, &id, "node");
        }
        for n in 0..100 {
            graph.add_hyperedge(hyper_viz::Hyperedge::new(
                format!("group-{n}"),
                ["0", "1", "2", "3"],
            ));
        }
        graph.add_hyperedge(hyper_viz::Hyperedge::new("pair", ["0", "1"]));
        graph.add_hyperedge(hyper_viz::Hyperedge::new(
            "large",
            (0..30).map(|n| n.to_string()),
        ));
        let scene = graph.project(hyper_viz::Projection::StarCentroid);
        let selected_ids = |scene: &HypergraphScene| -> HashSet<String> {
            outline_sample(scene)
                .into_iter()
                .map(|index| scene.hyperedges[index].id.clone())
                .collect()
        };
        let sample = selected_ids(&scene);
        assert_eq!(sample.len(), 64);
        assert!(!sample.contains("pair") && !sample.contains("large"));
        let mut reordered = scene.clone();
        reordered.hyperedges.reverse();
        assert_eq!(sample, selected_ids(&reordered));
        assert_eq!(scene.hyperedges.len(), 102, "sampling only adds boundaries");
    }

    #[test]
    fn structural_tour_applies_hull_style_after_the_overview_preset() {
        let mut app = App::new();
        app.insert_resource(ShowcaseConfig {
            frames: 120,
            fps: 30,
            warmup: 0,
            capture: None,
            report: Default::default(),
            live_layout: false,
            clean: true,
            realtime: true,
            structural: true,
            hull_opacity: Some(0.025),
            hull_outlines: true,
        })
        .init_resource::<LayoutSettings>()
        .init_resource::<HyperedgeHullSettings>()
        .init_resource::<LinkRenderSettings>()
        .insert_resource(GraphLayout::from_scene(
            hyper_viz::sample_coauthorship().project(hyper_viz::Projection::StarCentroid),
            &LayoutSettings::default(),
        ))
        .add_systems(Update, prepare);
        app.update();
        let hulls = app.world().resource::<HyperedgeHullSettings>();
        assert!(hulls.enabled && hulls.wireframe);
        assert_eq!(hulls.opacity, 0.025);
        assert!(!app.world().resource::<TourHullOutlines>().0.is_empty());
        assert!(!app.world().resource::<GraphLayout>().running);
    }

    #[test]
    fn realtime_capture_pipelines_readbacks_instead_of_skipping_camera_frames() {
        let directory = tempfile::tempdir().unwrap();
        let frames = directory.path().join("frames");
        std::fs::create_dir(&frames).unwrap();
        let mut app = App::new();
        app.add_message::<AppExit>()
            .insert_resource(ShowcaseConfig {
                frames: 120,
                fps: 30,
                warmup: 0,
                capture: Some(frames),
                report: directory.path().join("report.csv"),
                live_layout: false,
                clean: true,
                realtime: true,
                structural: true,
                hull_opacity: Some(0.025),
                hull_outlines: true,
            })
            .insert_resource(GraphLayout::from_scene(
                hyper_viz::Hypergraph::new()
                    .vertex("a", "a", "node")
                    .project(hyper_viz::Projection::StarCentroid),
                &LayoutSettings::default(),
            ))
            .insert_resource(Tour {
                frame: 0,
                settle: 30,
                pending: 0,
                center: Vec3::ZERO,
                radius: 1.,
                started: Instant::now(),
                last: Instant::now(),
                samples: vec![],
                layout_ms: vec![],
                finished: false,
                motion_started: Some(Instant::now()),
                capture_times: vec![],
            })
            .add_systems(Update, capture);
        for _ in 0..4 {
            app.update();
        }
        assert_eq!(
            app.world_mut()
                .query::<&Screenshot>()
                .iter(app.world())
                .count(),
            3,
            "Keep a bounded three-frame readback pipeline full during realtime capture"
        );
        assert_eq!(app.world().resource::<Tour>().capture_times.len(), 3);
        app.world_mut().resource_mut::<Tour>().motion_started =
            Some(Instant::now() - std::time::Duration::from_secs(5));
        app.update();
        assert!(
            !app.world().resource::<Tour>().finished,
            "Do not close the window before pending readbacks have completed"
        );
    }

    #[test]
    fn realtime_camera_keeps_wall_clock_speed_when_frames_are_dropped() {
        assert_eq!(tour_progress(5, 120, 30, true, 1.0), 0.25);
        assert_eq!(tour_progress(80, 120, 30, true, 1.0), 0.25);
        assert_eq!(tour_progress(5, 120, 30, true, 4.0), 1.0);
        assert_eq!(tour_progress(500, 120, 30, true, 0.0), 0.0);
    }

    #[test]
    fn offline_camera_stays_frame_indexed() {
        assert_eq!(tour_progress(30, 120, 30, false, 100.0), 0.25);
        assert_eq!(tour_progress(120, 120, 30, false, 0.0), 1.0);
    }

    #[test]
    fn tail_latency_is_not_hidden_by_average() {
        assert_eq!(percentile(&[10.0, 10.0, 10.0, 200.0, 10.0], 0.5), 10.0);
        assert_eq!(percentile(&[10.0, 10.0, 10.0, 200.0, 10.0], 0.95), 200.0);
    }
}
