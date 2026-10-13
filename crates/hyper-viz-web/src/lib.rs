//! Browser SDK: portable command processing and isolated WASM viewer runtimes.

#[cfg(all(target_arch = "wasm32", feature = "webgpu", feature = "webgl2"))]
compile_error!("Select one renderer: webgpu or webgl2, using --no-default-features");
#[cfg(all(
    target_arch = "wasm32",
    not(any(feature = "webgpu", feature = "webgl2"))
))]
compile_error!("Select either webgpu or webgl2");

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
mod bridge;
#[cfg(target_arch = "wasm32")]
mod runtime;
