use std::str::FromStr;

use tauri::{
    State,
    ipc::{Channel, InvokeBody, Request},
};

use crate::{
    detector::{
        DetectionRequestMetadata, DetectionResponse, DetectorInfo, DetectorService,
        InitializationEvent,
    },
    error::{AppError, ErrorCode},
};

const REQUEST_ID: &str = "x-vision-request-id";
const MODEL_GENERATION: &str = "x-vision-model-generation";
const FRAME_WIDTH: &str = "x-vision-frame-width";
const FRAME_HEIGHT: &str = "x-vision-frame-height";
const THRESHOLD: &str = "x-vision-threshold";

#[tauri::command]
pub async fn initialize_detector(
    state: State<'_, DetectorService>,
    on_event: Channel<InitializationEvent>,
) -> Result<DetectorInfo, AppError> {
    let service = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || service.initialize(Some(on_event)))
        .await
        .map_err(|error| {
            AppError::new(
                ErrorCode::SessionCreateFailed,
                "Detector initialization task failed.",
            )
            .with_detail(error.to_string())
        })?
}

#[tauri::command]
pub async fn detect_frame(
    request: Request<'_>,
    state: State<'_, DetectorService>,
) -> Result<DetectionResponse, AppError> {
    let metadata = metadata_from_headers(request.headers())?;
    let frame = match request.body() {
        InvokeBody::Raw(bytes) => bytes.clone(),
        InvokeBody::Json(_) => {
            return Err(AppError::new(
                ErrorCode::InvalidFrame,
                "Frame must use a raw binary IPC body.",
            ));
        }
    };
    let service = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || service.detect(frame, metadata))
        .await
        .map_err(|error| {
            AppError::new(ErrorCode::InferenceFailed, "Detector request task failed.")
                .with_detail(error.to_string())
        })?
}

fn metadata_from_headers(
    headers: &tauri::http::HeaderMap,
) -> Result<DetectionRequestMetadata, AppError> {
    Ok(DetectionRequestMetadata {
        request_id: parse_header(headers, REQUEST_ID)?,
        model_generation: parse_header(headers, MODEL_GENERATION)?,
        width: parse_header(headers, FRAME_WIDTH)?,
        height: parse_header(headers, FRAME_HEIGHT)?,
        threshold: parse_header(headers, THRESHOLD)?,
    })
}

fn parse_header<T>(headers: &tauri::http::HeaderMap, name: &str) -> Result<T, AppError>
where
    T: FromStr,
{
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| {
            AppError::new(
                ErrorCode::InvalidFrame,
                format!("Frame metadata header '{name}' is missing or invalid."),
            )
        })
}

#[cfg(test)]
mod tests {
    use tauri::http::{HeaderMap, HeaderValue};

    use super::{
        FRAME_HEIGHT, FRAME_WIDTH, MODEL_GENERATION, REQUEST_ID, THRESHOLD, metadata_from_headers,
    };

    #[test]
    fn parses_raw_frame_metadata_headers() {
        let mut headers = HeaderMap::new();
        for (name, value) in [
            (REQUEST_ID, "7"),
            (MODEL_GENERATION, "4"),
            (FRAME_WIDTH, "384"),
            (FRAME_HEIGHT, "384"),
            (THRESHOLD, "0.5"),
        ] {
            headers.insert(name, HeaderValue::from_str(value).unwrap());
        }
        let metadata = metadata_from_headers(&headers).unwrap();
        assert_eq!(metadata.request_id, 7);
        assert_eq!(metadata.model_generation, 4);
        assert_eq!(metadata.threshold, 0.5);
    }

    #[test]
    fn rejects_missing_or_malformed_metadata_headers() {
        assert!(metadata_from_headers(&HeaderMap::new()).is_err());
        let mut headers = HeaderMap::new();
        headers.insert(REQUEST_ID, HeaderValue::from_static("not-a-number"));
        assert!(metadata_from_headers(&headers).is_err());
    }
}
