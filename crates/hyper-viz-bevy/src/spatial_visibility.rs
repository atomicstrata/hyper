//! The same bounded line list drives rendering and picking.
use crate::focus::FocusScope;
use hyper_viz::HypergraphScene;

#[derive(Clone, Copy, Debug)]
pub struct VisibleLine {
    pub source: usize,
    pub target: usize,
    pub hyperedge: Option<usize>,
}
pub fn visible_lines(
    scene: &HypergraphScene,
    focus: &FocusScope,
    hide_hubs: bool,
    budget: usize,
) -> (Vec<VisibleLine>, usize) {
    let mut lines = Vec::new();
    let mut total = 0;
    let mut push = |line: VisibleLine| {
        if !focus.contains(line.source)
            || !focus.contains(line.target)
            || line
                .hyperedge
                .is_some_and(|i| !focus.contains_hyperedge(scene, i))
        {
            return;
        }
        total += 1;
        if lines.len() < budget {
            lines.push(line);
        }
    };
    if hide_hubs {
        for (i, e) in scene.hyperedges.iter().enumerate() {
            if e.member_indices.len() == 2 {
                push(VisibleLine {
                    source: e.member_indices[0],
                    target: e.member_indices[1],
                    hyperedge: Some(i),
                });
            }
        }
    } else {
        let ids: std::collections::HashMap<_, _> = scene
            .hyperedges
            .iter()
            .enumerate()
            .map(|(i, e)| (e.id.as_str(), i))
            .collect();
        for line in &scene.links {
            push(VisibleLine {
                source: line.source,
                target: line.target,
                hyperedge: line
                    .hyperedge_id
                    .as_deref()
                    .and_then(|id| ids.get(id).copied()),
            });
        }
    }
    let omitted = total - lines.len();
    (lines, omitted)
}
#[cfg(test)]
mod tests {
    use super::*;
    use hyper_viz::{Hypergraph, Projection};
    #[test]
    fn budget_and_explicit_scope_agree_for_rendering_and_picking() {
        let scene = Hypergraph::new()
            .vertex("a", "a", "v")
            .vertex("b", "b", "v")
            .vertex("c", "c", "v")
            .hyperedge("ab", ["a", "b"], "ab")
            .hyperedge("bc", ["b", "c"], "bc")
            .project(Projection::StarCentroid);
        let mut focus = FocusScope::default();
        let (visible, omitted) = visible_lines(&scene, &focus, true, 1);
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].hyperedge, Some(0));
        assert_eq!(omitted, 1);
        focus.hyperedges = Some(std::collections::HashSet::from([1]));
        let (visible, omitted) = visible_lines(&scene, &focus, true, 1);
        assert_eq!(visible[0].hyperedge, Some(1));
        assert_eq!(omitted, 0);
    }
}

#[derive(bevy::prelude::Resource, Default)]
pub struct LineCache {
    pub segments: Vec<(VisibleLine, bevy::prelude::Vec3, bevy::prelude::Vec3)>,
    pub omitted: usize,
    key: Option<(u64, u64, u64, bool, usize, bool, FocusScope)>,
}
pub fn cache_lines(
    layout: bevy::prelude::Res<crate::graph::GraphLayout>,
    epoch: bevy::prelude::Res<crate::graph::GraphSceneEpoch>,
    focus: bevy::prelude::Res<FocusScope>,
    hulls: bevy::prelude::Res<crate::hyperedge_hull::HyperedgeHullSettings>,
    settings: bevy::prelude::Res<crate::render::LinkRenderSettings>,
    mut cache: bevy::prelude::ResMut<LineCache>,
) {
    let key = (
        epoch.0,
        layout.iterations(),
        layout.positions_revision,
        hulls.hide_hubs,
        settings.budget(),
        settings.opacity > 0.,
        focus.clone(),
    );
    if cache.key.as_ref() == Some(&key) {
        return;
    }
    let (lines, omitted) = visible_lines(&layout.scene, &focus, hulls.hide_hubs, settings.budget());
    cache.omitted = omitted;
    cache.segments = if settings.opacity > 0. {
        lines
            .into_iter()
            .filter_map(|line| {
                Some((
                    line,
                    layout.position_at(line.source)?,
                    layout.position_at(line.target)?,
                ))
            })
            .collect()
    } else {
        Vec::new()
    };
    cache.key = Some(key);
}
