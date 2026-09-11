use std::collections::HashMap;
use std::time::SystemTime;

use bevy::prelude::*;
use hyper_viz::{ForceLayout3D, HypergraphScene, LayoutConfig, Projection, load_json, project};

#[derive(Resource)]
pub struct GraphLayout {
    pub layout: ForceLayout3D,
    pub node_count: usize,
    pub link_count: usize,
    pub running: bool,
    pub iterations_per_frame: usize,
    pub scene: HypergraphScene,
}

impl GraphLayout {
    pub fn iterations(&self) -> u64 {
        self.layout.iterations
    }

    pub fn bevy_positions(&self) -> Vec<Vec3> {
        self.layout
            .positions
            .iter()
            .map(|p| Vec3::new(p.x, p.y, p.z))
            .collect()
    }

    pub fn position_at(&self, index: usize) -> Option<Vec3> {
        self.layout
            .positions
            .get(index)
            .map(|p| Vec3::new(p.x, p.y, p.z))
    }

    pub fn edges(&self) -> &[(usize, usize)] {
        &self.layout.edges
    }
}

#[derive(Resource)]
pub struct LayoutSettings {
    pub config: LayoutConfig,
    pub iterations_per_frame: usize,
    pub node_size: f32,
    pub graph_path: Option<String>,
    pub watch: bool,
    pub projection: Projection,
}

/// Published with each replacement scene so renderers cannot consume a stale layout.
#[derive(Resource, Default)]
pub struct GraphSceneEpoch(pub u64);

#[derive(Resource)]
pub struct GraphWatchState {
    pub path: String,
    pub last_mtime: Option<SystemTime>,
}

/// In-process live scene updates from any embedding host.
#[derive(Resource)]
pub struct LiveSceneReceiver(pub std::sync::Mutex<std::sync::mpsc::Receiver<HypergraphScene>>);

impl Default for LayoutSettings {
    fn default() -> Self {
        Self {
            config: LayoutConfig::default(),
            iterations_per_frame: 10,
            node_size: 1.0,
            graph_path: None,
            watch: false,
            projection: Projection::Bipartite,
        }
    }
}

fn layout_from_path(
    path: &str,
    settings: &LayoutSettings,
) -> Result<GraphLayout, hyper_viz::VizError> {
    let graph = load_json(path)?;
    let scene = project(&graph, settings.projection);
    Ok(GraphLayout::from_scene(scene, settings))
}

impl GraphLayout {
    pub fn from_scene(scene: HypergraphScene, settings: &LayoutSettings) -> Self {
        let layout = ForceLayout3D::from_scene(&scene, settings.config.clone());
        let link_count = layout.edges.len();
        let node_count = scene.node_count();

        Self {
            layout,
            node_count,
            link_count,
            running: true,
            iterations_per_frame: settings.iterations_per_frame,
            scene,
        }
    }

    /// Hot-reload: keep force-layout positions for nodes that still exist (matched by stable id).
    pub fn from_scene_preserve(
        old: &GraphLayout,
        scene: HypergraphScene,
        settings: &LayoutSettings,
    ) -> Self {
        let seeds: HashMap<String, hyper_viz::Vec3> = old
            .scene
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.id.clone(), old.layout.positions[i]))
            .collect();
        let layout = ForceLayout3D::from_scene_with_seeds(&scene, settings.config.clone(), &seeds);
        let link_count = layout.edges.len();

        Self {
            layout,
            node_count: scene.node_count(),
            link_count,
            running: old.running,
            iterations_per_frame: old.iterations_per_frame,
            scene,
        }
    }
}

pub fn init_graph(
    mut commands: Commands,
    settings: Res<LayoutSettings>,
    initial: Option<Res<crate::InitialScene>>,
    mut watch: Option<ResMut<GraphWatchState>>,
) {
    let layout = if let Some(init) = initial {
        GraphLayout::from_scene(init.0.clone(), &settings)
    } else if let Some(path) = &settings.graph_path {
        match layout_from_path(path, &settings) {
            Ok(layout) => layout,
            Err(err) => {
                tracing::error!(%err, path, "failed to load graph JSON");
                return;
            }
        }
    } else {
        tracing::error!("no graph scene or path provided");
        return;
    };

    if let (Some(watch), Some(path)) = (watch.as_mut(), settings.graph_path.as_ref()) {
        watch.last_mtime = std::fs::metadata(path).ok().and_then(|m| m.modified().ok());
    }

    tracing::info!(
        nodes = layout.node_count,
        links = layout.link_count,
        hyperedges = layout.scene.hyperedge_count(),
        graph_id = %layout.scene.meta.id,
        watch = settings.watch,
        "Hypergraph scene initialized"
    );
    commands.insert_resource(layout);
}

pub fn poll_graph_watch(
    time: Res<Time>,
    mut commands: Commands,
    settings: Res<LayoutSettings>,
    mut watch: ResMut<GraphWatchState>,
    layout: Option<Res<GraphLayout>>,
    epoch: Res<GraphSceneEpoch>,
    mut last_check: Local<f32>,
) {
    if !settings.watch {
        return;
    }

    *last_check += time.delta_secs();
    if *last_check < 0.5 {
        return;
    }
    *last_check = 0.0;

    let Ok(mtime) = std::fs::metadata(&watch.path).and_then(|m| m.modified()) else {
        return;
    };

    let Ok(new_layout) = layout_from_path(&watch.path, &settings) else {
        return;
    };

    if layout
        .as_ref()
        .is_some_and(|current| current.scene == new_layout.scene)
    {
        watch.last_mtime = Some(mtime);
        return;
    }

    let merged = match layout.as_ref() {
        Some(current) => GraphLayout::from_scene_preserve(current, new_layout.scene, &settings),
        None => new_layout,
    };

    let next_epoch = epoch.0 + 1;
    tracing::info!(
        nodes = merged.node_count,
        hyperedges = merged.scene.hyperedge_count(),
        epoch = next_epoch,
        path = %watch.path,
        "reloaded hypergraph from watch"
    );
    commands.insert_resource(merged);
    commands.insert_resource(GraphSceneEpoch(next_epoch));
    watch.last_mtime = Some(mtime);
}

pub fn poll_live_scene(
    mut commands: Commands,
    settings: Res<LayoutSettings>,
    receiver: Option<Res<LiveSceneReceiver>>,
    layout: Option<Res<GraphLayout>>,
    epoch: Res<GraphSceneEpoch>,
) {
    let Some(receiver) = receiver else {
        return;
    };

    let mut latest: Option<HypergraphScene> = None;
    {
        let rx = receiver.0.lock().expect("live scene receiver lock");
        while let Ok(scene) = rx.try_recv() {
            latest = Some(scene);
        }
    }

    let Some(scene) = latest else {
        return;
    };

    if layout
        .as_ref()
        .is_some_and(|current| current.scene == scene)
    {
        return;
    }

    let merged = match layout.as_ref() {
        Some(current) => GraphLayout::from_scene_preserve(current, scene, &settings),
        None => GraphLayout::from_scene(scene, &settings),
    };

    let next_epoch = epoch.0 + 1;
    tracing::info!(
        nodes = merged.node_count,
        hyperedges = merged.scene.hyperedge_count(),
        epoch = next_epoch,
        graph_id = %merged.scene.meta.id,
        "reloaded hypergraph from live channel"
    );
    commands.insert_resource(merged);
    commands.insert_resource(GraphSceneEpoch(next_epoch));
}

pub fn step_layout(mut layout: ResMut<GraphLayout>) {
    if !layout.running {
        return;
    }

    let iters = if layout.node_count >= 5000 {
        layout.iterations_per_frame.min(2)
    } else if layout.node_count >= 1000 {
        layout.iterations_per_frame.min(5)
    } else {
        layout.iterations_per_frame
    };

    for _ in 0..iters {
        layout.layout.step();
    }
}
