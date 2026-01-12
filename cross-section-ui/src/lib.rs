//! 横断図・切削計算システム - egui版
//!
//! PDF横断図に準拠した横断図表示と切削計算

use eframe::egui::{self, Color32, Painter, Pos2, Stroke, Vec2, Rect};
use serde::{Deserialize, Serialize};

// dxf crate for proper DXF file generation
use dxf::Drawing;
use dxf::entities::{Entity, EntityType, Line, Text, RotatedDimension, DimensionBase};
use dxf::enums::{HorizontalTextJustification, VerticalTextJustification, DimensionType};
use dxf::{Color, Point};

// ============================================================================
// DXF Generation (using dxf crate)
// ============================================================================

/// LINE追加ヘルパー
fn add_line(drawing: &mut Drawing, x1: f64, y1: f64, x2: f64, y2: f64, color: i16, layer: &str) {
    let mut line = Line::default();
    line.p1 = Point::new(x1, y1, 0.0);
    line.p2 = Point::new(x2, y2, 0.0);
    let mut entity = Entity::new(EntityType::Line(line));
    entity.common.layer = layer.to_string();
    entity.common.color = Color::from_index(color as u8);
    drawing.add_entity(entity);
}

/// 水平アライメント
#[derive(Clone, Copy)]
enum TextAlign { Left, Center, Right }

/// TEXT追加ヘルパー（アライメント対応）
fn add_text(drawing: &mut Drawing, x: f64, y: f64, text: &str, height: f64, color: i16, layer: &str, align: TextAlign) {
    let mut t = Text::default();
    t.location = Point::new(x, y, 0.0);
    t.text_height = height;
    t.value = text.to_string();
    match align {
        TextAlign::Left => {
            t.horizontal_text_justification = HorizontalTextJustification::Left;
        }
        TextAlign::Center => {
            t.horizontal_text_justification = HorizontalTextJustification::Center;
            t.second_alignment_point = Point::new(x, y, 0.0);
        }
        TextAlign::Right => {
            t.horizontal_text_justification = HorizontalTextJustification::Right;
            t.second_alignment_point = Point::new(x, y, 0.0);
        }
    }
    t.vertical_text_justification = VerticalTextJustification::Middle;
    let mut entity = Entity::new(EntityType::Text(t));
    entity.common.layer = layer.to_string();
    entity.common.color = Color::from_index(color as u8);
    drawing.add_entity(entity);
}

/// Drawingオブジェクトを生成する
pub fn generate_drawing(section: &CrossSectionData) -> Drawing {
    let scale = 1000.0; // mm単位
    let data = &section.survey_data;

    let mut drawing = Drawing::new();
    drawing.header.version = dxf::enums::AcadVersion::R2010;  // サブクラスマーカー出力のため

    // レイヤー作成
    drawing.add_layer(dxf::tables::Layer {
        name: "GROUND".to_string(),
        color: Color::from_index(7),  // 白/黒
        ..Default::default()
    });
    drawing.add_layer(dxf::tables::Layer {
        name: "PLAN".to_string(),
        color: Color::from_index(1),  // 赤
        ..Default::default()
    });
    drawing.add_layer(dxf::tables::Layer {
        name: "TEXT".to_string(),
        color: Color::from_index(7),  // 白/黒
        ..Default::default()
    });
    drawing.add_layer(dxf::tables::Layer {
        name: "DIMENSION".to_string(),
        color: Color::from_index(8),  // グレー
        ..Default::default()
    });
    drawing.add_layer(dxf::tables::Layer {
        name: "CUTTING".to_string(),
        color: Color::from_index(5),  // 青
        ..Default::default()
    });

    if data.len() < 2 {
        return drawing;
    }

    let dl = section.dl;

    let to_dxf_x = |cumulative_distance: f64| -> f64 {
        cumulative_distance * scale
    };
    let to_dxf_y = |height: f64| -> f64 {
        (height - dl) * scale
    };

    let l_data = &data[0];
    let cl_data = &data[section.cl_index.min(data.len() - 1)];
    let r_data = &data[data.len() - 1];

    let left_distance = (cl_data.cumulative_distance - l_data.cumulative_distance).abs();
    let right_distance = (r_data.cumulative_distance - cl_data.cumulative_distance).abs();

    let left_slope = if left_distance > 0.0 {
        ((l_data.planned_height - cl_data.planned_height) / left_distance) * 100.0
    } else { 0.0 };
    let right_slope = if right_distance > 0.0 {
        ((r_data.planned_height - cl_data.planned_height) / right_distance) * 100.0
    } else { 0.0 };

    // ========== 地盤線（黒）==========
    for i in 0..data.len() - 1 {
        let p1_x = to_dxf_x(data[i].cumulative_distance);
        let p1_y = to_dxf_y(data[i].elevation);
        let p2_x = to_dxf_x(data[i + 1].cumulative_distance);
        let p2_y = to_dxf_y(data[i + 1].elevation);
        add_line(&mut drawing, p1_x, p1_y, p2_x, p2_y, 7, "GROUND");
    }

    // ========== 計画高線（赤）==========
    for i in 0..data.len() - 1 {
        let p1_x = to_dxf_x(data[i].cumulative_distance);
        let p1_y = to_dxf_y(data[i].planned_height);
        let p2_x = to_dxf_x(data[i + 1].cumulative_distance);
        let p2_y = to_dxf_y(data[i + 1].planned_height);
        add_line(&mut drawing, p1_x, p1_y, p2_x, p2_y, 1, "PLAN");
    }

    // ========== 切削底面線（青）==========
    for i in 0..data.len() - 1 {
        let p1_x = to_dxf_x(data[i].cumulative_distance);
        let p1_y = to_dxf_y(data[i].cutting_bottom);
        let p2_x = to_dxf_x(data[i + 1].cumulative_distance);
        let p2_y = to_dxf_y(data[i + 1].cutting_bottom);
        add_line(&mut drawing, p1_x, p1_y, p2_x, p2_y, 5, "CUTTING");
    }

    let text_height = 25.0;
    let cl_ground_y = to_dxf_y(cl_data.elevation);
    let flag_height_mm = 800.0;
    let flag_y = cl_ground_y + flag_height_mm;

    let l_x = to_dxf_x(l_data.cumulative_distance);
    let cl_x = to_dxf_x(cl_data.cumulative_distance);
    let r_x = to_dxf_x(r_data.cumulative_distance);

    // ========== 測点名（CL上）==========
    add_text(&mut drawing, cl_x, flag_y + 500.0,
        &section.survey_point_name, text_height * 1.5, 7, "TEXT", TextAlign::Center);

    // ========== CL GH, FH ==========
    add_text(&mut drawing, cl_x, flag_y + 300.0,
        &format!("GH={:.3}", cl_data.elevation), text_height, 7, "TEXT", TextAlign::Center);
    add_text(&mut drawing, cl_x, flag_y + 100.0,
        &format!("FH={:.3}", cl_data.planned_height), text_height, 1, "PLAN", TextAlign::Center);

    // ========== L側 GH, FH ==========
    let l_ground_y = to_dxf_y(l_data.elevation);
    add_text(&mut drawing, l_x, l_ground_y + 300.0,
        &format!("GH={:.3}", l_data.elevation), text_height, 7, "TEXT", TextAlign::Left);
    add_text(&mut drawing, l_x, l_ground_y + 100.0,
        &format!("FH={:.3}", l_data.planned_height), text_height, 1, "PLAN", TextAlign::Left);

    // ========== R側 GH, FH ==========
    let r_ground_y = to_dxf_y(r_data.elevation);
    add_text(&mut drawing, r_x, r_ground_y + 300.0,
        &format!("GH={:.3}", r_data.elevation), text_height, 7, "TEXT", TextAlign::Right);
    add_text(&mut drawing, r_x, r_ground_y + 100.0,
        &format!("FH={:.3}", r_data.planned_height), text_height, 1, "PLAN", TextAlign::Right);

    // ========== 寸法線による旗揚げ（幅員）==========
    let dim_base_y = cl_ground_y + flag_height_mm;
    let mid_l_x = (l_x + cl_x) / 2.0;
    let mid_r_x = (cl_x + r_x) / 2.0;

    // 左幅員の寸法
    {
        let mut dim_base = DimensionBase::default();
        dim_base.definition_point_1 = Point::new(cl_x, dim_base_y, 0.0);
        dim_base.text_mid_point = Point::new(mid_l_x, dim_base_y + 50.0, 0.0);
        dim_base.dimension_type = DimensionType::Aligned;
        dim_base.text = "".to_string();
        dim_base.dimension_style_name = "Standard".to_string();

        let mut rot_dim = RotatedDimension::default();
        rot_dim.dimension_base = dim_base;
        rot_dim.insertion_point = Point::new(l_x, dim_base_y, 0.0);
        rot_dim.definition_point_2 = Point::new(l_x, cl_ground_y, 0.0);
        rot_dim.definition_point_3 = Point::new(cl_x, cl_ground_y, 0.0);
        rot_dim.rotation_angle = 0.0;

        let mut entity = Entity::new(EntityType::RotatedDimension(rot_dim));
        entity.common.layer = "DIMENSION".to_string();
        entity.common.color = Color::from_index(8);
        drawing.add_entity(entity);
    }

    // 右幅員の寸法
    {
        let mut dim_base = DimensionBase::default();
        dim_base.definition_point_1 = Point::new(r_x, dim_base_y, 0.0);
        dim_base.text_mid_point = Point::new(mid_r_x, dim_base_y + 50.0, 0.0);
        dim_base.dimension_type = DimensionType::Aligned;
        dim_base.text = "".to_string();
        dim_base.dimension_style_name = "Standard".to_string();

        let mut rot_dim = RotatedDimension::default();
        rot_dim.dimension_base = dim_base;
        rot_dim.insertion_point = Point::new(cl_x, dim_base_y, 0.0);
        rot_dim.definition_point_2 = Point::new(cl_x, cl_ground_y, 0.0);
        rot_dim.definition_point_3 = Point::new(r_x, cl_ground_y, 0.0);
        rot_dim.rotation_angle = 0.0;

        let mut entity = Entity::new(EntityType::RotatedDimension(rot_dim));
        entity.common.layer = "DIMENSION".to_string();
        entity.common.color = Color::from_index(8);
        drawing.add_entity(entity);
    }

    // ========== 勾配テキスト ==========
    add_text(&mut drawing, mid_l_x, flag_y - text_height - 50.0,
        &format!("il={:.1}%", left_slope), text_height, 7, "TEXT", TextAlign::Center);
    add_text(&mut drawing, mid_r_x, flag_y - text_height - 50.0,
        &format!("ir={:.1}%", right_slope), text_height, 7, "TEXT", TextAlign::Center);

    // ========== DLラベル ==========
    add_text(&mut drawing, cl_x, to_dxf_y(dl) - 200.0,
        &format!("DL={:.3}", dl), text_height, 7, "TEXT", TextAlign::Left);

    // ========== ガイド線 ==========
    let guide_h_length_mm = 6000.0;
    let guide_v_length_mm = 1000.0;
    let cl_cumulative = cl_data.cumulative_distance;

    add_line(&mut drawing,
        to_dxf_x(cl_cumulative - guide_h_length_mm / 2000.0), to_dxf_y(dl),
        to_dxf_x(cl_cumulative + guide_h_length_mm / 2000.0), to_dxf_y(dl),
        8, "DIMENSION");

    add_line(&mut drawing,
        to_dxf_x(cl_cumulative), to_dxf_y(dl),
        to_dxf_x(cl_cumulative), to_dxf_y(dl + guide_v_length_mm / 1000.0),
        8, "DIMENSION");

    drawing
}

/// DXFバイト列を生成する（ダウンロード用）
pub fn generate_dxf_bytes(section: &CrossSectionData) -> Vec<u8> {
    let drawing = generate_drawing(section);
    let mut output: Vec<u8> = Vec::new();
    drawing.save(&mut output).expect("Failed to save DXF");
    output
}

// ============================================================================
// DXF Renderer
// ============================================================================

/// ACI色番号からColor32に変換（白背景用）
fn aci_to_color32(aci: i16) -> Color32 {
    match aci {
        1 => Color32::from_rgb(255, 0, 0),      // 赤
        2 => Color32::from_rgb(255, 255, 0),    // 黄
        3 => Color32::from_rgb(0, 255, 0),      // 緑
        4 => Color32::from_rgb(128, 128, 128),  // グレー
        5 => Color32::from_rgb(0, 0, 255),      // 青
        6 => Color32::from_rgb(255, 0, 255),    // マゼンタ
        7 => Color32::from_rgb(0, 0, 0),        // 黒（白背景用）
        8 => Color32::from_rgb(128, 128, 128),  // グレー
        9 => Color32::from_rgb(192, 192, 192),  // ライトグレー
        _ => Color32::from_rgb(100, 100, 100),
    }
}

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

        let padding = 0.85;
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

fn calc_dxf_bounds(drawing: &Drawing) -> (f32, f32, f32, f32) {
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
                let x = text.location.x as f32;
                let y = text.location.y as f32;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
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
        let margin = (max_x - min_x).max(max_y - min_y) * 0.1;
        (min_x - margin, min_y - margin, max_x + margin, max_y + margin)
    }
}

fn render_dxf(painter: &Painter, drawing: &Drawing, view: &DxfViewState) {
    for entity in drawing.entities() {
        let color_index = match entity.common.color.index() {
            Some(idx) => idx as i16,
            None => 7,
        };
        let color = aci_to_color32(color_index);

        match &entity.specific {
            EntityType::Line(line) => {
                let p1 = view.dxf_to_screen(line.p1.x, line.p1.y);
                let p2 = view.dxf_to_screen(line.p2.x, line.p2.y);
                painter.line_segment([p1, p2], Stroke::new(1.5, color));
            }
            EntityType::Text(text) => {
                let pos = view.dxf_to_screen(text.location.x, text.location.y);
                let align = match text.horizontal_text_justification {
                    HorizontalTextJustification::Left => egui::Align2::LEFT_CENTER,
                    HorizontalTextJustification::Center => egui::Align2::CENTER_CENTER,
                    HorizontalTextJustification::Right => egui::Align2::RIGHT_CENTER,
                    _ => egui::Align2::LEFT_CENTER,
                };
                let font_size = (text.text_height as f32 * view.zoom).clamp(8.0, 24.0);
                let font = egui::FontId::proportional(font_size);
                painter.text(pos, align, &text.value, font, color);
            }
            EntityType::RotatedDimension(dim) => {
                let p2 = &dim.definition_point_2;
                let p3 = &dim.definition_point_3;
                let ins = &dim.insertion_point;
                let text_pt = &dim.dimension_base.text_mid_point;

                let dim_y = ins.y;
                let left_x = p2.x.min(p3.x);
                let right_x = p2.x.max(p3.x);
                let dim_left = view.dxf_to_screen(left_x, dim_y);
                let dim_right = view.dxf_to_screen(right_x, dim_y);
                painter.line_segment([dim_left, dim_right], Stroke::new(1.0, color));

                let ext1_bottom = view.dxf_to_screen(p2.x, p2.y);
                let ext1_top = view.dxf_to_screen(p2.x, dim_y + 50.0);
                painter.line_segment([ext1_bottom, ext1_top], Stroke::new(0.8, color));

                let ext2_bottom = view.dxf_to_screen(p3.x, p3.y);
                let ext2_top = view.dxf_to_screen(p3.x, dim_y + 50.0);
                painter.line_segment([ext2_bottom, ext2_top], Stroke::new(0.8, color));

                let distance = (p3.x - p2.x).abs();
                let text_pos = view.dxf_to_screen(text_pt.x, text_pt.y);
                let font_size = (25.0 * view.zoom).clamp(8.0, 18.0);
                let font = egui::FontId::proportional(font_size);
                painter.text(text_pos, egui::Align2::CENTER_CENTER,
                    &format!("{:.2}", distance / 1000.0), font, color);
            }
            _ => {}
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn download_file(filename: &str, content: &[u8]) {
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
fn download_file(_filename: &str, _content: &[u8]) {}

// ============================================================================
// Data Structures
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SurveyRow {
    pub unit_distance: f64,
    pub elevation: f64,
    pub planned_height: f64,
    pub cumulative_distance: f64,
    pub cutting_bottom: f64,
}

impl SurveyRow {
    pub fn cutting_depth(&self) -> f64 { self.elevation - self.cutting_bottom }
    pub fn pavement_thickness(&self) -> f64 { self.planned_height - self.cutting_bottom }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrossSectionData {
    pub survey_point_name: String,
    pub dl: f64,
    pub cl_index: usize,
    pub l_to_cl_distance: f64,
    pub survey_data: Vec<SurveyRow>,
}

impl CrossSectionData {
    fn calc_cumulative_distances(unit_distances: &[f64], cl_index: usize) -> Vec<f64> {
        let mut cumulative = Vec::with_capacity(unit_distances.len());
        let mut sum = 0.0;
        for (i, &d) in unit_distances.iter().enumerate() {
            if i == 0 { cumulative.push(0.0); }
            else { sum += d; cumulative.push(sum); }
        }
        let cl_offset = cumulative[cl_index];
        cumulative.iter().map(|&c| c - cl_offset).collect()
    }

    fn from_3point(
        name: &str, w_l: f64, w_r: f64,
        gh_l: f64, gh_cl: f64, gh_r: f64,
        fh_l: f64, fh_cl: f64, fh_r: f64,
        dl: f64, cutting_depth: f64,
    ) -> Self {
        let unit_distances = vec![0.0, w_l, w_r];
        let cl_index = 1;
        let elevations = vec![gh_l, gh_cl, gh_r];
        let planned_heights = vec![fh_l, fh_cl, fh_r];
        let cumulative = Self::calc_cumulative_distances(&unit_distances, cl_index);
        let cutting_bottoms: Vec<f64> = planned_heights.iter().map(|&fh| fh - cutting_depth).collect();
        let l_to_cl = cumulative[0].abs();

        let survey_data: Vec<SurveyRow> = (0..unit_distances.len()).map(|i| SurveyRow {
            unit_distance: unit_distances[i],
            elevation: elevations[i],
            planned_height: planned_heights[i],
            cumulative_distance: cumulative[i],
            cutting_bottom: cutting_bottoms[i],
        }).collect();

        CrossSectionData {
            survey_point_name: name.to_string(),
            dl, cl_index, l_to_cl_distance: l_to_cl, survey_data
        }
    }

    pub fn all_samples() -> Vec<Self> {
        let cut = 0.05;
        vec![
            Self::from_3point("No.0",  2.75, 2.70,  9.500, 9.490, 9.427,  9.500, 9.490, 9.427,  9.0, cut),
            Self::from_3point("No.2",  2.60, 2.52,  11.862, 11.913, 11.836,  11.862, 11.913, 11.836,  11.0, cut),
            Self::from_3point("No.4",  2.56, 2.54,  14.633, 14.700, 14.644,  14.633, 14.700, 14.644,  14.0, cut),
            Self::from_3point("No.6",  2.53, 2.59,  17.417, 17.467, 17.403,  17.417, 17.467, 17.403,  17.0, cut),
            Self::from_3point("No.8",  2.53, 2.57,  19.846, 19.927, 19.855,  19.846, 19.927, 19.855,  19.0, cut),
            Self::from_3point("No.10", 2.58, 2.55,  20.505, 20.576, 20.525,  20.505, 20.576, 20.525,  20.0, cut),
            Self::from_3point("No.12", 2.56, 2.56,  20.967, 21.026, 20.973,  20.967, 21.026, 20.973,  20.0, cut),
            Self::from_3point("No.14", 2.55, 2.60,  22.360, 22.405, 22.354,  22.360, 22.405, 22.354,  22.0, cut),
            Self::from_3point("No.16", 2.61, 2.59,  25.135, 25.174, 25.111,  25.135, 25.174, 25.111,  25.0, cut),
            Self::from_3point("No.18", 2.55, 2.62,  27.595, 27.735, 27.749,  27.595, 27.735, 27.749,  27.0, cut),
        ]
    }
}

// ============================================================================
// Application
// ============================================================================

pub struct CrossSectionApp {
    sections: Vec<CrossSectionData>,
    selected_index: Option<usize>,
    dxf_drawing: Option<Drawing>,
    dxf_view_state: DxfViewState,
}

impl Default for CrossSectionApp {
    fn default() -> Self {
        Self {
            sections: Vec::new(),
            selected_index: None,
            dxf_drawing: None,
            dxf_view_state: DxfViewState::default(),
        }
    }
}

impl CrossSectionApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::default();
        app.load_samples();
        app
    }

    fn load_samples(&mut self) {
        self.sections = CrossSectionData::all_samples();
        self.selected_index = Some(0);
        self.update_dxf_preview();
    }

    fn update_dxf_preview(&mut self) {
        if let Some(idx) = self.selected_index {
            if let Some(section) = self.sections.get(idx) {
                let drawing = generate_drawing(section);
                let (min_x, min_y, max_x, max_y) = calc_dxf_bounds(&drawing);
                self.dxf_view_state.fit_to_dxf(min_x, min_y, max_x, max_y);
                self.dxf_drawing = Some(drawing);
            }
        }
    }
}

impl eframe::App for CrossSectionApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("side_panel").min_width(180.0).show(ctx, |ui| {
            ui.heading("Cross Section");
            ui.separator();

            if ui.button("Load Sample").clicked() {
                self.load_samples();
            }

            if self.selected_index.is_some() {
                if ui.button("Download DXF").clicked() {
                    if let Some(idx) = self.selected_index {
                        if let Some(section) = self.sections.get(idx) {
                            let dxf_content = generate_dxf_bytes(section);
                            let filename = format!("{}.dxf", section.survey_point_name);
                            download_file(&filename, &dxf_content);
                        }
                    }
                }
            }
            ui.separator();

            ui.label("Stations:");
            let mut new_selection = None;
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (i, section) in self.sections.iter().enumerate() {
                    let selected = self.selected_index == Some(i);
                    if ui.selectable_label(selected, &section.survey_point_name).clicked() {
                        new_selection = Some(i);
                    }
                }
            });
            if let Some(idx) = new_selection {
                self.selected_index = Some(idx);
                self.update_dxf_preview();
            }

            ui.separator();
            if let Some(idx) = self.selected_index {
                if let Some(section) = self.sections.get(idx) {
                    ui.label(format!("DL: {:.3}", section.dl));
                    ui.label(format!("L->CL: {:.2}m", section.l_to_cl_distance));
                }
            }

            ui.separator();
            ui.label("Legend:");
            ui.horizontal(|ui| {
                ui.colored_label(Color32::BLACK, "=");
                ui.label("Ground(GH)");
            });
            ui.horizontal(|ui| {
                ui.colored_label(Color32::from_rgb(255, 0, 0), "=");
                ui.label("Planned(FH)");
            });
            ui.horizontal(|ui| {
                ui.colored_label(Color32::from_rgb(0, 0, 255), "=");
                ui.label("Cutting");
            });
            ui.horizontal(|ui| {
                ui.colored_label(Color32::from_rgb(128, 128, 128), "=");
                ui.label("Dimension");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let available = ui.available_size();
            let (response, painter) = ui.allocate_painter(available, egui::Sense::click_and_drag());

            // 白背景
            painter.rect_filled(response.rect, 0.0, Color32::from_rgb(250, 250, 250));
            self.dxf_view_state.canvas_rect = response.rect;

            if response.dragged() {
                self.dxf_view_state.pan += response.drag_delta();
            }

            let scroll = ui.input(|i| i.raw_scroll_delta.y);
            if scroll != 0.0 {
                let zoom_factor = if scroll > 0.0 { 1.1 } else { 0.9 };
                self.dxf_view_state.zoom *= zoom_factor;
                self.dxf_view_state.zoom = self.dxf_view_state.zoom.clamp(0.01, 5.0);
            }

            if let Some(ref drawing) = self.dxf_drawing {
                render_dxf(&painter, drawing, &self.dxf_view_state);
            } else {
                painter.text(
                    response.rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Click 'Load Sample' to start",
                    egui::FontId::proportional(16.0),
                    Color32::GRAY
                );
            }

            if let Some(idx) = self.selected_index {
                if let Some(section) = self.sections.get(idx) {
                    egui::Window::new("Cutting Calculation")
                        .default_pos([response.rect.right() - 320.0, response.rect.bottom() - 150.0])
                        .show(ctx, |ui| {
                            egui::Grid::new("calc_table").striped(true).show(ui, |ui| {
                                ui.label("Dist");
                                ui.label("GH");
                                ui.label("FH");
                                ui.label("Cut");
                                ui.label("Depth(cm)");
                                ui.end_row();

                                for p in &section.survey_data {
                                    ui.label(format!("{:.2}", p.cumulative_distance));
                                    ui.label(format!("{:.3}", p.elevation));
                                    ui.label(format!("{:.3}", p.planned_height));
                                    ui.label(format!("{:.3}", p.cutting_bottom));
                                    ui.label(format!("{:.1}", p.cutting_depth() * 100.0));
                                    ui.end_row();
                                }
                            });
                        });
                }
            }
        });
    }
}

// ============================================================================
// WASM Entry Point
// ============================================================================

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{prelude::*, JsCast};
#[cfg(target_arch = "wasm32")]
use web_sys::HtmlCanvasElement;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).ok();

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;
    let document = window.document().ok_or_else(|| JsValue::from_str("No document"))?;
    let canvas = document
        .get_element_by_id("canvas")
        .ok_or_else(|| JsValue::from_str("No canvas element"))?
        .dyn_into::<HtmlCanvasElement>()
        .map_err(|_| JsValue::from_str("Not a canvas"))?;

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async move {
        eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(CrossSectionApp::new(cc)))),
            )
            .await
            .expect("Failed to start eframe");
    });

    Ok(())
}
