use std::{collections::HashMap, fs};

use serde::Deserialize;

use crate::error::{AppError, ErrorCode};

#[derive(Debug, Deserialize)]
struct ModelConfig {
    id2label: HashMap<String, String>,
}

pub fn load(path: &std::path::Path) -> Result<HashMap<u32, String>, AppError> {
    let bytes = fs::read(path).map_err(|error| {
        AppError::new(
            ErrorCode::ModelResourceMissing,
            "Detector configuration is missing.",
        )
        .with_detail(error.to_string())
    })?;
    let config: ModelConfig = serde_json::from_slice(&bytes).map_err(|error| {
        AppError::new(
            ErrorCode::ModelSchemaUnsupported,
            "Detector configuration is invalid.",
        )
        .with_detail(error.to_string())
    })?;

    config
        .id2label
        .into_iter()
        .map(|(id, label)| {
            id.parse::<u32>().map(|id| (id, label)).map_err(|error| {
                AppError::new(
                    ErrorCode::ModelSchemaUnsupported,
                    "Detector label mapping is invalid.",
                )
                .with_detail(error.to_string())
            })
        })
        .collect()
}
