use js_sys::{Function, Promise, Uint8Array};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use crate::state::InFlightRequest;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectorInfo {
    pub provider: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Detection {
    pub label_id: u32,
    pub label: String,
    pub score: f32,
    pub bounding_box: BoundingBox,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectionTiming {
    pub native_total_ms: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectionResponse {
    pub request_id: u64,
    pub model_generation: u64,
    pub detections: Vec<Detection>,
    pub timing: DetectionTiming,
}

#[wasm_bindgen(module = "vision-lab-tauri-bridge")]
extern "C" {
    #[wasm_bindgen(js_name = initializeDetector)]
    fn initialize_detector_js(on_event: &Function) -> Promise;

    #[wasm_bindgen(js_name = detectFrame)]
    fn detect_frame_js(
        frame: &Uint8Array,
        request_id: f64,
        model_generation: f64,
        threshold: f32,
    ) -> Promise;
}

pub async fn initialize(on_event: &Function) -> Result<DetectorInfo, JsValue> {
    let value = JsFuture::from(initialize_detector_js(on_event)).await?;
    serde_wasm_bindgen::from_value(value).map_err(|error| JsValue::from_str(&error.to_string()))
}

pub async fn detect(
    frame: &[u8],
    request: InFlightRequest,
    threshold: f32,
) -> Result<DetectionResponse, JsValue> {
    let bytes = Uint8Array::from(frame);
    let value = JsFuture::from(detect_frame_js(
        &bytes,
        request.request_id as f64,
        request.model_generation as f64,
        threshold,
    ))
    .await?;
    serde_wasm_bindgen::from_value(value).map_err(|error| JsValue::from_str(&error.to_string()))
}
