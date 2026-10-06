use crate::explorer_state::ExplorerState;
use bevy_egui::egui;
use hyper_viz::{DependencyIndex, DependencyScope, HypergraphScene};
use std::collections::{BTreeMap, HashMap};

pub struct CanvasNode {
    pub index: usize,
    pub position: [f32; 2],
}
pub fn layered_positions(
    index: &DependencyIndex,
    scope: &DependencyScope,
    canonical: &DependencyScope,
) -> Vec<CanvasNode> {
    let Some(center) = scope.center else {
        return Vec::new();
    };
    let mut columns: BTreeMap<i32, Vec<usize>> = BTreeMap::new();
    for &node in canonical
        .import_depths
        .keys()
        .chain(canonical.dependent_depths.keys())
    {
        let column = if node == center {
            0
        } else if let Some(depth) = canonical.dependent_depths.get(&node) {
            -(*depth as i32)
        } else {
            canonical.import_depths.get(&node).copied().unwrap_or(1) as i32
        };
        columns.entry(column).or_default().push(node);
    }
    let mut positions = HashMap::new();
    for (column, mut nodes) in columns {
        nodes.sort_by(|a, b| index.id(*a).cmp(&index.id(*b)));
        nodes.dedup();
        for (row, node) in nodes.iter().enumerate() {
            positions.insert(*node, [column as f32 * 340., row as f32 * 44.]);
        }
    }
    scope
        .nodes
        .iter()
        .map(|i| CanvasNode {
            index: *i,
            position: positions.get(i).copied().unwrap_or([0., 0.]),
        })
        .collect()
}

pub fn canvas(ui: &mut egui::Ui, state: &mut ExplorerState, scene: &HypergraphScene) {
    let nodes = layered_positions(&state.index, &state.scope, &state.canonical);
    let (response, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::drag());
    let rect = response.rect;
    if state.fit_requested && !nodes.is_empty() {
        let min = nodes
            .iter()
            .fold(egui::vec2(f32::INFINITY, f32::INFINITY), |v, n| {
                egui::vec2(v.x.min(n.position[0]), v.y.min(n.position[1]))
            });
        let max = nodes
            .iter()
            .fold(egui::vec2(f32::NEG_INFINITY, f32::NEG_INFINITY), |v, n| {
                egui::vec2(v.x.max(n.position[0]), v.y.max(n.position[1]))
            });
        let size = max - min;
        state.current.zoom = ((rect.width() - 300.) / size.x.max(1.))
            .min((rect.height() - 120.) / size.y.max(1.))
            .clamp(0.1, 1.);
        let midpoint = (min + max) * 0.5;
        state.current.pan = [
            -midpoint.x * state.current.zoom,
            -midpoint.y * state.current.zoom,
        ];
        state.fit_requested = false;
    }

    if response.dragged() {
        let d = response.drag_delta();
        state.current.pan[0] += d.x;
        state.current.pan[1] += d.y;
    }
    if response.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0. {
            state.current.zoom = (state.current.zoom * (scroll * 0.002).exp()).clamp(0.1, 4.);
        }
    }
    let zoom = state.current.zoom;
    let origin = rect.center() + egui::vec2(state.current.pan[0], state.current.pan[1]);
    let coords: HashMap<_, _> = nodes
        .iter()
        .map(|n| {
            (
                n.index,
                origin + egui::vec2(n.position[0], n.position[1]) * zoom,
            )
        })
        .collect();
    for &(source, target) in &state.scope.edges {
        let (Some(&a), Some(&b)) = (coords.get(&source), coords.get(&target)) else {
            continue;
        };
        if a == b {
            continue;
        }
        let delta = b - a;
        let unit = delta.normalized();
        let start = a + unit * 18. * zoom;
        let end = b - unit * 18. * zoom;
        let highlighted = state
            .path
            .as_ref()
            .is_some_and(|p| p.windows(2).any(|pair| pair == [source, target]));
        let color = if highlighted {
            egui::Color32::YELLOW
        } else {
            egui::Color32::from_rgb(100, 135, 175)
        };
        let stroke = egui::Stroke::new(if highlighted { 2.5 } else { 1. }, color);
        painter.line_segment([start, end], stroke);
        let head = 8. * zoom.clamp(0.6, 1.2);
        let perpendicular = egui::vec2(-unit.y, unit.x);
        painter.line_segment(
            [end, end - unit * head + perpendicular * head * 0.45],
            stroke,
        );
        painter.line_segment(
            [end, end - unit * head - perpendicular * head * 0.45],
            stroke,
        );
    }
    let mut select = None;
    for node in nodes {
        let p = coords[&node.index];
        let center = Some(node.index) == state.scope.center;
        let category = state.categories[node.index];
        let color = if center {
            egui::Color32::from_rgb(255, 193, 80)
        } else if category.external {
            egui::Color32::from_rgb(175, 145, 225)
        } else if category.test {
            egui::Color32::from_rgb(240, 145, 155)
        } else if category.tactic {
            egui::Color32::from_rgb(100, 210, 150)
        } else if category.umbrella {
            egui::Color32::from_rgb(225, 160, 85)
        } else {
            egui::Color32::from_rgb(115, 175, 235)
        };
        let hit = egui::Rect::from_center_size(
            p,
            if zoom >= 0.55 {
                egui::vec2(240., 38.) * zoom
            } else {
                egui::vec2(8., 8.)
            },
        );
        if !rect.intersects(hit) {
            continue;
        }
        painter.circle_filled(p, if center { 10. } else { (6. * zoom).max(2.) }, color);
        let id = &scene.nodes[node.index].id;
        let short = id
            .rsplit('.')
            .take(2)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join(".");
        if zoom >= 0.55 || center {
            let label_pos = p + egui::vec2(12., 0.);
            let galley = painter.layout_no_wrap(
                short,
                egui::FontId::proportional(if center {
                    13.
                } else {
                    13. * zoom.clamp(0.7, 1.5)
                }),
                color,
            );
            let label_pos = label_pos - egui::vec2(0., galley.size().y / 2.);
            painter.rect_filled(
                egui::Rect::from_min_size(label_pos, galley.size()).expand(2.),
                2.,
                ui.visuals().panel_fill,
            );
            painter.galley(label_pos, galley, color);
        }
        let both = node.index != state.scope.center.unwrap_or(usize::MAX)
            && state.scope.import_depths.contains_key(&node.index)
            && state.scope.dependent_depths.contains_key(&node.index);
        let tooltip = format!(
            "{}{}\n{}",
            id,
            if both { " (import and dependent)" } else { "" },
            scene.nodes[node.index]
                .attrs
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("")
        );
        if ui
            .interact(
                hit.intersect(rect),
                ui.id().with(node.index),
                egui::Sense::click(),
            )
            .on_hover_text(tooltip)
            .clicked()
        {
            select = Some(id.clone());
        }
    }
    if let Some(id) = select {
        state.select_module(&id);
    }
    painter.text(
        rect.left_top() + egui::vec2(16., 16.),
        egui::Align2::LEFT_TOP,
        "Dependents <- Center -> Imports\nArrows: importer -> imported · Drag to pan · Scroll to zoom · Zoom in for labels; hover for full names",
        egui::FontId::proportional(13.),
        egui::Color32::GRAY,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dual_role_nodes_once_and_retained_rows_stable() {
        let scope = DependencyScope {
            center: Some(0),
            nodes: vec![0, 1, 2],
            import_depths: HashMap::from([(0, 0), (1, 1), (2, 1)]),
            dependent_depths: HashMap::from([(0, 0), (1, 1)]),
            ..Default::default()
        };
        let index = DependencyIndex::default();
        let all = layered_positions(&index, &scope, &scope);
        assert_eq!(all.len(), 3);
        assert!(all[1].position[0] < 0.);
        let mut small = scope.clone();
        small.nodes = vec![0, 2];
        let visible = layered_positions(&index, &small, &scope);
        assert_eq!(visible[1].position, all[2].position);
        assert_eq!(all[0].position, [0., 0.]);
    }
    #[test]
    fn rows_survive_filtering_and_scene_reordering() {
        use hyper_viz::{Hyperedge, Hypergraph, Projection, ScopeOptions};
        let mut graph = Hypergraph::new()
            .vertex("a", "a", "module")
            .vertex("b", "b", "module")
            .vertex("c", "c", "module");
        for target in ["b", "c"] {
            let mut edge = Hyperedge::new(target, ["a", target]).with_kind("import");
            edge.attrs.insert("source".into(), "a".into());
            edge.attrs.insert("target".into(), target.into());
            graph.add_hyperedge(edge);
        }
        let positions = |scene: &HypergraphScene, filter: bool| {
            let index = DependencyIndex::new(scene);
            let center = index.node_index("a").unwrap();
            let canonical = index.scope(
                center,
                &ScopeOptions::default(),
                &vec![true; scene.node_count()],
            );
            let mut allowed = vec![true; scene.node_count()];
            if filter {
                allowed[index.node_index("b").unwrap()] = false;
            }
            let scope = index.scope(center, &ScopeOptions::default(), &allowed);
            layered_positions(&index, &scope, &canonical)
                .into_iter()
                .map(|n| (scene.nodes[n.index].id.clone(), n.position))
                .collect::<BTreeMap<_, _>>()
        };
        let scene = graph.project(Projection::StarCentroid);
        let all = positions(&scene, false);
        assert_eq!(all["c"], [340., 44.]);
        assert_eq!(positions(&scene, true)["c"], all["c"]);
        graph.vertices.reverse();
        let reversed = graph.project(Projection::StarCentroid);
        assert_eq!(positions(&reversed, false), all);
    }
}
