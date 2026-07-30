use std::{collections::HashMap, time::Instant};

use ndarray::Array4;
use ort::{
    execution_providers::DirectMLExecutionProvider,
    inputs,
    session::{Session, builder::GraphOptimizationLevel},
    value::TensorRef,
};

use crate::error::{AppError, ErrorCode};

use super::{
    INPUT_HEIGHT, INPUT_WIDTH, postprocess,
    resources::VerifiedResources,
    types::{Detection, DetectorInfo, ExecutionProvider, InitializationStage},
};

pub struct DetectorSession {
    session: Session,
    labels: HashMap<u32, String>,
    input_name: String,
}

impl DetectorSession {
    pub fn create(
        resources: &VerifiedResources,
        directml_device_id: i32,
        mut event: impl FnMut(InitializationStage, &str),
    ) -> Result<(Self, DetectorInfo), AppError> {
        event(
            InitializationStage::LoadingOnnxRuntime,
            "Loading ONNX Runtime",
        );
        event(
            InitializationStage::RegisteringDirectMl,
            "Registering DirectML",
        );
        event(
            InitializationStage::OptimizingGraph,
            "Creating optimized detector session",
        );
        let (session, provider, provider_fallback) =
            match commit_directml(&resources.model, directml_device_id) {
                Ok(session) => (session, ExecutionProvider::DirectMl, None),
                Err(directml_error) => {
                    event(
                        InitializationStage::FallingBackToCpu,
                        "DirectML unavailable; falling back to CPU",
                    );
                    let session = commit_cpu(&resources.model).map_err(|cpu_error| {
                        AppError::new(
                            ErrorCode::SessionCreateFailed,
                            "Detector could not start with DirectML or CPU.",
                        )
                        .with_detail(format!("DirectML: {}; CPU: {}", directml_error, cpu_error))
                    })?;
                    (
                        session,
                        ExecutionProvider::Cpu,
                        Some(format!("DirectML unavailable: {directml_error}")),
                    )
                }
            };

        let input_name = session
            .inputs
            .first()
            .ok_or_else(|| schema("Detector model has no input."))?
            .name
            .clone();
        if input_name != "pixel_values" {
            return Err(schema("Detector model input is unsupported."));
        }
        let output_names: Vec<_> = session
            .outputs
            .iter()
            .map(|output| output.name.clone())
            .collect();
        if !output_names.iter().any(|name| name == "logits")
            || !output_names.iter().any(|name| name == "pred_boxes")
        {
            return Err(schema("Detector model outputs are unsupported."));
        }
        let labels = super::labels::load(&resources.config)?;
        let mut detector = Self {
            session,
            labels,
            input_name: input_name.clone(),
        };

        event(InitializationStage::WarmingDetector, "Warming detector");
        let blank = Array4::<f32>::zeros((1, 3, INPUT_HEIGHT as usize, INPUT_WIDTH as usize));
        let warmup_started = Instant::now();
        detector.run_tensor(&blank, 1.0)?;
        let warmup_ms = millis(warmup_started.elapsed());

        let checksum_prefix = resources.model_checksum.chars().take(12).collect();
        let info = DetectorInfo {
            provider,
            provider_fallback,
            model_repository: resources.repository.clone(),
            model_revision: resources.revision.clone(),
            model_checksum_prefix: checksum_prefix,
            input_name,
            input_shape: [1, 3, INPUT_HEIGHT as usize, INPUT_WIDTH as usize],
            output_names,
            warmup_ms,
        };
        Ok((detector, info))
    }

    pub fn run_tensor(
        &mut self,
        input: &Array4<f32>,
        threshold: f32,
    ) -> Result<(Vec<Detection>, f64, f64), AppError> {
        let inference_started = Instant::now();
        let outputs = self
            .session
            .run(
                inputs![self.input_name.as_str() => TensorRef::from_array_view(input)
                    .map_err(inference)?],
            )
            .map_err(inference)?;
        let inference_ms = millis(inference_started.elapsed());
        let logits = outputs
            .get("logits")
            .ok_or_else(|| schema("Detector output 'logits' is missing."))?
            .try_extract_array::<f32>()
            .map_err(inference)?;
        let boxes = outputs
            .get("pred_boxes")
            .ok_or_else(|| schema("Detector output 'pred_boxes' is missing."))?
            .try_extract_array::<f32>()
            .map_err(inference)?;
        let logits_slice = logits
            .as_slice()
            .ok_or_else(|| schema("Detector logits are not contiguous."))?;
        let boxes_slice = boxes
            .as_slice()
            .ok_or_else(|| schema("Detector boxes are not contiguous."))?;
        let postprocess_started = Instant::now();
        let detections = postprocess::process(
            logits_slice,
            logits.shape(),
            boxes_slice,
            boxes.shape(),
            &self.labels,
            threshold,
        )?;
        let postprocess_ms = millis(postprocess_started.elapsed());
        Ok((detections, inference_ms, postprocess_ms))
    }
}

fn base_builder() -> Result<ort::session::builder::SessionBuilder, AppError> {
    Session::builder()
        .and_then(|builder| builder.with_optimization_level(GraphOptimizationLevel::Level3))
        .and_then(|builder| builder.with_intra_threads(4))
        .map_err(|error| {
            AppError::new(
                ErrorCode::OrtLoadFailed,
                "ONNX Runtime could not be configured.",
            )
            .with_detail(error.to_string())
        })
}

fn commit_cpu(path: &std::path::Path) -> Result<Session, AppError> {
    base_builder()?.commit_from_file(path).map_err(|error| {
        AppError::new(
            ErrorCode::SessionCreateFailed,
            "CPU detector session could not be created.",
        )
        .with_detail(error.to_string())
    })
}

fn commit_directml(path: &std::path::Path, device_id: i32) -> Result<Session, AppError> {
    base_builder()?
        .with_execution_providers([DirectMLExecutionProvider::default()
            .with_device_id(device_id)
            .build()
            .error_on_failure()])
        .and_then(|builder| builder.commit_from_file(path))
        .map_err(|error| {
            AppError::new(
                ErrorCode::ProviderUnavailable,
                "DirectML detector session could not be created.",
            )
            .with_detail(error.to_string())
        })
}

fn schema(message: &str) -> AppError {
    AppError::new(ErrorCode::ModelSchemaUnsupported, message)
}

fn inference(error: ort::Error) -> AppError {
    AppError::new(ErrorCode::InferenceFailed, "Detector inference failed.")
        .with_detail(error.to_string())
}

fn millis(duration: std::time::Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}
