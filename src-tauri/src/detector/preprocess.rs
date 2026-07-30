use ndarray::Array4;

use crate::error::{AppError, ErrorCode};

use super::{INPUT_HEIGHT, INPUT_WIDTH, types::DetectionRequestMetadata};

pub fn validate_frame(frame: &[u8], metadata: DetectionRequestMetadata) -> Result<(), AppError> {
    if metadata.width != INPUT_WIDTH || metadata.height != INPUT_HEIGHT {
        return Err(AppError::new(
            ErrorCode::InvalidFrame,
            format!("Frame must be {INPUT_WIDTH} x {INPUT_HEIGHT} pixels."),
        ));
    }
    if !metadata.threshold.is_finite() || !(0.0..=1.0).contains(&metadata.threshold) {
        return Err(AppError::new(
            ErrorCode::InvalidFrame,
            "Confidence threshold must be between 0 and 1.",
        ));
    }
    let expected = usize::try_from(metadata.width)
        .ok()
        .and_then(|width| {
            usize::try_from(metadata.height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| AppError::new(ErrorCode::InvalidFrame, "Frame dimensions overflow."))?;
    if frame.len() != expected {
        return Err(AppError::new(
            ErrorCode::InvalidFrame,
            format!("Frame contains {} bytes; expected {expected}.", frame.len()),
        ));
    }
    Ok(())
}

pub fn rgba_to_nchw(frame: &[u8]) -> Array4<f32> {
    let plane = (INPUT_WIDTH * INPUT_HEIGHT) as usize;
    let mut values = vec![0.0; plane * 3];
    for (index, pixel) in frame.chunks_exact(4).enumerate() {
        values[index] = f32::from(pixel[0]) / 255.0;
        values[plane + index] = f32::from(pixel[1]) / 255.0;
        values[plane * 2 + index] = f32::from(pixel[2]) / 255.0;
    }
    Array4::from_shape_vec((1, 3, INPUT_HEIGHT as usize, INPUT_WIDTH as usize), values)
        .expect("fixed detector tensor shape must match its allocation")
}

#[cfg(test)]
mod tests {
    use super::{rgba_to_nchw, validate_frame};
    use crate::detector::{INPUT_HEIGHT, INPUT_WIDTH, types::DetectionRequestMetadata};

    fn metadata() -> DetectionRequestMetadata {
        DetectionRequestMetadata {
            request_id: 1,
            model_generation: 2,
            width: INPUT_WIDTH,
            height: INPUT_HEIGHT,
            threshold: 0.5,
        }
    }

    #[test]
    fn validates_exact_checked_rgba_length() {
        let expected = (INPUT_WIDTH * INPUT_HEIGHT * 4) as usize;
        assert!(validate_frame(&vec![0; expected], metadata()).is_ok());
        assert!(validate_frame(&vec![0; expected - 1], metadata()).is_err());
        let mut wrong_size = metadata();
        wrong_size.width = 385;
        assert!(validate_frame(&vec![0; expected], wrong_size).is_err());
    }

    #[test]
    fn validates_threshold_boundaries() {
        for threshold in [0.0, 1.0] {
            let mut request = metadata();
            request.threshold = threshold;
            assert!(
                validate_frame(&vec![0; (INPUT_WIDTH * INPUT_HEIGHT * 4) as usize], request)
                    .is_ok()
            );
        }
        for threshold in [-0.1, 1.1, f32::NAN] {
            let mut request = metadata();
            request.threshold = threshold;
            assert!(
                validate_frame(&vec![0; (INPUT_WIDTH * INPUT_HEIGHT * 4) as usize], request)
                    .is_err()
            );
        }
    }

    #[test]
    fn converts_rgba_to_rescaled_rgb_nchw() {
        let mut frame = vec![0; (INPUT_WIDTH * INPUT_HEIGHT * 4) as usize];
        frame[..8].copy_from_slice(&[255, 128, 0, 7, 64, 32, 16, 9]);
        let tensor = rgba_to_nchw(&frame);
        assert_eq!(tensor[[0, 0, 0, 0]], 1.0);
        assert_eq!(tensor[[0, 0, 0, 1]], 64.0 / 255.0);
        assert_eq!(tensor[[0, 1, 0, 0]], 128.0 / 255.0);
        assert_eq!(tensor[[0, 2, 0, 1]], 16.0 / 255.0);
    }
}
