//! Each bindings factory owns one copy of these resources and one Bevy app.
use crate::bridge::{self, Command, SourceGraph};
use bevy::{
    log::LogPlugin,
    prelude::*,
    winit::{EventLoopProxyWrapper, WinitUserEvent},
};
use futures_util::future::{AbortHandle, Abortable};
use hyper_viz::{Hypergraph, Projection};
use hyper_viz_bevy::{
    HyperVisualizerPlugin,
    host::{ExternalUi, SessionConfig},
};
use js_sys::Function;
use std::{cell::RefCell, rc::Rc};
thread_local! { static PANIC_CALLBACK: RefCell<Option<Function>> = const { RefCell::new(None) }; }
use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use wasm_bindgen::prelude::*;

struct Request {
    id: u32,
    command: Command,
}
#[derive(Clone, Resource)]
struct Inbox(Arc<Mutex<VecDeque<Request>>>);

/// JS-owned handle. It never holds the Bevy world or a page-wide callback.
#[wasm_bindgen]
pub struct BrowserRuntime {
    inbox: Inbox,
    abort: AbortHandle,
    initializing: Arc<AtomicBool>,
    alive: Arc<AtomicBool>,
    proxy: Arc<Mutex<Option<winit::event_loop::EventLoopProxy<WinitUserEvent>>>>,
}

#[wasm_bindgen]
impl BrowserRuntime {
    pub fn send(&self, id: u32, raw: &str) -> Result<(), JsValue> {
        if !self.alive.load(Ordering::Relaxed) {
            return Err(JsValue::from_str("Viewer disposed"));
        }
        let command = serde_json::from_str(raw).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let mut queue = self
            .inbox
            .0
            .lock()
            .map_err(|_| JsValue::from_str("Command queue unavailable"))?;
        if queue.len() >= 64 {
            return Err(JsValue::from_str("Command queue full"));
        }
        queue.push_back(Request { id, command });
        if let Ok(proxy) = self.proxy.lock()
            && let Some(proxy) = proxy.as_ref()
        {
            let _ = proxy.send_event(WinitUserEvent::WakeUp);
        }
        Ok(())
    }

    /// Also usable while graphics initialization is still pending.
    pub fn stop(&self) {
        self.alive.store(false, Ordering::Relaxed);
        if self.initializing.load(Ordering::Relaxed) {
            self.abort.abort();
        }
        if let Ok(proxy) = self.proxy.lock()
            && let Some(proxy) = proxy.as_ref()
        {
            let _ = proxy.send_event(WinitUserEvent::WakeUp);
        }
    }
}

struct Callbacks {
    callback: Function,
    alive: Arc<AtomicBool>,
    ready: bool,
    last_selection: serde_json::Value,
    last_snapshot: String,
    elapsed: f32,
    replies: Vec<(u32, Result<(), String>)>,
}
impl Callbacks {
    fn emit(&self, value: serde_json::Value) {
        // Consumer exceptions must not unwind into the renderer.
        let _ = self
            .callback
            .call1(&JsValue::NULL, &JsValue::from_str(&value.to_string()));
    }
}
impl Drop for Callbacks {
    fn drop(&mut self) {
        self.alive.store(false, Ordering::Relaxed);
        PANIC_CALLBACK.with(|slot| *slot.borrow_mut() = None);
        self.emit(serde_json::json!({"type":"disposed"}));
    }
}

/// Start one app after the caller has instantiated this WASM module.
#[wasm_bindgen]
pub fn start(
    canvas_selector: String,
    callback: Function,
    session_key: Option<String>,
) -> Result<BrowserRuntime, JsValue> {
    PANIC_CALLBACK.with(|slot| *slot.borrow_mut() = Some(callback.clone()));
    std::panic::set_hook(Box::new(|info| {
        PANIC_CALLBACK.with(|slot| {
            if let Some(callback) = slot.borrow().as_ref() {
                let message = serde_json::json!({"type":"error", "error":info.to_string()});
                let _ = callback.call1(&JsValue::NULL, &JsValue::from_str(&message.to_string()));
            }
        });
        console_error_panic_hook::hook(info);
    }));
    use wasm_bindgen::JsCast;
    let canvas = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("Browser document unavailable"))?
        .query_selector(&canvas_selector)?
        .ok_or_else(|| JsValue::from_str("Canvas unavailable"))?
        .dyn_into::<web_sys::HtmlCanvasElement>()?;
    let inbox = Inbox(Arc::new(Mutex::new(VecDeque::new())));
    let alive = Arc::new(AtomicBool::new(true));
    let initializing = Arc::new(AtomicBool::new(true));
    let startup = Rc::new(RefCell::new(Some(callback)));
    let proxy = Arc::new(Mutex::new(None));
    let (abort, registration) = AbortHandle::new_pair();
    let handle = BrowserRuntime {
        inbox: inbox.clone(),
        alive: alive.clone(),
        proxy: proxy.clone(),
        abort,
        initializing: initializing.clone(),
    };
    // Initialize against this canvas before creating any Winit event loop.
    // WebGL2 needs a compatible surface; WebGPU failures remain cancellable.
    wasm_bindgen_futures::spawn_local(async move {
        let creation = match Abortable::new(prepare_renderer(canvas), registration).await {
            Ok(Ok(creation)) => creation,
            result => {
                if let Some(callback) = startup.borrow_mut().take() {
                    if let Ok(Err(error)) = result {
                        let event = serde_json::json!({"type":"error", "error":error});
                        let _ =
                            callback.call1(&JsValue::NULL, &JsValue::from_str(&event.to_string()));
                    }
                    PANIC_CALLBACK.with(|slot| *slot.borrow_mut() = None);
                    let _ = callback
                        .call1(&JsValue::NULL, &JsValue::from_str(r#"{"type":"disposed"}"#));
                }
                return;
            }
        };
        if !alive.load(Ordering::Relaxed) {
            if let Some(callback) = startup.borrow_mut().take() {
                PANIC_CALLBACK.with(|slot| *slot.borrow_mut() = None);
                let _ =
                    callback.call1(&JsValue::NULL, &JsValue::from_str(r#"{"type":"disposed"}"#));
            }
            return;
        }
        let Some(callback) = startup.borrow_mut().take() else {
            return;
        };
        let graph = Hypergraph::new();
        let window = WindowPlugin {
            primary_window: Some(Window {
                canvas: Some(canvas_selector),
                fit_canvas_to_parent: true,
                prevent_default_event_handling: true,
                resolution: (640u32, 480u32).into(),
                ..default()
            }),
            ..default()
        };
        let mut app = App::new();
        app.add_plugins(
            DefaultPlugins
                .set(window)
                .set(bevy::render::RenderPlugin {
                    render_creation: creation,
                    ..default()
                })
                .disable::<LogPlugin>(),
        )
        .insert_resource(SessionConfig {
            enabled: session_key.is_some(),
            storage_key: session_key,
            defer_initial_view: true,
        })
        .insert_resource(ClearColor(Color::srgb(0.05, 0.05, 0.08)))
        .add_plugins(HyperVisualizerPlugin::from_scene(
            graph.project(Projection::Bipartite),
        ))
        .insert_resource(SourceGraph(graph));
        app.world_mut().resource_mut::<ExternalUi>().enabled = true;
        app.insert_resource(inbox)
            .insert_non_send_resource(Callbacks {
                callback,
                alive,
                ready: false,
                last_selection: serde_json::Value::Null,
                last_snapshot: String::new(),
                elapsed: 0.,
                replies: Vec::new(),
            })
            .add_systems(Startup, install_device_loss)
            .add_systems(PreUpdate, pump)
            .add_systems(Last, publish);
        if let Ok(mut slot) = proxy.lock() {
            *slot = Some((**app.world().resource::<EventLoopProxyWrapper>()).clone());
        }
        initializing.store(false, Ordering::Relaxed);
        app.run();
    });
    Ok(handle)
}

async fn prepare_renderer(
    canvas: web_sys::HtmlCanvasElement,
) -> Result<bevy::render::settings::RenderCreation, String> {
    use bevy::render::{
        renderer::{
            RenderAdapter, RenderAdapterInfo, RenderDevice, RenderInstance, RenderQueue,
            WgpuWrapper,
        },
        settings::{RenderCreation, WgpuSettings},
    };
    let settings = WgpuSettings::default();
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: settings.backends.unwrap_or_default(),
        flags: settings.instance_flags,
        memory_budget_thresholds: settings.instance_memory_budget_thresholds,
        ..Default::default()
    });
    let surface = instance
        .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
        .map_err(|e| e.to_string())?;
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: settings.power_preference,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .map_err(|e| e.to_string())?;
    let info = adapter.get_info();
    // Match Bevy's default Functionality policy: use adapter-supported limits and
    // features, except primary mapped buffers on discrete adapters.
    let mut features = adapter.features();
    if info.device_type == wgpu::DeviceType::DiscreteGpu {
        features.remove(wgpu::Features::MAPPABLE_PRIMARY_BUFFERS);
    }
    features |= settings.features;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            required_features: features,
            required_limits: adapter.limits(),
            memory_hints: settings.memory_hints,
            ..Default::default()
        })
        .await
        .map_err(|e| e.to_string())?;
    Ok(RenderCreation::manual(
        RenderDevice::from(device),
        RenderQueue(Arc::new(WgpuWrapper::new(queue))),
        RenderAdapterInfo(WgpuWrapper::new(info)),
        RenderAdapter(Arc::new(WgpuWrapper::new(adapter))),
        RenderInstance(Arc::new(WgpuWrapper::new(instance))),
    ))
}

fn install_device_loss(device: Res<bevy::render::renderer::RenderDevice>) {
    device.wgpu_device().set_device_lost_callback(|reason, message| {
        PANIC_CALLBACK.with(|slot| {
            if let Some(callback) = slot.borrow().as_ref() {
                let event = serde_json::json!({"type":"error", "error":format!("GPU device lost ({reason:?}): {message}")});
                let _ = callback.call1(&JsValue::NULL, &JsValue::from_str(&event.to_string()));
            }
        });
    });
}

fn pump(world: &mut World) {
    let alive = world
        .non_send_resource::<Callbacks>()
        .alive
        .load(Ordering::Relaxed);
    if !alive {
        world.write_message(AppExit::Success);
        return;
    }
    let queue = world.resource::<Inbox>().clone();
    let requests: Vec<_> = queue
        .0
        .lock()
        .map(|mut q| q.drain(..).collect())
        .unwrap_or_default();
    // Winit routes pointer input per canvas; keyboard shortcuts additionally need focus.
    let focused = {
        let mut windows = world.query::<&Window>();
        windows.iter(world).any(|w| w.focused)
    };
    world.resource_mut::<ExternalUi>().keyboard_captured = !focused;
    for request in requests {
        let disposing = matches!(request.command, Command::Dispose);
        let result = bridge::apply(world, request.command);
        world
            .non_send_resource_mut::<Callbacks>()
            .replies
            .push((request.id, result));
        if disposing {
            world
                .non_send_resource::<Callbacks>()
                .alive
                .store(false, Ordering::Relaxed);
            break;
        }
    }
}

fn publish(world: &mut World) {
    let mut state = bridge::snapshot(world);
    if let Some(info) = world.get_resource::<bevy::render::renderer::RenderAdapterInfo>() {
        state["renderer"] = serde_json::json!({"backend":format!("{:?}",info.backend),
            "name":info.name, "driver":info.driver, "deviceType":format!("{:?}",info.device_type)});
    }
    let mut cameras = world.query::<&bevy_panorbit_camera::PanOrbitCamera>();
    if let Ok(camera) = cameras.single(world) {
        state["camera"] = serde_json::json!({
            "yaw": camera.yaw, "pitch":camera.pitch, "radius":camera.radius,
            "focus":camera.focus.to_array(),
            "target": {"yaw":camera.target_yaw, "pitch":camera.target_pitch,
                "radius":camera.target_radius, "focus":camera.target_focus.to_array()},
        });
    }
    let delta = world.resource::<Time>().delta_secs();
    let mut callbacks = world.non_send_resource_mut::<Callbacks>();
    if !callbacks.ready {
        callbacks.ready = true;
        callbacks.emit(serde_json::json!({"type":"ready", "state":state}));
    }
    if callbacks.last_selection != state["selection"] {
        callbacks.last_selection = state["selection"].clone();
        callbacks
            .emit(serde_json::json!({"type":"selectionchange", "selection": state["selection"]}));
    }
    for (id, result) in std::mem::take(&mut callbacks.replies) {
        callbacks.emit(match result {
            Ok(()) => serde_json::json!({"type":"ack", "id": id, "state":state}),
            Err(error) => serde_json::json!({"type":"ack", "id": id, "error":error}),
        });
    }
    callbacks.elapsed += delta;
    if callbacks.elapsed >= 0.2 {
        callbacks.elapsed = 0.;
        let json = state.to_string();
        if json != callbacks.last_snapshot {
            callbacks.last_snapshot = json;
            callbacks.emit(serde_json::json!({"type":"statechange", "state":state}));
        }
    }
}
