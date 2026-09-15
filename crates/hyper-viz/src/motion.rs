use std::collections::HashMap;
use std::f32::consts::TAU;

use crate::scene::{HypergraphScene, NodeRole};
use crate::semantics::{EdgeStatus, parse_status};

pub const ONE_SHOT_SECS: f32 = 0.7;
pub const ONE_SHOT_CAP: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatusMotion {
    pub scale: f32,
    pub glow: f32,
    pub opacity: f32,
}

impl Default for StatusMotion {
    fn default() -> Self {
        Self {
            scale: 1.0,
            glow: 0.0,
            opacity: 1.0,
        }
    }
}

impl StatusMotion {
    pub fn combine(self, other: StatusMotion) -> StatusMotion {
        StatusMotion {
            scale: self.scale * other.scale,
            glow: self.glow + other.glow,
            opacity: self.opacity * other.opacity,
        }
    }
}

/// Stable 0..TAU offset so neighbors do not throb in lockstep.
pub fn id_phase(id: &str) -> f32 {
    let mut h: u32 = 2_166_136_261;
    for b in id.as_bytes() {
        h ^= u32::from(*b);
        h = h.wrapping_mul(16_777_619);
    }
    (h as f32 / u32::MAX as f32) * TAU
}

/// Empty / omitted status does not loop. Explicit `"active"` does.
pub fn looping_status(raw: &str) -> Option<EdgeStatus> {
    let raw = raw.trim();
    if raw.is_empty() {
        None
    } else {
        Some(parse_status(raw))
    }
}

pub fn status_motion(status: EdgeStatus, elapsed_secs: f32, phase: f32) -> StatusMotion {
    let (period, scale_amp, glow_amp, opacity_mid, opacity_amp) = match status {
        EdgeStatus::Active => (2.4, 0.12, 0.28, 1.0, 0.14),
        EdgeStatus::Shadowed => (3.2, 0.0, 0.16, 0.58, 0.30),
        EdgeStatus::Rejected => (0.9, 0.16, 0.55, 0.72, 0.28),
    };
    let wave = (elapsed_secs * TAU / period + phase).sin();
    let glow_01 = wave * 0.5 + 0.5;
    StatusMotion {
        scale: 1.0 + scale_amp * wave,
        glow: glow_amp * glow_01,
        opacity: opacity_mid + opacity_amp * wave,
    }
}

pub fn status_motion_for(raw: &str, elapsed_secs: f32, phase: f32) -> StatusMotion {
    looping_status(raw)
        .map(|status| status_motion(status, elapsed_secs, phase))
        .unwrap_or_default()
}

pub fn apply_motion_rgba(
    base: crate::semantics::Rgba,
    motion: StatusMotion,
) -> crate::semantics::Rgba {
    crate::semantics::Rgba::new(
        (base.r + motion.glow).min(1.0),
        (base.g + motion.glow).min(1.0),
        (base.b + motion.glow).min(1.0),
        (base.a * motion.opacity).clamp(0.02, 1.0),
    )
}

/// `progress` is 0 at start, 1 when the burst is done.
pub fn one_shot_motion(progress: f32) -> StatusMotion {
    let k = (1.0 - progress.clamp(0.0, 1.0)).powi(2);
    StatusMotion {
        scale: 1.0 + 0.35 * k,
        glow: 0.55 * k,
        opacity: 1.0,
    }
}

/// New or status-changed ids. Empty if the diff looks like a full replace.
pub fn one_shot_ids(prev: &HashMap<String, String>, next: &HashMap<String, String>) -> Vec<String> {
    let mut out: Vec<String> = next
        .iter()
        .filter_map(|(id, status)| match prev.get(id) {
            None => Some(id.clone()),
            Some(old) if old != status => Some(id.clone()),
            _ => None,
        })
        .collect();
    if out.len() > ONE_SHOT_CAP {
        out.clear();
    }
    out
}

pub fn vertex_status_key(id: &str) -> String {
    format!("v:{id}")
}

pub fn hyperedge_status_key(id: &str) -> String {
    format!("he:{id}")
}

pub fn scene_status_snapshot(scene: &HypergraphScene) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for node in &scene.nodes {
        if node.role == NodeRole::Vertex {
            map.insert(vertex_status_key(&node.id), String::new());
        }
    }
    for he in &scene.hyperedges {
        map.insert(hyperedge_status_key(&he.id), he.status.clone());
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omitted_status_does_not_loop() {
        let a = status_motion_for("", 0.0, 0.0);
        let b = status_motion_for("", 1.7, 2.1);
        assert_eq!(a, b);
        assert_eq!(a, StatusMotion::default());
    }

    #[test]
    fn explicit_active_loops() {
        let a = status_motion_for("active", 0.0, 0.0);
        let b = status_motion_for("active", 0.6, 0.0);
        assert_ne!(a, b);
        assert!((b.scale - 1.0).abs() > 0.05);
    }

    #[test]
    fn rejected_moves_more_than_active() {
        let samples = [0.0, 0.3, 0.6, 0.9, 1.2];
        let active_span = amplitude(EdgeStatus::Active, &samples);
        let rejected_span = amplitude(EdgeStatus::Rejected, &samples);
        assert!(rejected_span > active_span);
    }

    #[test]
    fn shadowed_breathes_opacity_not_scale() {
        let m = status_motion(EdgeStatus::Shadowed, 0.8, 0.0);
        assert!((m.scale - 1.0).abs() < 1e-5);
        assert!(m.opacity >= 0.28 && m.opacity <= 0.88);
    }

    #[test]
    fn rejected_pulses_opacity_for_hulls() {
        let a = status_motion(EdgeStatus::Rejected, 0.0, 0.0);
        let b = status_motion(EdgeStatus::Rejected, 0.45, 0.0);
        assert!((a.opacity - b.opacity).abs() > 0.15);
        assert!(a.glow.max(b.glow) > 0.2);
    }

    #[test]
    fn id_phase_splits_neighbors() {
        let a = id_phase("alice");
        let b = id_phase("bob");
        assert!((a - b).abs() > 0.05);
    }

    #[test]
    fn one_shot_starts_large_and_settles() {
        let start = one_shot_motion(0.0);
        let end = one_shot_motion(1.0);
        assert!(start.scale > end.scale);
        assert!(start.glow > end.glow);
        assert!((end.scale - 1.0).abs() < 1e-5);
        assert!(end.glow.abs() < 1e-5);
    }

    #[test]
    fn one_shot_ids_detects_new_and_status_flip() {
        let prev = HashMap::from([
            ("v:alice".into(), String::new()),
            ("he:paper-a".into(), "active".into()),
        ]);
        let next = HashMap::from([
            ("v:alice".into(), String::new()),
            ("v:bob".into(), String::new()),
            ("he:paper-a".into(), "rejected".into()),
        ]);
        let mut ids = one_shot_ids(&prev, &next);
        ids.sort();
        assert_eq!(ids, vec!["he:paper-a", "v:bob"]);
    }

    #[test]
    fn one_shot_ids_skips_full_replace() {
        let prev = HashMap::new();
        let next: HashMap<String, String> = (0..ONE_SHOT_CAP + 1)
            .map(|i| (format!("v:{i}"), String::new()))
            .collect();
        assert!(one_shot_ids(&prev, &next).is_empty());
    }

    #[test]
    fn snapshot_keys_vertices_and_hyperedges() {
        let graph = crate::schema::sample_coauthorship();
        let scene = crate::project::project(&graph, crate::project::Projection::Bipartite);
        let snap = scene_status_snapshot(&scene);
        assert!(snap.contains_key(&vertex_status_key("alice")));
        assert_eq!(
            snap.get(&hyperedge_status_key("paper-a"))
                .map(String::as_str),
            Some("active")
        );
        assert_eq!(
            snap.get(&hyperedge_status_key("paper-b"))
                .map(String::as_str),
            Some("shadowed")
        );
        assert_eq!(
            snap.get(&hyperedge_status_key("paper-c"))
                .map(String::as_str),
            Some("rejected")
        );
    }

    fn amplitude(status: EdgeStatus, samples: &[f32]) -> f32 {
        let scales: Vec<f32> = samples
            .iter()
            .map(|t| status_motion(status, *t, 0.0).scale)
            .collect();
        scales.iter().cloned().fold(f32::MIN, f32::max)
            - scales.iter().cloned().fold(f32::MAX, f32::min)
    }
}
