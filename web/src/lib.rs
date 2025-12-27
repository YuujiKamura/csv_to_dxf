//! DXF Viewer - WebAssembly Module
//!
//! A DXF file viewer that runs in the browser using WebAssembly.

mod dxf_parser;

use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;
use std::f64::consts::PI;

pub use dxf_parser::*;

/// DXF Viewer state
#[wasm_bindgen]
pub struct DxfViewer {
    parse_result: Option<DxfParseResult>,
    scale: f64,
    offset_x: f64,
    offset_y: f64,
    canvas_width: f64,
    canvas_height: f64,
}

#[wasm_bindgen]
impl DxfViewer {
    /// Create a new DXF viewer
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            parse_result: None,
            scale: 1.0,
            offset_x: 0.0,
            offset_y: 0.0,
            canvas_width: 800.0,
            canvas_height: 600.0,
        }
    }

    /// Load and parse DXF content
    #[wasm_bindgen]
    pub fn load_dxf(&mut self, content: &str) -> Result<String, JsValue> {
        let parser = DxfParser::new();
        let result = parser.parse(content);

        let summary = format!(
            "Loaded: {} lines, {} circles, {} arcs, {} polylines, {} texts",
            result.lines.len(),
            result.circles.len(),
            result.arcs.len(),
            result.polylines.len(),
            result.texts.len()
        );

        // Auto-fit to canvas
        self.fit_to_canvas(&result);

        self.parse_result = Some(result);
        Ok(summary)
    }

    /// Get parse result as JSON
    #[wasm_bindgen]
    pub fn get_result_json(&self) -> Result<String, JsValue> {
        match &self.parse_result {
            Some(result) => serde_json::to_string(result)
                .map_err(|e| JsValue::from_str(&e.to_string())),
            None => Ok("null".to_string()),
        }
    }

    /// Calculate scale and offset to fit content in canvas
    fn fit_to_canvas(&mut self, result: &DxfParseResult) {
        let (min_x, min_y, max_x, max_y) = self.calculate_bounds(result);

        if max_x <= min_x || max_y <= min_y {
            return;
        }

        let content_width = max_x - min_x;
        let content_height = max_y - min_y;

        // Add 10% padding
        let scale_x = self.canvas_width * 0.9 / content_width;
        let scale_y = self.canvas_height * 0.9 / content_height;
        self.scale = scale_x.min(scale_y);

        // Center the content
        let center_x = (min_x + max_x) / 2.0;
        let center_y = (min_y + max_y) / 2.0;

        self.offset_x = self.canvas_width / 2.0 - center_x * self.scale;
        self.offset_y = self.canvas_height / 2.0 + center_y * self.scale; // Y is flipped
    }

    fn calculate_bounds(&self, result: &DxfParseResult) -> (f64, f64, f64, f64) {
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;

        // Check header extents first
        if let Some(header) = &result.header {
            if header.ext_min != (0.0, 0.0, 0.0) || header.ext_max != (0.0, 0.0, 0.0) {
                return (
                    header.ext_min.0,
                    header.ext_min.1,
                    header.ext_max.0,
                    header.ext_max.1,
                );
            }
        }

        // Calculate from entities
        for line in &result.lines {
            min_x = min_x.min(line.x1).min(line.x2);
            min_y = min_y.min(line.y1).min(line.y2);
            max_x = max_x.max(line.x1).max(line.x2);
            max_y = max_y.max(line.y1).max(line.y2);
        }

        for circle in &result.circles {
            min_x = min_x.min(circle.center_x - circle.radius);
            min_y = min_y.min(circle.center_y - circle.radius);
            max_x = max_x.max(circle.center_x + circle.radius);
            max_y = max_y.max(circle.center_y + circle.radius);
        }

        for arc in &result.arcs {
            min_x = min_x.min(arc.center_x - arc.radius);
            min_y = min_y.min(arc.center_y - arc.radius);
            max_x = max_x.max(arc.center_x + arc.radius);
            max_y = max_y.max(arc.center_y + arc.radius);
        }

        for poly in &result.polylines {
            for (x, y) in &poly.vertices {
                min_x = min_x.min(*x);
                min_y = min_y.min(*y);
                max_x = max_x.max(*x);
                max_y = max_y.max(*y);
            }
        }

        for text in &result.texts {
            min_x = min_x.min(text.x);
            min_y = min_y.min(text.y);
            max_x = max_x.max(text.x);
            max_y = max_y.max(text.y);
        }

        if min_x == f64::MAX {
            (0.0, 0.0, 100.0, 100.0)
        } else {
            (min_x, min_y, max_x, max_y)
        }
    }

    /// Set canvas size
    #[wasm_bindgen]
    pub fn set_canvas_size(&mut self, width: f64, height: f64) {
        self.canvas_width = width;
        self.canvas_height = height;
        if let Some(result) = &self.parse_result {
            let result_clone = result.clone();
            self.fit_to_canvas(&result_clone);
        }
    }

    /// Pan the view
    #[wasm_bindgen]
    pub fn pan(&mut self, dx: f64, dy: f64) {
        self.offset_x += dx;
        self.offset_y += dy;
    }

    /// Zoom the view
    #[wasm_bindgen]
    pub fn zoom(&mut self, factor: f64, center_x: f64, center_y: f64) {
        let old_scale = self.scale;
        self.scale *= factor;

        // Zoom towards the center point
        self.offset_x = center_x - (center_x - self.offset_x) * (self.scale / old_scale);
        self.offset_y = center_y - (center_y - self.offset_y) * (self.scale / old_scale);
    }

    /// Render to canvas
    #[wasm_bindgen]
    pub fn render(&self, ctx: &CanvasRenderingContext2d) -> Result<(), JsValue> {
        let result = match &self.parse_result {
            Some(r) => r,
            None => return Ok(()),
        };

        // Clear canvas
        ctx.set_fill_style(&JsValue::from_str("#1a1a2e"));
        ctx.fill_rect(0.0, 0.0, self.canvas_width, self.canvas_height);

        ctx.set_line_width(1.0);

        // Draw lines
        for line in &result.lines {
            ctx.set_stroke_style(&self.color_to_css(line.color));
            ctx.begin_path();
            let (x1, y1) = self.model_to_view(line.x1, line.y1);
            let (x2, y2) = self.model_to_view(line.x2, line.y2);
            ctx.move_to(x1, y1);
            ctx.line_to(x2, y2);
            ctx.stroke();
        }

        // Draw circles
        for circle in &result.circles {
            ctx.set_stroke_style(&self.color_to_css(circle.color));
            ctx.begin_path();
            let (cx, cy) = self.model_to_view(circle.center_x, circle.center_y);
            let r = circle.radius * self.scale;
            ctx.arc(cx, cy, r, 0.0, 2.0 * PI)?;
            ctx.stroke();
        }

        // Draw arcs
        for arc in &result.arcs {
            ctx.set_stroke_style(&self.color_to_css(arc.color));
            ctx.begin_path();
            let (cx, cy) = self.model_to_view(arc.center_x, arc.center_y);
            let r = arc.radius * self.scale;
            // DXF uses degrees, counter-clockwise; Canvas uses radians
            // Y is flipped, so we need to flip the angles
            let start = -arc.end_angle * PI / 180.0;
            let end = -arc.start_angle * PI / 180.0;
            ctx.arc(cx, cy, r, start, end)?;
            ctx.stroke();
        }

        // Draw polylines
        for poly in &result.polylines {
            if poly.vertices.is_empty() {
                continue;
            }

            ctx.set_stroke_style(&self.color_to_css(poly.color));
            ctx.begin_path();

            let (x, y) = self.model_to_view(poly.vertices[0].0, poly.vertices[0].1);
            ctx.move_to(x, y);

            for (px, py) in poly.vertices.iter().skip(1) {
                let (x, y) = self.model_to_view(*px, *py);
                ctx.line_to(x, y);
            }

            if poly.is_closed {
                ctx.close_path();
            }
            ctx.stroke();
        }

        // Draw texts
        let font_size = (12.0 * self.scale).max(8.0).min(24.0);
        ctx.set_font(&format!("{}px sans-serif", font_size as i32));

        for text in &result.texts {
            ctx.set_fill_style(&self.color_to_css(text.color));
            let (x, y) = self.model_to_view(text.x, text.y);

            ctx.save();
            ctx.translate(x, y)?;
            ctx.rotate(-text.rotation * PI / 180.0)?;
            ctx.fill_text(&text.text, 0.0, 0.0)?;
            ctx.restore();
        }

        Ok(())
    }

    /// Convert model coordinates to view coordinates
    fn model_to_view(&self, x: f64, y: f64) -> (f64, f64) {
        let vx = x * self.scale + self.offset_x;
        let vy = -y * self.scale + self.offset_y; // Flip Y axis
        (vx, vy)
    }

    /// Convert DXF color index to CSS color
    fn color_to_css(&self, color: i32) -> JsValue {
        let css = match color {
            1 => "#ff0000",   // Red
            2 => "#ffff00",   // Yellow
            3 => "#00ff00",   // Green
            4 => "#00ffff",   // Cyan
            5 => "#0000ff",   // Blue
            6 => "#ff00ff",   // Magenta
            7 | 0 => "#ffffff", // White (default)
            8 => "#808080",   // Gray
            9 => "#c0c0c0",   // Light gray
            _ => "#ffffff",   // Default white
        };
        JsValue::from_str(css)
    }
}

impl Default for DxfViewer {
    fn default() -> Self {
        Self::new()
    }
}
