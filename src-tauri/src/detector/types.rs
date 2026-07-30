use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExecutionProvider {
    DirectMl,
    Cpu,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectionRequestMetadata {
    pub request_id: u64,
    pub model_generation: u64,
    pub width: u32,
    pub height: u32,
    pub threshold: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Detection {
    pub label_id: u32,
    pub label: String,
    pub score: f32,
    pub bounding_box: BoundingBox,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectionTiming {
    pub preprocess_ms: f64,
    pub inference_ms: f64,
    pub postprocess_ms: f64,
    pub native_total_ms: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectionResponse {
    pub request_id: u64,
    pub model_generation: u64,
    pub detections: Vec<Detection>,
    pub timing: DetectionTiming,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectorInfo {
    pub provider: ExecutionProvider,
    pub provider_fallback: Option<String>,
    pub model_repository: String,
    pub model_revision: String,
    pub model_checksum_prefix: String,
    pub input_name: String,
    pub input_shape: [usize; 4],
    pub output_names: Vec<String>,
    pub warmup_ms: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InitializationStage {
    ValidatingModel,
    LoadingOnnxRuntime,
    RegisteringDirectMl,
    FallingBackToCpu,
    OptimizingGraph,
    WarmingDetector,
    Ready,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializationEvent {
    pub stage: InitializationStage,
    pub message: String,
}
