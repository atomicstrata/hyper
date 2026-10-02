use clap::Parser;
use hyper_viz::{Projection, load_json, project, sample_coauthorship};
use hyper_viz_bevy::{
    VisualizerConfig, run_from_graph, run_visualizer, run_visualizer_from_path_with,
};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(
    name = "hyper",
    about = "General-purpose 3D hypergraph viewer",
    after_help = "Omit FILE to open the built-in coauthorship demo."
)]
struct Args {
    /// Path to hypergraph JSON (`hypergraph.v1`; legacy host exports also accepted).
    file: Option<String>,

    /// Reload when the file changes.
    #[arg(short, long)]
    watch: bool,

    /// Projection: bipartite (default), clique, star.
    #[arg(short, long, default_value = "bipartite")]
    projection: String,

    /// Run a finite, deterministic camera tour with all import lines enabled.
    #[arg(long, conflicts_with = "watch")]
    tour: bool,

    /// Save a numbered PNG sequence to a NEW directory (requires --tour).
    #[arg(long, requires = "tour")]
    capture: Option<std::path::PathBuf>,

    /// Number of tour frames. At 30 fps, 900 frames makes a 30-second video.
    #[arg(long, default_value_t = 900, value_parser = clap::value_parser!(u32).range(1..))]
    frames: u32,

    /// Playback fps for the captured sequence; does not claim real-time speed.
    #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(u32).range(1..))]
    fps: u32,

    /// Deterministic force-layout steps before the tour.
    #[arg(long, default_value_t = 120)]
    warmup: u32,

    /// Frame timings and capture metadata CSV.
    #[arg(long, default_value = "target/mathlib/benchmark.csv")]
    report: std::path::PathBuf,

    /// Benchmark a moving force layout (one step/frame), rather than frozen geometry.
    #[arg(long, requires = "tour", conflicts_with = "capture")]
    live_layout: bool,
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("hyper=info".parse().unwrap())
                .add_directive("hyper_viz_bevy=info".parse().unwrap()),
        )
        .init();

    let args = Args::parse();
    let projection = Projection::parse(&args.projection).unwrap_or_else(|| {
        tracing::warn!(
            projection = %args.projection,
            "unknown projection, falling back to bipartite"
        );
        Projection::Bipartite
    });

    if args.tour {
        let graph = args
            .file
            .as_deref()
            .map(|p| load_json(p).expect("load hypergraph JSON"))
            .unwrap_or_else(sample_coauthorship);
        hyper_viz_bevy::showcase::run_showcase(
            project(&graph, projection),
            hyper_viz_bevy::showcase::ShowcaseConfig {
                frames: args.frames,
                fps: args.fps,
                warmup: args.warmup,
                capture: args.capture,
                report: args.report,
                live_layout: args.live_layout,
            },
        );
        return;
    }

    match (args.file.as_deref(), args.watch) {
        (None, _) => {
            let scene = project(&sample_coauthorship(), projection);
            run_visualizer(scene);
        }
        (Some(path), true) => {
            run_visualizer_from_path_with(
                path.to_string(),
                true,
                projection,
                VisualizerConfig::default(),
            );
        }
        (Some(path), false) => {
            let graph = load_json(path).expect("load hypergraph JSON");
            if projection == Projection::Bipartite {
                run_from_graph(graph);
            } else {
                let scene = project(&graph, projection);
                run_visualizer(scene);
            }
        }
    }
}
