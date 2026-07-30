use wasm_bindgen::{JsCast, JsValue};
use web_sys::{CanvasRenderingContext2d, Document, HtmlCanvasElement};

pub const CAPTURE_SIZE: u32 = 384;

pub struct FrameCapture {
    canvas: HtmlCanvasElement,
    context: CanvasRenderingContext2d,
}

impl FrameCapture {
    pub fn new(document: &Document) -> Result<Self, JsValue> {
        let canvas = document
            .create_element("canvas")?
            .dyn_into::<HtmlCanvasElement>()?;
        canvas.set_width(CAPTURE_SIZE);
        canvas.set_height(CAPTURE_SIZE);
        let options = js_sys::Object::new();
        js_sys::Reflect::set(&options, &"alpha".into(), &JsValue::FALSE)?;
        js_sys::Reflect::set(&options, &"willReadFrequently".into(), &JsValue::TRUE)?;
        let context = canvas
            .get_context_with_context_options("2d", &options)?
            .ok_or_else(|| JsValue::from_str("2D frame capture is unavailable"))?
            .dyn_into::<CanvasRenderingContext2d>()?;
        Ok(Self { canvas, context })
    }

    pub fn capture(&self, source: &HtmlCanvasElement) -> Result<Vec<u8>, JsValue> {
        self.context
            .draw_image_with_html_canvas_element_and_dw_and_dh(
                source,
                0.0,
                0.0,
                f64::from(self.canvas.width()),
                f64::from(self.canvas.height()),
            )?;
        Ok(self
            .context
            .get_image_data(
                0.0,
                0.0,
                f64::from(self.canvas.width()),
                f64::from(self.canvas.height()),
            )?
            .data()
            .to_vec())
    }
}
