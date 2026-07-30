use std::{
    collections::HashMap,
    fs,
    path::Path,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use ndarray::Array4;
use ort::{
    execution_providers::DirectMLExecutionProvider,
    inputs,
    session::{Session, builder::GraphOptimizationLevel},
    value::TensorRef,
};
use serde::{Deserialize, Serialize};

pub const INPUT_WIDTH: u32 = 384;
pub const INPUT_HEIGHT: u32 = 384;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    Auto,
    Cpu,
    DirectMl,
}

impl Provider {
    pub fn parse(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "cpu" => Ok(Self::Cpu),
            "directml" | "dml" => Ok(Self::DirectMl),
            _ => bail!("unsupported provider '{value}'; expected auto, cpu, or directml"),
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Cpu => "cpu",
            Self::DirectMl => "directml",
        }
    }
}

#[derive(Debug, Deserialize)]
struct ModelConfig {
    id2label: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoundingBox {
    pub origin_x: f32,
    pub origin_y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Detection {
    pub label_id: usize,
    pub label: String,
    pub score: f32,
    pub bounding_box: BoundingBox,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Timings {
    pub session_create_ms: f64,
    pub image_decode_ms: f64,
    pub preprocess_ms: f64,
    pub warmup_ms: f64,
    pub inference_ms: f64,
    pub inference_samples_ms: Vec<f64>,
    pub postprocess_ms: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunResult {
    pub requested_provider: String,
    pub provider: String,
    pub provider_fallback: Option<String>,
    pub threshold: f32,
    pub input_names: Vec<String>,
    pub output_names: Vec<String>,
    pub input_shape: [usize; 4],
    pub logits_shape: Vec<usize>,
    pub pred_boxes_shape: Vec<usize>,
    pub timings: Timings,
    pub detections: Vec<Detection>,
}

#[derive(Debug, Clone, Copy)]
pub struct RunOptions {
    pub provider: Provider,
    pub directml_device_id: i32,
    pub threshold: f32,
    pub warmup_runs: usize,
    pub measured_runs: usize,
}

pub fn run(
    model_path: &Path,
    config_path: &Path,
    image_path: &Path,
    options: RunOptions,
) -> Result<RunResult> {
    let RunOptions {
        provider,
        directml_device_id,
        threshold,
        warmup_runs,
        measured_runs,
    } = options;
    if !threshold.is_finite() || !(0.0..=1.0).contains(&threshold) {
        bail!("threshold must be finite and between 0 and 1");
    }
    if measured_runs == 0 {
        bail!("measured run count must be greater than zero");
    }

    let config: ModelConfig = serde_json::from_slice(
        &fs::read(config_path)
            .with_context(|| format!("failed to read {}", config_path.display()))?,
    )
    .with_context(|| format!("failed to parse {}", config_path.display()))?;

    let session_started = Instant::now();
    let (mut session, selected_provider, provider_fallback) =
        create_session(model_path, provider, directml_device_id)?;
    let session_create = session_started.elapsed();

    let input_names = session
        .inputs
        .iter()
        .map(|input| input.name.clone())
        .collect();
    let output_names = session
        .outputs
        .iter()
        .map(|output| output.name.clone())
        .collect();

    let decode_started = Instant::now();
    let image = image::open(image_path)
        .with_context(|| format!("failed to decode {}", image_path.display()))?
        .to_rgb8();
    let image_decode = decode_started.elapsed();
    if image.dimensions() != (INPUT_WIDTH, INPUT_HEIGHT) {
        bail!(
            "fixture must be {INPUT_WIDTH}x{INPUT_HEIGHT}, got {}x{}",
            image.width(),
            image.height()
        );
    }

    let preprocess_started = Instant::now();
    let mut input = Array4::<f32>::zeros((1, 3, INPUT_HEIGHT as usize, INPUT_WIDTH as usize));
    for (x, y, pixel) in image.enumerate_pixels() {
        let x = x as usize;
        let y = y as usize;
        input[[0, 0, y, x]] = f32::from(pixel[0]) / 255.0;
        input[[0, 1, y, x]] = f32::from(pixel[1]) / 255.0;
        input[[0, 2, y, x]] = f32::from(pixel[2]) / 255.0;
    }
    let preprocess = preprocess_started.elapsed();

    let input_name = session
        .inputs
        .first()
        .context("model has no inputs")?
        .name
        .clone();
    let warmup_started = Instant::now();
    for _ in 0..warmup_runs {
        session.run(inputs![input_name.as_str() => TensorRef::from_array_view(&input)?])?;
    }
    let warmup = warmup_started.elapsed();

    let mut inference_samples = Vec::with_capacity(measured_runs);
    for _ in 1..measured_runs {
        let inference_started = Instant::now();
        session.run(inputs![input_name.as_str() => TensorRef::from_array_view(&input)?])?;
        inference_samples.push(inference_started.elapsed());
    }
    let inference_started = Instant::now();
    let outputs = session.run(inputs![input_name => TensorRef::from_array_view(&input)?])?;
    inference_samples.push(inference_started.elapsed());
    let inference = inference_samples.iter().sum::<Duration>() / measured_runs as u32;

    let postprocess_started = Instant::now();
    let logits = outputs
        .get("logits")
        .context("model output 'logits' is missing")?
        .try_extract_array::<f32>()?;
    let boxes = outputs
        .get("pred_boxes")
        .context("model output 'pred_boxes' is missing")?
        .try_extract_array::<f32>()?;

    if logits.ndim() != 3 || boxes.ndim() != 3 {
        bail!(
            "unexpected output ranks: logits={}, pred_boxes={}",
            logits.ndim(),
            boxes.ndim()
        );
    }
    if logits.shape()[0] != 1 || boxes.shape()[0] != 1 || boxes.shape()[2] != 4 {
        bail!(
            "unexpected output shapes: logits={:?}, pred_boxes={:?}",
            logits.shape(),
            boxes.shape()
        );
    }
    if logits.shape()[1] != boxes.shape()[1] {
        bail!("logit and box query counts differ");
    }

    let logits_shape = logits.shape().to_vec();
    let pred_boxes_shape = boxes.shape().to_vec();
    let query_count = logits.shape()[1];
    let class_count = logits.shape()[2];
    let background_class = class_count - 1;
    let mut detections = Vec::new();

    for query in 0..query_count {
        let max_logit = (0..class_count)
            .map(|class| logits[[0, query, class]])
            .fold(f32::NEG_INFINITY, f32::max);
        let denominator: f32 = (0..class_count)
            .map(|class| (logits[[0, query, class]] - max_logit).exp())
            .sum();
        let (label_id, score) = (0..class_count)
            .map(|class| {
                (
                    class,
                    (logits[[0, query, class]] - max_logit).exp() / denominator,
                )
            })
            .max_by(|left, right| left.1.total_cmp(&right.1))
            .context("model returned no classes")?;

        if label_id == background_class || score < threshold {
            continue;
        }

        let label = config
            .id2label
            .get(&label_id.to_string())
            .cloned()
            .with_context(|| format!("model config has no label for class {label_id}"))?;
        let center_x = boxes[[0, query, 0]];
        let center_y = boxes[[0, query, 1]];
        let width = boxes[[0, query, 2]];
        let height = boxes[[0, query, 3]];
        detections.push(Detection {
            label_id,
            label,
            score,
            bounding_box: BoundingBox {
                origin_x: (center_x - width / 2.0) * INPUT_WIDTH as f32,
                origin_y: (center_y - height / 2.0) * INPUT_HEIGHT as f32,
                width: width * INPUT_WIDTH as f32,
                height: height * INPUT_HEIGHT as f32,
            },
        });
    }

    detections.sort_by(|left, right| right.score.total_cmp(&left.score));
    let postprocess = postprocess_started.elapsed();

    Ok(RunResult {
        requested_provider: provider.name().to_owned(),
        provider: selected_provider.name().to_owned(),
        provider_fallback,
        threshold,
        input_names,
        output_names,
        input_shape: [1, 3, INPUT_HEIGHT as usize, INPUT_WIDTH as usize],
        logits_shape,
        pred_boxes_shape,
        timings: Timings {
            session_create_ms: millis(session_create),
            image_decode_ms: millis(image_decode),
            preprocess_ms: millis(preprocess),
            warmup_ms: millis(warmup),
            inference_ms: millis(inference),
            inference_samples_ms: inference_samples.into_iter().map(millis).collect(),
            postprocess_ms: millis(postprocess),
        },
        detections,
    })
}

fn create_session(
    model_path: &Path,
    provider: Provider,
    directml_device_id: i32,
) -> Result<(Session, Provider, Option<String>)> {
    match provider {
        Provider::Cpu => Ok((commit_cpu_session(model_path)?, Provider::Cpu, None)),
        Provider::DirectMl => Ok((
            commit_directml_session(model_path, directml_device_id)?,
            Provider::DirectMl,
            None,
        )),
        Provider::Auto => match commit_directml_session(model_path, directml_device_id) {
            Ok(session) => Ok((session, Provider::DirectMl, None)),
            Err(error) => Ok((
                commit_cpu_session(model_path)?,
                Provider::Cpu,
                Some(format!("DirectML unavailable: {error:#}")),
            )),
        },
    }
}

fn base_session_builder() -> Result<ort::session::builder::SessionBuilder> {
    Ok(Session::builder()?
        .with_optimization_level(GraphOptimizationLevel::Level3)?
        .with_intra_threads(4)?)
}

fn commit_cpu_session(model_path: &Path) -> Result<Session> {
    base_session_builder()?
        .commit_from_file(model_path)
        .with_context(|| format!("failed to load ONNX model {} on CPU", model_path.display()))
}

fn commit_directml_session(model_path: &Path, device_id: i32) -> Result<Session> {
    base_session_builder()?
        .with_execution_providers([DirectMLExecutionProvider::default()
            .with_device_id(device_id)
            .build()
            .error_on_failure()])?
        .commit_from_file(model_path)
        .with_context(|| {
            format!(
                "failed to load ONNX model {} on DirectML device {device_id}",
                model_path.display()
            )
        })
}

fn millis(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

#[cfg(test)]
mod tests {
    use super::Provider;

    #[test]
    fn parses_provider_names() {
        assert_eq!(Provider::parse("cpu").unwrap(), Provider::Cpu);
        assert_eq!(Provider::parse("DML").unwrap(), Provider::DirectMl);
        assert_eq!(Provider::parse("auto").unwrap(), Provider::Auto);
        assert!(Provider::parse("cuda").is_err());
    }
}
