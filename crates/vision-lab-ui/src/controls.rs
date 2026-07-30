use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::{Document, Element, Event, EventTarget, HtmlElement, HtmlInputElement};

pub struct Dom {
    pub detect_button: HtmlElement,
    pub threshold: HtmlInputElement,
    pub threshold_value: Element,
    pub upload_button: HtmlElement,
    pub reset_button: HtmlElement,
    pub file_input: HtmlInputElement,
    pub dropzone: Element,
    status_chip: Element,
    status_text: Element,
    stat_latency: Element,
    stat_fps: Element,
    stat_count: Element,
    loader: Element,
    loader_bar: HtmlElement,
    loader_log: Element,
}

impl Dom {
    pub fn new(document: &Document) -> Result<Self, JsValue> {
        Ok(Self {
            detect_button: element(document, "detectBtn")?,
            threshold: element(document, "threshold")?,
            threshold_value: element(document, "thresholdValue")?,
            upload_button: element(document, "uploadBtn")?,
            reset_button: element(document, "resetBtn")?,
            file_input: element(document, "modelFileInput")?,
            dropzone: element(document, "dropzone")?,
            status_chip: element(document, "statusChip")?,
            status_text: element(document, "statusText")?,
            stat_latency: element(document, "statLatency")?,
            stat_fps: element(document, "statFps")?,
            stat_count: element(document, "statCount")?,
            loader: element(document, "loader")?,
            loader_bar: element(document, "loaderBar")?,
            loader_log: element(document, "loaderLog")?,
        })
    }

    pub fn set_loader(&self, text: &str, progress: f64) -> Result<(), JsValue> {
        self.loader_log.set_text_content(Some(text));
        self.loader_bar
            .style()
            .set_property("width", &format!("{}%", (progress * 100.0).round()))
    }

    pub fn hide_loader(&self) -> Result<(), JsValue> {
        self.loader.class_list().add_1("done")
    }

    pub fn set_status(&self, text: &str, state: Option<&str>) -> Result<(), JsValue> {
        self.status_text.set_text_content(Some(text));
        let classes = self.status_chip.class_list();
        for class in ["live", "paused", "error"] {
            classes.remove_1(class)?;
        }
        if let Some(state) = state {
            classes.add_1(state)?;
        }
        Ok(())
    }

    pub fn set_detect_idle(&self) -> Result<(), JsValue> {
        self.detect_button
            .class_list()
            .remove_2("loading", "live")?;
        self.detect_button.remove_attribute("disabled")?;
        self.detect_button.set_text_content(Some("DETECT"));
        Ok(())
    }

    pub fn set_detect_loading(&self) -> Result<(), JsValue> {
        self.detect_button.class_list().remove_1("live")?;
        self.detect_button.class_list().add_1("loading")?;
        self.detect_button.set_attribute("disabled", "")?;
        self.detect_button.set_text_content(Some("VALIDATE"));
        Ok(())
    }

    pub fn set_detect_stage(&self, stage: &str) {
        let label = match stage {
            "validatingModel" => "VALIDATE",
            "loadingOnnxRuntime" => "RUNTIME",
            "registeringDirectMl" => "DIRECTML",
            "fallingBackToCpu" => "CPU FALLBACK",
            "optimizingGraph" => "OPTIMIZE",
            "warmingDetector" => "WARMUP",
            "ready" => "READY",
            _ => "LOADING",
        };
        self.detect_button.set_text_content(Some(label));
    }

    pub fn set_detect_live(&self) -> Result<(), JsValue> {
        self.detect_button.class_list().remove_1("loading")?;
        self.detect_button.class_list().add_1("live")?;
        self.detect_button.remove_attribute("disabled")?;
        self.detect_button.set_text_content(Some("STOP"));
        Ok(())
    }

    pub fn update_stats(&self, latency: f64, fps: f64, count: usize) {
        self.stat_latency
            .set_inner_html(&format!("{}<small>ms</small>", latency.round() as i64));
        self.stat_fps
            .set_inner_html(&format!("{}<small>fps</small>", fps.round() as i64));
        self.stat_count.set_text_content(Some(&count.to_string()));
    }

    pub fn show_dropzone(&self, visible: bool) -> Result<(), JsValue> {
        if visible {
            self.dropzone.class_list().add_1("visible")
        } else {
            self.dropzone.class_list().remove_1("visible")
        }
    }
}

pub fn listen(
    target: &EventTarget,
    event: &str,
    callback: Closure<dyn FnMut(Event)>,
) -> Result<Closure<dyn FnMut(Event)>, JsValue> {
    target.add_event_listener_with_callback(event, callback.as_ref().unchecked_ref())?;
    Ok(callback)
}

pub fn element<T: JsCast>(document: &Document, id: &str) -> Result<T, JsValue> {
    Ok(document
        .get_element_by_id(id)
        .ok_or_else(|| JsValue::from_str(&format!("missing #{id}")))?
        .dyn_into::<T>()?)
}
