//! Inspect exact directed neighbors without opening a window.
use hyper_viz::{DependencyIndex, Projection, ScopeOptions, load_json};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let file = args
        .next()
        .ok_or("usage: inspect_dependencies GRAPH MODULE")?;
    let id = args
        .next()
        .ok_or("usage: inspect_dependencies GRAPH MODULE")?;
    let scene = load_json(file)?.project(Projection::StarCentroid);
    let index = DependencyIndex::new(&scene);
    let center = index.node_index(&id).ok_or("module not found")?;
    for (label, nodes) in [
        ("Imports", index.imports(center)),
        ("Dependents", index.dependents(center)),
    ] {
        println!("{label}: {}", nodes.len());
        for &node in nodes {
            println!("  {}", scene.nodes[node].id);
        }
    }
    let scope = index.scope(
        center,
        &ScopeOptions::default(),
        &vec![true; scene.node_count()],
    );
    println!(
        "Scope: {} modules, {} directed edges, {} omitted",
        scope.nodes.len(),
        scope.edges.len(),
        scope.omitted
    );
    for warning in index.warnings() {
        eprintln!("Warning: {warning}");
    }
    Ok(())
}
