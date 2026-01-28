//! DxfViewState - DXF描画のビュー状態管理
//!
//! パン/ズーム状態、座標変換、境界計算を提供

use eframe::egui::{Pos2, Vec2, Rect};
use dxf::Drawing;
use dxf::entities::EntityType;
use dxf::enums::{HorizontalTextJustification, VerticalTextJustification};

use crate::drawing_ir::ViewState as IrViewState;

// ============================================================================
// DxfViewState
// ============================================================================

#[derive(Clone)]
pub struct DxfViewState {
    pub zoom: f32,
    pub pan: Vec2,
    pub canvas_rect: Rect,
}

impl Default for DxfViewState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: Vec2::ZERO,
            canvas_rect: Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 400.0)),
        }
    }
}

impl DxfViewState {
    pub fn dxf_to_screen(&self, x: f64, y: f64) -> Pos2 {
        Pos2::new(
            self.canvas_rect.min.x + self.pan.x + (x as f32) * self.zoom,
            self.canvas_rect.min.y + self.pan.y - (y as f32) * self.zoom,
        )
    }

    pub fn fit_to_dxf(&mut self, min_x: f32, min_y: f32, max_x: f32, max_y: f32) {
        let content_width = (max_x - min_x).max(0.1);
        let content_height = (max_y - min_y).max(0.1);

        let padding = 0.95; // 画面の95%を使用（5%マージン）
        let zoom_x = self.canvas_rect.width() * padding / content_width;
        let zoom_y = self.canvas_rect.height() * padding / content_height;
        self.zoom = zoom_x.min(zoom_y);

        let center_x = (min_x + max_x) / 2.0;
        let center_y = (min_y + max_y) / 2.0;

        self.pan = Vec2::new(
            self.canvas_rect.width() / 2.0 - center_x * self.zoom,
            self.canvas_rect.height() / 2.0 + center_y * self.zoom,
        );
    }
}

impl From<&DxfViewState> for IrViewState {
    fn from(state: &DxfViewState) -> Self {
        IrViewState {
            zoom: state.zoom,
            pan: state.pan,
            canvas_rect: state.canvas_rect,
        }
    }
}

// ============================================================================
// calc_dxf_bounds
// ============================================================================

pub fn calc_dxf_bounds(drawing: &Drawing) -> (f32, f32, f32, f32) {
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;

    for entity in drawing.entities() {
        match &entity.specific {
            EntityType::Line(line) => {
                min_x = min_x.min(line.p1.x as f32).min(line.p2.x as f32);
                min_y = min_y.min(line.p1.y as f32).min(line.p2.y as f32);
                max_x = max_x.max(line.p1.x as f32).max(line.p2.x as f32);
                max_y = max_y.max(line.p1.y as f32).max(line.p2.y as f32);
            }
            EntityType::Text(text) => {
                // テキストの実際の描画範囲を推定
                let base_x = text.location.x as f32;
                let base_y = text.location.y as f32;
                let text_height = text.text_height as f32;
                // テキスト幅を推定（文字数 * 高さ * 0.6）
                let estimated_width = text.value.len() as f32 * text_height * 0.6;

                // アライメントに基づくオフセット計算
                // 水平方向: 描画開始位置
                let (text_left, text_right) = match text.horizontal_text_justification {
                    HorizontalTextJustification::Left => (base_x, base_x + estimated_width),
                    HorizontalTextJustification::Center => (base_x - estimated_width / 2.0, base_x + estimated_width / 2.0),
                    HorizontalTextJustification::Right => (base_x - estimated_width, base_x),
                    _ => (base_x, base_x + estimated_width),
                };

                // 垂直方向: 描画範囲
                let (text_bottom, text_top) = match text.vertical_text_justification {
                    VerticalTextJustification::Baseline | VerticalTextJustification::Bottom => (base_y - text_height, base_y),
                    VerticalTextJustification::Middle => (base_y - text_height / 2.0, base_y + text_height / 2.0),
                    VerticalTextJustification::Top => (base_y, base_y + text_height),
                };

                // 回転を考慮（簡易版: 回転時は4コーナーをすべてチェック）
                let angle_rad = (text.rotation as f32).to_radians();
                if angle_rad.abs() > 0.01 {
                    // 回転がある場合：4コーナーを回転して最大範囲を取る
                    let corners = [
                        (text_left, text_bottom),
                        (text_right, text_bottom),
                        (text_right, text_top),
                        (text_left, text_top),
                    ];
                    let cos_a = angle_rad.cos();
                    let sin_a = angle_rad.sin();
                    for (cx, cy) in corners {
                        let dx = cx - base_x;
                        let dy = cy - base_y;
                        let rx = base_x + dx * cos_a - dy * sin_a;
                        let ry = base_y + dx * sin_a + dy * cos_a;
                        min_x = min_x.min(rx);
                        min_y = min_y.min(ry);
                        max_x = max_x.max(rx);
                        max_y = max_y.max(ry);
                    }
                } else {
                    min_x = min_x.min(text_left);
                    min_y = min_y.min(text_bottom);
                    max_x = max_x.max(text_right);
                    max_y = max_y.max(text_top);
                }
            }
            EntityType::RotatedDimension(dim) => {
                let points = [
                    &dim.definition_point_2,
                    &dim.definition_point_3,
                    &dim.insertion_point,
                    &dim.dimension_base.text_mid_point,
                ];
                for p in points {
                    min_x = min_x.min(p.x as f32);
                    min_y = min_y.min(p.y as f32);
                    max_x = max_x.max(p.x as f32);
                    max_y = max_y.max(p.y as f32);
                }
            }
            _ => {}
        }
    }

    if min_x == f32::MAX {
        (0.0, 0.0, 1000.0, 1000.0)
    } else {
        // マージンなしで返す（レイアウト計算用にタイトなバウンドが必要）
        (min_x, min_y, max_x, max_y)
    }
}

// ============================================================================
// download_file
// ============================================================================

#[cfg(target_arch = "wasm32")]
pub fn download_file(filename: &str, content: &[u8]) {
    use wasm_bindgen::JsCast;
    let window = match web_sys::window() { Some(w) => w, None => return };
    let document = match window.document() { Some(d) => d, None => return };

    let uint8_array = js_sys::Uint8Array::from(content);
    let blob_parts = js_sys::Array::new();
    blob_parts.push(&uint8_array);

    let options = web_sys::BlobPropertyBag::new();
    options.set_type("application/dxf");

    let blob = match web_sys::Blob::new_with_u8_array_sequence_and_options(&blob_parts, &options) {
        Ok(b) => b, Err(_) => return
    };

    let url = match web_sys::Url::create_object_url_with_blob(&blob) {
        Ok(u) => u, Err(_) => return
    };

    if let Ok(element) = document.create_element("a") {
        if let Some(anchor) = element.dyn_ref::<web_sys::HtmlAnchorElement>() {
            anchor.set_href(&url);
            anchor.set_download(filename);
            anchor.click();
        }
    }
    let _ = web_sys::Url::revoke_object_url(&url);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn download_file(_filename: &str, _content: &[u8]) {}
