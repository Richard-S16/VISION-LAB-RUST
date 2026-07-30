use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Clone, Error, Serialize)]
#[serde(rename_all = "camelCase")]
#[error("{message}")]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    ModelResourceMissing,
    ModelChecksumMismatch,
    OrtLoadFailed,
    ProviderUnavailable,
    SessionCreateFailed,
    ModelSchemaUnsupported,
    InvalidFrame,
    DetectorBusy,
    DetectorUnavailable,
    InferenceFailed,
    InferenceTimeout,
    AppShuttingDown,
}

impl AppError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            detail: None,
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{AppError, ErrorCode};

    #[test]
    fn error_serialization_has_stable_code() {
        let value =
            serde_json::to_value(AppError::new(ErrorCode::InvalidFrame, "Frame is invalid."))
                .unwrap();
        assert_eq!(value["code"], "INVALID_FRAME");
        assert_eq!(value["message"], "Frame is invalid.");
        assert!(value.get("detail").is_none());
    }
}
