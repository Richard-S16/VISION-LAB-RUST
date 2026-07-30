use std::{cell::RefCell, rc::Rc};

use js_sys::{Function, Reflect};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use wasm_bindgen_futures::spawn_local;
use web_sys::{Document, DragEvent, Event, EventTarget, HtmlCanvasElement, Window};

use crate::{
    capture::FrameCapture,
    controls::{Dom, element, listen},
    ipc::{self, Detection, DetectionResponse},
    overlay::DetectionOverlay,
    scene_bridge,
    smoothing::{LabelSmoother, SmoothedDetection},
    state::{AppState, DetectionState, DetectorState, ModelState},
};

const MINIMUM_INTERVAL_MS: f64 = 300.0;

thread_local! {
    static APPLICATION: RefCell<Option<Rc<RefCell<App>>>> = const { RefCell::new(None) };
}

pub struct App {
    window: Window,
    render_canvas: HtmlCanvasElement,
    dom: Dom,
    overlay: DetectionOverlay,
    capture: FrameCapture,
    state: AppState,
    smoother: LabelSmoother,
    drag_depth: u32,
    listeners: Vec<Closure<dyn FnMut(Event)>>,
    animation_frame: Option<Closure<dyn FnMut(f64)>>,
    disposed: bool,
}

pub async fn boot() -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("window is unavailable"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("document is unavailable"))?;
    let render_canvas = element::<HtmlCanvasElement>(&document, "renderCanvas")?;
    let overlay_canvas = element::<HtmlCanvasElement>(&document, "overlayCanvas")?;
    let dom = Dom::new(&document)?;
    dom.set_loader("loading 3d model...", 0.15)?;

    let app = Rc::new(RefCell::new(App {
        window,
        render_canvas: render_canvas.clone(),
        dom,
        overlay: DetectionOverlay::new(overlay_canvas)?,
        capture: FrameCapture::new(&document)?,
        state: AppState::default(),
        smoother: LabelSmoother::default(),
        drag_depth: 0,
        listeners: Vec::new(),
        animation_frame: None,
        disposed: false,
    }));

    if let Err(error) = scene_bridge::initialize(&render_canvas).await {
        let message = js_error(&error);
        let mut app = app.borrow_mut();
        app.state.fail_boot(message.clone());
        app.dom.set_loader(&format!("error: {message}"), 0.0)?;
        app.dom.set_status("ERROR", Some("error"))?;
        return Err(error);
    }

    {
        let mut app_mut = app.borrow_mut();
        app_mut.state.finish_boot();
        app_mut.dom.set_loader("ready", 1.0)?;
        app_mut.dom.hide_loader()?;
        app_mut.dom.set_status("READY", None)?;
        app_mut.dom.set_detect_idle()?;
        app_mut.resize()?;
    }
    bind_controls(&app, &document)?;
    start_scheduler(&app)?;
    APPLICATION.with(|application| application.replace(Some(app)));
    Ok(())
}

impl App {
    fn toggle_detection(app: Rc<RefCell<Self>>) {
        if app.borrow().state.detection == DetectionState::Running {
            app.borrow_mut().stop_detection();
            return;
        }

        let detector_state = app.borrow().state.detector.clone();
        if matches!(detector_state, DetectorState::Ready { .. }) {
            app.borrow_mut().start_detection();
            return;
        }
        if matches!(detector_state, DetectorState::Loading) {
            return;
        }

        {
            let mut current = app.borrow_mut();
            current.state.detector = DetectorState::Loading;
            let _ = current.dom.set_detect_loading();
        }
        let weak = Rc::downgrade(&app);
        spawn_local(async move {
            let progress_weak = weak.clone();
            let progress = Closure::<dyn FnMut(JsValue)>::new(move |event| {
                let Some(app) = progress_weak.upgrade() else {
                    return;
                };
                if let Ok(stage) = Reflect::get(&event, &"stage".into())
                    && let Some(stage) = stage.as_string()
                {
                    app.borrow().dom.set_detect_stage(&stage);
                }
            });
            let result = ipc::initialize(progress.as_ref().unchecked_ref::<Function>()).await;
            let Some(app) = weak.upgrade() else {
                return;
            };
            match result {
                Ok(info) => {
                    app.borrow_mut().state.detector = DetectorState::Ready {
                        provider: info.provider.clone(),
                    };
                    web_sys::console::info_1(
                        &format!("[detector] native {}", info.provider).into(),
                    );
                    app.borrow_mut().start_detection();
                }
                Err(error) => {
                    let message = js_error(&error);
                    let mut current = app.borrow_mut();
                    current.state.detector = DetectorState::Failed {
                        message: message.clone(),
                    };
                    current.smoother.clear();
                    current.overlay.clear();
                    let _ = current.dom.set_detect_idle();
                    let _ = current.dom.set_status("ERROR", Some("error"));
                    web_sys::console::error_1(&error);
                }
            }
        });
    }

    fn start_detection(&mut self) {
        if self.state.start_detection() {
            let _ = self.dom.set_detect_live();
            let _ = self.dom.set_status("LIVE", Some("live"));
        }
    }

    fn stop_detection(&mut self) {
        self.state.stop_detection();
        self.smoother.clear();
        self.overlay.clear();
        let _ = self.dom.set_detect_idle();
        let status = if matches!(self.state.model, ModelState::Loading { .. }) {
            ("LOADING", Some("paused"))
        } else {
            ("READY", None)
        };
        let _ = self.dom.set_status(status.0, status.1);
    }

    fn tick(app: Rc<RefCell<Self>>, now_ms: f64) {
        let job = {
            let mut current = app.borrow_mut();
            if current.disposed || !current.state.can_request(now_ms, MINIMUM_INTERVAL_MS) {
                return;
            }
            let request = current
                .state
                .begin_request(now_ms)
                .expect("request conditions were checked");
            match current.capture.capture(&current.render_canvas) {
                Ok(frame) => Some((frame, request, current.state.threshold)),
                Err(error) => {
                    current.state.fail_request(request.request_id);
                    web_sys::console::error_1(&error);
                    None
                }
            }
        };
        let Some((frame, request, threshold)) = job else {
            return;
        };
        let weak = Rc::downgrade(&app);
        spawn_local(async move {
            let result = ipc::detect(&frame, request, threshold).await;
            let Some(app) = weak.upgrade() else {
                return;
            };
            match result {
                Ok(response) => app
                    .borrow_mut()
                    .handle_response(request.request_id, response),
                Err(error) => {
                    app.borrow_mut().state.fail_request(request.request_id);
                    web_sys::console::error_2(&"inference failed".into(), &error);
                }
            }
        });
    }

    fn handle_response(&mut self, expected_request_id: u64, mut response: DetectionResponse) {
        if response.request_id != expected_request_id {
            self.state.fail_request(expected_request_id);
            return;
        }
        if !self
            .state
            .complete_request(response.request_id, response.model_generation)
        {
            return;
        }

        let mut samples: Vec<_> = response
            .detections
            .iter()
            .map(|detection| SmoothedDetection {
                label: detection.label.clone(),
                score: detection.score,
            })
            .collect();
        self.smoother.stabilize(&mut samples);
        for (detection, sample) in response.detections.iter_mut().zip(samples) {
            detection.label = sample.label;
            detection.score = sample.score;
        }

        let dpr = self.window.device_pixel_ratio();
        if let Err(error) = self.overlay.draw(&response.detections, dpr) {
            web_sys::console::error_1(&error);
        }
        self.dom.update_stats(
            response.timing.native_total_ms,
            scene_bridge::get_fps(),
            response.detections.len(),
        );
        expose_detections(&self.window, &response.detections);
    }

    fn set_threshold(&mut self, value: f32) {
        self.state.set_threshold(value);
        self.smoother.clear();
        self.dom
            .threshold_value
            .set_text_content(Some(&format!("{}%", (value * 100.0).round() as i32)));
    }

    fn begin_model_load(&mut self) -> u64 {
        let generation = self.state.start_model_load();
        self.overlay.clear();
        self.smoother.clear();
        let _ = self.dom.set_status("LOADING", Some("paused"));
        generation
    }

    fn finish_model_load(&mut self, generation: u64, result: Result<(), JsValue>) {
        match result {
            Ok(()) if self.state.finish_model_load(generation) => {
                let running = self.state.detection == DetectionState::Running;
                let _ = self.dom.set_status(
                    if running { "LIVE" } else { "READY" },
                    running.then_some("live"),
                );
            }
            Err(error) => {
                let message = js_error(&error);
                if self.state.fail_model_load(generation, message) {
                    let _ = self.dom.set_status("ERROR", Some("error"));
                }
                web_sys::console::error_1(&error);
            }
            _ => {}
        }
    }

    fn load_files(app: Rc<RefCell<Self>>, files: JsValue) {
        let generation = app.borrow_mut().begin_model_load();
        let weak = Rc::downgrade(&app);
        spawn_local(async move {
            let result = scene_bridge::load_files(&files, generation).await;
            if let Some(app) = weak.upgrade() {
                app.borrow_mut().finish_model_load(generation, result);
            }
        });
    }

    fn load_default(app: Rc<RefCell<Self>>) {
        let generation = app.borrow_mut().begin_model_load();
        let weak = Rc::downgrade(&app);
        spawn_local(async move {
            let result = scene_bridge::load_default(generation).await;
            if let Some(app) = weak.upgrade() {
                app.borrow_mut().finish_model_load(generation, result);
            }
        });
    }

    fn resize(&mut self) -> Result<(), JsValue> {
        scene_bridge::resize();
        self.overlay.resize(
            self.render_canvas.client_width(),
            self.render_canvas.client_height(),
            self.window.device_pixel_ratio(),
        )
    }

    fn dispose(&mut self) {
        if self.disposed {
            return;
        }
        self.disposed = true;
        self.state.stop_detection();
        self.overlay.clear();
        scene_bridge::dispose();
    }
}

fn bind_controls(app: &Rc<RefCell<App>>, _document: &Document) -> Result<(), JsValue> {
    let detect_target: EventTarget = app.borrow().dom.detect_button.clone().dyn_into()?;
    let weak = Rc::downgrade(app);
    let closure = Closure::<dyn FnMut(Event)>::new(move |_| {
        if let Some(app) = weak.upgrade() {
            App::toggle_detection(app);
        }
    });
    app.borrow_mut()
        .listeners
        .push(listen(&detect_target, "click", closure)?);

    let threshold_target: EventTarget = app.borrow().dom.threshold.clone().dyn_into()?;
    let weak = Rc::downgrade(app);
    let closure = Closure::<dyn FnMut(Event)>::new(move |_| {
        let Some(app) = weak.upgrade() else {
            return;
        };
        let value = app.borrow().dom.threshold.value_as_number() as f32 / 100.0;
        app.borrow_mut().set_threshold(value);
    });
    app.borrow_mut()
        .listeners
        .push(listen(&threshold_target, "input", closure)?);

    let upload_target: EventTarget = app.borrow().dom.upload_button.clone().dyn_into()?;
    let weak = Rc::downgrade(app);
    let closure = Closure::<dyn FnMut(Event)>::new(move |_| {
        if let Some(app) = weak.upgrade() {
            app.borrow().dom.file_input.click();
        }
    });
    app.borrow_mut()
        .listeners
        .push(listen(&upload_target, "click", closure)?);

    let file_target: EventTarget = app.borrow().dom.file_input.clone().dyn_into()?;
    let weak = Rc::downgrade(app);
    let closure = Closure::<dyn FnMut(Event)>::new(move |_| {
        let Some(app) = weak.upgrade() else {
            return;
        };
        let files = app.borrow().dom.file_input.files();
        app.borrow().dom.file_input.set_value("");
        if let Some(files) = files
            && files.length() > 0
        {
            App::load_files(app, files.into());
        }
    });
    app.borrow_mut()
        .listeners
        .push(listen(&file_target, "change", closure)?);

    let reset_target: EventTarget = app.borrow().dom.reset_button.clone().dyn_into()?;
    let weak = Rc::downgrade(app);
    let closure = Closure::<dyn FnMut(Event)>::new(move |_| {
        if let Some(app) = weak.upgrade() {
            App::load_default(app);
        }
    });
    app.borrow_mut()
        .listeners
        .push(listen(&reset_target, "click", closure)?);

    let window_target: EventTarget = app.borrow().window.clone().dyn_into()?;
    bind_window_events(app, &window_target)?;
    Ok(())
}

fn bind_window_events(app: &Rc<RefCell<App>>, target: &EventTarget) -> Result<(), JsValue> {
    let weak = Rc::downgrade(app);
    let closure = Closure::<dyn FnMut(Event)>::new(move |event: Event| {
        event.prevent_default();
        if let Some(app) = weak.upgrade() {
            let mut app = app.borrow_mut();
            app.drag_depth += 1;
            let _ = app.dom.show_dropzone(true);
        }
    });
    app.borrow_mut()
        .listeners
        .push(listen(target, "dragenter", closure)?);

    let closure = Closure::<dyn FnMut(Event)>::new(|event: Event| event.prevent_default());
    app.borrow_mut()
        .listeners
        .push(listen(target, "dragover", closure)?);

    let weak = Rc::downgrade(app);
    let closure = Closure::<dyn FnMut(Event)>::new(move |event: Event| {
        event.prevent_default();
        if let Some(app) = weak.upgrade() {
            let mut app = app.borrow_mut();
            app.drag_depth = app.drag_depth.saturating_sub(1);
            if app.drag_depth == 0 {
                let _ = app.dom.show_dropzone(false);
            }
        }
    });
    app.borrow_mut()
        .listeners
        .push(listen(target, "dragleave", closure)?);

    let weak = Rc::downgrade(app);
    let closure = Closure::<dyn FnMut(Event)>::new(move |event: Event| {
        event.prevent_default();
        let Some(app) = weak.upgrade() else {
            return;
        };
        {
            let mut current = app.borrow_mut();
            current.drag_depth = 0;
            let _ = current.dom.show_dropzone(false);
        }
        let Ok(event) = event.dyn_into::<DragEvent>() else {
            return;
        };
        if let Some(files) = event.data_transfer().and_then(|transfer| transfer.files())
            && files.length() > 0
        {
            App::load_files(app, files.into());
        }
    });
    app.borrow_mut()
        .listeners
        .push(listen(target, "drop", closure)?);

    let weak = Rc::downgrade(app);
    let closure = Closure::<dyn FnMut(Event)>::new(move |_| {
        if let Some(app) = weak.upgrade()
            && let Err(error) = app.borrow_mut().resize()
        {
            web_sys::console::error_1(&error);
        }
    });
    app.borrow_mut()
        .listeners
        .push(listen(target, "resize", closure)?);

    let weak = Rc::downgrade(app);
    let closure = Closure::<dyn FnMut(Event)>::new(move |_| {
        if let Some(app) = weak.upgrade() {
            app.borrow_mut().dispose();
        }
    });
    app.borrow_mut()
        .listeners
        .push(listen(target, "beforeunload", closure)?);
    Ok(())
}

fn start_scheduler(app: &Rc<RefCell<App>>) -> Result<(), JsValue> {
    let weak = Rc::downgrade(app);
    let closure = Closure::<dyn FnMut(f64)>::new(move |now_ms| {
        let Some(app) = weak.upgrade() else {
            return;
        };
        {
            let current = app.borrow();
            if current.disposed {
                return;
            }
            if let Some(callback) = &current.animation_frame {
                let _ = current
                    .window
                    .request_animation_frame(callback.as_ref().unchecked_ref());
            }
        }
        App::tick(app, now_ms);
    });
    app.borrow_mut().animation_frame = Some(closure);
    let current = app.borrow();
    current.window.request_animation_frame(
        current
            .animation_frame
            .as_ref()
            .expect("animation callback was assigned")
            .as_ref()
            .unchecked_ref(),
    )?;
    Ok(())
}

fn expose_detections(window: &Window, detections: &[Detection]) {
    if let Ok(value) = serde_wasm_bindgen::to_value(detections) {
        let _ = Reflect::set(window.as_ref(), &"__lastDetections".into(), &value);
    }
}

fn js_error(error: &JsValue) -> String {
    error
        .as_string()
        .or_else(|| Reflect::get(error, &"message".into()).ok()?.as_string())
        .unwrap_or_else(|| "unknown application error".to_owned())
}
