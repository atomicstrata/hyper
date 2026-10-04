//! Isolated conversion, projection, layout, and hull CPU timings.
use hyper_viz::{ForceLayout3D, LayoutConfig, NodeRole, Projection};
use serde_json::json;
use std::{
    collections::{BTreeSet, HashMap},
    hint::black_box,
    time::Instant,
};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let raw = std::fs::read_to_string(&args[1]).unwrap();
    let hif_raw =
        std::fs::read_to_string(args[1].trim_end_matches(".json").to_string() + ".hif.json")
            .unwrap();
    let start = Instant::now();
    let graph = hyper_viz::from_json_str(&raw).unwrap();
    let native_ms = start.elapsed().as_secs_f64() * 1000.;
    let start = Instant::now();
    let document = hyper_viz::parse_hif(&hif_raw).unwrap();
    let hif_ms = start.elapsed().as_secs_f64() * 1000.;
    let start = Instant::now();
    let converted = document.to_hypergraph().unwrap();
    let convert_ms = start.elapsed().as_secs_f64() * 1000.;
    assert_eq!(converted.vertices.len(), graph.vertices.len());
    assert_eq!(converted.hyperedges.len(), graph.hyperedges.len());
    // These fixtures intentionally use string IDs. Verify every record, not just counts.
    for (node, native) in document.nodes().unwrap().iter().zip(&converted.vertices) {
        let hyper_viz::HifId::String(id) = &node.node else {
            panic!("fixture ID must be a string")
        };
        assert_eq!(native.id, format!("s:{id}"));
        assert_eq!(native.attrs, node.attrs.clone().unwrap_or_default());
    }
    for (edge, native) in graph.hyperedges.iter().zip(&converted.hyperedges) {
        assert_eq!(native.id, format!("s:{}", edge.id));
        assert_eq!(
            native.vertices,
            edge.vertices
                .iter()
                .map(|id| format!("s:{id}"))
                .collect::<Vec<_>>()
        );
    }
    for (edge, native) in document.edges().unwrap().iter().zip(&converted.hyperedges) {
        assert_eq!(native.attrs, edge.attrs.clone().unwrap_or_default());
    }
    assert_eq!(
        converted.meta.attrs,
        serde_json::from_value::<serde_json::Map<String, serde_json::Value>>(
            serde_json::to_value(&document).unwrap()["metadata"].clone()
        )
        .unwrap()
    );
    let roundtrip = hyper_viz::parse_hif(&hyper_viz::serialize_hif(&document).unwrap()).unwrap();
    assert_eq!(document, roundtrip);
    let start = Instant::now();
    let scene = graph.project(Projection::Bipartite);
    let projection_ms = start.elapsed().as_secs_f64() * 1000.;
    assert!(scene.warnings.is_empty(), "invalid canonical projection");
    assert_eq!(
        scene.links.len(),
        graph
            .hyperedges
            .iter()
            .map(|e| e.vertices.len())
            .sum::<usize>()
    );
    let expected: BTreeSet<_> = graph
        .hyperedges
        .iter()
        .flat_map(|e| e.vertices.iter().map(|v| (e.id.clone(), v.clone())))
        .collect();
    let actual: BTreeSet<_> = scene
        .links
        .iter()
        .map(|link| {
            assert_eq!(scene.nodes[link.source].role, NodeRole::HyperedgeHub);
            assert_eq!(scene.nodes[link.target].role, NodeRole::Vertex);
            (
                link.hyperedge_id.clone().unwrap(),
                scene.nodes[link.target].id.clone(),
            )
        })
        .collect();
    assert_eq!(
        actual, expected,
        "every incidence must connect its original group/member"
    );
    let positions: HashMap<_, _> = graph
        .vertices
        .iter()
        .map(|v| {
            let p = v.attrs["position"].as_array().unwrap();
            (
                v.id.as_str(),
                hyper_viz::Vec3::new(
                    p[0].as_f64().unwrap() as f32,
                    p[1].as_f64().unwrap() as f32,
                    p[2].as_f64().unwrap() as f32,
                ),
            )
        })
        .collect();
    let mut layout = ForceLayout3D::from_scene(&scene, LayoutConfig::default());
    let edges: HashMap<_, _> = graph
        .hyperedges
        .iter()
        .map(|e| (e.id.as_str(), e))
        .collect();
    for node in &scene.nodes {
        layout.positions[node.index] = if node.role == NodeRole::Vertex {
            positions[node.id.as_str()]
        } else {
            let e = edges[node.hyperedge_id.as_deref().unwrap()];
            let mut p = hyper_viz::Vec3::ZERO;
            for v in &e.vertices {
                p += positions[v.as_str()];
            }
            if e.vertices.is_empty() {
                p
            } else {
                p / e.vertices.len() as f32
            }
        };
    }
    // Warm the same solver, then record consecutive steps; not layout convergence.
    for _ in 0..5 {
        layout.step();
    }
    let mut steps = Vec::new();
    for _ in 0..20 {
        let start = Instant::now();
        layout.step();
        steps.push(start.elapsed().as_secs_f64() * 1000.);
    }
    let start = Instant::now();
    let mut hulls = 0;
    for e in &graph.hyperedges {
        let points: Vec<_> = e
            .vertices
            .iter()
            .map(|v| {
                let p = positions[v.as_str()];
                [p.x, p.y, p.z]
            })
            .collect();
        if black_box(hyper_viz::hull_from_points(&points)).is_some() {
            hulls += 1;
        }
    }
    let hull_ms = start.elapsed().as_secs_f64() * 1000.;
    let result = json!({"tool":"hyper-core","dataset":graph.meta.id,"native_parse_ms":native_ms,"hif_parse_validate_ms":hif_ms,"hif_viewer_conversion_ms":convert_ms,"bipartite_projection_ms":projection_ms,"layout_step_ms":steps,"all_hulls_cpu_ms":hull_ms,"hulls":hulls,"hull_vertex_cap":hyper_viz::MAX_HULL_VERTICES,"correctness":"passed document roundtrip, every converted string ID and member, node metadata and exact projected incidences","vertices":graph.vertices.len(),"hyperedges":graph.hyperedges.len(),"links":scene.links.len(),"layout_settings":"LayoutConfig::default,5warmup20steps,seeded common XYZ"});
    std::fs::write(&args[2], serde_json::to_vec_pretty(&result).unwrap()).unwrap();
}
