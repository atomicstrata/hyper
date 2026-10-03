//! Finite measurement of the normal native dependency canvas (no tour systems).
//! HYPER_VIZ_SESSION=off cargo run --release -p hyper-viz-bevy --example module_benchmark -- GRAPH MODULE OUTPUT
use bevy::{
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};
use hyper_viz::{Projection, load_json};
use hyper_viz_bevy::{ViewMode, VisualizerConfig, visualizer_app_with_config};
use std::{path::PathBuf, time::Instant};
#[derive(Resource)]
struct Measurement {
    frame: usize,
    last: Instant,
    samples: Vec<f64>,
    output: PathBuf,
}
fn measure(mut commands: Commands, mut m: ResMut<Measurement>, mut exits: MessageWriter<AppExit>) {
    let now = Instant::now();
    let ms = now.duration_since(m.last).as_secs_f64() * 1000.;
    m.last = now;
    m.frame += 1;
    if m.frame > 60 {
        m.samples.push(ms);
    }
    if m.frame == 90 {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(m.output.join("canvas.png")));
    }
    if m.frame == 240 {
        let csv = format!(
            "frame,milliseconds\n{}",
            m.samples
                .iter()
                .enumerate()
                .map(|(i, ms)| format!("{i},{ms:.3}\n"))
                .collect::<String>()
        );
        std::fs::write(m.output.join("frames.csv"), csv).unwrap();
        m.samples.sort_by(f64::total_cmp);
        let n = m.samples.len();
        println!(
            "Native canvas: median {:.3} ms, p95 {:.3} ms, {} measured frames",
            m.samples[n / 2],
            m.samples[(n * 95 / 100).min(n - 1)],
            n
        );
        exits.write(AppExit::Success);
    }
}
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert!(
        (3..=4).contains(&args.len()),
        "usage: module_benchmark GRAPH MODULE OUTPUT [auto|spatial|dependencies]"
    );
    let output = PathBuf::from(&args[2]);
    std::fs::create_dir_all(&output).unwrap();
    let scene = load_json(&args[0])
        .unwrap()
        .project(Projection::StarCentroid);
    let config = VisualizerConfig::new("Module explorer validation")
        .with_view_mode(match args.get(3).map(String::as_str) {
            Some("auto") => ViewMode::Auto,
            Some("spatial") => ViewMode::Spatial,
            _ => ViewMode::Dependencies,
        })
        .with_module(&args[1])
        .with_size(1920, 1080);
    let mut app = visualizer_app_with_config(scene, config);
    for mut window in app
        .world_mut()
        .query::<&mut Window>()
        .iter_mut(app.world_mut())
    {
        window.resolution =
            bevy::window::WindowResolution::new(1920, 1080).with_scale_factor_override(1.0);
    }
    app.insert_resource(bevy::winit::WinitSettings::continuous())
        .insert_resource(Measurement {
            frame: 0,
            last: Instant::now(),
            samples: Vec::new(),
            output,
        })
        .add_systems(Last, measure)
        .run();
}
