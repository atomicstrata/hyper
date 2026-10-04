//! Native 3D interactive hypergraph viewer (Bevy 0.18 + egui).
//!
//! Optional companion to [`hyper_viz`]. Hosts that only need schema, projection,
//! or headless layout should depend on `hyper-viz` and skip this crate.
//!
//! # Standalone window
//!
//! ```no_run
//! use hyper_viz::Hypergraph;
//! use hyper_viz_bevy::run_from_graph;
//!
//! let graph = Hypergraph::new()
//!     .vertex("a", "A", "person")
//!     .vertex("b", "B", "person")
//!     .hyperedge("ab", ["a", "b"], "pair");
//! run_from_graph(graph);
//! ```
//!
//! # Embed in an existing Bevy `App`
//!
//! Add [`HyperVisualizerPlugin`] **after** `DefaultPlugins`. The plugin owns the
//! orbit camera, egui HUD, picking, and layout systems.
//!
//! ```no_run
//! use bevy::prelude::*;
//! use hyper_viz::{Hypergraph, Projection};
//! use hyper_viz_bevy::HyperVisualizerPlugin;
//!
//! let scene = Hypergraph::new().project(Projection::Bipartite);
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(HyperVisualizerPlugin::from_scene(scene))
//!     .run();
//! ```

mod animation;
mod camera;
mod focus;
mod graph;
mod hyperedge_hull;
mod inspect;
mod interaction;
mod node_visual;
mod pick;
mod render;
mod search;
mod session;
pub use search::{SearchPage, SearchProvider, SearchRow};
#[cfg(feature = "benchmark")]
pub mod benchmark;
pub mod showcase;
mod ui;

pub use render::LinkRenderSettings;

#[cfg(not(target_arch = "wasm32"))]
use bevy::app::TerminalCtrlCHandlerPlugin;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::winit::{EventLoopProxyWrapper, WinitUserEvent};
use hyper_viz::{Hypergraph, HypergraphScene, Projection, project};
use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::time::Duration;

/// Window title and size used by the standalone [`run_visualizer`] helpers.
#[derive(Clone, Debug)]
pub struct VisualizerConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
}

impl Default for VisualizerConfig {
    fn default() -> Self {
        Self {
            title: "Hypergraph".to_string(),
            width: 1600,
            height: 900,
        }
    }
}

impl VisualizerConfig {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Self::default()
        }
    }

    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }
}

/// Scene injected before startup. Hosts can also insert this resource themselves.
#[derive(Resource, Clone)]
pub struct InitialScene(pub HypergraphScene);

/// When set by an embedding host, Ctrl-C on another thread exits the Bevy app after flush.
#[derive(Resource, Clone)]
pub struct ExternalShutdown(pub Arc<AtomicBool>);

/// Bevy plugin: force layout, rendering, picking, and HUD.
///
/// Does **not** add `DefaultPlugins`. Standalone helpers ([`run_visualizer`])
/// install those for you.
pub struct HyperVisualizerPlugin {
    settings: graph::LayoutSettings,
    scene: Option<HypergraphScene>,
    watch: bool,
    live_rx: Arc<std::sync::Mutex<Option<Receiver<HypergraphScene>>>>,
    shutdown: Option<Arc<AtomicBool>>,
    search: Option<Vec<(String, SearchProvider)>>,
}

impl HyperVisualizerPlugin {
    /// Visualize an already-projected scene.
    pub fn from_scene(scene: HypergraphScene) -> Self {
        Self {
            settings: graph::LayoutSettings::default(),
            scene: Some(scene),
            watch: false,
            live_rx: Arc::new(std::sync::Mutex::new(None)),
            shutdown: None,
            search: None,
        }
    }

    /// Load native or HIF JSON from `path`. Set `watch` to reload on file change.
    pub fn from_path(path: impl Into<String>, watch: bool) -> Self {
        let path = path.into();
        Self {
            settings: graph::LayoutSettings {
                graph_path: Some(path),
                watch,
                ..Default::default()
            },
            scene: None,
            watch,
            live_rx: Arc::new(std::sync::Mutex::new(None)),
            shutdown: None,
            search: None,
        }
    }

    /// Select the format for initial loading and every watched reload.
    pub fn with_input_format(mut self, format: hyper_viz::InputFormat) -> Self {
        self.settings.input_format = format;
        self
    }

    pub fn with_projection(mut self, projection: Projection) -> Self {
        self.settings.projection = projection;
        self
    }

    /// Push replacement scenes from any host thread (`try_recv` each frame).
    pub fn with_live(self, rx: Receiver<HypergraphScene>) -> Self {
        *self.live_rx.lock().expect("live scene receiver lock") = Some(rx);
        self
    }

    fn with_search(mut self, provider: Vec<(String, SearchProvider)>) -> Self {
        self.search = Some(provider);
        self
    }

    /// Host thread sets this flag to request a clean `AppExit`.
    pub fn with_shutdown(mut self, flag: Arc<AtomicBool>) -> Self {
        self.shutdown = Some(flag);
        self
    }
}

impl Plugin for HyperVisualizerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.settings.clone())
            .init_resource::<graph::GraphSceneEpoch>()
            .add_plugins(focus::FocusPlugin)
            .add_plugins(camera::CameraPlugin)
            .add_plugins(render::RenderPlugin)
            .add_plugins(animation::StatusAnimationPlugin)
            .add_plugins(hyperedge_hull::HyperedgeHullPlugin)
            .add_plugins(ui::UiPlugin)
            .add_plugins(interaction::InteractionPlugin)
            .add_plugins(session::SessionPlugin)
            .add_systems(Startup, graph::init_graph)
            .add_systems(
                Update,
                graph::step_layout.run_if(resource_exists::<graph::GraphLayout>),
            );

        if self.watch {
            if let Some(path) = self.settings.graph_path.clone() {
                app.insert_resource(graph::GraphWatchState {
                    path,
                    last_mtime: None,
                });
            }
            app.add_systems(PreUpdate, graph::poll_graph_watch);
        }

        if let Some(rx) = self
            .live_rx
            .lock()
            .expect("live scene receiver lock")
            .take()
        {
            app.insert_resource(graph::LiveSceneReceiver(std::sync::Mutex::new(rx)));
            app.add_systems(PreUpdate, graph::poll_live_scene);
        }

        if let Some(provider) = &self.search {
            app.insert_resource(search::SearchState::new(
                provider.clone(),
                self.scene.clone(),
            ));
            app.add_systems(bevy_egui::EguiPrimaryContextPass, search::search_panel);
        }

        if let Some(flag) = self.shutdown.clone() {
            app.insert_resource(ExternalShutdown(flag));
            app.add_systems(PostStartup, start_shutdown_watcher);
            app.add_systems(Update, poll_external_shutdown);
        }

        if let Some(scene) = self.scene.clone() {
            app.insert_resource(InitialScene(scene));
        }
    }
}

/// Blocking native window with default config.
pub fn run_visualizer(scene: HypergraphScene) {
    run_visualizer_with(scene, VisualizerConfig::default());
}

pub fn run_visualizer_with(scene: HypergraphScene, config: VisualizerConfig) {
    visualizer_app_with(HyperVisualizerPlugin::from_scene(scene), config, false).run();
}

pub fn run_visualizer_from_path(path: String, watch: bool) {
    run_visualizer_from_path_with(
        path,
        watch,
        Projection::Bipartite,
        VisualizerConfig::default(),
    );
}

pub fn run_visualizer_from_path_with(
    path: String,
    watch: bool,
    projection: Projection,
    config: VisualizerConfig,
) {
    run_visualizer_from_path_with_format(
        path,
        watch,
        projection,
        config,
        hyper_viz::InputFormat::Auto,
    );
}

/// File viewer with an explicit interchange format, including watched updates.
pub fn run_visualizer_from_path_with_format(
    path: String,
    watch: bool,
    projection: Projection,
    config: VisualizerConfig,
    format: hyper_viz::InputFormat,
) {
    visualizer_app_with(
        HyperVisualizerPlugin::from_path(path, watch)
            .with_projection(projection)
            .with_input_format(format),
        config,
        false,
    )
    .run();
}

/// Native viewer with optional host-backed search outside the rendered scene.
pub fn run_visualizer_live_with_search(
    initial: HypergraphScene,
    rx: Receiver<HypergraphScene>,
    shutdown: Option<Arc<AtomicBool>>,
    provider: SearchProvider,
) {
    run_visualizer_live_with_search_modes(initial, rx, shutdown, vec![("Search".into(), provider)]);
}

/// Native viewer with selectable host retrieval modes, executed off the render thread.
pub fn run_visualizer_live_with_search_modes(
    initial: HypergraphScene,
    rx: Receiver<HypergraphScene>,
    shutdown: Option<Arc<AtomicBool>>,
    providers: Vec<(String, SearchProvider)>,
) {
    let mut plugin = HyperVisualizerPlugin::from_scene(initial)
        .with_live(rx)
        .with_search(providers);
    if let Some(flag) = shutdown {
        plugin = plugin.with_shutdown(flag);
    }
    visualizer_app_with(plugin, VisualizerConfig::default(), true).run();
}

/// Run the viewer with in-process scene updates (any host can push scenes).
pub fn run_visualizer_live(
    initial: HypergraphScene,
    rx: Receiver<HypergraphScene>,
    shutdown: Option<Arc<AtomicBool>>,
) {
    let mut plugin = HyperVisualizerPlugin::from_scene(initial).with_live(rx);
    if let Some(flag) = shutdown {
        plugin = plugin.with_shutdown(flag);
    }
    visualizer_app_with(plugin, VisualizerConfig::default(), true).run();
}

pub fn run_from_graph(graph: Hypergraph) {
    let scene = project(&graph, Projection::Bipartite);
    run_visualizer(scene);
}

/// Build a standalone Bevy `App` (window + visualizer) without running it.
///
/// Useful when a host wants to insert extra plugins or resources first.
pub fn visualizer_app(scene: HypergraphScene) -> App {
    visualizer_app_with(
        HyperVisualizerPlugin::from_scene(scene),
        VisualizerConfig::default(),
        false,
    )
}

fn visualizer_app_with(
    plugin: HyperVisualizerPlugin,
    config: VisualizerConfig,
    quiet_bevy: bool,
) -> App {
    let (width, height) = session::saved_window_size().unwrap_or((config.width, config.height));
    let window = WindowPlugin {
        primary_window: Some(Window {
            title: config.title,
            resolution: (width, height).into(),
            present_mode: bevy::window::PresentMode::AutoVsync,
            #[cfg(target_arch = "wasm32")]
            canvas: Some("#mind-canvas".into()),
            #[cfg(target_arch = "wasm32")]
            fit_canvas_to_parent: true,
            #[cfg(target_arch = "wasm32")]
            prevent_default_event_handling: true,
            ..default()
        }),
        ..default()
    };

    let mut plugins = DefaultPlugins.set(window);
    if quiet_bevy {
        plugins = plugins.disable::<LogPlugin>();
        #[cfg(not(target_arch = "wasm32"))]
        {
            plugins = plugins.disable::<TerminalCtrlCHandlerPlugin>();
        }
    }

    let mut app = App::new();
    app.add_plugins(plugins)
        .insert_resource(ClearColor(Color::srgb(0.05, 0.05, 0.08)))
        .add_plugins(plugin);
    app
}

/// Unpark winit when a host thread sets the shutdown flag (Bevy otherwise waits on window events).
fn start_shutdown_watcher(
    flag: Res<ExternalShutdown>,
    proxy: Res<EventLoopProxyWrapper>,
    mut started: Local<bool>,
) {
    if *started {
        return;
    }
    *started = true;

    let flag = Arc::clone(&flag.0);
    let event_proxy = clone_event_proxy(proxy.deref());
    std::thread::spawn(move || {
        while !flag.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(50));
        }
        while flag.load(Ordering::Relaxed) {
            let _ = event_proxy.send_event(WinitUserEvent::WakeUp);
            std::thread::sleep(Duration::from_millis(50));
        }
    });
}

fn clone_event_proxy(
    proxy: &winit::event_loop::EventLoopProxy<WinitUserEvent>,
) -> winit::event_loop::EventLoopProxy<WinitUserEvent> {
    proxy.clone()
}

fn poll_external_shutdown(
    flag: Res<ExternalShutdown>,
    mut exit: MessageWriter<AppExit>,
    mut done: Local<bool>,
) {
    if *done || !flag.0.load(Ordering::Relaxed) {
        return;
    }
    exit.write(AppExit::Success);
    *done = true;
}

#[cfg(test)]
mod plugin_api {
    use super::*;
    use hyper_viz::{Hypergraph, Projection};

    #[test]
    fn plugin_constructs_from_scene_and_path() {
        let scene = Hypergraph::new().project(Projection::Bipartite);
        let _from_scene = HyperVisualizerPlugin::from_scene(scene);
        let _from_path = HyperVisualizerPlugin::from_path("graph.json", true)
            .with_projection(Projection::CliqueExpansion);
    }
}
