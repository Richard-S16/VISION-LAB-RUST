#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootState {
    Starting,
    Ready,
    Failed { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectorState {
    Uninitialized,
    Loading,
    Ready { provider: String },
    Failed { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelState {
    Loading { generation: u64 },
    Ready { generation: u64 },
    Failed { generation: u64, message: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectionState {
    Stopped,
    Running,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InFlightRequest {
    pub request_id: u64,
    pub model_generation: u64,
    pub detection_epoch: u64,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub boot: BootState,
    pub detector: DetectorState,
    pub model: ModelState,
    pub detection: DetectionState,
    pub threshold: f32,
    pub next_request_id: u64,
    pub in_flight: Option<InFlightRequest>,
    pub detection_epoch: u64,
    pub last_request_started_ms: f64,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            boot: BootState::Starting,
            detector: DetectorState::Uninitialized,
            model: ModelState::Loading { generation: 0 },
            detection: DetectionState::Stopped,
            threshold: 0.5,
            next_request_id: 0,
            in_flight: None,
            detection_epoch: 0,
            last_request_started_ms: f64::NEG_INFINITY,
        }
    }
}

impl AppState {
    pub fn finish_boot(&mut self) {
        self.boot = BootState::Ready;
        self.model = ModelState::Ready { generation: 1 };
    }

    pub fn fail_boot(&mut self, message: impl Into<String>) {
        self.boot = BootState::Failed {
            message: message.into(),
        };
    }

    pub fn set_threshold(&mut self, threshold: f32) {
        self.threshold = threshold;
    }

    pub fn start_detection(&mut self) -> bool {
        if !matches!(self.detector, DetectorState::Ready { .. })
            || !matches!(self.model, ModelState::Ready { .. })
        {
            return false;
        }
        self.detection_epoch += 1;
        self.detection = DetectionState::Running;
        self.last_request_started_ms = f64::NEG_INFINITY;
        true
    }

    pub fn stop_detection(&mut self) {
        self.detection_epoch += 1;
        self.detection = DetectionState::Stopped;
    }

    pub fn start_model_load(&mut self) -> u64 {
        let generation = self.model_generation().saturating_add(1);
        self.model = ModelState::Loading { generation };
        generation
    }

    pub fn finish_model_load(&mut self, generation: u64) -> bool {
        if !matches!(self.model, ModelState::Loading { generation: current } if current == generation)
        {
            return false;
        }
        self.model = ModelState::Ready { generation };
        true
    }

    pub fn fail_model_load(&mut self, generation: u64, message: impl Into<String>) -> bool {
        if !matches!(self.model, ModelState::Loading { generation: current } if current == generation)
        {
            return false;
        }
        self.model = ModelState::Failed {
            generation,
            message: message.into(),
        };
        true
    }

    pub fn model_generation(&self) -> u64 {
        match self.model {
            ModelState::Loading { generation }
            | ModelState::Ready { generation }
            | ModelState::Failed { generation, .. } => generation,
        }
    }

    pub fn can_request(&self, now_ms: f64, minimum_interval_ms: f64) -> bool {
        matches!(self.boot, BootState::Ready)
            && matches!(self.detector, DetectorState::Ready { .. })
            && matches!(self.model, ModelState::Ready { .. })
            && self.detection == DetectionState::Running
            && self.in_flight.is_none()
            && now_ms - self.last_request_started_ms >= minimum_interval_ms
    }

    pub fn begin_request(&mut self, now_ms: f64) -> Option<InFlightRequest> {
        if self.in_flight.is_some() || self.detection != DetectionState::Running {
            return None;
        }
        self.next_request_id += 1;
        self.last_request_started_ms = now_ms;
        let request = InFlightRequest {
            request_id: self.next_request_id,
            model_generation: self.model_generation(),
            detection_epoch: self.detection_epoch,
        };
        self.in_flight = Some(request);
        Some(request)
    }

    pub fn complete_request(&mut self, request_id: u64, model_generation: u64) -> bool {
        let Some(request) = self.in_flight else {
            return false;
        };
        if request.request_id != request_id {
            return false;
        }
        self.in_flight = None;
        request.model_generation == model_generation
            && request.model_generation == self.model_generation()
            && request.detection_epoch == self.detection_epoch
            && self.detection == DetectionState::Running
            && matches!(self.model, ModelState::Ready { .. })
    }

    pub fn fail_request(&mut self, request_id: u64) {
        if matches!(self.in_flight, Some(request) if request.request_id == request_id) {
            self.in_flight = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AppState, DetectionState, DetectorState, ModelState};

    fn ready() -> AppState {
        let mut state = AppState::default();
        state.finish_boot();
        state.detector = DetectorState::Ready {
            provider: "cpu".to_owned(),
        };
        assert!(state.start_detection());
        state
    }

    #[test]
    fn threshold_survives_detector_initialization() {
        let mut state = AppState::default();
        state.set_threshold(0.72);
        state.detector = DetectorState::Loading;
        state.detector = DetectorState::Ready {
            provider: "directMl".to_owned(),
        };
        assert_eq!(state.threshold, 0.72);
    }

    #[test]
    fn stop_rejects_in_flight_result() {
        let mut state = ready();
        let request = state.begin_request(0.0).unwrap();
        state.stop_detection();
        assert!(!state.complete_request(request.request_id, request.model_generation));
        assert!(state.in_flight.is_none());
    }

    #[test]
    fn restart_rejects_previous_epoch() {
        let mut state = ready();
        let request = state.begin_request(0.0).unwrap();
        state.stop_detection();
        assert!(state.start_detection());
        assert!(!state.complete_request(request.request_id, request.model_generation));
    }

    #[test]
    fn model_load_invalidates_result_and_stale_completion() {
        let mut state = ready();
        let request = state.begin_request(0.0).unwrap();
        let first = state.start_model_load();
        let second = state.start_model_load();
        assert!(!state.finish_model_load(first));
        assert!(state.finish_model_load(second));
        assert!(!state.complete_request(request.request_id, request.model_generation));
        assert_eq!(state.model, ModelState::Ready { generation: second });
    }

    #[test]
    fn scheduler_enforces_interval_and_single_flight() {
        let mut state = ready();
        assert!(state.can_request(0.0, 300.0));
        let request = state.begin_request(0.0).unwrap();
        assert!(!state.can_request(500.0, 300.0));
        assert!(state.complete_request(request.request_id, request.model_generation));
        assert!(!state.can_request(299.0, 300.0));
        assert!(state.can_request(300.0, 300.0));
    }

    #[test]
    fn detector_must_be_ready_to_start() {
        let mut state = AppState::default();
        state.finish_boot();
        assert!(!state.start_detection());
        assert_eq!(state.detection, DetectionState::Stopped);
    }
}
