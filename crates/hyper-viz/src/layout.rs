//! CPU 3D force-directed layout with Barnes-Hut octree approximation.
//!
//! Ported from vibe-graph-bevy; Bevy-free so layout stays headless and testable.

use crate::scene::{HypergraphScene, LinkKind};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn splat(v: f32) -> Self {
        Self::new(v, v, v)
    }

    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    pub fn length_squared(self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    pub fn normalize(self) -> Self {
        let len = self.length().max(f32::EPSILON);
        Self::new(self.x / len, self.y / len, self.z / len)
    }

    pub fn normalize_or_zero(self) -> Self {
        if self.length_squared() < f32::EPSILON {
            Self::ZERO
        } else {
            self.normalize()
        }
    }
}

impl std::ops::SubAssign for Vec3 {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
        self.z -= rhs.z;
    }
}

impl std::ops::Div<f32> for Vec3 {
    type Output = Self;
    fn div(self, rhs: f32) -> Self {
        Self::new(self.x / rhs, self.y / rhs, self.z / rhs)
    }
}

impl std::ops::Add for Vec3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl std::ops::AddAssign for Vec3 {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl std::ops::Sub for Vec3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl std::ops::Neg for Vec3 {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z)
    }
}

impl std::ops::Mul<f32> for Vec3 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl std::iter::Sum for Vec3 {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, |acc, v| acc + v)
    }
}

#[derive(Debug, Clone)]
pub struct LayoutConfig {
    pub dt: f32,
    pub damping: f32,
    pub repulsion: f32,
    pub attraction: f32,
    /// Barnes-Hut opening angle. Lower = more accurate, higher = faster.
    pub theta: f32,
    pub gravity: f32,
    pub ideal_length: f32,
    pub max_tree_depth: usize,
    /// Pull member vertices toward hyperedge centroid (StarCentroid scenes).
    pub centroid_attraction: f32,
    pub topology: crate::TopologySettings,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            dt: 0.3,
            damping: 0.85,
            repulsion: 500.0,
            attraction: 0.005,
            theta: 0.8,
            gravity: 0.02,
            ideal_length: 30.0,
            max_tree_depth: 14,
            centroid_attraction: 0.008,
            topology: Default::default(),
        }
    }
}

pub struct ForceLayout3D {
    pub positions: Vec<Vec3>,
    pub velocities: Vec<Vec3>,
    pub edges: Vec<(usize, usize)>,
    pub centroid_groups: Vec<Vec<usize>>,
    pub config: LayoutConfig,
    pub iterations: u64,
    topology_forces: crate::topology::TopologyForces,
}

impl ForceLayout3D {
    pub fn new(n: usize, edges: Vec<(usize, usize)>, config: LayoutConfig) -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let spread = (n as f32).sqrt() * 5.0;

        let positions: Vec<Vec3> = (0..n)
            .map(|_| {
                Vec3::new(
                    rng.gen_range(-spread..spread),
                    rng.gen_range(-spread..spread),
                    rng.gen_range(-spread..spread),
                )
            })
            .collect();

        let topology_forces = crate::topology::TopologyForces::from_pairs(n, edges.clone());
        Self {
            positions,
            velocities: vec![Vec3::ZERO; n],
            edges,
            centroid_groups: Vec::new(),
            config,
            iterations: 0,
            topology_forces,
        }
    }

    pub fn from_scene(scene: &HypergraphScene, config: LayoutConfig) -> Self {
        let n = scene.node_count();
        let edges = scene.dyadic_edges();
        let mut layout = Self::new(n, edges, config);

        if scene.links.is_empty() && !scene.hyperedges.is_empty() {
            layout.centroid_groups = scene
                .hyperedges
                .iter()
                .map(|he| he.member_indices.clone())
                .filter(|members| members.len() >= 2)
                .collect();
        }

        layout.topology_forces = crate::topology::TopologyForces::from_scene(scene);
        layout
    }

    pub fn seed_from_topology(&mut self, scene: &HypergraphScene, iterations: usize) {
        self.config.topology.normalize();
        self.topology_forces = crate::topology::TopologyForces::from_scene(scene);
        let ids: Vec<_> = scene.nodes.iter().map(|n| n.id.clone()).collect();
        self.positions =
            self.topology_forces
                .spectral_positions(&ids, &self.config.topology, iterations);
        self.velocities.fill(Vec3::ZERO);
    }

    pub fn pair_opacity(&self, a: usize, b: usize) -> f32 {
        if self.config.topology.model == crate::LayoutModel::Legacy {
            return 1.;
        }
        self.topology_forces
            .pair_opacity(a, b, self.config.topology.hub_fading)
    }

    pub fn set_opacity(&self, arity: usize) -> f32 {
        if self.config.topology.model == crate::LayoutModel::Legacy {
            return 1.;
        }
        1. / (arity.max(1) as f32).powf(self.config.topology.size_fading)
    }

    /// Rebuild layout for a new scene, pinning positions for nodes whose ids appear in `seeds`.
    pub fn from_scene_with_seeds(
        scene: &HypergraphScene,
        config: LayoutConfig,
        seeds: &std::collections::HashMap<String, Vec3>,
    ) -> Self {
        let mut layout = Self::from_scene(scene, config);
        for (i, node) in scene.nodes.iter().enumerate() {
            if let Some(pos) = seeds.get(&node.id) {
                layout.positions[i] = *pos;
                layout.velocities[i] = Vec3::ZERO;
            }
        }
        layout
    }

    pub fn step(&mut self) {
        let n = self.positions.len();
        if n == 0 {
            return;
        }

        let mut forces = vec![Vec3::ZERO; n];

        let tree = OctTree::build(&self.positions, self.config.max_tree_depth);
        for (i, force) in forces.iter_mut().enumerate().take(n) {
            *force +=
                tree.compute_repulsion(self.positions[i], self.config.repulsion, self.config.theta);
        }

        self.config.topology.normalize();
        if self.config.topology.model == crate::LayoutModel::Legacy {
            for &(src, tgt) in &self.edges {
                let delta = self.positions[tgt] - self.positions[src];
                let dist = delta.length().max(0.01);
                let displacement = dist - self.config.ideal_length;
                let force = delta.normalize() * displacement * self.config.attraction;
                forces[src] += force;
                forces[tgt] -= force;
            }

            if self.config.centroid_attraction > 0.0 {
                for group in &self.centroid_groups {
                    if group.len() < 2 {
                        continue;
                    }
                    let centroid =
                        group.iter().map(|&i| self.positions[i]).sum::<Vec3>() / group.len() as f32;
                    for &idx in group {
                        let delta = centroid - self.positions[idx];
                        forces[idx] += delta * self.config.centroid_attraction;
                    }
                }
            }
        } else {
            self.topology_forces
                .add_forces(&self.positions, &mut forces, &self.config.topology);
        }

        for (i, force) in forces.iter_mut().enumerate().take(n) {
            *force -= self.positions[i] * self.config.gravity;
        }

        let dt = self.config.dt;
        let damping = self.config.damping;
        for (i, force) in forces.iter().enumerate().take(n) {
            self.velocities[i] = (self.velocities[i] + *force * dt) * damping;
            if self.config.topology.model != crate::LayoutModel::Legacy {
                let distance = self.velocities[i].length() * dt.abs();
                if distance > self.config.topology.max_displacement {
                    self.velocities[i] =
                        self.velocities[i] * (self.config.topology.max_displacement / distance);
                }
            }
            self.positions[i] += self.velocities[i] * dt;
        }

        self.iterations += 1;
    }
}

/// Incidence links for bipartite scenes; clique pairs otherwise.
pub fn layout_edges_from_scene(scene: &HypergraphScene) -> Vec<(usize, usize)> {
    scene
        .links
        .iter()
        .filter(|link| matches!(link.kind, LinkKind::Incidence | LinkKind::CliquePair))
        .map(|link| (link.source, link.target))
        .collect()
}

// ---- Barnes-Hut Octree ----

#[derive(Default)]
struct OctNode {
    center_of_mass: Vec3,
    mass: f32,
    width: f32,
    children: [i32; 8],
}

struct OctTree {
    nodes: Vec<OctNode>,
}

impl OctTree {
    fn build(positions: &[Vec3], max_depth: usize) -> Self {
        if positions.is_empty() {
            return Self {
                nodes: vec![OctNode::default()],
            };
        }

        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);
        for p in positions {
            min = Vec3::new(min.x.min(p.x), min.y.min(p.y), min.z.min(p.z));
            max = Vec3::new(max.x.max(p.x), max.y.max(p.y), max.z.max(p.z));
        }

        let padding = (max.x - min.x).max(max.y - min.y).max(max.z - min.z) * 0.1 + 1.0;
        min -= Vec3::splat(padding);
        max += Vec3::splat(padding);

        let width = (max.x - min.x).max(max.y - min.y).max(max.z - min.z);
        let center = (min + max) * 0.5;
        let origin = center - Vec3::splat(width * 0.5);

        let mut nodes = Vec::with_capacity(positions.len() * 2);
        let indices: Vec<usize> = (0..positions.len()).collect();

        let mut builder = TreeBuilder {
            positions,
            nodes: &mut nodes,
            max_depth,
        };
        builder.build_node(&indices, origin, width, 0);

        Self { nodes }
    }

    fn compute_repulsion(&self, pos: Vec3, strength: f32, theta: f32) -> Vec3 {
        if self.nodes.is_empty() {
            return Vec3::ZERO;
        }
        self.repulse_recursive(0, pos, strength, theta)
    }

    fn repulse_recursive(&self, idx: usize, pos: Vec3, strength: f32, theta: f32) -> Vec3 {
        let node = &self.nodes[idx];
        if node.mass == 0.0 {
            return Vec3::ZERO;
        }

        let delta = pos - node.center_of_mass;
        let dist_sq = delta.length_squared().max(0.01);
        let dist = dist_sq.sqrt();

        let is_leaf = node.children.iter().all(|&c| c < 0);
        if is_leaf || (node.width / dist) < theta {
            return delta.normalize_or_zero() * strength * node.mass / dist_sq;
        }

        let mut force = Vec3::ZERO;
        for &child_idx in &node.children {
            if child_idx >= 0 {
                force += self.repulse_recursive(child_idx as usize, pos, strength, theta);
            }
        }
        force
    }
}

struct TreeBuilder<'a> {
    positions: &'a [Vec3],
    nodes: &'a mut Vec<OctNode>,
    max_depth: usize,
}

impl<'a> TreeBuilder<'a> {
    fn build_node(&mut self, indices: &[usize], origin: Vec3, width: f32, depth: usize) -> i32 {
        if indices.is_empty() {
            return -1;
        }

        let node_idx = self.nodes.len() as i32;
        self.nodes.push(OctNode::default());

        let mass = indices.len() as f32;
        let com = indices.iter().map(|&i| self.positions[i]).sum::<Vec3>() / mass;

        if indices.len() == 1 || depth >= self.max_depth {
            self.nodes[node_idx as usize] = OctNode {
                center_of_mass: com,
                mass,
                width,
                children: [-1; 8],
            };
            return node_idx;
        }

        let half = width * 0.5;
        let mid = origin + Vec3::splat(half);

        let mut buckets: [Vec<usize>; 8] = Default::default();
        for &i in indices {
            let p = self.positions[i];
            let octant = ((p.x >= mid.x) as usize)
                | (((p.y >= mid.y) as usize) << 1)
                | (((p.z >= mid.z) as usize) << 2);
            buckets[octant].push(i);
        }

        let offsets = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(half, 0.0, 0.0),
            Vec3::new(0.0, half, 0.0),
            Vec3::new(half, half, 0.0),
            Vec3::new(0.0, 0.0, half),
            Vec3::new(half, 0.0, half),
            Vec3::new(0.0, half, half),
            Vec3::new(half, half, half),
        ];

        let mut children = [-1i32; 8];
        for (oct, bucket) in buckets.iter().enumerate() {
            children[oct] = self.build_node(bucket, origin + offsets[oct], half, depth + 1);
        }

        self.nodes[node_idx as usize] = OctNode {
            center_of_mass: com,
            mass,
            width,
            children,
        };

        node_idx
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{Projection, project};
    use crate::schema::Hypergraph;

    #[test]
    fn layout_converges_on_small_graph() {
        let edges = vec![(0, 1), (1, 2), (2, 3), (3, 0)];
        let mut layout = ForceLayout3D::new(4, edges, LayoutConfig::default());
        for _ in 0..200 {
            layout.step();
        }
        let energy: f32 = layout.velocities.iter().map(|v| v.length_squared()).sum();
        assert!(energy < 100.0, "layout should settle, energy={energy}");
    }

    #[test]
    fn from_scene_bipartite_has_incidence_edges() {
        let graph = Hypergraph::new()
            .with_id("c")
            .vertex("v:1", "a", "entity")
            .vertex("v:2", "b", "value")
            .vertex("v:3", "m", "episode")
            .vertex("v:4", "r", "predicate")
            .hyperedge("e:1", ["v:1", "v:2", "v:3", "v:4"], "a r b");
        let scene = project(&graph, Projection::Bipartite);
        let layout = ForceLayout3D::from_scene(&scene, LayoutConfig::default());
        assert_eq!(layout.edges.len(), 4);
        assert!(layout.centroid_groups.is_empty());
    }

    #[test]
    fn from_scene_with_seeds_pins_known_nodes() {
        use std::collections::HashMap;

        let graph = Hypergraph::new()
            .with_id("c")
            .vertex("v:1", "a", "entity")
            .vertex("v:2", "b", "entity")
            .hyperedge("e:1", ["v:1", "v:2"], "a r b");
        let scene = project(&graph, Projection::Bipartite);
        let seed = Vec3::new(42.0, -3.0, 7.0);
        let mut seeds = HashMap::new();
        seeds.insert(scene.nodes[0].id.clone(), seed);

        let layout = ForceLayout3D::from_scene_with_seeds(&scene, LayoutConfig::default(), &seeds);
        assert_eq!(layout.positions[0], seed);
    }
}
