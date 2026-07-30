use wasm_bindgen::{JsCast, JsValue};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use crate::ipc::Detection;

pub struct DetectionOverlay {
    canvas: HtmlCanvasElement,
    context: CanvasRenderingContext2d,
}

impl DetectionOverlay {
    pub fn new(canvas: HtmlCanvasElement) -> Result<Self, JsValue> {
        let context = canvas
            .get_context("2d")?
            .ok_or_else(|| JsValue::from_str("2D overlay is unavailable"))?
            .dyn_into::<CanvasRenderingContext2d>()?;
        Ok(Self { canvas, context })
    }

    pub fn resize(&self, css_width: i32, css_height: i32, dpr: f64) -> Result<(), JsValue> {
        let width = (f64::from(css_width.max(1)) * dpr).round() as u32;
        let height = (f64::from(css_height.max(1)) * dpr).round() as u32;
        self.canvas.set_width(width);
        self.canvas.set_height(height);
        self.canvas
            .style()
            .set_property("width", &format!("{}px", css_width.max(1)))?;
        self.canvas
            .style()
            .set_property("height", &format!("{}px", css_height.max(1)))?;
        self.clear();
        Ok(())
    }

    pub fn clear(&self) {
        self.context.clear_rect(
            0.0,
            0.0,
            f64::from(self.canvas.width()),
            f64::from(self.canvas.height()),
        );
    }

    pub fn draw(&self, detections: &[Detection], dpr: f64) -> Result<(), JsValue> {
        self.clear();
        let canvas_width = f64::from(self.canvas.width());
        let canvas_height = f64::from(self.canvas.height());
        self.context.set_line_join("round");
        self.context
            .set_font(&format!("{}px \"JetBrains Mono\", monospace", 11.0 * dpr));
        self.context.set_text_baseline("middle");

        for detection in detections {
            let left = f64::from(detection.bounding_box.x.clamp(0.0, 1.0)) * canvas_width;
            let top = f64::from(detection.bounding_box.y.clamp(0.0, 1.0)) * canvas_height;
            let right = f64::from(
                (detection.bounding_box.x + detection.bounding_box.width).clamp(0.0, 1.0),
            ) * canvas_width;
            let bottom = f64::from(
                (detection.bounding_box.y + detection.bounding_box.height).clamp(0.0, 1.0),
            ) * canvas_height;
            let width = right - left;
            let height = bottom - top;
            if width <= 0.0 || height <= 0.0 {
                continue;
            }

            self.context.set_stroke_style_str("rgba(0, 229, 255, 0.35)");
            self.context.set_line_width(dpr);
            self.context.stroke_rect(left, top, width, height);

            let arm = (14.0 * dpr).min(width / 4.0).min(height / 4.0);
            self.context.set_stroke_style_str("#00e5ff");
            self.context.set_line_width(2.5 * dpr);
            self.context.begin_path();
            corner(&self.context, left, top, arm, 1.0, 1.0);
            corner(&self.context, right, top, arm, -1.0, 1.0);
            corner(&self.context, right, bottom, arm, -1.0, -1.0);
            corner(&self.context, left, bottom, arm, 1.0, -1.0);
            self.context.stroke();

            let label = format!(
                "{} {}%",
                detection.label.to_uppercase(),
                (detection.score * 100.0).round() as i32
            );
            let padding_x = 6.0 * dpr;
            let chip_height = 20.0 * dpr;
            let measured = self.context.measure_text(&label)?.width() + padding_x * 2.0;
            let chip_width = measured.min(canvas_width);
            let chip_x = left.min((canvas_width - chip_width).max(0.0));
            let chip_y = if top >= chip_height {
                top - chip_height
            } else {
                top.min((canvas_height - chip_height).max(0.0))
            };
            self.context.set_fill_style_str("rgba(0, 229, 255, 0.92)");
            self.context
                .fill_rect(chip_x, chip_y, chip_width, chip_height);
            self.context.set_fill_style_str("#04161a");
            self.context
                .fill_text(&label, chip_x + padding_x, chip_y + chip_height / 2.0 + dpr)?;
        }
        Ok(())
    }
}

fn corner(
    context: &CanvasRenderingContext2d,
    x: f64,
    y: f64,
    arm: f64,
    horizontal: f64,
    vertical: f64,
) {
    context.move_to(x + horizontal * arm, y);
    context.line_to(x, y);
    context.line_to(x, y + vertical * arm);
}
