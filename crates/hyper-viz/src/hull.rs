//! Convex hull mesh generation for hyperedge member sets (headless, Bevy-free).

const EPS: f32 = 1e-5;

/// Triangle mesh for a hyperedge hull: member positions + indexed faces.
#[derive(Debug, Clone, PartialEq)]
pub struct HullMesh {
    pub positions: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    /// Undirected edges for wireframe overlays.
    pub edges: Vec<(u32, u32)>,
}

/// Build a hull mesh from member vertex positions.
///
/// - 3 points → filled triangle
/// - 4 points → tetrahedron (or coplanar triangle fan)
/// - 5+ points → 3D convex hull via face enumeration
pub fn hull_from_points(points: &[[f32; 3]]) -> Option<HullMesh> {
    match points.len() {
        0..=2 => None,
        3 => Some(triangle_mesh(points)),
        4 => Some(tetrahedron_or_fan(points)),
        _ => convex_hull_mesh(points),
    }
}

fn triangle_mesh(points: &[[f32; 3]]) -> HullMesh {
    let positions = points.to_vec();
    let indices = oriented_triangle_indices(points, 0, 1, 2).to_vec();
    let edges = triangle_edges(&oriented_triangle_indices(points, 0, 1, 2));
    HullMesh {
        positions,
        indices,
        edges,
    }
}

fn tetrahedron_or_fan(points: &[[f32; 3]]) -> HullMesh {
    if let Some((i, j, k)) = first_non_collinear_triple(points) {
        let normal = face_normal(points[i], points[j], points[k]);
        let fourth = (0..4).find(|&m| m != i && m != j && m != k).unwrap_or(3);
        let dist = dot(sub(points[fourth], points[i]), normal).abs();
        if dist > EPS {
            return tetrahedron_mesh(points);
        }
    }
    coplanar_fan(points)
}

fn tetrahedron_mesh(points: &[[f32; 3]]) -> HullMesh {
    let center = centroid(points);
    let faces = [
        oriented_triangle_indices(points, 0, 1, 2),
        oriented_triangle_indices(points, 0, 1, 3),
        oriented_triangle_indices(points, 0, 2, 3),
        oriented_triangle_indices(points, 1, 2, 3),
    ];
    let mut indices = Vec::with_capacity(12);
    for face in faces {
        if outward_face(&face, points, center) {
            indices.extend_from_slice(&face);
        } else {
            indices.extend_from_slice(&[face[0], face[2], face[1]]);
        }
    }
    let edges = unique_edges(&indices);
    HullMesh {
        positions: points.to_vec(),
        indices,
        edges,
    }
}

fn coplanar_fan(points: &[[f32; 3]]) -> HullMesh {
    let hull_2d = convex_hull_2d_indices(points);
    if hull_2d.len() < 3 {
        return triangle_mesh(&points[..3.min(points.len())]);
    }
    let mut indices = Vec::new();
    let root = hull_2d[0] as usize;
    for i in 1..(hull_2d.len().saturating_sub(1)) {
        indices.extend_from_slice(&oriented_triangle_indices(
            points,
            root,
            hull_2d[i] as usize,
            hull_2d[i + 1] as usize,
        ));
    }
    let edges = unique_edges(&indices);
    HullMesh {
        positions: points.to_vec(),
        indices,
        edges,
    }
}

fn convex_hull_mesh(points: &[[f32; 3]]) -> Option<HullMesh> {
    if are_coplanar(points) {
        return Some(coplanar_fan(points));
    }

    let n = points.len();
    let center = centroid(points);
    let mut indices = Vec::new();

    for i in 0..n {
        for j in (i + 1)..n {
            for k in (j + 1)..n {
                let normal = face_normal(points[i], points[j], points[k]);
                if length_squared(normal) < EPS * EPS {
                    continue;
                }
                let mut positive = 0usize;
                let mut negative = 0usize;
                for (m, point) in points.iter().enumerate() {
                    if m == i || m == j || m == k {
                        continue;
                    }
                    let side = dot(sub(*point, points[i]), normal);
                    if side > EPS {
                        positive += 1;
                    } else if side < -EPS {
                        negative += 1;
                    }
                }
                if positive == 0 || negative == 0 {
                    let face = oriented_triangle_indices(points, i, j, k);
                    if outward_face(&face, points, center) {
                        indices.extend_from_slice(&face);
                    } else {
                        indices.extend_from_slice(&[face[0], face[2], face[1]]);
                    }
                }
            }
        }
    }

    if indices.is_empty() {
        return Some(coplanar_fan(points));
    }

    let edges = unique_edges(&indices);
    Some(HullMesh {
        positions: points.to_vec(),
        indices,
        edges,
    })
}

fn convex_hull_2d_indices(points: &[[f32; 3]]) -> Vec<u32> {
    let (origin, u, v) = projection_basis(points);
    let projected: Vec<(u32, f32, f32)> = points
        .iter()
        .enumerate()
        .map(|(idx, p)| {
            let d = sub(*p, origin);
            (idx as u32, dot(d, u), dot(d, v))
        })
        .collect();

    let mut hull: Vec<(f32, f32, u32)> =
        projected.iter().map(|(idx, x, y)| (*x, *y, *idx)).collect();
    hull.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    });
    hull.dedup_by(|a, b| (a.0 - b.0).abs() < EPS && (a.1 - b.1).abs() < EPS);

    if hull.len() < 3 {
        return hull.iter().map(|(_, _, idx)| *idx).collect();
    }

    let mut stack: Vec<(f32, f32, u32)> = Vec::new();
    for point in &hull {
        while stack.len() >= 2 {
            let a = stack[stack.len() - 2];
            let b = stack[stack.len() - 1];
            if cross2d(b.0 - a.0, b.1 - a.1, point.0 - b.0, point.1 - b.1) <= EPS {
                stack.pop();
            } else {
                break;
            }
        }
        stack.push(*point);
    }

    let mut lower = stack.clone();
    stack.clear();
    for point in hull.iter().rev() {
        while stack.len() >= 2 {
            let a = stack[stack.len() - 2];
            let b = stack[stack.len() - 1];
            if cross2d(b.0 - a.0, b.1 - a.1, point.0 - b.0, point.1 - b.1) <= EPS {
                stack.pop();
            } else {
                break;
            }
        }
        stack.push(*point);
    }

    lower.pop();
    stack.pop();
    lower.extend(stack);
    lower.iter().map(|(_, _, idx)| *idx).collect()
}

fn projection_basis(points: &[[f32; 3]]) -> ([f32; 3], [f32; 3], [f32; 3]) {
    let origin = points[0];
    let mut u = normalize(sub(points[1], origin));
    if length_squared(u) < EPS * EPS {
        u = [1.0, 0.0, 0.0];
    }
    let mut v = if points.len() > 2 {
        let raw = sub(points[2], origin);
        let proj = scale(u, dot(raw, u));
        normalize(sub(raw, proj))
    } else {
        [0.0, 1.0, 0.0]
    };
    if length_squared(v) < EPS * EPS {
        v = if u[0].abs() < 0.9 {
            normalize(cross(u, [1.0, 0.0, 0.0]))
        } else {
            normalize(cross(u, [0.0, 1.0, 0.0]))
        };
    }
    (origin, u, v)
}

fn are_coplanar(points: &[[f32; 3]]) -> bool {
    let Some((i, j, k)) = first_non_collinear_triple(points) else {
        return true;
    };
    let normal = face_normal(points[i], points[j], points[k]);
    for (idx, point) in points.iter().enumerate() {
        if idx == i || idx == j || idx == k {
            continue;
        }
        if dot(sub(*point, points[i]), normal).abs() > EPS {
            return false;
        }
    }
    true
}

fn first_non_collinear_triple(points: &[[f32; 3]]) -> Option<(usize, usize, usize)> {
    let n = points.len();
    for i in 0..n {
        for j in (i + 1)..n {
            for k in (j + 1)..n {
                if length_squared(face_normal(points[i], points[j], points[k])) > EPS * EPS {
                    return Some((i, j, k));
                }
            }
        }
    }
    None
}

fn oriented_triangle_indices(points: &[[f32; 3]], a: usize, b: usize, c: usize) -> [u32; 3] {
    let center = centroid(points);
    let face = [a as u32, b as u32, c as u32];
    if outward_face(&face, points, center) {
        face
    } else {
        [face[0], face[2], face[1]]
    }
}

fn outward_face(face: &[u32; 3], points: &[[f32; 3]], center: [f32; 3]) -> bool {
    let a = points[face[0] as usize];
    let b = points[face[1] as usize];
    let c = points[face[2] as usize];
    let face_center = scale(add(add(a, b), c), 1.0 / 3.0);
    let normal = face_normal(a, b, c);
    dot(normal, sub(face_center, center)) > 0.0
}

fn triangle_edges(indices: &[u32; 3]) -> Vec<(u32, u32)> {
    vec![
        (indices[0], indices[1]),
        (indices[1], indices[2]),
        (indices[2], indices[0]),
    ]
}

fn unique_edges(indices: &[u32]) -> Vec<(u32, u32)> {
    let mut edges = Vec::new();
    for tri in indices.chunks_exact(3) {
        for &(a, b) in &[(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
            let key = if a < b { (a, b) } else { (b, a) };
            if !edges.contains(&key) {
                edges.push(key);
            }
        }
    }
    edges
}

fn centroid(points: &[[f32; 3]]) -> [f32; 3] {
    let mut sum = [0.0f32; 3];
    for p in points {
        sum = add(sum, *p);
    }
    scale(sum, 1.0 / points.len() as f32)
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(v: [f32; 3], s: f32) -> [f32; 3] {
    [v[0] * s, v[1] * s, v[2] * s]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn cross2d(ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    ax * by - ay * bx
}

fn length_squared(v: [f32; 3]) -> f32 {
    dot(v, v)
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = length_squared(v).sqrt();
    if len < EPS { v } else { scale(v, 1.0 / len) }
}

fn face_normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    cross(sub(b, a), sub(c, a))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triangle_hull_has_one_face() {
        let points = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let hull = hull_from_points(&points).expect("triangle hull");
        assert_eq!(hull.positions.len(), 3);
        assert_eq!(hull.indices.len(), 3);
        assert_eq!(hull.edges.len(), 3);
    }

    #[test]
    fn tetrahedron_hull_has_four_faces() {
        let points = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ];
        let hull = hull_from_points(&points).expect("tetrahedron hull");
        assert_eq!(hull.indices.len(), 12);
        assert!(hull.edges.len() >= 6);
    }

    #[test]
    fn coplanar_quad_is_triangulated() {
        let points = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ];
        let hull = hull_from_points(&points).expect("quad hull");
        assert_eq!(hull.indices.len(), 6);
    }

    #[test]
    fn pentagon_hull_encloses_points() {
        let points = [
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [1.0, 2.0, 0.0],
            [1.0, 1.0, 2.0],
            [0.5, 0.5, 0.5],
        ];
        let hull = hull_from_points(&points).expect("pentagon hull");
        assert!(!hull.indices.is_empty());
        assert!(!hull.edges.is_empty());
    }

    #[test]
    fn two_points_yield_none() {
        assert!(hull_from_points(&[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]]).is_none());
    }
}
