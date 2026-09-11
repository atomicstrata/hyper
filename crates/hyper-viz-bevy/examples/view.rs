//! View a hypergraph JSON snapshot in the native 3D viewer.
//!
//! ```bash
//! cargo run -p hyper-viz-bevy --example view -- fixtures/sample.json
//! cargo run -p hyper-viz-bevy --example view -- --watch path/to/hypergraph.json
//! ```

use std::env;

use hyper_viz::{Projection, load_json, project};
use hyper_viz_bevy::{run_visualizer, run_visualizer_from_path};
use tracing_subscriber::EnvFilter;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("hyper_viz_bevy=info".parse().unwrap()),
        )
        .init();

    let args: Vec<String> = env::args().skip(1).collect();
    let watch = args.iter().any(|a| a == "--watch");
    let path = args
        .iter()
        .find(|a| !a.starts_with("--"))
        .expect("usage: view [--watch] <hypergraph.json>");

    if watch {
        run_visualizer_from_path(path.clone(), true);
        return;
    }

    let graph = load_json(path).expect("load hypergraph JSON");
    let scene = project(&graph, Projection::Bipartite);
    run_visualizer(scene);
}
