//! Weighted hypergraph attraction and sparse connectivity-based initialization.
//! The initializer is an approximate normalized-incidence diffusion embedding,
//! not a community classifier. Node labels and kinds never enter its coordinates.
use crate::{HypergraphScene, Vec3};
use serde::{Deserialize, Serialize};

/// Attraction law used by [`crate::ForceLayout3D`]; repulsion stays Barnes–Hut.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayoutModel {
    /// Existing spring/centroid forces controlled by [`crate::LayoutConfig`].
    #[default]
    Legacy,
    /// Linear attraction weighted by pair degree, set size, and memberships.
    Normalized,
    /// The same weights with logarithmic attraction at large distances.
    LinLog,
}
/// Weights and display multipliers for structural layout models.
///
/// Star scenes distinguish arity-two pairs from larger centroid sets. Scenes
/// with projected links (bipartite/clique) use those links as pairs instead.
/// Values are clamped by [`Self::normalize`] before structural stepping/seeding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TopologySettings {
    /// Legacy by default; structural models are opt-in for headless callers.
    pub model: LayoutModel,
    /// Pair coefficient in 0–10; default 0.1.
    pub pair_attraction: f32,
    /// Centroid-set coefficient in 0–10; default 1.
    pub set_attraction: f32,
    /// Degree/membership exponent in 0–1; default 1.
    pub hub_normalization: f32,
    /// Set weight divisor exponent for `(arity - 1)`, in 0–1; default 1.
    pub size_normalization: f32,
    /// Extra multiplier for `dependency_group` sets, in 0–1; default 0.1.
    pub derived_set_influence: f32,
    /// LinLog distance scale in 0.01–100,000; default 100.
    pub linlog_scale: f32,
    /// Pair opacity degree exponent in 0–1; default 0.5. Does not change forces.
    pub hub_fading: f32,
    /// Hull opacity arity exponent in 0–1; default 0.3. Does not change forces.
    pub size_fading: f32,
    /// Maximum displacement per structural step in 0.01–1,000; default 10.
    pub max_displacement: f32,
}
impl Default for TopologySettings {
    fn default() -> Self {
        Self {
            model: LayoutModel::Legacy,
            pair_attraction: 0.1,
            set_attraction: 1.,
            hub_normalization: 1.,
            size_normalization: 1.,
            derived_set_influence: 0.1,
            linlog_scale: 100.,
            hub_fading: 0.5,
            size_fading: 0.3,
            max_displacement: 10.,
        }
    }
}
impl TopologySettings {
    /// Clamp finite values to supported ranges; replace non-finite values with defaults.
    pub fn normalize(&mut self) {
        fn bound(v: &mut f32, lo: f32, hi: f32, fallback: f32) {
            *v = if v.is_finite() {
                v.clamp(lo, hi)
            } else {
                fallback
            };
        }
        let d = Self::default();
        bound(&mut self.pair_attraction, 0., 10., d.pair_attraction);
        bound(&mut self.set_attraction, 0., 10., d.set_attraction);
        bound(&mut self.hub_normalization, 0., 1., d.hub_normalization);
        bound(&mut self.size_normalization, 0., 1., d.size_normalization);
        bound(
            &mut self.derived_set_influence,
            0.,
            1.,
            d.derived_set_influence,
        );
        bound(&mut self.linlog_scale, 0.01, 100000., d.linlog_scale);
        bound(&mut self.hub_fading, 0., 1., d.hub_fading);
        bound(&mut self.size_fading, 0., 1., d.size_fading);
        bound(&mut self.max_displacement, 0.01, 1000., d.max_displacement);
    }
}
#[derive(Debug, Clone)]
struct Set {
    members: Vec<usize>,
    derived: bool,
}
#[derive(Default)]
pub(crate) struct TopologyForces {
    pairs: Vec<(usize, usize)>,
    sets: Vec<Set>,
    degrees: Vec<usize>,
    set_degrees: Vec<usize>,
    pair_weights: Vec<f32>,
    set_weights: Vec<f32>,
    key: Option<TopologySettings>,
}
impl TopologyForces {
    pub fn from_pairs(n: usize, pairs: Vec<(usize, usize)>) -> Self {
        let mut degrees = vec![0; n];
        let pairs: Vec<_> = pairs
            .into_iter()
            .filter(|&(a, b)| a < n && b < n && a != b)
            .collect();
        for &(a, b) in &pairs {
            degrees[a] += 1;
            degrees[b] += 1;
        }
        Self {
            pairs,
            degrees,
            set_degrees: vec![0; n],
            ..Default::default()
        }
    }
    pub fn from_scene(scene: &HypergraphScene) -> Self {
        if !scene.links.is_empty() {
            return Self::from_pairs(scene.node_count(), scene.dyadic_edges());
        }
        let n = scene.node_count();
        let mut result = Self::from_pairs(n, Vec::new());
        for e in &scene.hyperedges {
            let mut members: Vec<_> = e
                .member_indices
                .iter()
                .copied()
                .filter(|&i| i < n)
                .collect();
            members.sort_unstable();
            members.dedup();
            if members.len() == 2 {
                result.pairs.push((members[0], members[1]));
            } else if members.len() > 2 {
                for &i in &members {
                    result.set_degrees[i] += 1;
                }
                result.sets.push(Set {
                    members,
                    derived: e.kind == "dependency_group",
                });
            }
        }
        for &(a, b) in &result.pairs {
            result.degrees[a] += 1;
            result.degrees[b] += 1;
        }
        result
    }
    fn weights(&mut self, settings: &TopologySettings) {
        if self.key.as_ref() == Some(settings) {
            return;
        }
        self.pair_weights = self
            .pairs
            .iter()
            .map(|&(a, b)| {
                settings.pair_attraction
                    / (self.degrees[a].max(self.degrees[b]).max(1) as f32)
                        .powf(settings.hub_normalization)
            })
            .collect();
        self.set_weights = self
            .sets
            .iter()
            .map(|set| {
                let memberships = set
                    .members
                    .iter()
                    .map(|&i| self.set_degrees[i])
                    .max()
                    .unwrap_or(1)
                    .max(1);
                settings.set_attraction
                    * if set.derived {
                        settings.derived_set_influence
                    } else {
                        1.
                    }
                    / ((set.members.len() - 1) as f32).powf(settings.size_normalization)
                    / (memberships as f32).powf(settings.hub_normalization)
            })
            .collect();
        self.key = Some(settings.clone());
    }
    fn attraction(delta: Vec3, settings: &TopologySettings) -> Vec3 {
        if settings.model != LayoutModel::LinLog {
            return delta;
        }
        let r = delta.length();
        if r < 0.000001 {
            return delta;
        }
        delta * (settings.linlog_scale * (r / settings.linlog_scale).ln_1p() / r)
    }
    pub fn add_forces(
        &mut self,
        positions: &[Vec3],
        forces: &mut [Vec3],
        settings: &TopologySettings,
    ) {
        self.weights(settings);
        for (&(a, b), &w) in self.pairs.iter().zip(&self.pair_weights) {
            let f = Self::attraction(positions[b] - positions[a], settings) * w;
            forces[a] += f;
            forces[b] -= f;
        }
        for (set, &w) in self.sets.iter().zip(&self.set_weights) {
            if w == 0. {
                continue;
            }
            let center =
                set.members.iter().map(|&i| positions[i]).sum::<Vec3>() / set.members.len() as f32;
            for &i in &set.members {
                forces[i] += Self::attraction(center - positions[i], settings) * w;
            }
        }
    }
    pub fn pair_opacity(&self, a: usize, b: usize, exponent: f32) -> f32 {
        1. / (self
            .degrees
            .get(a)
            .copied()
            .unwrap_or(1)
            .max(self.degrees.get(b).copied().unwrap_or(1))
            .max(1) as f32)
            .powf(exponent)
    }
    pub fn spectral_positions(
        &mut self,
        ids: &[String],
        settings: &TopologySettings,
        iterations: usize,
    ) -> Vec<Vec3> {
        self.weights(settings);
        let n = ids.len();
        if n == 0 {
            return Vec::new();
        }
        let mut degree = vec![0f64; n];
        let mut parent: Vec<_> = (0..n).collect();
        fn root(p: &mut [usize], mut i: usize) -> usize {
            while p[i] != i {
                p[i] = p[p[i]];
                i = p[i];
            }
            i
        }
        for (&(a, b), &w) in self.pairs.iter().zip(&self.pair_weights) {
            if w <= 0. {
                continue;
            }
            degree[a] += 2. * w as f64;
            degree[b] += 2. * w as f64;
            let ra = root(&mut parent, a);
            let rb = root(&mut parent, b);
            parent[rb] = ra;
        }
        for (set, &w) in self.sets.iter().zip(&self.set_weights) {
            if w <= 0. {
                continue;
            }
            let ra = root(&mut parent, set.members[0]);
            for &i in &set.members {
                degree[i] += w as f64;
                let rb = root(&mut parent, i);
                parent[rb] = ra;
            }
        }
        let mut groups = std::collections::BTreeMap::<usize, Vec<usize>>::new();
        for i in 0..n {
            let r = root(&mut parent, i);
            groups.entry(r).or_default().push(i);
        }
        let mut components: Vec<_> = groups.into_values().collect();
        for c in &mut components {
            c.sort_by(|&a, &b| ids[a].cmp(&ids[b]));
        }
        components.sort_by(|a, b| {
            b.len()
                .cmp(&a.len())
                .then_with(|| ids[a[0]].cmp(&ids[b[0]]))
        });
        let inv: Vec<_> = degree
            .iter()
            .map(|d| if *d > 0. { 1. / d.sqrt() } else { 0. })
            .collect();
        let project = |x: &mut [f64]| {
            for c in &components {
                let denom: f64 = c.iter().map(|&i| degree[i]).sum();
                if denom == 0. {
                    continue;
                }
                let dot: f64 = c.iter().map(|&i| x[i] * degree[i].sqrt()).sum();
                for &i in c {
                    x[i] -= dot / denom * degree[i].sqrt();
                }
            }
        };
        let product = |x: &[f64], y: &mut [f64]| {
            y.fill(0.);
            for (&(a, b), &w) in self.pairs.iter().zip(&self.pair_weights) {
                let sum = (x[a] * inv[a] + x[b] * inv[b]) * w as f64;
                y[a] += sum * inv[a];
                y[b] += sum * inv[b];
            }
            for (set, &w) in self.sets.iter().zip(&self.set_weights) {
                let sum: f64 = set.members.iter().map(|&i| x[i] * inv[i]).sum();
                let mean = sum * w as f64 / set.members.len() as f64;
                for &i in &set.members {
                    y[i] += mean * inv[i];
                }
            }
        };
        let mut vectors: Vec<Vec<f64>> = (0..3)
            .map(|axis| ids.iter().map(|id| neutral(id, axis) as f64).collect())
            .collect();
        let orthogonalize = |vectors: &mut [Vec<f64>]| {
            for k in 0..vectors.len() {
                project(&mut vectors[k]);
                for j in 0..k {
                    let dot: f64 = vectors[k].iter().zip(&vectors[j]).map(|(a, b)| a * b).sum();
                    let (previous, current) = vectors.split_at_mut(k);
                    for (v, basis) in current[0].iter_mut().zip(&previous[j]) {
                        *v -= dot * basis;
                    }
                }
                let norm = vectors[k].iter().map(|v| v * v).sum::<f64>().sqrt();
                if norm > 1e-15 {
                    for v in &mut vectors[k] {
                        *v /= norm;
                    }
                } else {
                    vectors[k].fill(0.);
                }
            }
        };
        orthogonalize(&mut vectors);
        let mut scratch = vec![0.; n];
        for _ in 0..iterations.clamp(8, 512) {
            for v in &mut vectors {
                product(v, &mut scratch);
                std::mem::swap(v, &mut scratch);
            }
            orthogonalize(&mut vectors);
        }
        let mut eigenvalues = vec![0.; 3];
        for (axis, v) in vectors.iter().enumerate() {
            product(v, &mut scratch);
            eigenvalues[axis] = v
                .iter()
                .zip(&scratch)
                .map(|(a, b)| a * b)
                .sum::<f64>()
                .clamp(0., 1.)
                .powi(64);
        }
        // Global diffusion amplitude cancels during spatial scaling. Normalize
        // before converting to f32 so strongly contracting spectra retain shape.
        let largest = eigenvalues.iter().copied().fold(0.0_f64, f64::max);
        if largest > 0. {
            for value in &mut eigenvalues {
                *value /= largest;
            }
        }
        let mut output = vec![Vec3::ZERO; n];
        for (ci, c) in components.iter().enumerate() {
            let target = 600.
                * (c.len() as f32 / components[0].len() as f32)
                    .cbrt()
                    .max(0.05);
            let center = if ci == 0 {
                Vec3::ZERO
            } else {
                let angle = ci as f32 * 2.399963;
                Vec3::new(
                    angle.cos(),
                    angle.sin(),
                    ((ci as f32 * 0.618034).fract() - 0.5) * 1.4,
                ) * (1800. + ci as f32 * 30.)
            };
            // Robust scaling gives the dense core room even when a slow
            // spectral mode localizes on a small dangling branch. Each axis
            // uses its median and median absolute deviation, rather
            // than a global RMS dominated by extreme vertices.
            let mut centers = [0.; 3];
            let mut scales = [0.; 3];
            for axis in 0..3 {
                let mut values: Vec<_> = c
                    .iter()
                    .map(|&i| vectors[axis][i] * eigenvalues[axis])
                    .collect();
                values.sort_by(f64::total_cmp);
                centers[axis] = values[values.len() / 2];
                let mut deviations: Vec<_> =
                    values.iter().map(|v| (*v - centers[axis]).abs()).collect();
                deviations.sort_by(f64::total_cmp);
                scales[axis] = deviations[(deviations.len() - 1) / 2];
            }
            for &i in c {
                let coordinate = |axis: usize| {
                    if scales[axis] > 1e-30 {
                        ((vectors[axis][i] * eigenvalues[axis] - centers[axis]) / scales[axis])
                            as f32
                            * target
                    } else {
                        neutral(&ids[i], axis) * target
                    }
                };
                output[i] = Vec3::new(coordinate(0), coordinate(1), coordinate(2));
                // Spectral modes can localize on a short dangling branch. A
                // smooth radial compression keeps those vertices in the view;
                // neutral fine-scale jitter prevents coincident memberships.
                let radius = output[i].length();
                output[i] = output[i] * (3. * target / (3. * target + radius));
                output[i] += Vec3::new(
                    neutral(&ids[i], 0),
                    neutral(&ids[i], 1),
                    neutral(&ids[i], 2),
                ) * (target * 0.05);
                output[i] += center;
            }
        }
        output
    }
}
fn neutral(id: &str, axis: usize) -> f32 {
    let h = id.bytes().fold(0xcbf29ce484222325_u64, |h, b| {
        (h ^ b as u64).wrapping_mul(0x100000001b3)
    });
    (((h >> (axis * 16)) & 0xffff) as f32 / 32767.5) - 1.
}

#[cfg(test)]
mod tests {
    use crate::{
        ForceLayout3D, Hypergraph, LayoutConfig, LayoutModel, Projection, TopologySettings, Vec3,
    };

    fn planted() -> crate::HypergraphScene {
        let mut g = Hypergraph::new();
        for i in 0..17 {
            g = g.vertex(format!("v{i}"), format!("v{i}"), "same-label");
        }
        for (name, ids) in [
            ("left", (0..8).collect::<Vec<_>>()),
            ("right", (8..16).collect()),
        ] {
            g = g.hyperedge(name, ids.iter().map(|i| format!("v{i}")), name);
        }
        g = g.hyperedge("bridge", ["v7", "v8"], "Bridge");
        for i in 0..16 {
            g = g.hyperedge(
                format!("spoke{i}"),
                ["v16".to_string(), format!("v{i}")],
                "Spoke",
            );
        }
        g.hyperedge("umbrella", (0..17).map(|i| format!("v{i}")), "Umbrella")
            .project(Projection::StarCentroid)
    }
    fn structural() -> LayoutConfig {
        let mut config = LayoutConfig::default();
        config.topology.model = LayoutModel::LinLog;
        config
    }
    #[test]
    fn structural_positions_reveal_blocks_despite_a_universal_hub() {
        let scene = planted();
        let mut layout = ForceLayout3D::from_scene(&scene, structural());
        layout.seed_from_topology(&scene, 256);
        let distance = |a: usize, b: usize| (layout.positions[a] - layout.positions[b]).length();
        let mut within = 0.;
        let mut cross = 0.;
        for a in 0..8 {
            for b in 8..16 {
                cross += distance(a, b);
            }
        }
        for a in 0..8 {
            for b in 0..8 {
                within += distance(a, b) + distance(a + 8, b + 8);
            }
        }
        assert!(
            within / 128. < cross / 64. * 0.45,
            "Within-block distances must be clearly smaller: within={} cross={}",
            within / 128.,
            cross / 64.
        );
        assert_eq!(layout.positions.len(), 17);
        assert_eq!(layout.iterations, 0);
        let original = layout.positions.clone();
        let mut relabeled = scene.clone();
        for n in &mut relabeled.nodes {
            n.kind = "changed".into();
            n.label = "changed".into();
        }
        layout.seed_from_topology(&relabeled, 256);
        assert_eq!(
            original, layout.positions,
            "Subject labels cannot create structural clusters"
        );
    }
    #[test]
    fn structural_positions_keep_isolates_and_disconnected_nodes_finite() {
        let scene = Hypergraph::new()
            .vertex("a", "a", "x")
            .vertex("b", "b", "x")
            .vertex("c", "c", "x")
            .vertex("d", "d", "x")
            .vertex("isolated", "isolated", "x")
            .hyperedge("ab", ["a", "b"], "ab")
            .hyperedge("cd", ["c", "d"], "cd")
            .project(Projection::StarCentroid);
        let mut layout = ForceLayout3D::from_scene(&scene, structural());
        layout.seed_from_topology(&scene, 64);
        assert_eq!(layout.positions.len(), 5);
        assert!(
            layout
                .positions
                .iter()
                .all(|p| p.x.is_finite() && p.y.is_finite() && p.z.is_finite())
        );
        assert!(layout.positions[0] != layout.positions[1]);
        assert!((layout.positions[0] - layout.positions[2]).length() > 100.);
        let empty = Hypergraph::new().project(Projection::StarCentroid);
        let mut layout = ForceLayout3D::from_scene(&empty, structural());
        layout.seed_from_topology(&empty, 64);
        assert!(layout.positions.is_empty());
    }
    #[test]
    fn structural_positions_follow_ids_after_scene_reordering() {
        let scene = planted();
        let mut layout = ForceLayout3D::from_scene(&scene, structural());
        layout.seed_from_topology(&scene, 128);
        let original = layout.positions.clone();
        let mut reordered = scene.clone();
        reordered.nodes.reverse();
        let n = reordered.nodes.len();
        for e in &mut reordered.hyperedges {
            for i in &mut e.member_indices {
                *i = n - 1 - *i;
            }
        }
        let mut moved = ForceLayout3D::from_scene(&reordered, structural());
        moved.seed_from_topology(&reordered, 128);
        for (i, p) in original.iter().enumerate() {
            assert!((*p - moved.positions[n - 1 - i]).length() < 0.001);
        }
    }
    #[test]
    fn a_sparse_tail_does_not_compress_the_connected_core() {
        let mut g = Hypergraph::new();
        for i in 0..160 {
            g = g.vertex(format!("n{i}"), format!("n{i}"), "same");
        }
        for block in 0..8 {
            g = g.hyperedge(
                format!("block{block}"),
                (block * 16..(block + 1) * 16).map(|i| format!("n{i}")),
                "block",
            );
        }
        for a in 0..8 {
            for b in a + 1..8 {
                g = g.hyperedge(
                    format!("bridge{a}-{b}"),
                    [format!("n{}", a * 16), format!("n{}", b * 16)],
                    "bridge",
                );
            }
        }
        for i in 128..160 {
            g = g.hyperedge(
                format!("tail{i}"),
                [
                    format!("n{}", if i == 128 { 0 } else { i - 1 }),
                    format!("n{i}"),
                ],
                "tail",
            );
        }
        let scene = g.project(Projection::StarCentroid);
        let mut layout = ForceLayout3D::from_scene(&scene, structural());
        layout.seed_from_topology(&scene, 256);
        let center = layout.positions[..128].iter().copied().sum::<Vec3>() / 128.;
        let mut radii: Vec<_> = layout.positions[..128]
            .iter()
            .map(|p| (*p - center).length())
            .collect();
        radii.sort_by(f32::total_cmp);
        assert!(
            radii[64] > 150.,
            "Dense core compressed to median radius {}",
            radii[64]
        );
    }
    #[test]
    fn derived_set_influence_changes_dynamics_without_removing_the_set() {
        let mut g = Hypergraph::new()
            .vertex("a", "a", "x")
            .vertex("b", "b", "x")
            .vertex("c", "c", "x");
        g.add_hyperedge(
            crate::Hyperedge::new("set", ["a", "b", "c"]).with_kind("dependency_group"),
        );
        let scene = g.project(Projection::StarCentroid);
        let mut config = structural();
        config.topology.model = LayoutModel::Normalized;
        config.topology.set_attraction = 0.1;
        config.repulsion = 0.;
        config.gravity = 0.;
        config.dt = 1.;
        config.damping = 1.;
        let mut layout = ForceLayout3D::from_scene(&scene, config);
        layout.positions = vec![Vec3::new(-3., 0., 0.), Vec3::ZERO, Vec3::new(3., 0., 0.)];
        layout.step();
        assert!((layout.positions[0].x + 2.985).abs() < 0.00001);
        layout.config.topology.derived_set_influence = 0.;
        layout.velocities.fill(Vec3::ZERO);
        let before = layout.positions.clone();
        layout.step();
        assert_eq!(layout.positions, before);
        assert_eq!(scene.hyperedge_count(), 1);
    }
    #[test]
    fn normalized_pair_force_is_symmetric_and_weight_edits_take_effect() {
        let scene = Hypergraph::new()
            .vertex("a", "a", "x")
            .vertex("b", "b", "x")
            .hyperedge("ab", ["a", "b"], "ab")
            .project(Projection::StarCentroid);
        let config = LayoutConfig {
            repulsion: 0.,
            gravity: 0.,
            dt: 1.,
            damping: 1.,
            topology: TopologySettings {
                model: LayoutModel::Normalized,
                pair_attraction: 0.1,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut layout = ForceLayout3D::from_scene(&scene, config);
        layout.positions = vec![Vec3::new(-10., 0., 0.), Vec3::new(10., 0., 0.)];
        layout.step();
        assert!((layout.positions[0].x + 8.).abs() < 0.00001);
        assert!((layout.positions[1].x - 8.).abs() < 0.00001);
        layout.config.topology.pair_attraction = 0.;
        layout.velocities.fill(Vec3::ZERO);
        let before = layout.positions.clone();
        layout.step();
        assert_eq!(layout.positions, before);
    }
    #[test]
    fn linlog_pair_force_has_logarithmic_growth_and_equal_opposite_motion() {
        let scene = Hypergraph::new()
            .vertex("a", "a", "x")
            .vertex("b", "b", "x")
            .hyperedge("ab", ["a", "b"], "ab")
            .project(Projection::StarCentroid);
        let config = LayoutConfig {
            repulsion: 0.,
            gravity: 0.,
            dt: 1.,
            damping: 1.,
            topology: TopologySettings {
                model: LayoutModel::LinLog,
                linlog_scale: 10.,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut layout = ForceLayout3D::from_scene(&scene, config);
        layout.positions = vec![Vec3::new(-10., 0., 0.), Vec3::new(10., 0., 0.)];
        layout.step();
        // F = 0.1 * 10 * ln(1 + 20/10) = ln(3).
        assert!((layout.positions[0].x - (-10. + 3_f32.ln())).abs() < 0.00001);
        assert_eq!(layout.positions[0] + layout.positions[1], Vec3::ZERO);
    }
    #[test]
    fn crowded_initialization_cannot_launch_nodes_out_of_the_view() {
        let scene = Hypergraph::new()
            .vertex("a", "a", "x")
            .vertex("b", "b", "x")
            .hyperedge("ab", ["a", "b"], "ab")
            .project(Projection::StarCentroid);
        let mut config = structural();
        config.gravity = 0.;
        let mut layout = ForceLayout3D::from_scene(&scene, config);
        layout.positions = vec![Vec3::ZERO, Vec3::new(0.01, 0., 0.)];
        let before = layout.positions.clone();
        layout.step();
        for (a, b) in layout.positions.iter().zip(&before) {
            assert!((*a - *b).length() <= 10.0001);
        }
    }
    #[test]
    fn hub_normalization_caps_total_pair_pull_without_removing_pairs() {
        let mut g = Hypergraph::new().vertex("hub", "hub", "x");
        for i in 0..100 {
            g = g.vertex(format!("n{i}"), "n", "x").hyperedge(
                format!("e{i}"),
                ["hub".to_owned(), format!("n{i}")],
                "e",
            );
        }
        let scene = g.project(Projection::StarCentroid);
        let mut config = structural();
        config.topology.model = LayoutModel::Normalized;
        config.topology.pair_attraction = 0.1;
        config.repulsion = 0.;
        config.gravity = 0.;
        config.dt = 1.;
        config.damping = 1.;
        let mut layout = ForceLayout3D::from_scene(&scene, config);
        layout.positions.fill(Vec3::new(10., 0., 0.));
        layout.positions[0] = Vec3::ZERO;
        layout.step();
        assert!((layout.positions[0].x - 1.).abs() < 0.0001);
        assert_eq!(layout.positions.len(), 101);
    }
}
