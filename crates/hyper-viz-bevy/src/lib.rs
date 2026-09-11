pub mod camera;
pub mod graph;
pub mod hyperedge_hull;
pub mod interaction;
pub mod node_visual;
pub mod pick;
pub mod render;
pub mod ui;

use bevy::app::TerminalCtrlCHandlerPlugin;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::winit::{EventLoopProxyWrapper, WinitUserEvent};
use hyper_viz::{Hypergraph, HypergraphScene, Projection, project};
use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

#[derive(Resource)]
pub struct InitialScene(pub HypergraphScene);

/// When set by an embedding host, Ctrl-C on another thread exits the Bevy app after flush.
#[derive(Resource)]
pub struct ExternalShutdown(pub Arc<AtomicBool>);

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

pub fn run_visualizer(scene: HypergraphScene) {
    run_visualizer_with(scene, VisualizerConfig::default());
}

pub fn run_visualizer_with(scene: HypergraphScene, config: VisualizerConfig) {
    let settings = graph::LayoutSettings::default();
    build_app(settings, Some(scene), false, None, false, None, config).run();
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
    let settings = graph::LayoutSettings {
        graph_path: Some(path),
        watch,
        projection,
        ..Default::default()
    };
    build_app(settings, None, watch, None, false, None, config).run();
}

/// Run the viewer with in-process scene updates (any host can push scenes).
pub fn run_visualizer_live(
    initial: HypergraphScene,
    rx: std::sync::mpsc::Receiver<HypergraphScene>,
    shutdown: Option<Arc<AtomicBool>>,
) {
    let settings = graph::LayoutSettings::default();
    build_app(
        settings,
        Some(initial),
        false,
        Some(std::sync::Mutex::new(rx)),
        true,
        shutdown,
        VisualizerConfig::default(),
    )
    .run();
}

pub fn run_from_graph(graph: Hypergraph) {
    let scene = project(&graph, Projection::Bipartite);
    run_visualizer(scene);
}

#[allow(clippy::too_many_arguments)]
fn build_app(
    settings: graph::LayoutSettings,
    scene: Option<HypergraphScene>,
    watch: bool,
    live_rx: Option<std::sync::Mutex<std::sync::mpsc::Receiver<HypergraphScene>>>,
    embedded: bool,
    shutdown: Option<Arc<AtomicBool>>,
    config: VisualizerConfig,
) -> App {
    let window = WindowPlugin {
        primary_window: Some(Window {
            title: config.title,
            resolution: (config.width, config.height).into(),
            present_mode: bevy::window::PresentMode::AutoVsync,
            ..default()
        }),
        ..default()
    };

    let mut plugins = DefaultPlugins.set(window);
    if embedded {
        plugins = plugins
            .disable::<LogPlugin>()
            .disable::<TerminalCtrlCHandlerPlugin>();
    }

    let mut app = App::new();
    app.add_plugins(plugins)
        .insert_resource(ClearColor(Color::srgb(0.05, 0.05, 0.08)))
        .insert_resource(settings)
        .init_resource::<graph::GraphSceneEpoch>()
        .add_plugins(camera::CameraPlugin)
        .add_plugins(render::RenderPlugin)
        .add_plugins(hyperedge_hull::HyperedgeHullPlugin)
        .add_plugins(ui::UiPlugin)
        .add_plugins(interaction::InteractionPlugin)
        .add_systems(Startup, graph::init_graph)
        .add_systems(
            Update,
            graph::step_layout.run_if(resource_exists::<graph::GraphLayout>),
        );

    if watch {
        if let Some(path) = app
            .world()
            .resource::<graph::LayoutSettings>()
            .graph_path
            .clone()
        {
            app.insert_resource(graph::GraphWatchState {
                path,
                last_mtime: None,
            });
        }
        app.add_systems(Update, graph::poll_graph_watch);
    }

    if let Some(rx) = live_rx {
        app.insert_resource(graph::LiveSceneReceiver(rx));
        app.add_systems(Update, graph::poll_live_scene);
    }

    if let Some(flag) = shutdown {
        app.insert_resource(ExternalShutdown(flag));
        app.add_systems(PostStartup, start_shutdown_watcher);
        app.add_systems(Update, poll_external_shutdown);
    }

    if let Some(scene) = scene {
        app.insert_resource(InitialScene(scene));
    }

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
