use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone, PartialEq)]
pub struct SmoothedDetection {
    pub label: String,
    pub score: f32,
}

#[derive(Debug, Default)]
pub struct LabelSmoother {
    history: VecDeque<SmoothedDetection>,
}

impl LabelSmoother {
    pub fn stabilize(&mut self, detections: &mut [SmoothedDetection]) {
        if detections.len() != 1 {
            self.clear();
            return;
        }
        self.history.push_back(detections[0].clone());
        if self.history.len() > 3 {
            self.history.pop_front();
        }
        if self.history.len() < 3 {
            return;
        }
        let mut counts = HashMap::new();
        for sample in &self.history {
            *counts.entry(sample.label.as_str()).or_insert(0) += 1;
        }
        let Some((majority, count)) = counts.into_iter().max_by_key(|(_, count)| *count) else {
            return;
        };
        if count < 2 || majority == detections[0].label {
            return;
        }
        if let Some(sample) = self
            .history
            .iter()
            .rev()
            .find(|sample| sample.label == majority)
        {
            detections[0].label.clone_from(&sample.label);
            detections[0].score = sample.score;
        }
    }

    pub fn clear(&mut self) {
        self.history.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::{LabelSmoother, SmoothedDetection};

    fn detection(label: &str, score: f32) -> SmoothedDetection {
        SmoothedDetection {
            label: label.to_owned(),
            score,
        }
    }

    #[test]
    fn majority_replaces_label_and_matching_score_together() {
        let mut smoother = LabelSmoother::default();
        smoother.stabilize(&mut [detection("car", 0.91)]);
        smoother.stabilize(&mut [detection("car", 0.87)]);
        let mut current = [detection("truck", 0.76)];
        smoother.stabilize(&mut current);
        assert_eq!(current[0], detection("car", 0.87));
    }

    #[test]
    fn zero_or_multiple_detections_clear_history() {
        let mut smoother = LabelSmoother::default();
        smoother.stabilize(&mut [detection("car", 0.9)]);
        smoother.stabilize(&mut []);
        smoother.stabilize(&mut [detection("car", 0.8)]);
        let mut current = [detection("truck", 0.7)];
        smoother.stabilize(&mut current);
        assert_eq!(current[0].label, "truck");

        smoother.stabilize(&mut [detection("car", 0.9), detection("person", 0.8)]);
        smoother.stabilize(&mut [detection("car", 0.9)]);
        let mut current = [detection("truck", 0.7)];
        smoother.stabilize(&mut current);
        assert_eq!(current[0].label, "truck");
    }
}
