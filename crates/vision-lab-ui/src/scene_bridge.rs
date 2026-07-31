use js_sys::Promise;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::HtmlCanvasElement;

#[wasm_bindgen(module = "vision-lab-scene-bridge")]
extern "C" {
    #[wasm_bindgen(js_name = initializeScene)]
    fn initialize_scene_js(canvas: &HtmlCanvasElement) -> Promise;

    #[wasm_bindgen(js_name = loadDefaultModel)]
    fn load_default_js(generation: f64) -> Promise;

    #[wasm_bindgen(js_name = beginModelReplacement)]
    fn begin_replacement_js(generation: f64);

    #[wasm_bindgen(js_name = loadModelFiles)]
    fn load_files_js(files: &JsValue, generation: f64) -> Promise;

    #[wasm_bindgen(js_name = resizeScene)]
    pub fn resize();

    #[wasm_bindgen(js_name = getSceneFps)]
    pub fn get_fps() -> f64;

    #[wasm_bindgen(js_name = disposeScene)]
    pub fn dispose();
}

pub fn begin_replacement(generation: u64) {
    begin_replacement_js(generation as f64);
}

pub async fn initialize(canvas: &HtmlCanvasElement) -> Result<(), JsValue> {
    JsFuture::from(initialize_scene_js(canvas)).await?;
    Ok(())
}

pub async fn load_default(generation: u64) -> Result<(), JsValue> {
    JsFuture::from(load_default_js(generation as f64)).await?;
    Ok(())
}

pub async fn load_files(files: &JsValue, generation: u64) -> Result<(), JsValue> {
    JsFuture::from(load_files_js(files, generation as f64)).await?;
    Ok(())
}
