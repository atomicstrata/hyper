//! `cargo run -p hyper-viz --example hif_roundtrip -- input.json output.json`
use hyper_viz::{load_hif, save_hif};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: hif_roundtrip INPUT OUTPUT".into());
    }
    let document = load_hif(&args[1])?;
    save_hif(&args[2], &document)?;
    println!("Preserved {} incidences", document.incidences().len());
    match document.to_hypergraph() {
        Ok(graph) => println!(
            "Viewer compatible: {} nodes / {} edges",
            graph.vertices.len(),
            graph.hyperedges.len()
        ),
        Err(error) => println!("Document preserved; viewer conversion unavailable: {error}"),
    }
    Ok(())
}
