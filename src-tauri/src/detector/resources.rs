use std::{collections::HashMap, fs, path::PathBuf};

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::error::{AppError, ErrorCode};

#[derive(Debug, Clone)]
pub struct DetectorResources {
    pub model: PathBuf,
    pub config: PathBuf,
    pub checksums: PathBuf,
}

#[derive(Debug, Clone)]
pub struct VerifiedResources {
    pub model: PathBuf,
    pub config: PathBuf,
    pub repository: String,
    pub revision: String,
    pub model_checksum: String,
}

#[derive(Debug, Deserialize)]
struct Manifest {
    repository: String,
    revision: String,
    files: HashMap<String, FileChecksum>,
}

#[derive(Debug, Deserialize)]
struct FileChecksum {
    bytes: u64,
    sha256: String,
}

impl DetectorResources {
    pub fn from_resource_dir(resource_dir: PathBuf) -> Self {
        let detector = resource_dir.join("detector");
        Self {
            model: detector.join("model.onnx"),
            config: detector.join("config.json"),
            checksums: detector.join("checksums.json"),
        }
    }

    #[cfg(test)]
    pub fn from_repository(repository: &std::path::Path) -> Self {
        let detector = repository.join("models/rfdetr-nano");
        Self {
            model: detector.join("model.onnx"),
            config: detector.join("config.json"),
            checksums: detector.join("checksums.json"),
        }
    }

    pub fn verify(&self) -> Result<VerifiedResources, AppError> {
        let manifest_bytes =
            fs::read(&self.checksums).map_err(|error| missing(error.to_string()))?;
        let manifest: Manifest = serde_json::from_slice(&manifest_bytes).map_err(|error| {
            AppError::new(
                ErrorCode::ModelChecksumMismatch,
                "Detector checksum manifest is invalid.",
            )
            .with_detail(error.to_string())
        })?;

        let model_checksum = verify_file(&self.model, "model.onnx", &manifest)?;
        verify_file(&self.config, "config.json", &manifest)?;

        Ok(VerifiedResources {
            model: self.model.clone(),
            config: self.config.clone(),
            repository: manifest.repository,
            revision: manifest.revision,
            model_checksum,
        })
    }
}

fn verify_file(
    path: &std::path::Path,
    name: &str,
    manifest: &Manifest,
) -> Result<String, AppError> {
    let expected = manifest.files.get(name).ok_or_else(|| {
        AppError::new(
            ErrorCode::ModelChecksumMismatch,
            format!("Detector checksum for {name} is missing."),
        )
    })?;
    let bytes = fs::read(path).map_err(|error| missing(error.to_string()))?;
    if bytes.len() as u64 != expected.bytes {
        return Err(mismatch(name));
    }
    let actual = format!("{:x}", Sha256::digest(&bytes));
    if !actual.eq_ignore_ascii_case(&expected.sha256) {
        return Err(mismatch(name));
    }
    Ok(actual)
}

fn missing(detail: String) -> AppError {
    AppError::new(
        ErrorCode::ModelResourceMissing,
        "Bundled detector resources are missing.",
    )
    .with_detail(detail)
}

fn mismatch(name: &str) -> AppError {
    AppError::new(
        ErrorCode::ModelChecksumMismatch,
        format!("Bundled detector resource {name} failed integrity validation."),
    )
}

#[cfg(test)]
mod tests {
    use super::DetectorResources;

    #[test]
    fn verifies_repository_resources() {
        let repository = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let verified = DetectorResources::from_repository(&repository)
            .verify()
            .unwrap();
        assert_eq!(
            verified.revision,
            "eae21cee0687a91bcf9fa071605c48d7705d2d91"
        );
        assert!(verified.model_checksum.starts_with("9cbac6b1"));
    }
}
