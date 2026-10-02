use std::collections::HashMap;
use std::time::SystemTime;

use bevy::prelude::*;
use hyper_viz::{
    ForceLayout3D, HypergraphScene, LayoutConfig, Projection, load_json, project, scenes_equivalent,
};

#[derive(Resource)]
pub struct GraphLayout {
    pub layout: ForceLayout3D,
    pub node_count: usize,
    pub link_count: usize,
    pub running: bool,
    pub iterations_per_frame: usize,
    pub scene: HypergraphScene,
    /// Hyperedge status keyed by hub scene index (avoids O(nodes×edges) scans).
    pub hub_status: Vec<Option<String>>,
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
}

#[derive(Resource, Clone)]
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
            iterations_per_frame: 4,
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
        let hub_status = hub_status_from_scene(&scene);

        Self {
            layout,
            node_count,
            link_count,
            running: true,
            iterations_per_frame: settings.iterations_per_frame,
            scene,
            hub_status,
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
            hub_status: hub_status_from_scene(&scene),
            scene,
        }
    }
}

fn hub_status_from_scene(scene: &HypergraphScene) -> Vec<Option<String>> {
    let mut hub_status = vec![None; scene.node_count()];
    for he in &scene.hyperedges {
        if let Some(slot) = hub_status.get_mut(he.hub_index) {
            *slot = Some(he.status.clone());
        }
    }
    hub_status
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

    if let Some(watch) = watch.as_mut() {
        // The file may have been replaced while the initial scene was loading.
        // Let the first poll validate it before claiming any timestamp.
        watch.last_mtime = None;
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
    if layout.is_some() && watch.last_mtime == Some(mtime) {
        return;
    }

    let Ok(graph) = load_json(&watch.path) else {
        return;
    };
    let scene = project(&graph, settings.projection);

    if layout
        .as_ref()
        .is_some_and(|current| scenes_equivalent(&current.scene, &scene))
    {
        watch.last_mtime = Some(mtime);
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
    search: Option<ResMut<crate::search::SearchState>>,
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

    if let Some(mut search) = search {
        latest = search.scene_update(latest);
    }
    let Some(scene) = latest else {
        return;
    };

    if layout
        .as_ref()
        .is_some_and(|current| scenes_equivalent(&current.scene, &scene))
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

    let iters = if layout.node_count >= 800 {
        layout.iterations_per_frame.min(2)
    } else if layout.node_count >= 400 {
        layout.iterations_per_frame.min(3)
    } else {
        layout.iterations_per_frame
    };

    for _ in 0..iters {
        layout.layout.step();
    }
}

#[cfg(test)]
mod watch_tests {
    use super::*;
    use hyper_viz::{Hypergraph, Vertex, save_json};
    use std::time::Duration;

    fn fixture() -> (tempfile::TempDir, App) {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("graph.json");
        let graph = Hypergraph::new();
        save_json(&path, &graph).unwrap();
        let settings = LayoutSettings {
            watch: true,
            ..Default::default()
        };
        let layout = GraphLayout::from_scene(project(&graph, settings.projection), &settings);
        let mut time = Time::<()>::default();
        time.advance_by(Duration::from_millis(500));
        let mut app = App::new();
        app.insert_resource(time)
            .insert_resource(settings)
            .insert_resource(layout)
            .insert_resource(GraphSceneEpoch::default())
            .insert_resource(GraphWatchState {
                path: path.to_str().unwrap().into(),
                last_mtime: Some(std::fs::metadata(&path).unwrap().modified().unwrap()),
            })
            .add_systems(Update, poll_graph_watch);
        (tmp, app)
    }

    #[test]
    fn unchanged_file_stamp_does_not_replace_the_live_scene() {
        let (_tmp, mut app) = fixture();
        // An embedding host may have changed the live scene since the file was
        // consumed. An unchanged file must not overwrite it on the next poll.
        let mut live = Hypergraph::new();
        live.vertices.push(Vertex::new("v", "Live node"));
        let settings = app.world().resource::<LayoutSettings>();
        let layout = GraphLayout::from_scene(project(&live, settings.projection), settings);
        app.insert_resource(layout);
        app.update();
        assert_eq!(app.world().resource::<GraphLayout>().node_count, 1);
        assert_eq!(app.world().resource::<GraphSceneEpoch>().0, 0);
    }

    #[test]
    fn first_poll_validates_file_against_the_initial_scene() {
        let (_tmp, mut app) = fixture();
        let mut initial = Hypergraph::new();
        initial.vertices.push(Vertex::new("old", "Old node"));
        let settings = app.world().resource::<LayoutSettings>();
        let initial = project(&initial, settings.projection);
        let path = app.world().resource::<GraphWatchState>().path.clone();
        app.world_mut().resource_mut::<LayoutSettings>().graph_path = Some(path);
        app.insert_resource(crate::InitialScene(initial));
        app.add_systems(Startup, init_graph);
        app.update();
        // The file contains the newer empty scene; its timestamp must not be
        // claimed by initialization of an older scene.
        assert_eq!(app.world().resource::<GraphLayout>().node_count, 0);
        assert_eq!(app.world().resource::<GraphSceneEpoch>().0, 1);
    }

    #[test]
    fn changed_file_reloads_once_and_preserves_existing_positions() {
        let (tmp, mut app) = fixture();
        let path = tmp.path().join("graph.json");
        let mut graph = Hypergraph::new();
        graph.vertices.push(Vertex::new("v", "Node"));
        save_json(&path, &graph).unwrap();
        // Force a pending version independently of filesystem timestamp granularity.
        app.world_mut().resource_mut::<GraphWatchState>().last_mtime = None;
        app.update();
        assert_eq!(app.world().resource::<GraphLayout>().node_count, 1);
        assert_eq!(app.world().resource::<GraphSceneEpoch>().0, 1);
        let position = app.world().resource::<GraphLayout>().layout.positions[0];
        graph.vertices[0].label = "Renamed".into();
        save_json(&path, &graph).unwrap();
        app.world_mut().resource_mut::<GraphWatchState>().last_mtime = None;
        app.update();
        assert_eq!(app.world().resource::<GraphSceneEpoch>().0, 2);
        assert_eq!(
            app.world().resource::<GraphLayout>().layout.positions[0],
            position
        );
        app.update();
        assert_eq!(app.world().resource::<GraphSceneEpoch>().0, 2);
    }

    #[test]
    fn invalid_file_keeps_scene_and_retries_the_same_stamp() {
        let (tmp, mut app) = fixture();
        let path = tmp.path().join("graph.json");
        std::fs::write(&path, "partial JSON").unwrap();
        app.world_mut().resource_mut::<GraphWatchState>().last_mtime = None;
        app.update();
        assert_eq!(app.world().resource::<GraphSceneEpoch>().0, 0);
        assert!(
            app.world()
                .resource::<GraphWatchState>()
                .last_mtime
                .is_none()
        );
        let mut graph = Hypergraph::new();
        graph.vertices.push(Vertex::new("v", "Node"));
        save_json(&path, &graph).unwrap();
        app.update();
        assert_eq!(app.world().resource::<GraphSceneEpoch>().0, 1);
    }
}
