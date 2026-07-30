use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

use tauri::ipc::Channel;

use crate::error::{AppError, ErrorCode};

use super::{
    preprocess,
    resources::DetectorResources,
    session::DetectorSession,
    types::{
        DetectionRequestMetadata, DetectionResponse, DetectionTiming, DetectorInfo,
        InitializationEvent, InitializationStage,
    },
};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub struct DetectorService {
    inner: Arc<Inner>,
}

struct Inner {
    resources: DetectorResources,
    lifecycle: Mutex<Lifecycle>,
    timeout: Duration,
    directml_device_id: i32,
}

enum Lifecycle {
    Uninitialized,
    Initializing,
    Ready(Box<WorkerHandle>),
    Failed,
    ShuttingDown,
}

struct WorkerHandle {
    sender: mpsc::SyncSender<Message>,
    busy: Arc<AtomicBool>,
    join: Option<thread::JoinHandle<()>>,
    info: DetectorInfo,
}

enum Message {
    Infer {
        frame: Vec<u8>,
        metadata: DetectionRequestMetadata,
        response: mpsc::Sender<Result<DetectionResponse, AppError>>,
    },
    Shutdown,
}

impl DetectorService {
    pub fn new(resources: DetectorResources) -> Self {
        Self::with_options(resources, DEFAULT_TIMEOUT, 0)
    }

    fn with_options(
        resources: DetectorResources,
        timeout: Duration,
        directml_device_id: i32,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                resources,
                lifecycle: Mutex::new(Lifecycle::Uninitialized),
                timeout,
                directml_device_id,
            }),
        }
    }

    pub fn initialize(
        &self,
        events: Option<Channel<InitializationEvent>>,
    ) -> Result<DetectorInfo, AppError> {
        {
            let mut lifecycle = self.inner.lifecycle.lock().map_err(poisoned)?;
            match &*lifecycle {
                Lifecycle::Ready(worker) => return Ok(worker.info.clone()),
                Lifecycle::Initializing => {
                    return Err(AppError::new(
                        ErrorCode::DetectorBusy,
                        "Detector initialization is already in progress.",
                    ));
                }
                Lifecycle::ShuttingDown => return Err(shutting_down()),
                Lifecycle::Uninitialized | Lifecycle::Failed => {
                    *lifecycle = Lifecycle::Initializing;
                }
            }
        }

        let thread_events = events.clone();
        let send_event = move |stage, message: &str| {
            if let Some(channel) = &thread_events {
                let _ = channel.send(InitializationEvent {
                    stage,
                    message: message.to_owned(),
                });
            }
        };
        send_event(
            InitializationStage::ValidatingModel,
            "Validating bundled detector",
        );
        let resources = match self.inner.resources.verify() {
            Ok(resources) => resources,
            Err(error) => {
                self.mark_failed();
                return Err(error);
            }
        };

        let (messages, receiver) = mpsc::sync_channel(1);
        let (startup, startup_receiver) = mpsc::sync_channel(0);
        let busy = Arc::new(AtomicBool::new(false));
        let worker_busy = busy.clone();
        let directml_device_id = self.inner.directml_device_id;
        let join = thread::Builder::new()
            .name("vision-lab-detector".to_owned())
            .spawn(move || {
                let created = DetectorSession::create(&resources, directml_device_id, send_event);
                match created {
                    Ok((session, info)) => {
                        if startup.send(Ok(info)).is_ok() {
                            worker_loop(session, receiver, worker_busy);
                        }
                    }
                    Err(error) => {
                        let _ = startup.send(Err(error));
                    }
                }
            })
            .map_err(|error| {
                self.mark_failed();
                AppError::new(
                    ErrorCode::SessionCreateFailed,
                    "Detector worker could not start.",
                )
                .with_detail(error.to_string())
            })?;

        match startup_receiver.recv() {
            Ok(Ok(info)) => {
                let mut lifecycle = self.inner.lifecycle.lock().map_err(poisoned)?;
                if matches!(*lifecycle, Lifecycle::ShuttingDown) {
                    drop(lifecycle);
                    let _ = messages.send(Message::Shutdown);
                    let _ = join.join();
                    return Err(shutting_down());
                }
                *lifecycle = Lifecycle::Ready(Box::new(WorkerHandle {
                    sender: messages,
                    busy,
                    join: Some(join),
                    info: info.clone(),
                }));
                if let Some(channel) = &events {
                    let _ = channel.send(InitializationEvent {
                        stage: InitializationStage::Ready,
                        message: "Detector ready".to_owned(),
                    });
                }
                Ok(info)
            }
            Ok(Err(error)) => {
                let _ = join.join();
                self.mark_failed();
                Err(error)
            }
            Err(error) => {
                let _ = join.join();
                self.mark_failed();
                Err(AppError::new(
                    ErrorCode::SessionCreateFailed,
                    "Detector worker stopped during initialization.",
                )
                .with_detail(error.to_string()))
            }
        }
    }

    pub fn detect(
        &self,
        frame: Vec<u8>,
        metadata: DetectionRequestMetadata,
    ) -> Result<DetectionResponse, AppError> {
        preprocess::validate_frame(&frame, metadata)?;
        let (sender, busy) = {
            let lifecycle = self.inner.lifecycle.lock().map_err(poisoned)?;
            match &*lifecycle {
                Lifecycle::Ready(worker) => (worker.sender.clone(), worker.busy.clone()),
                Lifecycle::Initializing => {
                    return Err(AppError::new(
                        ErrorCode::DetectorBusy,
                        "Detector is still initializing.",
                    ));
                }
                Lifecycle::ShuttingDown => return Err(shutting_down()),
                Lifecycle::Uninitialized | Lifecycle::Failed => {
                    return Err(AppError::new(
                        ErrorCode::DetectorUnavailable,
                        "Detector is not initialized.",
                    ));
                }
            }
        };
        if busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(AppError::new(
                ErrorCode::DetectorBusy,
                "Detector is processing another frame.",
            ));
        }
        let (response, receiver) = mpsc::channel();
        if let Err(error) = sender.try_send(Message::Infer {
            frame,
            metadata,
            response,
        }) {
            busy.store(false, Ordering::Release);
            return Err(match error {
                mpsc::TrySendError::Full(_) => AppError::new(
                    ErrorCode::DetectorBusy,
                    "Detector is processing another frame.",
                ),
                mpsc::TrySendError::Disconnected(_) => AppError::new(
                    ErrorCode::DetectorUnavailable,
                    "Detector worker is unavailable.",
                ),
            });
        }
        await_response(receiver, self.inner.timeout)
    }

    pub fn shutdown(&self) {
        let worker = {
            let Ok(mut lifecycle) = self.inner.lifecycle.lock() else {
                return;
            };
            match std::mem::replace(&mut *lifecycle, Lifecycle::ShuttingDown) {
                Lifecycle::Ready(worker) => Some(worker),
                _ => None,
            }
        };
        if let Some(mut worker) = worker {
            let _ = worker.sender.send(Message::Shutdown);
            if let Some(join) = worker.join.take() {
                let _ = join.join();
            }
        }
    }

    fn mark_failed(&self) {
        if let Ok(mut lifecycle) = self.inner.lifecycle.lock()
            && !matches!(*lifecycle, Lifecycle::ShuttingDown)
        {
            *lifecycle = Lifecycle::Failed;
        }
    }
}

fn worker_loop(
    mut session: DetectorSession,
    receiver: mpsc::Receiver<Message>,
    busy: Arc<AtomicBool>,
) {
    while let Ok(message) = receiver.recv() {
        match message {
            Message::Infer {
                frame,
                metadata,
                response,
            } => {
                let total_started = Instant::now();
                let preprocess_started = Instant::now();
                let input = preprocess::rgba_to_nchw(&frame);
                let preprocess_ms = millis(preprocess_started.elapsed());
                let result = session.run_tensor(&input, metadata.threshold).map(
                    |(detections, inference_ms, postprocess_ms)| DetectionResponse {
                        request_id: metadata.request_id,
                        model_generation: metadata.model_generation,
                        detections,
                        timing: DetectionTiming {
                            preprocess_ms,
                            inference_ms,
                            postprocess_ms,
                            native_total_ms: millis(total_started.elapsed()),
                        },
                    },
                );
                busy.store(false, Ordering::Release);
                let _ = response.send(result);
            }
            Message::Shutdown => break,
        }
    }
}

fn poisoned<T>(error: std::sync::PoisonError<T>) -> AppError {
    AppError::new(
        ErrorCode::DetectorUnavailable,
        "Detector state is unavailable.",
    )
    .with_detail(error.to_string())
}

fn shutting_down() -> AppError {
    AppError::new(ErrorCode::AppShuttingDown, "Application is shutting down.")
}

fn await_response(
    receiver: mpsc::Receiver<Result<DetectionResponse, AppError>>,
    timeout: Duration,
) -> Result<DetectionResponse, AppError> {
    receiver
        .recv_timeout(timeout)
        .map_err(|error| match error {
            mpsc::RecvTimeoutError::Timeout => {
                AppError::new(ErrorCode::InferenceTimeout, "Detector inference timed out.")
            }
            mpsc::RecvTimeoutError::Disconnected => AppError::new(
                ErrorCode::DetectorUnavailable,
                "Detector worker stopped unexpectedly.",
            ),
        })?
}

fn millis(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

#[cfg(test)]
mod tests {
    use std::{path::Path, thread, time::Duration};

    use crate::{
        detector::{
            INPUT_HEIGHT, INPUT_WIDTH,
            resources::DetectorResources,
            types::{DetectionRequestMetadata, ExecutionProvider},
        },
        error::ErrorCode,
    };

    use super::{DetectorService, Lifecycle, await_response};

    #[test]
    fn response_wait_has_a_typed_timeout() {
        let (_sender, receiver) = std::sync::mpsc::channel();
        let error = await_response(receiver, Duration::from_millis(1)).unwrap_err();
        assert_eq!(error.code, ErrorCode::InferenceTimeout);
    }

    #[test]
    fn raw_fixture_uses_cpu_fallback_and_rejects_concurrent_work() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let service = DetectorService::with_options(
            DetectorResources::from_repository(&repository),
            Duration::from_secs(30),
            999,
        );
        let info = service.initialize(None).unwrap();
        assert_eq!(info.provider, ExecutionProvider::Cpu);
        assert!(info.provider_fallback.is_some());

        let frame =
            image::open(repository.join("tests/fixtures/frames/car-front-three-quarter.png"))
                .unwrap()
                .to_rgba8()
                .into_raw();
        let metadata = DetectionRequestMetadata {
            request_id: 42,
            model_generation: 7,
            width: INPUT_WIDTH,
            height: INPUT_HEIGHT,
            threshold: 0.5,
        };
        let detector = service.clone();
        let first_frame = frame.clone();
        let first = thread::spawn(move || detector.detect(first_frame, metadata));

        for _ in 0..100 {
            let is_busy = {
                let lifecycle = service.inner.lifecycle.lock().unwrap();
                matches!(&*lifecycle, Lifecycle::Ready(worker) if worker.busy.load(std::sync::atomic::Ordering::Acquire))
            };
            if is_busy {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        let busy_error = service.detect(frame, metadata).unwrap_err();
        assert_eq!(busy_error.code, ErrorCode::DetectorBusy);

        let response = first.join().unwrap().unwrap();
        assert_eq!(response.request_id, 42);
        assert_eq!(response.model_generation, 7);
        assert_eq!(
            response
                .detections
                .iter()
                .map(|detection| detection.label.as_str())
                .collect::<Vec<_>>(),
            ["car", "person"]
        );
        assert!((response.detections[0].score - 0.996_601_1).abs() <= 0.000_01);
        let car = &response.detections[0].bounding_box;
        assert_eq!((car.x * INPUT_WIDTH as f32).trunc(), 109.0);
        assert_eq!((car.y * INPUT_HEIGHT as f32).trunc(), 132.0);
        assert!(response.timing.inference_ms > 0.0);
        assert!(response.timing.native_total_ms >= response.timing.inference_ms);

        service.shutdown();
    }
}
