use std::collections::HashMap;

use crate::error::{AppError, ErrorCode};

use super::types::{BoundingBox, Detection};

pub fn process(
    logits: &[f32],
    logits_shape: &[usize],
    boxes: &[f32],
    boxes_shape: &[usize],
    labels: &HashMap<u32, String>,
    threshold: f32,
) -> Result<Vec<Detection>, AppError> {
    validate_shapes(logits, logits_shape, boxes, boxes_shape)?;
    let queries = logits_shape[1];
    let classes = logits_shape[2];
    let background = classes - 1;
    let mut detections = Vec::new();

    for query in 0..queries {
        let row = &logits[query * classes..(query + 1) * classes];
        let max_logit = row.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let denominator: f32 = row.iter().map(|value| (*value - max_logit).exp()).sum();
        if !denominator.is_finite() || denominator <= 0.0 {
            return Err(schema("Detector returned invalid classification values."));
        }
        let (label_id, score) = row
            .iter()
            .enumerate()
            .map(|(index, value)| (index, (*value - max_logit).exp() / denominator))
            .max_by(|left, right| left.1.total_cmp(&right.1))
            .ok_or_else(|| schema("Detector returned no classes."))?;
        if label_id == background || score < threshold {
            continue;
        }

        let label_id = label_id as u32;
        let label = labels
            .get(&label_id)
            .cloned()
            .ok_or_else(|| schema("Detector returned an unmapped class."))?;
        let values = &boxes[query * 4..query * 4 + 4];
        if !values.iter().all(|value| value.is_finite()) {
            continue;
        }
        let left = (values[0] - values[2] / 2.0).clamp(0.0, 1.0);
        let top = (values[1] - values[3] / 2.0).clamp(0.0, 1.0);
        let right = (values[0] + values[2] / 2.0).clamp(0.0, 1.0);
        let bottom = (values[1] + values[3] / 2.0).clamp(0.0, 1.0);
        if right <= left || bottom <= top {
            continue;
        }
        detections.push(Detection {
            label_id,
            label,
            score,
            bounding_box: BoundingBox {
                x: left,
                y: top,
                width: right - left,
                height: bottom - top,
            },
        });
    }

    detections.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.label_id.cmp(&right.label_id))
    });
    Ok(detections)
}

fn validate_shapes(
    logits: &[f32],
    logits_shape: &[usize],
    boxes: &[f32],
    boxes_shape: &[usize],
) -> Result<(), AppError> {
    let valid = logits_shape.len() == 3
        && boxes_shape.len() == 3
        && logits_shape[0] == 1
        && boxes_shape[0] == 1
        && logits_shape[1] == boxes_shape[1]
        && logits_shape[2] > 1
        && boxes_shape[2] == 4
        && logits_shape.iter().product::<usize>() == logits.len()
        && boxes_shape.iter().product::<usize>() == boxes.len();
    if valid {
        Ok(())
    } else {
        Err(schema("Detector output schema is unsupported."))
    }
}

fn schema(message: &str) -> AppError {
    AppError::new(ErrorCode::ModelSchemaUnsupported, message)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::process;

    #[test]
    fn selects_classes_clamps_boxes_and_sorts_scores() {
        let labels = HashMap::from([(1, "person".to_owned()), (3, "car".to_owned())]);
        let logits = [
            -2.0, 0.0, -2.0, 4.0, -2.0, // car
            -2.0, 3.0, -2.0, 0.0, -2.0, // person
        ];
        let boxes = [0.1, 0.1, 0.4, 0.4, 0.5, 0.5, 0.2, 0.3];
        let detections = process(&logits, &[1, 2, 5], &boxes, &[1, 2, 4], &labels, 0.5).unwrap();
        assert_eq!(detections.len(), 2);
        assert_eq!(detections[0].label, "car");
        assert_eq!(detections[0].bounding_box.x, 0.0);
        assert_eq!(detections[0].bounding_box.y, 0.0);
        assert!(detections[0].bounding_box.width <= 1.0);
    }

    #[test]
    fn rejects_bad_shapes_and_degenerate_boxes() {
        let labels = HashMap::from([(1, "person".to_owned())]);
        assert!(process(&[0.0], &[1, 1], &[0.0; 4], &[1, 1, 4], &labels, 0.5).is_err());
        let result = process(
            &[0.0, 3.0, -2.0],
            &[1, 1, 3],
            &[0.5, 0.5, -0.1, 0.2],
            &[1, 1, 4],
            &labels,
            0.5,
        )
        .unwrap();
        assert!(result.is_empty());
    }
}
