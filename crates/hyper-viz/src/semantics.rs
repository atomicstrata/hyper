use crate::scene::{NodeRole, SceneNode};

/// RGBA in 0..1 for render backends.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Rgba {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
}

/// Visual status mapped from a free-form status string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeStatus {
    Active,
    Attention,
    Shadowed,
    Rejected,
}

pub fn parse_status(raw: &str) -> EdgeStatus {
    match raw.trim().to_ascii_lowercase().as_str() {
        "" | "active" | "ok" => EdgeStatus::Active,
        "attention" | "focus" => EdgeStatus::Attention,
        "shadowed" | "inactive" | "hidden" | "stale" => EdgeStatus::Shadowed,
        "rejected" | "deleted" | "invalid" => EdgeStatus::Rejected,
        _ => EdgeStatus::Active,
    }
}

/// Extra opacity multiplier when attention highlight mode is on.
/// Off: 1.0 (shadowed/rejected already use [`status_opacity`]).
pub fn attention_dim(status: EdgeStatus, attention_mode: bool) -> f32 {
    if !attention_mode {
        return 1.0;
    }
    match status {
        EdgeStatus::Attention => 1.0,
        EdgeStatus::Active => 0.08,
        EdgeStatus::Shadowed | EdgeStatus::Rejected => 0.03,
    }
}

/// Live-work vertices: not shadowed or rejected.
pub fn is_live_status(raw: &str) -> bool {
    !matches!(
        parse_status(raw),
        EdgeStatus::Shadowed | EdgeStatus::Rejected
    )
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeVisualStyle {
    pub base: Rgba,
    pub emissive: Rgba,
    pub radius_scale: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinkVisualStyle {
    pub color: Rgba,
}

/// Stable color for a free-form vertex kind. Well-known names keep a fixed
/// palette; everything else is hashed so unrelated apps still get distinct hues.
pub fn kind_color(kind: &str) -> Rgba {
    match kind {
        "entity" | "person" | "author" | "agent" => Rgba::new(0.0, 0.83, 1.0, 1.0),
        "predicate" | "relation" => Rgba::new(0.55, 0.45, 1.0, 1.0),
        "value" | "literal" => Rgba::new(0.2, 0.9, 0.55, 1.0),
        "time" | "timestamp" => Rgba::new(1.0, 0.65, 0.2, 1.0),
        "episode" | "event" | "session" => Rgba::new(0.75, 0.75, 0.8, 1.0),
        "paper" | "document" | "item" => Rgba::new(0.95, 0.85, 0.3, 1.0),
        "place" | "location" => Rgba::new(0.4, 0.85, 0.45, 1.0),
        "hyperedge" => Rgba::new(1.0, 0.35, 0.55, 1.0),
        "" => Rgba::new(0.6, 0.6, 0.65, 1.0),
        other => hash_color(other),
    }
}

/// Stable hue from any string (vertex kinds or hyperedge ids).
pub fn hyperedge_color(id: &str) -> Rgba {
    hash_color(id)
}

fn hash_color(kind: &str) -> Rgba {
    let mut h: u32 = 2_166_136_261;
    for b in kind.as_bytes() {
        h ^= u32::from(*b);
        h = h.wrapping_mul(16_777_619);
    }
    let hue = (h % 360) as f32;
    hsl_to_rgb(hue, 0.55, 0.55)
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> Rgba {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = h / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r1, g1, b1) = match hp as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    Rgba::new(r1 + m, g1 + m, b1 + m, 1.0)
}

fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) * 0.5;
    let d = max - min;
    if d < 1e-6 {
        return (0.0, 0.0, l);
    }
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min).max(1e-6)
    };
    let h = if (max - r).abs() < 1e-6 {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if (max - g).abs() < 1e-6 {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h * 60.0, s, l)
}

/// Hover/selection keep the original hue and lift saturation instead of swapping color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Emphasis {
    #[default]
    Rest,
    Hover,
    Selected,
}

impl Emphasis {
    /// Selected wins when both flags are set so pointing at a selection
    /// does not collapse it into the weaker hover treatment.
    pub fn from_flags(hovered: bool, selected: bool) -> Self {
        if selected {
            Self::Selected
        } else if hovered {
            Self::Hover
        } else {
            Self::Rest
        }
    }
}

/// Hover pops size; selected stays close to rest size so saturation/glow carry it.
pub fn emphasis_radius_scale(hovered: bool, selected: bool) -> f32 {
    match (hovered, selected) {
        (true, true) => 1.28,
        (true, false) => 1.22,
        (false, true) => 1.06,
        (false, false) => 1.0,
    }
}

pub fn emphasize(color: Rgba, emphasis: Emphasis) -> Rgba {
    let (h, s, l) = rgb_to_hsl(color.r, color.g, color.b);
    let (s, l) = match emphasis {
        Emphasis::Rest => (s, l),
        // Hover: slight lift, keep the local hue/chroma so it does not read as selected.
        Emphasis::Hover => ((s * 0.9 + 0.08).min(0.72), (l + 0.10).min(0.70)),
        Emphasis::Selected => ((s * 0.05 + 0.98).min(1.0), (l + 0.16).min(0.82)),
    };
    let mut out = hsl_to_rgb(h, s, l);
    out.a = color.a;
    out
}

pub fn saturation(color: Rgba) -> f32 {
    rgb_to_hsl(color.r, color.g, color.b).1
}

pub fn hub_color(status: EdgeStatus) -> Rgba {
    match status {
        EdgeStatus::Active | EdgeStatus::Attention => Rgba::new(1.0, 0.35, 0.55, 1.0),
        EdgeStatus::Shadowed => Rgba::new(0.45, 0.45, 0.5, 0.85),
        EdgeStatus::Rejected => Rgba::new(0.9, 0.2, 0.2, 0.7),
    }
}

pub fn status_opacity(status: EdgeStatus) -> f32 {
    match status {
        EdgeStatus::Active | EdgeStatus::Attention => 1.0,
        EdgeStatus::Shadowed => 0.45,
        EdgeStatus::Rejected => 0.35,
    }
}

pub fn link_color(status: Option<&str>) -> Rgba {
    link_color_for(None, status)
}

pub fn link_color_for(hyperedge_id: Option<&str>, status: Option<&str>) -> Rgba {
    let base = hyperedge_id
        .map(hyperedge_color)
        .unwrap_or(Rgba::new(0.3, 0.5, 0.7, 1.0));
    let alpha = match status.map(parse_status) {
        Some(EdgeStatus::Shadowed) => 0.18,
        Some(EdgeStatus::Rejected) => 0.12,
        Some(EdgeStatus::Active) | Some(EdgeStatus::Attention) | None => 0.4,
    };
    Rgba::new(base.r, base.g, base.b, alpha)
}

fn tint_by_status(color: Rgba, status: EdgeStatus) -> Rgba {
    let o = status_opacity(status);
    Rgba::new(
        color.r * o + 0.45 * (1.0 - o),
        color.g * o + 0.45 * (1.0 - o),
        color.b * o + 0.5 * (1.0 - o),
        color.a,
    )
}

pub fn node_style(node: &SceneNode, hyperedge_status: Option<&str>) -> NodeVisualStyle {
    let (base, radius_scale) = match node.role {
        NodeRole::Vertex => {
            let status = parse_status(&node.status);
            (tint_by_status(kind_color(&node.kind), status), 1.0)
        }
        NodeRole::HyperedgeHub => {
            let id = node.hyperedge_id.as_deref().unwrap_or(&node.id);
            let status = parse_status(hyperedge_status.unwrap_or(node.status.as_str()));
            (tint_by_status(hyperedge_color(id), status), 1.35)
        }
    };

    let emissive = Rgba::new(base.r * 0.25, base.g * 0.25, base.b * 0.25, 1.0);
    NodeVisualStyle {
        base,
        emissive,
        radius_scale,
    }
}

pub fn link_style(status: Option<&str>) -> LinkVisualStyle {
    link_style_for(None, status)
}

pub fn link_style_for(hyperedge_id: Option<&str>, status: Option<&str>) -> LinkVisualStyle {
    LinkVisualStyle {
        color: link_color_for(hyperedge_id, status),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HullVisualStyle {
    pub fill: Rgba,
    pub wire: Rgba,
}

pub fn hull_style(status: &str, opacity: f32) -> HullVisualStyle {
    hull_style_for("", status, opacity)
}

/// Fill/wire colors unique per hyperedge id so overlapping sets stay distinguishable.
pub fn hull_style_for(id: &str, status: &str, opacity: f32) -> HullVisualStyle {
    let parsed = parse_status(status);
    let base = if id.is_empty() {
        hub_color(parsed)
    } else {
        tint_by_status(hyperedge_color(id), parsed)
    };
    let alpha = opacity * status_opacity(parsed);
    HullVisualStyle {
        fill: Rgba::new(base.r, base.g, base.b, alpha),
        wire: Rgba::new(base.r, base.g, base.b, (alpha * 2.0).min(0.9)),
    }
}

pub fn hull_style_emphasized(
    id: &str,
    status: &str,
    opacity: f32,
    emphasis: Emphasis,
) -> HullVisualStyle {
    let style = hull_style_for(id, status, opacity);
    match emphasis {
        Emphasis::Rest => style,
        Emphasis::Hover => HullVisualStyle {
            fill: Rgba::new(
                style.fill.r,
                style.fill.g,
                style.fill.b,
                (style.fill.a * 1.12).min(0.42),
            ),
            wire: {
                let wire = emphasize(style.wire, Emphasis::Hover);
                Rgba::new(wire.r, wire.g, wire.b, (style.wire.a * 1.7 + 0.2).min(1.0))
            },
        },
        Emphasis::Selected => {
            let fill = emphasize(style.fill, Emphasis::Selected);
            let wire = emphasize(style.wire, Emphasis::Selected);
            HullVisualStyle {
                fill: Rgba::new(fill.r, fill.g, fill.b, (style.fill.a * 2.15).min(0.82)),
                wire: Rgba::new(wire.r, wire.g, wire.b, 1.0),
            }
        }
    }
}

pub fn scaled_radius(node_count: usize, size_multiplier: f32) -> f32 {
    let base = if node_count >= 5000 {
        0.3
    } else if node_count >= 1000 {
        0.5
    } else {
        0.8
    };
    base * size_multiplier
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::NodeRole;

    #[test]
    fn shadowed_hub_is_dimmer_than_active() {
        let active = hub_color(EdgeStatus::Active);
        let shadowed = hub_color(EdgeStatus::Shadowed);
        assert!(active.r > shadowed.r);
        assert!(status_opacity(EdgeStatus::Shadowed) < status_opacity(EdgeStatus::Active));
        assert_eq!(parse_status("inactive"), EdgeStatus::Shadowed);
    }

    #[test]
    fn unknown_kinds_are_stable() {
        assert_eq!(kind_color("molecule"), kind_color("molecule"));
        assert_ne!(kind_color("molecule"), kind_color("reaction"));
    }

    #[test]
    fn hull_colors_differ_by_hyperedge_id() {
        let a = hull_style_for("paper-a", "active", 0.3);
        let c = hull_style_for("paper-c", "active", 0.3);
        let lab = hull_style_for("lab", "active", 0.3);
        assert_ne!(a.fill, c.fill);
        assert_ne!(c.fill, lab.fill);
        assert_eq!(
            hull_style_for("paper-a", "active", 0.3).fill,
            hull_style_for("paper-a", "active", 0.3).fill
        );
    }

    #[test]
    fn selected_is_clearly_more_saturated_than_hover() {
        let base = hyperedge_color("paper-a");
        let hover = emphasize(base, Emphasis::Hover);
        let selected = emphasize(base, Emphasis::Selected);
        assert!(saturation(selected) > saturation(base) + 0.2);
        assert!(saturation(selected) - saturation(hover) > 0.2);
        let (h0, _, _) = rgb_to_hsl(base.r, base.g, base.b);
        let (h1, _, _) = rgb_to_hsl(selected.r, selected.g, selected.b);
        let dh = (h0 - h1).abs();
        assert!(dh.min(360.0 - dh) < 8.0);
    }

    #[test]
    fn selected_wins_over_hover_flag() {
        assert_eq!(Emphasis::from_flags(true, true), Emphasis::Selected);
        assert_eq!(Emphasis::from_flags(true, false), Emphasis::Hover);
        assert!(emphasis_radius_scale(true, false) > emphasis_radius_scale(false, true));
    }

    #[test]
    fn hull_emphasis_keeps_identity_and_raises_alpha() {
        let rest = hull_style_emphasized("paper-a", "active", 0.3, Emphasis::Rest);
        let selected = hull_style_emphasized("paper-a", "active", 0.3, Emphasis::Selected);
        assert!(saturation(selected.fill) >= saturation(rest.fill));
        assert!(selected.fill.a > rest.fill.a);
        assert_ne!(selected.fill, rest.fill);
    }

    #[test]
    fn vertex_and_hub_styles_differ() {
        let vertex = SceneNode {
            id: "v:1".into(),
            index: 0,
            role: NodeRole::Vertex,
            kind: "person".into(),
            label: "Alice".into(),
            hyperedge_id: None,
            status: String::new(),
        };
        let hub = SceneNode {
            id: "e:1".into(),
            index: 1,
            role: NodeRole::HyperedgeHub,
            kind: "hyperedge".into(),
            label: "Paper A".into(),
            hyperedge_id: Some("e:1".into()),
            status: String::new(),
        };
        assert_ne!(
            node_style(&vertex, None).base,
            node_style(&hub, Some("active")).base
        );
    }

    #[test]
    fn parse_attention_aliases() {
        assert_eq!(parse_status("attention"), EdgeStatus::Attention);
        assert_eq!(parse_status("focus"), EdgeStatus::Attention);
        assert_eq!(parse_status("ATTENTION"), EdgeStatus::Attention);
    }

    #[test]
    fn attention_dim_table() {
        assert_eq!(attention_dim(EdgeStatus::Attention, false), 1.0);
        assert_eq!(attention_dim(EdgeStatus::Active, false), 1.0);
        assert_eq!(attention_dim(EdgeStatus::Shadowed, false), 1.0);
        assert_eq!(attention_dim(EdgeStatus::Attention, true), 1.0);
        assert!((attention_dim(EdgeStatus::Active, true) - 0.08).abs() < f32::EPSILON);
        assert!((attention_dim(EdgeStatus::Shadowed, true) - 0.03).abs() < f32::EPSILON);
        assert!((attention_dim(EdgeStatus::Rejected, true) - 0.03).abs() < f32::EPSILON);
    }

    #[test]
    fn shadowed_vertex_is_tinted() {
        let live = SceneNode {
            id: "wt".into(),
            index: 0,
            role: NodeRole::Vertex,
            kind: "worktree".into(),
            label: "main".into(),
            hyperedge_id: None,
            status: String::new(),
        };
        let backlog = SceneNode {
            status: "shadowed".into(),
            ..live.clone()
        };
        assert_ne!(
            node_style(&live, None).base,
            node_style(&backlog, None).base
        );
    }

    #[test]
    fn live_status_excludes_shadowed() {
        assert!(is_live_status(""));
        assert!(is_live_status("attention"));
        assert!(!is_live_status("shadowed"));
        assert!(!is_live_status("rejected"));
    }
}
