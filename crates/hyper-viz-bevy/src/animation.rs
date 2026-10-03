use std::collections::HashMap;

use bevy::prelude::*;
use hyper_viz::{
    HypergraphScene, NodeRole, ONE_SHOT_SECS, StatusMotion, hyperedge_status_key, id_phase,
    one_shot_ids, one_shot_motion, scene_status_snapshot, status_motion_for, vertex_status_key,
};

use crate::graph::{GraphLayout, GraphSceneEpoch};

#[derive(Resource, Default)]
pub struct StatusBursts {
    pub remaining: HashMap<String, f32>,
}

pub struct StatusAnimationPlugin;

impl Plugin for StatusAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StatusBursts>().add_systems(
            Update,
            (
                detect_status_changes.run_if(resource_exists::<GraphLayout>),
                tick_status_bursts,
            )
                .chain(),
        );
    }
}

pub fn motion_for(
    raw_status: &str,
    id: &str,
    elapsed_secs: f32,
    bursts: &StatusBursts,
    burst_key: &str,
) -> StatusMotion {
    let looping = status_motion_for(raw_status, elapsed_secs, id_phase(id));
    match bursts.remaining.get(burst_key) {
        Some(remaining) => {
            let progress = 1.0 - (*remaining / ONE_SHOT_SECS).clamp(0.0, 1.0);
            looping.combine(one_shot_motion(progress))
        }
        None => looping,
    }
}

pub fn node_motion(
    scene: &HypergraphScene,
    index: usize,
    elapsed_secs: f32,
    bursts: &StatusBursts,
) -> StatusMotion {
    let Some(node) = scene.nodes.get(index) else {
        return StatusMotion::default();
    };
    match node.role {
        NodeRole::Vertex => motion_for(
            "",
            &node.id,
            elapsed_secs,
            bursts,
            &vertex_status_key(&node.id),
        ),
        NodeRole::HyperedgeHub => {
            let raw = scene
                .hyperedges
                .iter()
                .find(|he| he.hub_index == Some(index))
                .map(|he| he.status.as_str())
                .unwrap_or("");
            let key_id = node.hyperedge_id.as_deref().unwrap_or(&node.id);
            motion_for(
                raw,
                key_id,
                elapsed_secs,
                bursts,
                &hyperedge_status_key(key_id),
            )
        }
    }
}

fn detect_status_changes(
    layout: Res<GraphLayout>,
    epoch: Res<GraphSceneEpoch>,
    mut bursts: ResMut<StatusBursts>,
    mut prev: Local<Option<(u64, HashMap<String, String>)>>,
) {
    if prev.as_ref().is_some_and(|(e, _)| *e == epoch.0) {
        return;
    }
    let next = scene_status_snapshot(&layout.scene);
    if let Some((_, old)) = prev.as_ref() {
        let ids = one_shot_ids(old, &next);
        if !ids.is_empty() {
            tracing::debug!(count = ids.len(), "status one-shot bursts");
            for id in ids {
                bursts.remaining.insert(id, ONE_SHOT_SECS);
            }
        }
    }
    *prev = Some((epoch.0, next));
}

fn tick_status_bursts(time: Res<Time>, mut bursts: ResMut<StatusBursts>) {
    if bursts.remaining.is_empty() {
        return;
    }
    let dt = time.delta_secs();
    bursts.remaining.retain(|_, remaining| {
        *remaining -= dt;
        *remaining > 0.0
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::LayoutSettings;
    use hyper_viz::{Hyperedge, Hypergraph, Projection, project};

    fn scene_with_status(status: &str) -> hyper_viz::HypergraphScene {
        let mut graph = Hypergraph::new().vertex("alice", "Alice", "person");
        graph.add_hyperedge(
            Hyperedge::new("paper-a", ["alice"])
                .with_label("A")
                .with_status(status),
        );
        project(&graph, Projection::Bipartite)
    }

    #[test]
    fn status_flip_starts_a_one_shot_burst() {
        let settings = LayoutSettings::default();
        let mut app = App::new();
        app.insert_resource(GraphLayout::from_scene(
            scene_with_status("active"),
            &settings,
        ))
        .insert_resource(GraphSceneEpoch(0))
        .init_resource::<StatusBursts>()
        .add_systems(Update, detect_status_changes);

        app.update();
        assert!(app.world().resource::<StatusBursts>().remaining.is_empty());

        app.insert_resource(GraphLayout::from_scene(
            scene_with_status("rejected"),
            &settings,
        ));
        app.insert_resource(GraphSceneEpoch(1));
        app.update();

        assert!(
            app.world()
                .resource::<StatusBursts>()
                .remaining
                .contains_key("he:paper-a")
        );
    }
}
