//! DXF Viewer - WebAssembly Module
//!
//! A DXF file viewer that runs in the browser using WebAssembly.
//! Uses the `dxf` crate for parsing.

use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;
use std::f64::consts::PI;
use std::io::Cursor;

/// Entity data extracted from DXF for rendering
#[derive(Clone, Default)]
struct RenderData {
    lines: Vec<LineData>,
    circles: Vec<CircleData>,
    arcs: Vec<ArcData>,
    polylines: Vec<PolylineData>,
    texts: Vec<TextData>,
    bounds: (f64, f64, f64, f64),
}

#[derive(Clone)]
struct LineData {
    x1: f64, y1: f64,
    x2: f64, y2: f64,
    color: i16,
}

#[derive(Clone)]
struct CircleData {
    center_x: f64, center_y: f64,
    radius: f64,
    color: i16,
}

#[derive(Clone)]
struct ArcData {
    center_x: f64, center_y: f64,
    radius: f64,
    start_angle: f64,
    end_angle: f64,
    color: i16,
}

#[derive(Clone)]
struct PolylineData {
    vertices: Vec<(f64, f64)>,
    is_closed: bool,
    color: i16,
}

#[derive(Clone)]
struct TextData {
    x: f64, y: f64,
    height: f64,
    text: String,
    rotation: f64,
    color: i16,
}

/// DXF Viewer state
#[wasm_bindgen]
pub struct DxfViewer {
    render_data: Option<RenderData>,
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
            render_data: None,
            scale: 1.0,
            offset_x: 0.0,
            offset_y: 0.0,
            canvas_width: 800.0,
            canvas_height: 600.0,
        }
    }

    /// Load and parse DXF content using the dxf crate
    #[wasm_bindgen]
    pub fn load_dxf(&mut self, content: &str) -> Result<String, JsValue> {
        let mut cursor = Cursor::new(content.as_bytes());

        let drawing = dxf::Drawing::load(&mut cursor)
            .map_err(|e| JsValue::from_str(&format!("Parse error: {}", e)))?;

        let render_data = self.extract_render_data(&drawing);

        let summary = format!(
            "Loaded: {} lines, {} circles, {} arcs, {} polylines, {} texts",
            render_data.lines.len(),
            render_data.circles.len(),
            render_data.arcs.len(),
            render_data.polylines.len(),
            render_data.texts.len()
        );

        self.fit_to_canvas(&render_data);
        self.render_data = Some(render_data);

        Ok(summary)
    }

    /// Extract render data from DXF drawing
    fn extract_render_data(&self, drawing: &dxf::Drawing) -> RenderData {
        let mut data = RenderData::default();
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;

        for entity in drawing.entities() {
            let color = entity.common.color.index().unwrap_or(7) as i16;

            match &entity.specific {
                dxf::entities::EntityType::Line(line) => {
                    let x1 = line.p1.x;
                    let y1 = line.p1.y;
                    let x2 = line.p2.x;
                    let y2 = line.p2.y;

                    min_x = min_x.min(x1).min(x2);
                    min_y = min_y.min(y1).min(y2);
                    max_x = max_x.max(x1).max(x2);
                    max_y = max_y.max(y1).max(y2);

                    data.lines.push(LineData {
                        x1, y1, x2, y2, color,
                    });
                }
                dxf::entities::EntityType::Circle(circle) => {
                    let cx = circle.center.x;
                    let cy = circle.center.y;
                    let r = circle.radius;

                    min_x = min_x.min(cx - r);
                    min_y = min_y.min(cy - r);
                    max_x = max_x.max(cx + r);
                    max_y = max_y.max(cy + r);

                    data.circles.push(CircleData {
                        center_x: cx,
                        center_y: cy,
                        radius: r,
                        color,
                    });
                }
                dxf::entities::EntityType::Arc(arc) => {
                    let cx = arc.center.x;
                    let cy = arc.center.y;
                    let r = arc.radius;

                    min_x = min_x.min(cx - r);
                    min_y = min_y.min(cy - r);
                    max_x = max_x.max(cx + r);
                    max_y = max_y.max(cy + r);

                    data.arcs.push(ArcData {
                        center_x: cx,
                        center_y: cy,
                        radius: r,
                        start_angle: arc.start_angle,
                        end_angle: arc.end_angle,
                        color,
                    });
                }
                dxf::entities::EntityType::LwPolyline(poly) => {
                    let vertices: Vec<(f64, f64)> = poly.vertices.iter()
                        .map(|v| (v.x, v.y))
                        .collect();

                    for (x, y) in &vertices {
                        min_x = min_x.min(*x);
                        min_y = min_y.min(*y);
                        max_x = max_x.max(*x);
                        max_y = max_y.max(*y);
                    }

                    data.polylines.push(PolylineData {
                        vertices,
                        is_closed: poly.flags & 1 != 0,
                        color,
                    });
                }
                dxf::entities::EntityType::Text(text) => {
                    let x = text.location.x;
                    let y = text.location.y;

                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);

                    data.texts.push(TextData {
                        x, y,
                        height: text.text_height,
                        text: text.value.clone(),
                        rotation: text.rotation,
                        color,
                    });
                }
                dxf::entities::EntityType::MText(mtext) => {
                    let x = mtext.insertion_point.x;
                    let y = mtext.insertion_point.y;

                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);

                    data.texts.push(TextData {
                        x, y,
                        height: mtext.initial_text_height,
                        text: mtext.text.clone(),
                        rotation: mtext.rotation_angle,
                        color,
                    });
                }
                _ => {}
            }
        }

        if min_x == f64::MAX {
            data.bounds = (0.0, 0.0, 100.0, 100.0);
        } else {
            data.bounds = (min_x, min_y, max_x, max_y);
        }

        data
    }

    /// Get result as JSON
    #[wasm_bindgen]
    pub fn get_result_json(&self) -> Result<String, JsValue> {
        match &self.render_data {
            Some(data) => {
                let json = format!(
                    r#"{{"lines":{},"circles":{},"arcs":{},"polylines":{},"texts":{}}}"#,
                    data.lines.len(),
                    data.circles.len(),
                    data.arcs.len(),
                    data.polylines.len(),
                    data.texts.len()
                );
                Ok(json)
            }
            None => Ok("null".to_string()),
        }
    }

    /// Calculate scale and offset to fit content in canvas
    fn fit_to_canvas(&mut self, data: &RenderData) {
        let (min_x, min_y, max_x, max_y) = data.bounds;

        if max_x <= min_x || max_y <= min_y {
            return;
        }

        let content_width = max_x - min_x;
        let content_height = max_y - min_y;

        let scale_x = self.canvas_width * 0.9 / content_width;
        let scale_y = self.canvas_height * 0.9 / content_height;
        self.scale = scale_x.min(scale_y);

        let center_x = (min_x + max_x) / 2.0;
        let center_y = (min_y + max_y) / 2.0;

        self.offset_x = self.canvas_width / 2.0 - center_x * self.scale;
        self.offset_y = self.canvas_height / 2.0 + center_y * self.scale;
    }

    /// Set canvas size
    #[wasm_bindgen]
    pub fn set_canvas_size(&mut self, width: f64, height: f64) {
        self.canvas_width = width;
        self.canvas_height = height;
        if let Some(data) = &self.render_data {
            let data_clone = data.clone();
            self.fit_to_canvas(&data_clone);
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

        self.offset_x = center_x - (center_x - self.offset_x) * (self.scale / old_scale);
        self.offset_y = center_y - (center_y - self.offset_y) * (self.scale / old_scale);
    }

    /// Render to canvas
    #[wasm_bindgen]
    pub fn render(&self, ctx: &CanvasRenderingContext2d) -> Result<(), JsValue> {
        let data = match &self.render_data {
            Some(d) => d,
            None => return Ok(()),
        };

        // Clear canvas
        ctx.set_fill_style(&JsValue::from_str("#1a1a2e"));
        ctx.fill_rect(0.0, 0.0, self.canvas_width, self.canvas_height);

        ctx.set_line_width(1.0);

        // Draw lines
        for line in &data.lines {
            ctx.set_stroke_style(&self.color_to_css(line.color));
            ctx.begin_path();
            let (x1, y1) = self.model_to_view(line.x1, line.y1);
            let (x2, y2) = self.model_to_view(line.x2, line.y2);
            ctx.move_to(x1, y1);
            ctx.line_to(x2, y2);
            ctx.stroke();
        }

        // Draw circles
        for circle in &data.circles {
            ctx.set_stroke_style(&self.color_to_css(circle.color));
            ctx.begin_path();
            let (cx, cy) = self.model_to_view(circle.center_x, circle.center_y);
            let r = circle.radius * self.scale;
            ctx.arc(cx, cy, r, 0.0, 2.0 * PI)?;
            ctx.stroke();
        }

        // Draw arcs
        for arc in &data.arcs {
            ctx.set_stroke_style(&self.color_to_css(arc.color));
            ctx.begin_path();
            let (cx, cy) = self.model_to_view(arc.center_x, arc.center_y);
            let r = arc.radius * self.scale;
            let start = -arc.end_angle * PI / 180.0;
            let end = -arc.start_angle * PI / 180.0;
            ctx.arc(cx, cy, r, start, end)?;
            ctx.stroke();
        }

        // Draw polylines
        for poly in &data.polylines {
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
        for text in &data.texts {
            let font_size = (text.height * self.scale).max(4.0);
            ctx.set_font(&format!("{}px sans-serif", font_size as i32));
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
        let vy = -y * self.scale + self.offset_y;
        (vx, vy)
    }

    /// Convert DXF color index to CSS color
    fn color_to_css(&self, color: i16) -> JsValue {
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
