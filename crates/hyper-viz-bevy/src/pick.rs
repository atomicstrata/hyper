//! Pointer picking geometry — kept Bevy-light so hits can be unit-tested.

use bevy::math::{Vec2, Vec3};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerHit {
    Vertex(usize),
    Hyperedge(usize),
}

#[derive(Debug, Clone, Copy)]
pub struct Ray {
    pub origin: Vec3,
    pub dir: Vec3,
}

/// Closest approach of a ray to a sphere. Returns `t` along the ray if within `radius`.
pub fn ray_hits_sphere(ray: Ray, center: Vec3, radius: f32) -> Option<f32> {
    let to_center = center - ray.origin;
    let t = to_center.dot(ray.dir);
    if t < 0.0 {
        return None;
    }
    let closest = ray.origin + ray.dir * t;
    let dist = (closest - center).length();
    if dist <= radius { Some(t) } else { None }
}

/// Distance from a ray to a finite segment, plus `t` at the closest point on the ray.
pub fn ray_segment_hit(ray: Ray, a: Vec3, b: Vec3, max_dist: f32) -> Option<f32> {
    let ab = b - a;
    let ab_len_sq = ab.length_squared();
    if ab_len_sq < 1e-8 {
        return ray_hits_sphere(ray, a, max_dist);
    }

    let w0 = a - ray.origin;
    let aa = ab_len_sq;
    let bb = ab.dot(ray.dir);
    let cc = ray.dir.length_squared().max(1e-8);
    let dd = ab.dot(w0);
    let ee = ray.dir.dot(w0);
    let denom = aa * cc - bb * bb;
    let s = if denom.abs() < 1e-8 {
        0.0
    } else {
        ((bb * ee - cc * dd) / denom).clamp(0.0, 1.0)
    };
    let on_seg = a + ab * s;
    let t = (on_seg - ray.origin).dot(ray.dir);
    if t < 0.0 {
        return None;
    }
    let on_ray = ray.origin + ray.dir * t;
    if (on_seg - on_ray).length() <= max_dist {
        Some(t)
    } else {
        None
    }
}

/// Möller–Trumbore. Returns `t` if the ray hits the triangle.
pub fn ray_hits_triangle(ray: Ray, a: Vec3, b: Vec3, c: Vec3) -> Option<f32> {
    const EPS: f32 = 1e-6;
    let e1 = b - a;
    let e2 = c - a;
    let p = ray.dir.cross(e2);
    let det = e1.dot(p);
    if det.abs() < EPS {
        return None;
    }
    let inv = 1.0 / det;
    let tvec = ray.origin - a;
    let u = tvec.dot(p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = tvec.cross(e1);
    let v = ray.dir.dot(q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = e2.dot(q) * inv;
    if t > EPS { Some(t) } else { None }
}

pub fn point_in_polygon(point: Vec2, polygon: &[Vec2]) -> bool {
    let mut inside = false;
    let n = polygon.len();
    for i in 0..n {
        let j = (i + 1) % n;
        let pi = polygon[i];
        let pj = polygon[j];
        if ((pi.y > point.y) != (pj.y > point.y))
            && (point.x < (pj.x - pi.x) * (point.y - pi.y) / (pj.y - pi.y) + pi.x)
        {
            inside = !inside;
        }
    }
    inside
}

/// Among hull hits, prefer the tightest set (fewest members), then the closest `t`.
pub fn best_hyperedge_hit(hits: &[(usize, usize, f32)]) -> Option<(usize, f32)> {
    hits.iter()
        .copied()
        .min_by(|a, b| {
            a.1.cmp(&b.1)
                .then_with(|| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
        })
        .map(|(idx, _, t)| (idx, t))
}

/// Vertices win when they are at least as close as a hull (nodes are the precise target).
pub fn resolve_hit(
    vertex: Option<(usize, f32)>,
    hyperedge: Option<(usize, f32)>,
) -> Option<PointerHit> {
    match (vertex, hyperedge) {
        (Some((v, vt)), Some((e, et))) => {
            if vt <= et + 0.25 {
                Some(PointerHit::Vertex(v))
            } else {
                Some(PointerHit::Hyperedge(e))
            }
        }
        (Some((v, _)), None) => Some(PointerHit::Vertex(v)),
        (None, Some((e, _))) => Some(PointerHit::Hyperedge(e)),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sphere_in_front_of_camera() {
        let ray = Ray {
            origin: Vec3::ZERO,
            dir: Vec3::Z,
        };
        assert!(ray_hits_sphere(ray, Vec3::new(0.0, 0.0, 10.0), 1.0).is_some());
        assert!(ray_hits_sphere(ray, Vec3::new(5.0, 0.0, 10.0), 1.0).is_none());
    }

    #[test]
    fn triangle_is_hittable() {
        let ray = Ray {
            origin: Vec3::new(0.0, 0.0, -1.0),
            dir: Vec3::Z,
        };
        let t = ray_hits_triangle(
            ray,
            Vec3::new(-1.0, -1.0, 2.0),
            Vec3::new(1.0, -1.0, 2.0),
            Vec3::new(0.0, 1.0, 2.0),
        );
        assert!(t.is_some());
        assert!(t.unwrap() > 0.0);
    }

    #[test]
    fn nested_hull_prefers_smaller_arity() {
        let hits = [(0, 5, 1.0), (1, 3, 1.1), (2, 4, 0.9)];
        let (idx, _) = best_hyperedge_hit(&hits).unwrap();
        assert_eq!(idx, 1);
    }

    #[test]
    fn vertex_wins_when_closer() {
        assert_eq!(
            resolve_hit(Some((0, 1.0)), Some((2, 1.5))),
            Some(PointerHit::Vertex(0))
        );
        assert_eq!(
            resolve_hit(Some((0, 3.0)), Some((2, 1.0))),
            Some(PointerHit::Hyperedge(2))
        );
    }

    #[test]
    fn segment_near_ray_hits() {
        let ray = Ray {
            origin: Vec3::ZERO,
            dir: Vec3::Z,
        };
        let hit = ray_segment_hit(
            ray,
            Vec3::new(-1.0, 0.0, 5.0),
            Vec3::new(1.0, 0.0, 5.0),
            0.2,
        );
        assert!(hit.is_some());
        assert!(
            ray_segment_hit(
                ray,
                Vec3::new(-1.0, 3.0, 5.0),
                Vec3::new(1.0, 3.0, 5.0),
                0.2
            )
            .is_none()
        );
    }

    #[test]
    fn point_in_unit_square() {
        let square = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(0.0, 10.0),
        ];
        assert!(point_in_polygon(Vec2::new(5.0, 5.0), &square));
        assert!(!point_in_polygon(Vec2::new(15.0, 5.0), &square));
    }
}
