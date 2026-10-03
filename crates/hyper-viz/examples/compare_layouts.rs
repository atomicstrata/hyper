//! Compare spatial models with neutral seeds; no nodes or hyperedges are removed.
//! cargo run --release -p hyper-viz --example compare_layouts -- GRAPH OUTPUT_DIR [STEPS]
use hyper_viz::{ForceLayout3D, LayoutConfig, LayoutModel, Projection, Vec3, load_json};
use std::{fs, io::Write, time::Instant};
fn neutral(id: &str) -> Vec3 {
    let h = id.bytes().fold(0xcbf29ce484222325_u64, |h, b| {
        (h ^ b as u64).wrapping_mul(0x100000001b3)
    });
    let c = |shift| (((h >> shift) & 0xffff_u64) as f32 / 32767.5) - 1.;
    Vec3::new(c(0), c(16), c(32)) * 500.
}
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert!(
        (2..=3).contains(&args.len()),
        "usage: compare_layouts GRAPH OUTPUT_DIR [STEPS]"
    );
    fs::create_dir_all(&args[1]).unwrap();
    let steps = args
        .get(2)
        .map(|n| n.parse::<usize>().unwrap())
        .unwrap_or(800);
    assert!(steps <= 10000, "steps must be <= 10000");
    let scene = load_json(&args[0])
        .unwrap()
        .project(Projection::StarCentroid);
    for (name, model, spectral) in [
        ("legacy-neutral", LayoutModel::Legacy, false),
        ("normalized-neutral", LayoutModel::Normalized, false),
        ("linlog-neutral", LayoutModel::LinLog, false),
        ("linlog-structural", LayoutModel::LinLog, true),
        ("normalized-structural", LayoutModel::Normalized, true),
    ] {
        let mut config = LayoutConfig {
            gravity: if model == LayoutModel::Legacy {
                0.001
            } else {
                0.00001
            },
            centroid_attraction: 0.0005,
            ..Default::default()
        };
        config.topology.model = model;
        let mut layout = ForceLayout3D::from_scene(&scene, config);
        let start = Instant::now();
        if spectral {
            layout.seed_from_topology(&scene, 256);
        } else {
            layout.positions = scene.nodes.iter().map(|n| neutral(&n.id)).collect();
        }
        let init_ms = start.elapsed().as_secs_f64() * 1000.;
        for _ in 0..steps {
            layout.step();
        }
        let mut f = fs::File::create(format!("{}/{name}.csv", args[1])).unwrap();
        writeln!(f, "id,kind,x,y,z").unwrap();
        for (n, p) in scene.nodes.iter().zip(&layout.positions) {
            writeln!(f, "{},{},{},{},{}", n.id, n.kind, p.x, p.y, p.z).unwrap();
        }
        let mut import_distance = 0.;
        let mut imports = 0.;
        for e in &scene.hyperedges {
            if e.kind == "import"
                && e.member_indices.len() == 2
                && e.member_indices
                    .iter()
                    .all(|&i| scene.nodes[i].id.starts_with("Mathlib."))
            {
                import_distance += (layout.positions[e.member_indices[0]]
                    - layout.positions[e.member_indices[1]])
                    .length() as f64;
                imports += 1.;
            }
        }
        let ids: Vec<_> = scene
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.id.starts_with("Mathlib."))
            .map(|(i, _)| i)
            .collect();
        let mut same = 0.;
        let mut diff = 0.;
        let mut sn = 0.;
        let mut dn = 0.;
        let mut random = 0.;
        let mut random_count = 0.;
        for k in 0..10000 {
            if ids.is_empty() {
                break;
            }
            let a = ids[k * 31 % ids.len()];
            let b = ids[(k * 7919 + 101) % ids.len()];
            if a == b {
                continue;
            }
            let d = (layout.positions[a] - layout.positions[b]).length() as f64;
            random += d;
            random_count += 1.;
            if scene.nodes[a].kind == scene.nodes[b].kind {
                same += d;
                sn += 1.;
            } else {
                diff += d;
                dn += 1.;
            }
        }
        println!(
            "{name}: vertices={} hyperedges={} init_ms={init_ms:.2} total_ms={:.2} steps={steps} internal_mathlib_import/random={:.4} same_subject/different_subject={:.4}",
            scene.nodes.len(),
            scene.hyperedges.len(),
            start.elapsed().as_secs_f64() * 1000.,
            (import_distance / imports) / (random / random_count),
            (same / sn) / (diff / dn)
        );
    }
}
