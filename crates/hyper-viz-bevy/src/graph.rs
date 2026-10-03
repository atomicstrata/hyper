use std::collections::HashMap;
use std::time::SystemTime;

use bevy::prelude::*;
use hyper_viz::{
    ForceLayout3D, HypergraphScene, InputFormat, LayoutConfig, Projection, load_json_with_format,
    project, scenes_equivalent,
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
    pub last_steps: usize,
    pub last_step_ms: f64,
    pub positions_revision: u64,
    pub initialization_ms: f64,
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
    pub input_format: InputFormat,
    pub view_mode: hyper_viz::session::ViewMode,
    pub initial_module: Option<String>,
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
            input_format: InputFormat::Auto,
            view_mode: hyper_viz::session::ViewMode::Auto,
            initial_module: None,
        }
    }
}

fn layout_from_path(
    path: &str,
    settings: &LayoutSettings,
) -> Result<GraphLayout, hyper_viz::VizError> {
    let graph = load_json_with_format(path, settings.input_format)?;
    let scene = project(&graph, settings.projection);
    Ok(GraphLayout::from_scene(scene, settings))
}

impl GraphLayout {
    pub fn from_scene(scene: HypergraphScene, settings: &LayoutSettings) -> Self {
        let large =
            scene.vertices_count() >= 1000 && !hyper_viz::DependencyIndex::new(&scene).is_empty();
        let mut config = settings.config.clone();
        if large {
            config.gravity = 0.001;
            config.centroid_attraction = 0.0005;
        }
        let layout = ForceLayout3D::from_scene(&scene, config);
        let link_count = layout.edges.len();
        let node_count = scene.node_count();
        let hub_status = hub_status_from_scene(&scene);

        let mut result = Self {
            layout,
            node_count,
            link_count,
            running: true,
            iterations_per_frame: settings.iterations_per_frame,
            scene,
            hub_status,
            last_steps: 0,
            last_step_ms: 0.,
            positions_revision: 0,
            initialization_ms: 0.,
        };
        if large {
            seed_neutral(&mut result);
        }
        result
    }

    /// Keep every vertex and hyperedge available in a stable spatial overview.
    pub fn full_graph_overview(
        &mut self,
        settings: &mut LayoutSettings,
        hulls: &mut crate::hyperedge_hull::HyperedgeHullSettings,
        lines: &mut crate::render::LinkRenderSettings,
    ) {
        settings.config.gravity = 0.00001;
        settings.config.topology = hyper_viz::TopologySettings {
            model: hyper_viz::LayoutModel::Normalized,
            ..Default::default()
        };
        settings.node_size = 8.;
        settings.config.centroid_attraction = 0.0005;
        settings.config.repulsion = 500.;
        settings.config.dt = 0.3;
        settings.config.damping = 0.85;
        self.layout.config = settings.config.clone();
        self.running = false;
        hulls.enabled = true;
        hulls.opacity = 0.001;
        hulls.wireframe = false;
        hulls.hide_hubs = true;
        lines.max_lines = 0;
        lines.opacity = 0.06;
    }

    /// Connectivity initialization plus bounded refinement, independent of labels.
    pub fn rebuild_structural(&mut self) {
        let start = std::time::Instant::now();
        self.layout.seed_from_topology(&self.scene, 256);
        for _ in 0..64 {
            self.layout.step();
        }
        self.initialization_ms = start.elapsed().as_secs_f64() * 1000.;
        self.positions_revision += 1;
        self.running = false;
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
            last_steps: 0,
            last_step_ms: 0.,
            positions_revision: 0,
            initialization_ms: old.initialization_ms,
            scene,
        }
    }
}

fn hub_status_from_scene(scene: &HypergraphScene) -> Vec<Option<String>> {
    let mut hub_status = vec![None; scene.node_count()];
    for he in &scene.hyperedges {
        if let Some(hub) = he.hub_index
            && let Some(slot) = hub_status.get_mut(hub)
        {
            *slot = Some(he.status.clone());
        }
    }
    hub_status
}

#[allow(clippy::too_many_arguments)]
pub fn init_graph(
    mut commands: Commands,
    mut settings: ResMut<LayoutSettings>,
    mut hulls: Option<ResMut<crate::hyperedge_hull::HyperedgeHullSettings>>,
    mut lines: Option<ResMut<crate::render::LinkRenderSettings>>,
    showcase: Option<Res<crate::showcase::ShowcaseConfig>>,
    session: Option<Res<crate::session::SessionStore>>,
    initial: Option<Res<crate::InitialScene>>,
    mut watch: Option<ResMut<GraphWatchState>>,
) {
    let mut layout = if let Some(init) = initial {
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
    settings.config = layout.layout.config.clone();
    if showcase.is_none()
        && layout.node_count >= 1000
        && layout.scene.links.is_empty()
        && !hyper_viz::DependencyIndex::new(&layout.scene).is_empty()
        && let (Some(hulls), Some(lines)) = (hulls.as_deref_mut(), lines.as_deref_mut())
    {
        layout.full_graph_overview(&mut settings, hulls, lines);
        let saved = session
            .as_ref()
            .filter(|store| store.enabled && store.has_saved_session);
        if let Some(store) = saved {
            crate::session::apply_layout_prefs(&store.session.prefs.layout, &mut settings);
            layout.layout.config = settings.config.clone();
            layout.iterations_per_frame = settings.iterations_per_frame;
        }
        layout.rebuild_structural();
        if let Some(store) = saved {
            layout.running = store.session.prefs.layout.running;
        }
    }
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

    let graph = match load_json_with_format(&watch.path, settings.input_format) {
        Ok(graph) => graph,
        Err(error) => {
            tracing::warn!(path = %watch.path, %error, "retaining scene after failed reload");
            return;
        }
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
        layout.last_steps = 0;
        layout.last_step_ms = 0.;
        return;
    }

    let start = std::time::Instant::now();
    let mut steps = 0;
    for _ in 0..layout.iterations_per_frame.max(1) {
        layout.layout.step();
        steps += 1;
        if start.elapsed().as_secs_f64() >= 0.008 {
            break;
        }
    }
    layout.last_steps = steps;
    layout.last_step_ms = start.elapsed().as_secs_f64() * 1000.;
}

/// Neutral stable-ID positions for force-model comparisons.
pub fn seed_neutral(layout: &mut GraphLayout) {
    let seed = |s: &str| {
        let h = s.bytes().fold(0xcbf29ce484222325_u64, |h, b| {
            (h ^ b as u64).wrapping_mul(0x100000001b3)
        });
        let c = |shift| (((h >> shift) & 0xffff_u64) as f32 / 32767.5) - 1.;
        hyper_viz::Vec3::new(c(0), c(16), c(32))
    };
    for (i, node) in layout.scene.nodes.iter().enumerate() {
        layout.layout.positions[i] = seed(&node.id) * 500.;
        layout.layout.velocities[i] = hyper_viz::Vec3::default();
    }
    // Mark a position revision for hulls even when the simulation is paused.
    layout.positions_revision += 1;
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
    fn hif_watch_obeys_explicit_format_and_keeps_scene_on_incompatible_update() {
        let (tmp, mut app) = fixture();
        let path = tmp.path().join("graph.json");
        std::fs::write(&path, r#"{"incidences":[{"node":"n","edge":"e"}]}"#).unwrap();
        app.world_mut()
            .resource_mut::<LayoutSettings>()
            .input_format = hyper_viz::InputFormat::Hif;
        app.world_mut().resource_mut::<GraphWatchState>().last_mtime = None;
        app.update();
        assert_eq!(
            app.world().resource::<GraphLayout>().scene.vertices_count(),
            1
        );
        assert_eq!(app.world().resource::<GraphSceneEpoch>().0, 1);
        std::fs::write(&path, r#"{"network-type":"directed","incidences":[]}"#).unwrap();
        app.world_mut().resource_mut::<GraphWatchState>().last_mtime = None;
        app.update();
        assert_eq!(app.world().resource::<GraphSceneEpoch>().0, 1);
        assert_eq!(
            app.world().resource::<GraphLayout>().scene.vertices_count(),
            1
        );
    }

    #[test]
    fn large_import_graph_starts_with_all_geometry_in_a_paused_overview() {
        let mut graph = Hypergraph::new();
        for n in 0..1000 {
            let id = n.to_string();
            graph = graph.vertex(&id, &id, "module");
        }
        for n in 1..1000 {
            let id = n.to_string();
            let mut edge =
                hyper_viz::Hyperedge::new(format!("import:{n}"), ["0", &id]).with_kind("import");
            edge.attrs.insert("source".into(), "0".into());
            edge.attrs.insert("target".into(), id.into());
            graph.add_hyperedge(edge);
        }
        graph = graph.hyperedge("group", ["0", "1", "2"], "Group");
        let mut app = App::new();
        app.init_resource::<LayoutSettings>()
            .init_resource::<crate::render::LinkRenderSettings>()
            .init_resource::<crate::hyperedge_hull::HyperedgeHullSettings>()
            .insert_resource(crate::InitialScene(graph.project(Projection::StarCentroid)))
            .add_systems(Startup, init_graph);
        app.update();
        let layout = app.world().resource::<GraphLayout>();
        let lines = app.world().resource::<crate::render::LinkRenderSettings>();
        let hulls = app
            .world()
            .resource::<crate::hyperedge_hull::HyperedgeHullSettings>();
        assert!(
            !layout.running,
            "Full geometry remains stable until Run is chosen"
        );
        assert!(hulls.enabled, "No higher-arity hyperedges hidden");
        assert!(!hulls.wireframe);
        assert!(hulls.opacity > 0. && hulls.opacity <= 0.01);
        let (visible, omitted) = crate::spatial_visibility::visible_lines(
            &layout.scene,
            &crate::focus::FocusScope::default(),
            hulls.hide_hubs,
            lines.budget(),
        );
        assert_eq!(visible.len(), 999);
        assert_eq!(omitted, 0);
        assert_eq!(layout.node_count, 1000);
        assert_eq!(
            layout.layout.config.topology.model,
            hyper_viz::LayoutModel::Normalized
        );
        assert_eq!(
            layout.iterations(),
            64,
            "Only actual refinement steps count"
        );
        assert!(layout.initialization_ms > 0.);
        assert!(
            layout
                .layout
                .positions
                .iter()
                .all(|p| p.x.is_finite() && p.y.is_finite() && p.z.is_finite())
        );
    }

    #[test]
    fn tour_initialization_keeps_explicit_line_opacity() {
        let mut graph = Hypergraph::new();
        for n in 0..1000 {
            let id = n.to_string();
            graph = graph.vertex(&id, &id, "module");
        }
        let mut edge = hyper_viz::Hyperedge::new("import", ["0", "1"]).with_kind("import");
        edge.attrs.insert("source".into(), "0".into());
        edge.attrs.insert("target".into(), "1".into());
        graph.add_hyperedge(edge);
        let mut app = App::new();
        app.init_resource::<LayoutSettings>()
            .insert_resource(crate::InitialScene(graph.project(Projection::StarCentroid)))
            .insert_resource(crate::render::LinkRenderSettings {
                max_lines: 0,
                opacity: 0.10,
            })
            .insert_resource(crate::showcase::ShowcaseConfig {
                frames: 1,
                fps: 30,
                warmup: 0,
                capture: None,
                report: Default::default(),
                live_layout: false,
            })
            .add_systems(Startup, init_graph);
        app.update();
        assert_eq!(
            app.world()
                .resource::<crate::render::LinkRenderSettings>()
                .opacity,
            0.10
        );
    }

    #[test]
    fn structural_rebuild_and_reload_preserve_complete_connectivity() {
        let graph = Hypergraph::new()
            .vertex("a", "a", "subject-one")
            .vertex("b", "b", "subject-two")
            .vertex("c", "c", "subject-three")
            .hyperedge("e", ["a", "b", "c"], "e");
        let mut settings = LayoutSettings::default();
        settings.config.topology.model = hyper_viz::LayoutModel::Normalized;
        let scene = graph.project(Projection::StarCentroid);
        let mut layout = GraphLayout::from_scene(scene.clone(), &settings);
        layout.rebuild_structural();
        let positions = layout.layout.positions.clone();
        let mut renamed = scene.clone();
        for node in &mut renamed.nodes {
            node.kind = "unrelated".into();
            node.label = "renamed".into();
        }
        let mut other = GraphLayout::from_scene(renamed.clone(), &settings);
        other.rebuild_structural();
        assert_eq!(
            other.layout.positions, positions,
            "Subject metadata does not create structure"
        );
        let reloaded = GraphLayout::from_scene_preserve(&layout, renamed, &settings);
        assert_eq!(reloaded.layout.positions, positions);
        assert_eq!(reloaded.layout.config.topology, settings.config.topology);
        assert_eq!(reloaded.scene.hyperedges.len(), 1);
        assert_eq!(reloaded.node_count, 3);
        assert!(!reloaded.running);
    }

    #[test]
    fn running_layout_steps_at_least_once_and_reports_actual_work() {
        let scene = Hypergraph::new()
            .vertex("a", "a", "vertex")
            .project(Projection::StarCentroid);
        let mut layout = GraphLayout::from_scene(scene, &LayoutSettings::default());
        layout.iterations_per_frame = 0;
        let mut app = App::new();
        app.insert_resource(layout).add_systems(Update, step_layout);
        app.update();
        let layout = app.world().resource::<GraphLayout>();
        assert_eq!(layout.iterations(), 1);
        assert_eq!(layout.last_steps, 1);
        assert!(layout.last_step_ms >= 0.);
        app.world_mut().resource_mut::<GraphLayout>().running = false;
        app.update();
        assert_eq!(app.world().resource::<GraphLayout>().last_steps, 0);
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
