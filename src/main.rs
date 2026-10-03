use clap::Parser;
use hyper_viz::{InputFormat, Projection, load_json_with_format, project, sample_coauthorship};
use hyper_viz_bevy::{
    ViewMode, VisualizerConfig, run_visualizer_from_path_with_format, run_visualizer_with,
};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(
    name = "hyper",
    version,
    about = "General-purpose 3D hypergraph viewer",
    after_help = "Omit FILE to open the built-in coauthorship demo."
)]
struct Args {
    /// Path to native hypergraph or HIF JSON (legacy host exports also accepted).
    file: Option<String>,

    /// Input format (also applies to watched reloads).
    #[arg(long, default_value = "auto", value_parser = ["auto", "hypergraph", "hif"])]
    format: String,

    /// Reload when the file changes.
    #[arg(short, long)]
    watch: bool,

    /// Projection: bipartite (default), clique, star.
    #[arg(short, long, default_value = "bipartite")]
    projection: String,

    /// View mode: auto selects Dependencies for graphs with validated imports.
    #[arg(long, default_value="auto", value_parser=["auto","spatial","dependencies"])]
    view: String,

    /// Exact initial module ID for the dependency explorer.
    #[arg(long)]
    module: Option<String>,

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
    let format = InputFormat::parse(&args.format).expect("validated input format");
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
            .map(|p| load_json_with_format(p, format).expect("load hypergraph JSON"))
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

    let mode = match args.view.as_str() {
        "spatial" => ViewMode::Spatial,
        "dependencies" => ViewMode::Dependencies,
        _ => ViewMode::Auto,
    };
    let mut config = VisualizerConfig::default().with_view_mode(mode);
    if let Some(id) = args.module {
        config = config.with_module(id);
    }
    if let Some(path) = args.file {
        run_visualizer_from_path_with_format(path, args.watch, projection, config, format);
    } else {
        run_visualizer_with(project(&sample_coauthorship(), projection), config);
    }
}
