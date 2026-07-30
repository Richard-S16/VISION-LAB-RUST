pub mod smoothing;
pub mod state;

#[cfg(target_arch = "wasm32")]
mod app;
#[cfg(target_arch = "wasm32")]
mod capture;
#[cfg(target_arch = "wasm32")]
mod controls;
#[cfg(target_arch = "wasm32")]
mod ipc;
#[cfg(target_arch = "wasm32")]
mod overlay;
#[cfg(target_arch = "wasm32")]
mod scene_bridge;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    wasm_bindgen_futures::spawn_local(async {
        if let Err(error) = app::boot().await {
            web_sys::console::error_1(&error);
        }
    });
}
