//! Reproducible camera tour using the real viewer renderer. PNG capture is
//! deliberately separate from live frame-time measurements.
use std::{collections::BTreeMap, io::Write, path::PathBuf, time::Instant};

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
}

#[derive(Resource)]
struct Tour {
    frame: u32,
    settle: u32,
    pending: bool,
    center: Vec3,
    radius: f32,
    started: Instant,
    last: Instant,
    samples: Vec<(u32, bool, f64)>,
    layout_ms: Vec<f64>,
    finished: bool,
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
        crate::HyperVisualizerPlugin::from_scene(scene),
        VisualizerConfig::new("hyper · source dependency atlas").with_size(1920, 1080),
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
    for i in 0..layout.node_count {
        let node = &layout.scene.nodes[i];
        let p = seed(&node.kind) * 650.0 + seed(&node.id) * 90.0;
        layout.layout.positions[i] = hyper_viz::Vec3::new(p.x, p.y, p.z);
    }
    let mut layout_ms = Vec::new();
    // Preserve readable subject clusters while letting dependency forces settle
    // local structure. These settings are part of the reproducible tour.
    layout.layout.config.gravity = 0.001;
    layout.layout.config.centroid_attraction = 0.0005;
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
        pending: false,
        center,
        radius,
        started: Instant::now(),
        last: Instant::now(),
        samples: vec![],
        layout_ms,
        finished: false,
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
    let elapsed_ms = now.duration_since(tour.last).as_secs_f64() * 1000.0;
    tour.last = now;
    if config.capture.is_none() && tour.settle >= 30 {
        let f = tour.frame;
        tour.samples.push((f, hulls.enabled, elapsed_ms));
        tour.frame += 1;
    }
    let t = tour.frame.min(config.frames) as f32 / config.frames as f32;
    let show_hulls = t >= 0.5;
    if hulls.enabled != show_hulls {
        hulls.enabled = show_hulls;
    }
    // Ease into the cloud, then return to the overview. A fixed frame clock
    // keeps output duration independent of render/readback/encoding speed.
    let zoom = 0.95 - 0.22 * (std::f32::consts::PI * t).sin().powi(2);
    let yaw = 0.4 + t * std::f32::consts::TAU * 0.7;
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
    let hulls = tour.frame as f32 / config.frames as f32 >= 0.5;
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
    let progress = (tour.frame as f32 / config.frames as f32).min(1.0);
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
    if tour.finished || tour.pending {
        return;
    }
    if tour.frame >= config.frames {
        write_report(&config, &tour, &layout);
        tour.finished = true;
        exit.write(AppExit::Success);
        return;
    }
    let Some(dir) = &config.capture else {
        return;
    };
    // Let extraction, pipelines and the camera catch up before every readback.
    if tour.settle < if tour.frame == 0 { 30 } else { 3 } {
        return;
    }
    let path = dir.join(format!("{:06}.png", tour.frame));
    tour.pending = true;
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
            tour.pending = false;
            tour.frame += 1;
            tour.settle = 0;
            if tour.frame % 30 == 0 {
                tracing::info!(frame = tour.frame, "captured");
            }
        },
    );
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
    let mut report = std::fs::File::create(&config.report).expect("create report");
    writeln!(report, "# {}\n# mode={}, layout_running={}, vertices={}, scene_nodes={}, hyperedges={}, all_import_lines=true, resolution=1920x1080, warmup={}, frames={}, output_fps={}, wall_seconds={:.3}",
        layout.scene.meta.id, if config.capture.is_some() { "capture (not a realtime benchmark)" } else { "live" },
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
    fn tail_latency_is_not_hidden_by_average() {
        assert_eq!(percentile(&[10.0, 10.0, 10.0, 200.0, 10.0], 0.5), 10.0);
        assert_eq!(percentile(&[10.0, 10.0, 10.0, 200.0, 10.0], 0.95), 200.0);
    }
}
