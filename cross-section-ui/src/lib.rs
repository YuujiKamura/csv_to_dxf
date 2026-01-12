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

/// 垂直アライメント
#[derive(Clone, Copy)]
enum VerticalAlign { Top, Middle, Bottom }

/// TEXT追加ヘルパー（回転・垂直アライメント対応）
/// rotation: 度数法（0=水平、90=下から上に読む方向）
fn add_text_rotated(
    drawing: &mut Drawing, x: f64, y: f64, text: &str, height: f64,
    color: i16, layer: &str, align: TextAlign, v_align: VerticalAlign, rotation: f64
) {
    let mut t = Text::default();
    t.location = Point::new(x, y, 0.0);
    t.text_height = height;
    t.value = text.to_string();
    t.rotation = rotation;

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

    match v_align {
        VerticalAlign::Top => {
            t.vertical_text_justification = VerticalTextJustification::Top;
        }
        VerticalAlign::Middle => {
            t.vertical_text_justification = VerticalTextJustification::Middle;
        }
        VerticalAlign::Bottom => {
            t.vertical_text_justification = VerticalTextJustification::Bottom;
        }
    }

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

    let text_height = 150.0;
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
// Multi-Section Grid Layout
// ============================================================================

/// 複数横断図をグリッド配置したDrawingを生成
/// 道路工事の配置ルール: 左下起点、列ごとに下から上へ、左から右へ
pub fn generate_multi_drawing(sections: &[CrossSectionData], columns: usize) -> Drawing {
    let scale = 1000.0;

    let mut drawing = Drawing::new();
    drawing.header.version = dxf::enums::AcadVersion::R2010;

    for (name, color_idx) in [
        ("GROUND", 7), ("PLAN", 1), ("TEXT", 7),
        ("DIMENSION", 8), ("CUTTING", 5), ("FRAME", 9)
    ] {
        drawing.add_layer(dxf::tables::Layer {
            name: name.to_string(),
            color: Color::from_index(color_idx),
            ..Default::default()
        });
    }

    if sections.is_empty() { return drawing; }

    let mut max_width: f64 = 0.0;
    let mut max_height: f64 = 0.0;

    for section in sections {
        if section.survey_data.len() < 2 { continue; }
        let data = &section.survey_data;
        let total_width = (data.last().unwrap().cumulative_distance
                         - data.first().unwrap().cumulative_distance).abs();
        max_width = max_width.max(total_width);
        let max_elev = data.iter().map(|d| d.elevation.max(d.planned_height)).fold(f64::MIN, f64::max);
        max_height = max_height.max(max_elev - section.dl + 1.5);
    }

    let cell_width = (max_width + 2.0) * scale;
    let cell_height = (max_height + 1.0) * scale;

    // 道路工事の配置: 左下起点、列ごとに下から上
    let rows_per_column = (sections.len() + columns - 1) / columns;  // ceil division

    for (idx, section) in sections.iter().enumerate() {
        if section.survey_data.len() < 2 { continue; }
        let col = idx / rows_per_column;           // 列番号（左から右）
        let row_in_col = idx % rows_per_column;    // 列内の行番号（下から上）
        let offset_x = col as f64 * cell_width;
        let offset_y = row_in_col as f64 * cell_height;  // 正の方向（下から上）
        draw_section_at_offset(&mut drawing, section, offset_x, offset_y, scale);
    }
    drawing
}

fn draw_section_at_offset(drawing: &mut Drawing, section: &CrossSectionData,
                          offset_x: f64, offset_y: f64, scale: f64) {
    let data = &section.survey_data;
    let dl = section.dl;
    let to_dxf_x = |d: f64| offset_x + d * scale;
    let to_dxf_y = |h: f64| offset_y + (h - dl) * scale;

    let l_data = &data[0];
    let cl_data = &data[section.cl_index.min(data.len() - 1)];
    let r_data = &data[data.len() - 1];

    let left_dist = (cl_data.cumulative_distance - l_data.cumulative_distance).abs();
    let right_dist = (r_data.cumulative_distance - cl_data.cumulative_distance).abs();
    let left_slope = if left_dist > 0.0 { ((l_data.planned_height - cl_data.planned_height) / left_dist) * 100.0 } else { 0.0 };
    let right_slope = if right_dist > 0.0 { ((r_data.planned_height - cl_data.planned_height) / right_dist) * 100.0 } else { 0.0 };

    for i in 0..data.len() - 1 {
        add_line(drawing, to_dxf_x(data[i].cumulative_distance), to_dxf_y(data[i].elevation),
            to_dxf_x(data[i + 1].cumulative_distance), to_dxf_y(data[i + 1].elevation), 7, "GROUND");
        add_line(drawing, to_dxf_x(data[i].cumulative_distance), to_dxf_y(data[i].planned_height),
            to_dxf_x(data[i + 1].cumulative_distance), to_dxf_y(data[i + 1].planned_height), 1, "PLAN");
        add_line(drawing, to_dxf_x(data[i].cumulative_distance), to_dxf_y(data[i].cutting_bottom),
            to_dxf_x(data[i + 1].cumulative_distance), to_dxf_y(data[i + 1].cutting_bottom), 5, "CUTTING");
    }

    let text_height = 150.0;
    let cl_ground_y = to_dxf_y(cl_data.elevation);
    let flag_y = cl_ground_y + 800.0;
    let l_x = to_dxf_x(l_data.cumulative_distance);
    let cl_x = to_dxf_x(cl_data.cumulative_distance);
    let r_x = to_dxf_x(r_data.cumulative_distance);

    add_text(drawing, cl_x, flag_y + 500.0, &section.survey_point_name, text_height * 1.5, 7, "TEXT", TextAlign::Center);
    add_text(drawing, cl_x, flag_y + 300.0, &format!("GH={:.3}", cl_data.elevation), text_height, 7, "TEXT", TextAlign::Center);
    add_text(drawing, cl_x, flag_y + 100.0, &format!("FH={:.3}", cl_data.planned_height), text_height, 1, "PLAN", TextAlign::Center);

    let l_ground_y = to_dxf_y(l_data.elevation);
    add_text(drawing, l_x, l_ground_y + 300.0, &format!("GH={:.3}", l_data.elevation), text_height, 7, "TEXT", TextAlign::Left);
    add_text(drawing, l_x, l_ground_y + 100.0, &format!("FH={:.3}", l_data.planned_height), text_height, 1, "PLAN", TextAlign::Left);

    let r_ground_y = to_dxf_y(r_data.elevation);
    add_text(drawing, r_x, r_ground_y + 300.0, &format!("GH={:.3}", r_data.elevation), text_height, 7, "TEXT", TextAlign::Right);
    add_text(drawing, r_x, r_ground_y + 100.0, &format!("FH={:.3}", r_data.planned_height), text_height, 1, "PLAN", TextAlign::Right);

    let mid_l_x = (l_x + cl_x) / 2.0;
    let mid_r_x = (cl_x + r_x) / 2.0;
    add_text(drawing, mid_l_x, flag_y - text_height - 50.0, &format!("il={:.1}%", left_slope), text_height, 7, "TEXT", TextAlign::Center);
    add_text(drawing, mid_r_x, flag_y - text_height - 50.0, &format!("ir={:.1}%", right_slope), text_height, 7, "TEXT", TextAlign::Center);

    // 寸法線（幅員）
    let dim_base_y = flag_y;

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

    add_text(drawing, cl_x, to_dxf_y(dl) - 200.0, &format!("DL={:.3}", dl), text_height, 7, "TEXT", TextAlign::Left);

    let cl_cumulative = cl_data.cumulative_distance;
    add_line(drawing, to_dxf_x(cl_cumulative - 3.0), to_dxf_y(dl), to_dxf_x(cl_cumulative + 3.0), to_dxf_y(dl), 8, "DIMENSION");
    add_line(drawing, to_dxf_x(cl_cumulative), to_dxf_y(dl), to_dxf_x(cl_cumulative), to_dxf_y(dl + 1.0), 8, "DIMENSION");
}

pub fn generate_multi_dxf_bytes(sections: &[CrossSectionData], columns: usize) -> Vec<u8> {
    let drawing = generate_multi_drawing(sections, columns);
    let mut output: Vec<u8> = Vec::new();
    drawing.save(&mut output).expect("Failed to save DXF");
    output
}

// ============================================================================
// Longitudinal Profile (縦断図)
// ============================================================================

/// 測点名から路線距離を取得（"No.X" → X * 10m）
fn parse_station_distance(name: &str) -> f64 {
    if name.starts_with("No.") {
        name[3..].parse::<f64>().unwrap_or(0.0) * 10.0
    } else {
        0.0
    }
}

/// 測点名がプラス杭かどうか判定（例: "No.1+5" はプラス杭）
fn is_plus_stake(name: &str) -> bool {
    name.contains('+')
}

/// 縦断図を生成（土木標準形式）
pub fn generate_longitudinal_drawing(sections: &[CrossSectionData]) -> Drawing {
    // スケール設定（DXF単位）
    let scale_x = 100.0;     // 横方向スケール（1m = 100単位）
    let scale_y = 500.0;     // 縦方向スケール（1m = 500単位）
    let text_height = 120.0; // 基本テキスト高さ
    let row_height = 350.0;  // データ表の行高さ（回転テキスト対応）
    let label_width = 500.0; // 左側のラベル幅

    let mut drawing = Drawing::new();
    drawing.header.version = dxf::enums::AcadVersion::R2010;

    // レイヤー作成
    for (name, color_idx) in [
        ("GROUND", 7),    // 黒 - 現地盤高
        ("PLAN", 1),      // 赤 - 計画高
        ("GRID", 8),      // グレー - グリッド
        ("TABLE", 7),     // 黒 - 表枠
        ("TEXT", 7),      // 黒 - テキスト
    ] {
        let mut layer = dxf::tables::Layer::default();
        layer.name = name.to_string();
        layer.color = Color::from_index(color_idx);
        drawing.add_layer(layer);
    }

    // データ収集（路線距離順にソート）
    let mut points: Vec<(f64, f64, f64, String)> = sections.iter().map(|s| {
        let cl = &s.survey_data[s.cl_index.min(s.survey_data.len() - 1)];
        let dist = parse_station_distance(&s.survey_point_name);
        (dist, cl.elevation, cl.planned_height, s.survey_point_name.clone())
    }).collect();
    points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    if points.is_empty() { return drawing; }

    // 範囲計算
    let min_dist = points.first().map(|p| p.0).unwrap_or(0.0);
    let max_dist = points.last().map(|p| p.0).unwrap_or(100.0);
    let min_elev = points.iter().map(|p| p.1.min(p.2)).fold(f64::MAX, f64::min);
    let max_elev = points.iter().map(|p| p.1.max(p.2)).fold(f64::MIN, f64::max);

    // DL（基準高）を1m単位で切り下げ
    let dl = (min_elev - 0.5).floor();
    let graph_top = (max_elev + 0.5).ceil();

    // 座標変換
    let to_dxf_x = |d: f64| label_width + (d - min_dist) * scale_x;
    let to_dxf_y = |h: f64| (h - dl) * scale_y;

    let graph_width = (max_dist - min_dist) * scale_x;
    let graph_height = (graph_top - dl) * scale_y;

    // データ表の行定義（上から下へ）
    let table_rows = [
        ("勾配", 0),
        ("盛土", 1),
        ("切土", 2),
        ("計画高", 3),
        ("地盤高", 4),
        ("追加距離", 5),
        ("単距離", 6),
        ("測点名", 7),
    ];
    let table_top = 0.0;
    let table_bottom = table_top - (table_rows.len() as f64) * row_height;

    // ===================
    // グラフ部分の描画
    // ===================

    // 横グリッド線（標高ライン）- 1m間隔
    let grid_step = 1.0;
    let mut elev = dl;
    while elev <= graph_top {
        let y = to_dxf_y(elev);
        add_line(&mut drawing, label_width, y, label_width + graph_width, y, 8, "GRID");
        // 標高ラベル（左側）- DL行は特別表示
        let label_text = if (elev - dl).abs() < 0.01 {
            format!("DL={:.0}", elev)
        } else {
            format!("{:.0}", elev)
        };
        add_text(&mut drawing, label_width - 80.0, y, &label_text, text_height * 0.9, 7, "TEXT", TextAlign::Right);
        elev += grid_step;
    }

    // グラフ枠線
    add_line(&mut drawing, label_width, 0.0, label_width + graph_width, 0.0, 7, "TABLE");
    add_line(&mut drawing, label_width, graph_height, label_width + graph_width, graph_height, 7, "TABLE");
    add_line(&mut drawing, label_width, 0.0, label_width, graph_height, 7, "TABLE");
    add_line(&mut drawing, label_width + graph_width, 0.0, label_width + graph_width, graph_height, 7, "TABLE");

    // 現地盤高線（黒）
    for i in 0..points.len() - 1 {
        add_line(&mut drawing,
            to_dxf_x(points[i].0), to_dxf_y(points[i].1),
            to_dxf_x(points[i + 1].0), to_dxf_y(points[i + 1].1),
            7, "GROUND");
    }

    // 計画高線（赤）
    for i in 0..points.len() - 1 {
        add_line(&mut drawing,
            to_dxf_x(points[i].0), to_dxf_y(points[i].2),
            to_dxf_x(points[i + 1].0), to_dxf_y(points[i + 1].2),
            1, "PLAN");
    }

    // ===================
    // データ表の描画
    // ===================

    // 表の横線
    for i in 0..=table_rows.len() {
        let y = table_top - (i as f64) * row_height;
        add_line(&mut drawing, 0.0, y, label_width + graph_width, y, 7, "TABLE");
    }

    // 左端・ラベル列の縦線
    add_line(&mut drawing, 0.0, table_top, 0.0, table_bottom, 7, "TABLE");
    add_line(&mut drawing, label_width, table_top, label_width, table_bottom, 7, "TABLE");

    // 行ラベル
    for (label, idx) in &table_rows {
        let y = table_top - (*idx as f64) * row_height - row_height / 2.0;
        add_text(&mut drawing, label_width / 2.0, y, label, text_height, 7, "TEXT", TextAlign::Center);
    }

    // 各測点のデータ
    let mut prev_dist = min_dist;
    let mut cum_dist = 0.0;

    for (i, (dist, gh, fh, name)) in points.iter().enumerate() {
        let x = to_dxf_x(*dist);

        // 縦線（グラフ上端から表下端まで）- 主要測点は太線(7)、プラス杭は細線(8)
        let line_color = if is_plus_stake(name) { 8 } else { 7 };
        add_line(&mut drawing, x, graph_height, x, table_bottom, line_color, "GRID");

        // 単距離・累積距離計算
        let unit_dist = if i == 0 { 0.0 } else { dist - prev_dist };
        cum_dist += unit_dist;

        // 盛土・切土計算
        let fill = if fh > gh { fh - gh } else { 0.0 };
        let cut = if gh > fh { gh - fh } else { 0.0 };

        // 勾配計算（次の点との区間）- 勾配はセル中央X座標に配置
        if i < points.len() - 1 {
            let next = &points[i + 1];
            let d_dist = next.0 - dist;
            let d_elev = next.2 - fh;
            if d_dist.abs() > 0.001 {
                let slope_pct = (d_elev / d_dist) * 100.0;
                let slope_str = format!("{:.3}%", slope_pct);
                // 勾配: 0°回転、上寄せ（row_height * 0.25）
                let slope_y = table_top - 0.0 * row_height - row_height * 0.25;
                let slope_mid_x = (x + to_dxf_x(next.0)) / 2.0;
                add_text_rotated(&mut drawing, slope_mid_x, slope_y, &slope_str,
                    text_height * 0.9, 7, "TEXT", TextAlign::Center, VerticalAlign::Top, 0.0);
            }
        }

        let cell_text_height = text_height * 0.9;

        // 盛土: 90°回転、中央配置（row_height / 2.0）
        if fill > 0.001 {
            let y = table_top - 1.0 * row_height - row_height / 2.0;
            add_text_rotated(&mut drawing, x, y, &format!("{:.3}", fill),
                cell_text_height, 7, "TEXT", TextAlign::Center, VerticalAlign::Middle, 90.0);
        }

        // 切土: 90°回転、中央配置（row_height / 2.0）
        if cut > 0.001 {
            let y = table_top - 2.0 * row_height - row_height / 2.0;
            add_text_rotated(&mut drawing, x, y, &format!("{:.3}", cut),
                cell_text_height, 7, "TEXT", TextAlign::Center, VerticalAlign::Middle, 90.0);
        }

        // 計画高(FH): 90°回転、中央配置（row_height / 2.0）
        {
            let y = table_top - 3.0 * row_height - row_height / 2.0;
            add_text_rotated(&mut drawing, x, y, &format!("{:.3}", fh),
                cell_text_height, 7, "TEXT", TextAlign::Center, VerticalAlign::Middle, 90.0);
        }

        // 地盤高(GH): 90°回転、中央配置（row_height / 2.0）
        {
            let y = table_top - 4.0 * row_height - row_height / 2.0;
            add_text_rotated(&mut drawing, x, y, &format!("{:.3}", gh),
                cell_text_height, 7, "TEXT", TextAlign::Center, VerticalAlign::Middle, 90.0);
        }

        // 追加距離: 90°回転、中央配置（row_height / 2.0）
        {
            let y = table_top - 5.0 * row_height - row_height / 2.0;
            add_text_rotated(&mut drawing, x, y, &format!("{:.3}", cum_dist),
                cell_text_height, 7, "TEXT", TextAlign::Center, VerticalAlign::Middle, 90.0);
        }

        // 単距離: 90°回転、中央配置（row_height / 2.0）
        {
            let y = table_top - 6.0 * row_height - row_height / 2.0;
            add_text_rotated(&mut drawing, x, y, &format!("{:.3}", unit_dist),
                cell_text_height, 7, "TEXT", TextAlign::Center, VerticalAlign::Middle, 90.0);
        }

        // 測点名: 90°回転、中央配置（row_height / 2.0）
        {
            let y = table_top - 7.0 * row_height - row_height / 2.0;
            add_text_rotated(&mut drawing, x, y, name,
                cell_text_height, 7, "TEXT", TextAlign::Center, VerticalAlign::Middle, 90.0);
        }

        prev_dist = *dist;
    }

    // 右端の縦線
    add_line(&mut drawing, label_width + graph_width, table_top, label_width + graph_width, table_bottom, 7, "TABLE");

    drawing
}

pub fn generate_longitudinal_dxf_bytes(sections: &[CrossSectionData]) -> Vec<u8> {
    let drawing = generate_longitudinal_drawing(sections);
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
                let font_size = text.text_height as f32 * view.zoom;
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
                let font_size = 150.0 * view.zoom;
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
        // 現地盤高(GH)と計画高(FH)に差をつけて切土・盛土を表現
        vec![
            Self::from_3point("No.0",  2.75, 2.70,  9.620, 9.610, 9.547,  9.500, 9.490, 9.427,  9.0, cut),  // 切土
            Self::from_3point("No.2",  2.60, 2.52,  11.762, 11.813, 11.736,  11.862, 11.913, 11.836,  11.0, cut),  // 盛土
            Self::from_3point("No.4",  2.56, 2.54,  14.733, 14.800, 14.744,  14.633, 14.700, 14.644,  14.0, cut),  // 切土
            Self::from_3point("No.6",  2.53, 2.59,  17.317, 17.367, 17.303,  17.417, 17.467, 17.403,  17.0, cut),  // 盛土
            Self::from_3point("No.8",  2.53, 2.57,  19.946, 20.027, 19.955,  19.846, 19.927, 19.855,  19.0, cut),  // 切土
            Self::from_3point("No.10", 2.58, 2.55,  20.405, 20.476, 20.425,  20.505, 20.576, 20.525,  20.0, cut),  // 盛土
            Self::from_3point("No.12", 2.56, 2.56,  21.067, 21.126, 21.073,  20.967, 21.026, 20.973,  20.0, cut),  // 切土
            Self::from_3point("No.14", 2.55, 2.60,  22.260, 22.305, 22.254,  22.360, 22.405, 22.354,  22.0, cut),  // 盛土
            Self::from_3point("No.16", 2.61, 2.59,  25.235, 25.274, 25.211,  25.135, 25.174, 25.111,  25.0, cut),  // 切土
            Self::from_3point("No.18", 2.55, 2.62,  27.495, 27.635, 27.649,  27.595, 27.735, 27.749,  27.0, cut),  // 盛土
        ]
    }
}

// ============================================================================
// Application
// ============================================================================

#[derive(PartialEq, Clone, Copy)]
enum ViewMode {
    Single,      // 単一横断図
    AllGrid,     // 全横断図グリッド
    Longitudinal, // 縦断図
}

pub struct CrossSectionApp {
    sections: Vec<CrossSectionData>,
    selected_index: Option<usize>,
    dxf_drawing: Option<Drawing>,
    dxf_view_state: DxfViewState,
    view_mode: ViewMode,
    grid_columns: usize,     // グリッドの列数
}

impl Default for CrossSectionApp {
    fn default() -> Self {
        Self {
            sections: Vec::new(),
            selected_index: None,
            dxf_drawing: None,
            dxf_view_state: DxfViewState::default(),
            view_mode: ViewMode::Single,
            grid_columns: 3,
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
        let drawing = match self.view_mode {
            ViewMode::AllGrid if !self.sections.is_empty() => {
                generate_multi_drawing(&self.sections, self.grid_columns)
            }
            ViewMode::Longitudinal if !self.sections.is_empty() => {
                generate_longitudinal_drawing(&self.sections)
            }
            _ => {
                if let Some(idx) = self.selected_index {
                    if let Some(section) = self.sections.get(idx) {
                        generate_drawing(section)
                    } else { return; }
                } else { return; }
            }
        };

        let (min_x, min_y, max_x, max_y) = calc_dxf_bounds(&drawing);
        self.dxf_view_state.fit_to_dxf(min_x, min_y, max_x, max_y);
        self.dxf_drawing = Some(drawing);
    }
}

impl eframe::App for CrossSectionApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let screen_width = ctx.screen_rect().width();
        let is_mobile = screen_width < 600.0;

        if is_mobile {
            // モバイル: トップバー + フルスクリーン図面
            egui::TopBottomPanel::top("mobile_top").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // 測点プルダウン
                    let current_name = self.selected_index
                        .and_then(|i| self.sections.get(i))
                        .map(|s| s.survey_point_name.as_str())
                        .unwrap_or("--");

                    let mut new_selection = None;
                    egui::ComboBox::from_id_salt("station_select")
                        .selected_text(current_name)
                        .show_ui(ui, |ui| {
                            for (i, section) in self.sections.iter().enumerate() {
                                if ui.selectable_label(
                                    self.selected_index == Some(i),
                                    &section.survey_point_name
                                ).clicked() {
                                    new_selection = Some(i);
                                }
                            }
                        });
                    if let Some(idx) = new_selection {
                        self.selected_index = Some(idx);
                        self.update_dxf_preview();
                    }

                    ui.separator();

                    if ui.button("Load").clicked() {
                        self.load_samples();
                    }

                    // 表示モード切替ボタン
                    let mode_text = match self.view_mode {
                        ViewMode::Single => "単一",
                        ViewMode::AllGrid => "全横断",
                        ViewMode::Longitudinal => "縦断",
                    };
                    ui.menu_button(mode_text, |ui| {
                        if ui.button("単一横断").clicked() {
                            self.view_mode = ViewMode::Single;
                            self.update_dxf_preview();
                            ui.close_menu();
                        }
                        if ui.button("全横断").clicked() {
                            self.view_mode = ViewMode::AllGrid;
                            self.update_dxf_preview();
                            ui.close_menu();
                        }
                        if ui.button("縦断図").clicked() {
                            self.view_mode = ViewMode::Longitudinal;
                            self.update_dxf_preview();
                            ui.close_menu();
                        }
                    });

                    // DXFダウンロード
                    match self.view_mode {
                        ViewMode::AllGrid => {
                            if ui.button("DXF").clicked() {
                                let dxf_content = generate_multi_dxf_bytes(&self.sections, self.grid_columns);
                                download_file("cross_sections_all.dxf", &dxf_content);
                            }
                        }
                        ViewMode::Longitudinal => {
                            if ui.button("DXF").clicked() {
                                let dxf_content = generate_longitudinal_dxf_bytes(&self.sections);
                                download_file("longitudinal.dxf", &dxf_content);
                            }
                        }
                        ViewMode::Single => {
                            if self.selected_index.is_some() && ui.button("DXF").clicked() {
                                if let Some(idx) = self.selected_index {
                                    if let Some(section) = self.sections.get(idx) {
                                        let dxf_content = generate_dxf_bytes(section);
                                        let filename = format!("{}.dxf", section.survey_point_name);
                                        download_file(&filename, &dxf_content);
                                    }
                                }
                            }
                        }
                    }
                });
            });
        } else {
            // デスクトップ: サイドパネル
            egui::SidePanel::left("side_panel").min_width(180.0).show(ctx, |ui| {
                ui.heading("Cross Section");
                ui.separator();

                if ui.button("Load Sample").clicked() {
                    self.load_samples();
                }

                // 表示モード切替
                ui.horizontal(|ui| {
                    ui.label("表示:");
                    if ui.selectable_label(self.view_mode == ViewMode::Single, "単一").clicked() {
                        self.view_mode = ViewMode::Single;
                        self.update_dxf_preview();
                    }
                    if ui.selectable_label(self.view_mode == ViewMode::AllGrid, "全横断").clicked() {
                        self.view_mode = ViewMode::AllGrid;
                        self.update_dxf_preview();
                    }
                    if ui.selectable_label(self.view_mode == ViewMode::Longitudinal, "縦断").clicked() {
                        self.view_mode = ViewMode::Longitudinal;
                        self.update_dxf_preview();
                    }
                });

                // AllGridモード時の列数調整
                if self.view_mode == ViewMode::AllGrid {
                    ui.horizontal(|ui| {
                        ui.label(format!("{}列", self.grid_columns));
                        if ui.small_button("+").clicked() && self.grid_columns < 5 {
                            self.grid_columns += 1;
                            self.update_dxf_preview();
                        }
                        if ui.small_button("-").clicked() && self.grid_columns > 1 {
                            self.grid_columns -= 1;
                            self.update_dxf_preview();
                        }
                    });
                }

                // DXFダウンロード
                if !self.sections.is_empty() {
                    match self.view_mode {
                        ViewMode::AllGrid => {
                            if ui.button("Download All DXF").clicked() {
                                let dxf_content = generate_multi_dxf_bytes(&self.sections, self.grid_columns);
                                download_file("cross_sections_all.dxf", &dxf_content);
                            }
                        }
                        ViewMode::Longitudinal => {
                            if ui.button("Download 縦断 DXF").clicked() {
                                let dxf_content = generate_longitudinal_dxf_bytes(&self.sections);
                                download_file("longitudinal.dxf", &dxf_content);
                            }
                        }
                        ViewMode::Single => {
                            if let Some(idx) = self.selected_index {
                                if let Some(section) = self.sections.get(idx) {
                                    if ui.button("Download DXF").clicked() {
                                        let dxf_content = generate_dxf_bytes(section);
                                        let filename = format!("{}.dxf", section.survey_point_name);
                                        download_file(&filename, &dxf_content);
                                    }
                                }
                            }
                        }
                    }
                }
                ui.separator();

                // 単一横断図モード時のみ測点リストを表示
                if self.view_mode == ViewMode::Single {
                    ui.label("Stations:");
                    let mut new_selection = None;
                    egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
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
                } else if self.view_mode == ViewMode::AllGrid {
                    ui.label(format!("全{}測点をグリッド表示", self.sections.len()));
                } else {
                    ui.label("縦断図: 全測点のCL高を接続");
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
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let available = ui.available_size();
            let (response, painter) = ui.allocate_painter(available, egui::Sense::click_and_drag());

            // 白背景
            painter.rect_filled(response.rect, 0.0, Color32::from_rgb(250, 250, 250));
            self.dxf_view_state.canvas_rect = response.rect;

            if response.dragged() {
                self.dxf_view_state.pan += response.drag_delta();
            }

            // マウスホイールズーム
            let scroll = ui.input(|i| i.raw_scroll_delta.y);
            if scroll != 0.0 {
                let zoom_factor = if scroll > 0.0 { 1.1 } else { 0.9 };
                self.dxf_view_state.zoom *= zoom_factor;
                self.dxf_view_state.zoom = self.dxf_view_state.zoom.clamp(0.01, 5.0);
            }

            // ピンチズーム（二本指）
            let zoom_delta = ui.input(|i| i.zoom_delta());
            if zoom_delta != 1.0 {
                self.dxf_view_state.zoom *= zoom_delta;
                self.dxf_view_state.zoom = self.dxf_view_state.zoom.clamp(0.01, 5.0);
            }

            if let Some(ref drawing) = self.dxf_drawing {
                render_dxf(&painter, drawing, &self.dxf_view_state);
            } else {
                painter.text(
                    response.rect.center(),
                    egui::Align2::CENTER_CENTER,
                    if is_mobile { "Tap 'Load'" } else { "Click 'Load Sample' to start" },
                    egui::FontId::proportional(16.0),
                    Color32::GRAY
                );
            }

            // デスクトップのみ: Cutting Calculation ウィンドウ
            if !is_mobile {
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
