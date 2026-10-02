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
