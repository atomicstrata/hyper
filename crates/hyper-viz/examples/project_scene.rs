//! Headless library example: build a hypergraph, project it, run force layout.
//!
//! ```bash
//! cargo run -p hyper-viz --example project_scene
//! ```

use hyper_viz::prelude::*;

fn main() {
    let graph = Hypergraph::new()
        .with_id("reactions")
        .with_title("Reactions")
        .vertex("h2", "H2", "molecule")
        .vertex("o2", "O2", "molecule")
        .vertex("h2o", "H2O", "molecule")
        .hyperedge("combust", ["h2", "o2", "h2o"], "2H2 + O2 → 2H2O");

    let scene = graph.project(Projection::Bipartite);
    let mut layout = ForceLayout3D::from_scene(&scene, LayoutConfig::default());
    for _ in 0..32 {
        layout.step();
    }

    println!(
        "{}: {} vertices, {} hyperedges, {} layout nodes",
        scene.meta.title,
        scene.vertices_count(),
        scene.hyperedge_count(),
        layout.positions.len()
    );
    for (node, pos) in scene.nodes.iter().zip(layout.positions.iter()) {
        println!(
            "  {}  ({:.1}, {:.1}, {:.1})",
            node.label, pos.x, pos.y, pos.z
        );
    }
}
