//! 横断図・切削計算システム - egui版
//!
//! PDF横断図に準拠した横断図表示と切削計算

use eframe::egui::{self, Color32, Painter, Pos2, Stroke, Vec2, Rect};
use serde::{Deserialize, Serialize};

// dxf crate for proper DXF file generation
use dxf::Drawing;
use dxf::entities::{Entity, EntityType, Line, Text};
use dxf::enums::{HorizontalTextJustification, VerticalTextJustification};
use dxf::{Color, Point};

#[cfg(target_arch = "wasm32")]
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
#[cfg(target_arch = "wasm32")]
use base64::Engine as _;

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
    t.horizontal_text_justification = match align {
        TextAlign::Left => HorizontalTextJustification::Left,
        TextAlign::Center => HorizontalTextJustification::Center,
        TextAlign::Right => HorizontalTextJustification::Right,
    };
    // 垂直配置がMiddleの場合、second_alignment_pointが必要
    t.second_alignment_point = Point::new(x, y, 0.0);
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

    t.horizontal_text_justification = match align {
        TextAlign::Left => HorizontalTextJustification::Left,
        TextAlign::Center => HorizontalTextJustification::Center,
        TextAlign::Right => HorizontalTextJustification::Right,
    };
    // 垂直配置がBaseline以外の場合、second_alignment_pointが必要
    t.second_alignment_point = Point::new(x, y, 0.0);

    t.vertical_text_justification = match v_align {
        VerticalAlign::Top => VerticalTextJustification::Top,
        VerticalAlign::Middle => VerticalTextJustification::Middle,
        VerticalAlign::Bottom => VerticalTextJustification::Bottom,
    };

    let mut entity = Entity::new(EntityType::Text(t));
    entity.common.layer = layer.to_string();
    entity.common.color = Color::from_index(color as u8);
    drawing.add_entity(entity);
}

/// 寸法線を線とテキストで描画（Dimensionエンティティを使わない）
fn add_dimension_as_lines(
    drawing: &mut Drawing,
    x1: f64, x2: f64, y: f64,
    text: &str, text_height: f64,
    color: i16, layer: &str,
) {
    let tick_up = 50.0;    // 端点マークの上方向
    let tick_down = 300.0; // 端点マークの下方向（アシを長く）

    // 水平線（寸法線本体）
    add_line(drawing, x1, y, x2, y, color, layer);

    // 左端の縦線（端点マーク）
    add_line(drawing, x1, y - tick_down, x1, y + tick_up, color, layer);

    // 右端の縦線（端点マーク）
    add_line(drawing, x2, y - tick_down, x2, y + tick_up, color, layer);

    // 中央にテキスト（寸法線の上にマージンを取る）
    let mid_x = (x1 + x2) / 2.0;
    add_text(drawing, mid_x, y + 250.0, text, text_height, color, layer, TextAlign::Center);
}

/// DL値を1m刻みに丸める（最小標高との差が0.2以下なら1m下げる）
fn round_dl(dl: f64) -> f64 {
    let min_elevation = dl + 1.0;  // 元の最小標高（DL = min - 1.0 で計算されている）
    let rounded = dl.ceil();
    let diff = min_elevation - rounded;
    if diff < 0.2 {
        rounded - 1.0
    } else {
        rounded
    }
}

/// アライメントテスト用DXF生成
/// 回転テキスト(-90°)の全アライメント組み合わせを表示
pub fn generate_alignment_test_drawing() -> Drawing {
    let mut drawing = Drawing::new();
    drawing.header.version = dxf::enums::AcadVersion::R2000;

    let mut layer = dxf::tables::Layer::default();
    layer.name = "TEST".to_string();
    layer.color = Color::from_index(7);
    drawing.add_layer(layer);

    let text_height = 100.0;
    let cell_width = 600.0;
    let cell_height = 400.0;

    let h_aligns = [
        (TextAlign::Left, "H:Left"),
        (TextAlign::Center, "H:Center"),
        (TextAlign::Right, "H:Right"),
    ];
    let v_aligns = [
        (VerticalAlign::Top, "V:Top"),
        (VerticalAlign::Middle, "V:Mid"),
        (VerticalAlign::Bottom, "V:Bot"),
    ];

    // グリッド描画
    for col in 0..=3 {
        let x = col as f64 * cell_width;
        add_line(&mut drawing, x, 0.0, x, -3.0 * cell_height, 8, "TEST");
    }
    for row in 0..=3 {
        let y = -(row as f64) * cell_height;
        add_line(&mut drawing, 0.0, y, 3.0 * cell_width, y, 8, "TEST");
    }

    // 各セルにテスト表示
    for (col, (h_align, h_name)) in h_aligns.iter().enumerate() {
        for (row, (v_align, v_name)) in v_aligns.iter().enumerate() {
            let x = col as f64 * cell_width + cell_width / 2.0;
            let y = -(row as f64) * cell_height - cell_height / 2.0;

            // アンカーポイントにマーカー（赤十字）
            add_line(&mut drawing, x - 30.0, y, x + 30.0, y, 1, "TEST");
            add_line(&mut drawing, x, y - 30.0, x, y + 30.0, 1, "TEST");

            // テスト文字列（-90°回転）
            let label = format!("{} {}", h_name, v_name);
            add_text_rotated(&mut drawing, x, y, &label,
                text_height, 7, "TEST", *h_align, *v_align, -90.0);
        }
    }

    // 説明テキスト
    add_text(&mut drawing, 0.0, 200.0, "Alignment Test: -90deg rotation", text_height, 7, "TEST", TextAlign::Left);
    add_text(&mut drawing, 0.0, 50.0, "Red cross = anchor point", text_height * 0.8, 1, "TEST", TextAlign::Left);

    drawing
}

/// アライメントテストDXFをバイト列で返す
pub fn generate_alignment_test_dxf_bytes() -> Vec<u8> {
    let drawing = generate_alignment_test_drawing();
    let mut buf = Vec::new();
    drawing.save(&mut buf).unwrap();
    buf
}

/// Drawingオブジェクトを生成する
pub fn generate_drawing(section: &CrossSectionData) -> Drawing {
    let scale = 1000.0; // mm単位
    let data = &section.survey_data;

    let mut drawing = Drawing::new();
    drawing.header.version = dxf::enums::AcadVersion::R2000;  // サブクラスマーカー出力のため

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

    let dl = round_dl(section.dl);

    // CLを原点（x=0）にするためのオフセット
    let cl_data = &data[section.cl_index.min(data.len() - 1)];
    let cl_offset = cl_data.cumulative_distance;

    let to_dxf_x = |cumulative_distance: f64| -> f64 {
        (cumulative_distance - cl_offset) * scale  // CLが原点に来るようにオフセット
    };
    let y_scale = scale * 2.0;  // 縦方向スケールを2倍（縦断図と同様）
    let to_dxf_y = |height: f64| -> f64 {
        (height - dl) * y_scale
    };

    let l_data = &data[0];
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

    let text_height = 300.0;  // モバイル表示用に2倍
    let pointer_offset = 500.0;  // ポインター分の上方オフセット

    // ========== 測点ポインター（逆三角形 + Vラベル）==========
    let pointer_size = 150.0;  // 逆三角形のサイズ
    for (i, pt) in data.iter().enumerate() {
        let x = to_dxf_x(pt.cumulative_distance);
        let y = to_dxf_y(pt.elevation);
        // 逆三角形（▽）を描画 - 青色
        let top_y = y + pointer_size;
        let half_w = pointer_size * 0.6;
        add_line(&mut drawing, x - half_w, top_y, x + half_w, top_y, 5, "CUTTING");
        add_line(&mut drawing, x - half_w, top_y, x, y, 5, "CUTTING");
        add_line(&mut drawing, x + half_w, top_y, x, y, 5, "CUTTING");
        // ラベル（V1, V2, ...）- 文字サイズ統一
        let label = format!("V{}", i + 1);
        add_text(&mut drawing, x, top_y + 300.0, &label, text_height, 5, "CUTTING", TextAlign::Center);
    }

    let cl_ground_y = to_dxf_y(cl_data.elevation);
    let flag_height_mm = 1600.0;  // 2倍
    let flag_y = cl_ground_y + flag_height_mm;

    let l_x = to_dxf_x(l_data.cumulative_distance);
    let cl_x = to_dxf_x(cl_data.cumulative_distance);
    let r_x = to_dxf_x(r_data.cumulative_distance);

    // ========== 測点名（CL上）==========
    add_text(&mut drawing, cl_x, flag_y + 1200.0,
        &section.survey_point_name, text_height * 1.5, 7, "TEXT", TextAlign::Center);

    // ========== CL GH, FH ==========
    add_text(&mut drawing, cl_x, flag_y + 800.0,
        &format!("GH={:.3}", cl_data.elevation), text_height, 7, "TEXT", TextAlign::Center);
    add_text(&mut drawing, cl_x, flag_y + 400.0,
        &format!("FH={:.3}", cl_data.planned_height), text_height, 1, "PLAN", TextAlign::Center);

    // ========== L側 GH, FH（ポインター分上にオフセット）==========
    let l_ground_y = to_dxf_y(l_data.elevation);
    add_text(&mut drawing, l_x, l_ground_y + 800.0 + pointer_offset,
        &format!("GH={:.3}", l_data.elevation), text_height, 7, "TEXT", TextAlign::Left);
    add_text(&mut drawing, l_x, l_ground_y + 400.0 + pointer_offset,
        &format!("FH={:.3}", l_data.planned_height), text_height, 1, "PLAN", TextAlign::Left);

    // ========== R側 GH, FH（ポインター分上にオフセット）==========
    let r_ground_y = to_dxf_y(r_data.elevation);
    add_text(&mut drawing, r_x, r_ground_y + 800.0 + pointer_offset,
        &format!("GH={:.3}", r_data.elevation), text_height, 7, "TEXT", TextAlign::Right);
    add_text(&mut drawing, r_x, r_ground_y + 400.0 + pointer_offset,
        &format!("FH={:.3}", r_data.planned_height), text_height, 1, "PLAN", TextAlign::Right);

    // ========== 寸法線による旗揚げ（幅員）==========
    let dim_base_y = cl_ground_y + flag_height_mm;
    let mid_l_x = (l_x + cl_x) / 2.0;
    let mid_r_x = (cl_x + r_x) / 2.0;

    // 左幅員の寸法（線とテキストで描画）
    let left_width = (cl_x - l_x).abs() / scale;  // メートル単位
    add_dimension_as_lines(&mut drawing, l_x, cl_x, dim_base_y,
        &format!("{:.2}", left_width), text_height, 7, "DIMENSION");

    // 右幅員の寸法（線とテキストで描画）
    let right_width = (r_x - cl_x).abs() / scale;  // メートル単位
    add_dimension_as_lines(&mut drawing, cl_x, r_x, dim_base_y,
        &format!("{:.2}", right_width), text_height, 7, "DIMENSION");

    // ========== 勾配テキスト（寸法線の上）==========
    add_text(&mut drawing, mid_l_x, flag_y + 600.0,
        &format!("il={:.1}%", left_slope), text_height, 7, "TEXT", TextAlign::Center);
    add_text(&mut drawing, mid_r_x, flag_y + 600.0,
        &format!("ir={:.1}%", right_slope), text_height, 7, "TEXT", TextAlign::Center);

    // ========== DLラベル ==========
    add_text(&mut drawing, cl_x, to_dxf_y(dl) + 300.0,  // 複数横断図と共通
        &format!("DL={:.3}  Scale:H1:V2", dl), text_height, 8, "TEXT", TextAlign::Left);

    // ========== 切削厚表示（DLライン上部） ==========
    let cutting_text_height = text_height;  // GH等と同じサイズ
    let cutting_y = to_dxf_y(dl) - 300.0;  // 2倍
    for (i, pt) in data.iter().enumerate() {
        let x = to_dxf_x(pt.cumulative_distance);
        let cutting_thickness_mm = (pt.elevation - pt.cutting_bottom) * 1000.0;
        let align = if i == 0 {
            TextAlign::Left
        } else if i == data.len() - 1 {
            TextAlign::Right
        } else {
            TextAlign::Center
        };
        add_text(&mut drawing, x, cutting_y,
            &format!("{:.0}", cutting_thickness_mm), cutting_text_height, 5, "CUTTING", align);
    }
    // ラベル「切削厚」を中央下に表示
    add_text(&mut drawing, cl_x, cutting_y - cutting_text_height - 100.0,
        "切削厚", cutting_text_height, 5, "CUTTING", TextAlign::Center);

    // ========== ガイド線 ==========
    let guide_v_length_mm = 1000.0;
    let cl_cumulative = cl_data.cumulative_distance;

    // DLライン（幅員と同じ長さ）
    add_line(&mut drawing, l_x, to_dxf_y(dl), r_x, to_dxf_y(dl), 8, "DIMENSION");

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
pub fn generate_multi_drawing(sections: &[CrossSectionData], columns: usize, column_gap: f64) -> Drawing {
    let scale = 1000.0;

    let mut drawing = Drawing::new();
    drawing.header.version = dxf::enums::AcadVersion::R2000;

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

    // 道路工事の配置: 左下起点、列ごとに下から上
    let rows_per_column = (sections.len() + columns - 1) / columns;  // ceil division

    // 列ごとの最大幅と全体の最大高さを計算
    let mut col_max_left: Vec<f64> = vec![0.0; columns];
    let mut col_max_right: Vec<f64> = vec![0.0; columns];
    let mut max_height: f64 = 0.0;

    for (idx, section) in sections.iter().enumerate() {
        if section.survey_data.len() < 2 { continue; }
        let col = idx / rows_per_column;
        let data = &section.survey_data;
        let min_dist = data.first().unwrap().cumulative_distance;
        let max_dist = data.last().unwrap().cumulative_distance;
        col_max_left[col] = col_max_left[col].max(min_dist.abs());
        col_max_right[col] = col_max_right[col].max(max_dist);
        let max_elev = data.iter().map(|d| d.elevation.max(d.planned_height)).fold(f64::MIN, f64::max);
        max_height = max_height.max(max_elev - section.dl + 1.5);
    }

    // 列ごとのセル幅と累積X位置を計算
    let mut col_widths: Vec<f64> = Vec::with_capacity(columns);
    let mut col_x_offsets: Vec<f64> = Vec::with_capacity(columns);
    let mut cumulative_x = 0.0;

    for col in 0..columns {
        let cell_width = (col_max_left[col] + col_max_right[col] + column_gap) * scale;
        col_widths.push(cell_width);
        // この列のCL位置: 累積X + 列内の左マージン
        let cl_x = cumulative_x + (col_max_left[col] + column_gap / 2.0) * scale;
        col_x_offsets.push(cl_x);
        cumulative_x += cell_width;
    }

    let cell_height = (max_height + 1.0) * scale * 2.0;  // 旗揚げ部分のマージン（縦スケール2倍）

    // セクションを描画
    for (idx, section) in sections.iter().enumerate() {
        if section.survey_data.len() < 2 { continue; }
        let col = idx / rows_per_column;
        let row_in_col = idx % rows_per_column;

        let offset_x = col_x_offsets[col];
        let offset_y = row_in_col as f64 * cell_height;

        draw_section_at_offset(&mut drawing, section, offset_x, offset_y, scale);
    }
    drawing
}

fn draw_section_at_offset(drawing: &mut Drawing, section: &CrossSectionData,
                          offset_x: f64, offset_y: f64, scale: f64) {
    let data = &section.survey_data;
    let dl = round_dl(section.dl);
    let to_dxf_x = |d: f64| offset_x + d * scale;
    let y_scale = scale * 2.0;  // 縦方向スケールを2倍（縦断図と同様）
    let to_dxf_y = |h: f64| offset_y + (h - dl) * y_scale;

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

    let text_height = 300.0;  // モバイル表示用に2倍
    let pointer_offset = 500.0;  // ポインター分の上方オフセット

    // ========== 測点ポインター（逆三角形 + Vラベル）==========
    let pointer_size = 150.0;
    for (i, pt) in data.iter().enumerate() {
        let x = to_dxf_x(pt.cumulative_distance);
        let y = to_dxf_y(pt.elevation);
        let top_y = y + pointer_size;
        let half_w = pointer_size * 0.6;
        add_line(drawing, x - half_w, top_y, x + half_w, top_y, 5, "CUTTING");
        add_line(drawing, x - half_w, top_y, x, y, 5, "CUTTING");
        add_line(drawing, x + half_w, top_y, x, y, 5, "CUTTING");
        let label = format!("V{}", i + 1);
        add_text(drawing, x, top_y + 300.0, &label, text_height, 5, "CUTTING", TextAlign::Center);
    }

    let cl_ground_y = to_dxf_y(cl_data.elevation);
    let flag_y = cl_ground_y + 1600.0;  // 2倍
    let l_x = to_dxf_x(l_data.cumulative_distance);
    let cl_x = to_dxf_x(cl_data.cumulative_distance);
    let r_x = to_dxf_x(r_data.cumulative_distance);

    add_text(drawing, cl_x, flag_y + 1300.0, &section.survey_point_name, text_height * 1.5, 7, "TEXT", TextAlign::Center);
    add_text(drawing, cl_x, flag_y + 800.0, &format!("GH={:.3}", cl_data.elevation), text_height, 7, "TEXT", TextAlign::Center);
    add_text(drawing, cl_x, flag_y + 400.0, &format!("FH={:.3}", cl_data.planned_height), text_height, 1, "PLAN", TextAlign::Center);

    let l_ground_y = to_dxf_y(l_data.elevation);
    add_text(drawing, l_x, l_ground_y + 800.0 + pointer_offset, &format!("GH={:.3}", l_data.elevation), text_height, 7, "TEXT", TextAlign::Left);
    add_text(drawing, l_x, l_ground_y + 400.0 + pointer_offset, &format!("FH={:.3}", l_data.planned_height), text_height, 1, "PLAN", TextAlign::Left);

    let r_ground_y = to_dxf_y(r_data.elevation);
    add_text(drawing, r_x, r_ground_y + 800.0 + pointer_offset, &format!("GH={:.3}", r_data.elevation), text_height, 7, "TEXT", TextAlign::Right);
    add_text(drawing, r_x, r_ground_y + 400.0 + pointer_offset, &format!("FH={:.3}", r_data.planned_height), text_height, 1, "PLAN", TextAlign::Right);

    let mid_l_x = (l_x + cl_x) / 2.0;
    let mid_r_x = (cl_x + r_x) / 2.0;
    add_text(drawing, mid_l_x, flag_y + 600.0, &format!("il={:.1}%", left_slope), text_height, 7, "TEXT", TextAlign::Center);
    add_text(drawing, mid_r_x, flag_y + 600.0, &format!("ir={:.1}%", right_slope), text_height, 7, "TEXT", TextAlign::Center);

    // 寸法線（幅員）- 線とテキストで描画
    let dim_base_y = flag_y;

    // 左幅員の寸法（線とテキストで描画）
    let left_width = (cl_data.cumulative_distance - l_data.cumulative_distance).abs();
    add_dimension_as_lines(drawing, l_x, cl_x, dim_base_y,
        &format!("{:.2}", left_width), text_height, 7, "DIMENSION");

    // 右幅員の寸法（線とテキストで描画）
    let right_width = (r_data.cumulative_distance - cl_data.cumulative_distance).abs();
    add_dimension_as_lines(drawing, cl_x, r_x, dim_base_y,
        &format!("{:.2}", right_width), text_height, 7, "DIMENSION");

    add_text(drawing, cl_x, to_dxf_y(dl) + 300.0, &format!("DL={:.3}  Scale:H1:V2", dl), text_height, 8, "TEXT", TextAlign::Left);

    // ========== 切削厚表示（DLライン上部） ==========
    let cutting_text_height = text_height;  // GH等と同じサイズ
    let cutting_y = to_dxf_y(dl) - 300.0;  // 2倍
    for (i, pt) in data.iter().enumerate() {
        let x = to_dxf_x(pt.cumulative_distance);
        let cutting_thickness_mm = (pt.elevation - pt.cutting_bottom) * 1000.0;
        let align = if i == 0 {
            TextAlign::Left
        } else if i == data.len() - 1 {
            TextAlign::Right
        } else {
            TextAlign::Center
        };
        add_text(drawing, x, cutting_y,
            &format!("{:.0}", cutting_thickness_mm), cutting_text_height, 5, "CUTTING", align);
    }
    // ラベル「切削厚」を中央下に表示
    add_text(drawing, cl_x, cutting_y - cutting_text_height - 100.0,
        "切削厚", cutting_text_height, 5, "CUTTING", TextAlign::Center);

    // DLライン（幅員と同じ長さ）
    add_line(drawing, l_x, to_dxf_y(dl), r_x, to_dxf_y(dl), 8, "DIMENSION");
    let cl_cumulative = cl_data.cumulative_distance;
    add_line(drawing, to_dxf_x(cl_cumulative), to_dxf_y(dl), to_dxf_x(cl_cumulative), to_dxf_y(dl + 1.0), 8, "DIMENSION");
}

pub fn generate_multi_dxf_bytes(sections: &[CrossSectionData], columns: usize, column_gap: f64) -> Vec<u8> {
    let drawing = generate_multi_drawing(sections, columns, column_gap);
    let mut output: Vec<u8> = Vec::new();
    drawing.save(&mut output).expect("Failed to save DXF");
    output
}

// ============================================================================
// Longitudinal Profile (縦断図)
// ============================================================================

/// 測点間隔のデフォルト値（メートル）
/// 土木工事では測点間隔は通常20mが標準
const DEFAULT_STATION_INTERVAL: f64 = 20.0;

/// 測点名から路線距離を取得（"No.X" → X * DEFAULT_STATION_INTERVAL）
fn parse_station_distance(name: &str) -> f64 {
    if name.starts_with("No.") {
        name[3..].parse::<f64>().unwrap_or(0.0) * DEFAULT_STATION_INTERVAL
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
    // スケール設定（DXF単位: mm単位に統一、1m = 1000単位）
    let scale_x = 1000.0;     // 横方向スケール（1m = 1000単位）
    let scale_y = 5000.0;     // 縦方向スケール（1m = 5000単位）縦横比 H1:V5
    let text_height = 1500.0; // 基本テキスト高さ（×10）
    let label_width = 7500.0; // 左側のラベル幅（×10）
    let _title_height = 0.0; // タイトルなし（ピッチリ）

    let mut drawing = Drawing::new();
    drawing.header.version = dxf::enums::AcadVersion::R2000;

    // レイヤー作成
    for (name, color_idx) in [
        ("GROUND", 7),    // 黒 - 現地盤高
        ("PLAN", 1),      // 赤 - 計画高
        ("GRID", 8),      // グレー - グリッド
        ("TABLE", 7),     // 黒 - 表枠
        ("TEXT", 7),      // 黒 - テキスト
        ("TITLE", 7),     // 黒 - タイトル
        ("ANNOTATION", 5), // 青 - 注釈
    ] {
        let mut layer = dxf::tables::Layer::default();
        layer.name = name.to_string();
        layer.color = Color::from_index(color_idx);
        drawing.add_layer(layer);
    }

    // データ収集（路線距離順にソート）
    // route_distanceがあればそれを優先、なければ測点名からパース
    let mut points: Vec<(f64, f64, f64, String)> = sections.iter().map(|s| {
        let cl = &s.survey_data[s.cl_index.min(s.survey_data.len() - 1)];
        let dist = s.route_distance.unwrap_or_else(|| parse_station_distance(&s.survey_point_name));
        (dist, cl.elevation, cl.planned_height, s.survey_point_name.clone())
    }).collect();
    points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    if points.is_empty() { return drawing; }

    // 通常行の高さ（固定）（×10）
    let row_height = 7500.0;
    // 測点名行の高さ（最大文字数に基づいて計算）
    let max_name_len = points.iter().map(|p| p.3.chars().count()).max().unwrap_or(6);
    let station_row_height = (max_name_len as f64 * text_height * 1.0).max(row_height);

    // 範囲計算
    let min_dist = points.first().map(|p| p.0).unwrap_or(0.0);
    let max_dist = points.last().map(|p| p.0).unwrap_or(100.0);
    let min_elev = points.iter().map(|p| p.1.min(p.2)).fold(f64::MAX, f64::min);
    let max_elev = points.iter().map(|p| p.1.max(p.2)).fold(f64::MIN, f64::max);

    // DL（基準高）を1m単位で切り下げ（上下2mマージン）
    let dl = (min_elev - 2.0).floor();
    let graph_top = (max_elev + 2.0).ceil();

    // 左右マージン（最初と最後の測点が表枠境界から離れるように）
    let margin_x = 2000.0;  // 2m分のマージン

    // 座標変換
    let to_dxf_x = |d: f64| label_width + margin_x + (d - min_dist) * scale_x;
    let to_dxf_y = |h: f64| (h - dl) * scale_y;

    let graph_width = (max_dist - min_dist) * scale_x + margin_x * 2.0;
    let graph_height = (graph_top - dl) * scale_y;

    // データ表の行定義（上から下へ）
    // 行0-6は通常高さ、行7（測点名）は独立した高さ
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
    let normal_rows_bottom = table_top - 7.0 * row_height;  // 行0-6
    let table_bottom = normal_rows_bottom - station_row_height;  // 行7（測点名）

    // タイトルは描画しない（ピッチリ）

    // ===================
    // グラフ部分の描画
    // ===================

    // 横グリッド線（標高ライン）- 1m間隔
    let grid_step = 1.0;
    let mut elev = dl;
    while elev <= graph_top {
        let y = to_dxf_y(elev);
        add_line(&mut drawing, label_width, y, label_width + graph_width, y, 8, "GRID");
        // 標高ラベル（左側）- DL行は特別表示、ボトムアライメント
        let is_dl_row = (elev - dl).abs() < 0.01;
        let label_text = if is_dl_row {
            format!("DL={:.0}", elev)
        } else {
            format!("{:.0}", elev)
        };
        let label_color = if is_dl_row { 8 } else { 7 };  // DL行はグレー
        add_text_rotated(&mut drawing, label_width - 800.0, y, &label_text,
            text_height * 0.9, label_color, "TEXT", TextAlign::Right, VerticalAlign::Bottom, 0.0);

        // 標高ラベル（右側）
        add_text_rotated(&mut drawing, label_width + graph_width + 800.0, y, &format!("{:.0}", elev),
            text_height * 0.9, 7, "TEXT", TextAlign::Left, VerticalAlign::Bottom, 0.0);
        elev += grid_step;
    }

    // 縮尺比率と単位の注釈（DL付近、標高ラベルの左側に配置）
    let dl_y = to_dxf_y(dl);
    let annotation_x = 500.0;  // 左端寄り（×10）
    // 縮尺比率: V:H=5:1
    add_text_rotated(&mut drawing, annotation_x, dl_y + scale_y * 0.3,
        "V:H=5:1", text_height * 0.8, 5, "ANNOTATION", TextAlign::Left, VerticalAlign::Bottom, 0.0);
    // 単位 (m)
    add_text_rotated(&mut drawing, annotation_x, dl_y + scale_y * 0.8,
        "単位(m)", text_height * 0.8, 7, "TEXT", TextAlign::Left, VerticalAlign::Bottom, 0.0);

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

    // 表の横線（行0-6は通常高さ、行7は測点名行の高さ）
    for i in 0..=7 {
        let y = table_top - (i as f64) * row_height;
        add_line(&mut drawing, 0.0, y, label_width + graph_width, y, 7, "TABLE");
    }
    // 最下行（測点名行の下端）
    add_line(&mut drawing, 0.0, table_bottom, label_width + graph_width, table_bottom, 7, "TABLE");

    // 左端・ラベル列の縦線
    add_line(&mut drawing, 0.0, table_top, 0.0, table_bottom, 7, "TABLE");
    add_line(&mut drawing, label_width, table_top, label_width, table_bottom, 7, "TABLE");

    // 行ラベル（行7は測点名行の高さを使用）
    for (label, idx) in &table_rows {
        let y = if *idx < 7 {
            table_top - (*idx as f64) * row_height - row_height / 2.0
        } else {
            normal_rows_bottom - station_row_height / 2.0
        };
        add_text(&mut drawing, label_width / 2.0, y, label, text_height, 7, "TEXT", TextAlign::Center);
    }

    // 各測点のデータ
    let mut prev_dist = min_dist;
    let mut cum_dist = 0.0;

    for (i, (dist, gh, fh, name)) in points.iter().enumerate() {
        let x = to_dxf_x(*dist);

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
                let slope_str = format!("{:+.3}%", slope_pct);  // 符号を明示
                // 勾配: -90°回転（他と同じ）
                let slope_y = table_top - 0.0 * row_height - 300.0;
                let slope_mid_x = (x + to_dxf_x(next.0)) / 2.0;
                add_text_rotated(&mut drawing, slope_mid_x, slope_y, &slope_str,
                    text_height * 0.9, 7, "TEXT", TextAlign::Left, VerticalAlign::Bottom, -90.0);
            }
        }

        let cell_text_height = text_height * 0.9;

        // 各フィールドの左上を基準に配置
        // -90°回転テキストでは、TextAlign::Left = テキストが上から下に伸びる
        // VerticalAlign::Bottom = テキストの下端が挿入点
        let top_margin = 300.0;  // セル上端からのマージン（×10）

        // 盛土: 行1の上端基準
        if fill > 0.001 {
            let y = table_top - 1.0 * row_height - top_margin;
            add_text_rotated(&mut drawing, x, y, &format!("{:.3}", fill),
                cell_text_height, 7, "TEXT", TextAlign::Left, VerticalAlign::Bottom, -90.0);
        }

        // 切土: 行2の上端基準
        if cut > 0.001 {
            let y = table_top - 2.0 * row_height - top_margin;
            add_text_rotated(&mut drawing, x, y, &format!("{:.3}", cut),
                cell_text_height, 7, "TEXT", TextAlign::Left, VerticalAlign::Bottom, -90.0);
        }

        // FH/GH/追加距離/単距離: 各行の上端基準
        let row_data: [(usize, String); 4] = [
            (3, format!("{:.3}", fh)),      // 計画高(FH)
            (4, format!("{:.3}", gh)),      // 地盤高(GH)
            (5, format!("{:.2}", cum_dist)), // 追加距離
            (6, format!("{:.2}", unit_dist)), // 単距離
        ];
        for (row_idx, text) in row_data {
            let y = table_top - row_idx as f64 * row_height - top_margin;
            add_text_rotated(&mut drawing, x, y, &text,
                cell_text_height, 7, "TEXT", TextAlign::Left, VerticalAlign::Bottom, -90.0);
        }

        // 測点名: 測点名行の上端基準
        let station_y = normal_rows_bottom - top_margin;
        add_text_rotated(&mut drawing, x, station_y, name,
            cell_text_height, 7, "TEXT", TextAlign::Left, VerticalAlign::Bottom, -90.0);

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

/// コンボビュー（縦断図＋全横断図）を生成
/// 縦断図を上部に、全横断図グリッドを下部に配置（横断図は2行で横長配置）
/// レイアウトマネージャ：センタリング揃え＋適切な間隔
pub fn generate_combo_drawing(sections: &[CrossSectionData], columns: usize, column_gap: f64) -> Drawing {
    if sections.is_empty() {
        return Drawing::new();
    }

    // 面積展開図を生成し、バウンディングボックスを取得
    let area_expansion = generate_area_expansion_drawing(sections);
    let (area_min_x, area_min_y, area_max_x, area_max_y) = calc_dxf_bounds(&area_expansion);
    let area_height = area_max_y - area_min_y;
    let area_center_x = (area_min_x + area_max_x) / 2.0;

    // 縦断図を生成し、バウンディングボックスを取得
    let longitudinal = generate_longitudinal_drawing(sections);
    let (long_min_x, long_min_y, long_max_x, long_max_y) = calc_dxf_bounds(&longitudinal);
    let long_height = long_max_y - long_min_y;
    let long_center_x = (long_min_x + long_max_x) / 2.0;

    // 横断図グリッド（指定列数・間隔を使用）
    let multi = generate_multi_drawing(sections, columns, column_gap);
    let (multi_min_x, multi_min_y, multi_max_x, multi_max_y) = calc_dxf_bounds(&multi);
    let multi_height = multi_max_y - multi_min_y;
    let multi_center_x = (multi_min_x + multi_max_x) / 2.0;

    // 新しいDrawingを作成
    let mut drawing = Drawing::new();
    drawing.header.version = dxf::enums::AcadVersion::R2000;

    // 全てのレイヤーをマージ
    for layer in area_expansion.layers() {
        let mut new_layer = dxf::tables::Layer::default();
        new_layer.name = layer.name.clone();
        new_layer.color = layer.color.clone();
        drawing.add_layer(new_layer);
    }
    for layer in longitudinal.layers() {
        if drawing.layers().find(|l| l.name == layer.name).is_none() {
            let mut new_layer = dxf::tables::Layer::default();
            new_layer.name = layer.name.clone();
            new_layer.color = layer.color.clone();
            drawing.add_layer(new_layer);
        }
    }
    for layer in multi.layers() {
        if drawing.layers().find(|l| l.name == layer.name).is_none() {
            let mut new_layer = dxf::tables::Layer::default();
            new_layer.name = layer.name.clone();
            new_layer.color = layer.color.clone();
            drawing.add_layer(new_layer);
        }
    }

    // === レイアウト計算 ===
    // 間隔（図面間のスペース）（×10）
    let spacing = 5000.0; // 5m間隔（mm単位）

    // 基準: 縦断図を原点に配置
    // 縦断図のエンティティをそのままコピー
    for entity in longitudinal.entities() {
        drawing.add_entity(entity.clone());
    }

    // 面積展開図: 縦断図の上に配置（両方label_width=750から開始なのでXオフセット不要）
    let area_x_offset = 0.0;
    let area_y_offset = long_max_y + spacing - area_min_y;
    for entity in area_expansion.entities() {
        let mut shifted_entity = entity.clone();
        shift_entity_xy(&mut shifted_entity, area_x_offset as f64, area_y_offset as f64);
        drawing.add_entity(shifted_entity);
    }

    // 横断図グリッド: 縦断図の下に配置
    let multi_x_offset = long_center_x - multi_center_x;
    let multi_y_offset = long_min_y - spacing - multi_height;
    for entity in multi.entities() {
        let mut shifted_entity = entity.clone();
        shift_entity_xy(&mut shifted_entity, multi_x_offset as f64, (multi_y_offset - multi_min_y) as f64);
        drawing.add_entity(shifted_entity);
    }

    drawing
}

/// エンティティのXY座標をシフトする
fn shift_entity_xy(entity: &mut dxf::entities::Entity, offset_x: f64, offset_y: f64) {
    use dxf::entities::EntityType;
    match &mut entity.specific {
        EntityType::Line(line) => {
            line.p1.x += offset_x;
            line.p1.y += offset_y;
            line.p2.x += offset_x;
            line.p2.y += offset_y;
        }
        EntityType::Text(text) => {
            text.location.x += offset_x;
            text.location.y += offset_y;
            text.second_alignment_point.x += offset_x;
            text.second_alignment_point.y += offset_y;
        }
        EntityType::MText(mtext) => {
            mtext.insertion_point.x += offset_x;
            mtext.insertion_point.y += offset_y;
        }
        EntityType::Circle(circle) => {
            circle.center.x += offset_x;
            circle.center.y += offset_y;
        }
        EntityType::Arc(arc) => {
            arc.center.x += offset_x;
            arc.center.y += offset_y;
        }
        EntityType::Polyline(_polyline) => {
            // Polyline vertices are stored separately in DXF, skip for now
        }
        EntityType::LwPolyline(lwpoly) => {
            for vertex in &mut lwpoly.vertices {
                vertex.x += offset_x;
                vertex.y += offset_y;
            }
        }
        EntityType::RotatedDimension(dim) => {
            dim.dimension_base.definition_point_1.x += offset_x;
            dim.dimension_base.definition_point_1.y += offset_y;
            dim.dimension_base.text_mid_point.x += offset_x;
            dim.dimension_base.text_mid_point.y += offset_y;
            dim.insertion_point.x += offset_x;
            dim.insertion_point.y += offset_y;
            dim.definition_point_2.x += offset_x;
            dim.definition_point_2.y += offset_y;
            dim.definition_point_3.x += offset_x;
            dim.definition_point_3.y += offset_y;
        }
        _ => {} // その他のエンティティは無視
    }
}

/// エンティティのY座標をシフトする
fn shift_entity_y(entity: &mut dxf::entities::Entity, offset: f64) {
    use dxf::entities::EntityType;
    match &mut entity.specific {
        EntityType::Line(line) => {
            line.p1.y += offset;
            line.p2.y += offset;
        }
        EntityType::Text(text) => {
            text.location.y += offset;
        }
        EntityType::MText(mtext) => {
            mtext.insertion_point.y += offset;
        }
        EntityType::Circle(circle) => {
            circle.center.y += offset;
        }
        EntityType::Arc(arc) => {
            arc.center.y += offset;
        }
        EntityType::Polyline(_polyline) => {
            // Polyline vertices are stored separately in DXF, skip for now
        }
        EntityType::LwPolyline(lwpoly) => {
            for vertex in &mut lwpoly.vertices {
                vertex.y += offset;
            }
        }
        EntityType::RotatedDimension(dim) => {
            dim.dimension_base.definition_point_1.y += offset;
            dim.dimension_base.text_mid_point.y += offset;
            dim.insertion_point.y += offset;
            dim.definition_point_2.y += offset;
            dim.definition_point_3.y += offset;
        }
        _ => {} // その他のエンティティは無視
    }
}

pub fn generate_combo_dxf_bytes(sections: &[CrossSectionData], columns: usize, column_gap: f64) -> Vec<u8> {
    let drawing = generate_combo_drawing(sections, columns, column_gap);
    let mut output: Vec<u8> = Vec::new();
    drawing.save(&mut output).expect("Failed to save DXF");
    output
}

// ============================================================================
// Area Expansion Drawing (面積展開図)
// ============================================================================

/// 面積展開図を生成
/// X軸: 縦断図と同じスケール（相対距離 * scale_x）
/// Y軸: 左幅員が正、右幅員が負（縦断図と同じscale_yで5:1）
pub fn generate_area_expansion_drawing(sections: &[CrossSectionData]) -> Drawing {
    // スケール設定（DXF単位: mm単位に統一、1m = 1000単位）
    let scale_x = 1000.0;     // 横方向スケール（1m = 1000単位）
    let scale_y = 1000.0;     // 縦方向スケール（1m = 1000単位）縦横比 H1:V1
    let text_height = 1500.0; // 基本テキスト高さ（×10）
    let label_width = 7500.0; // 左側ラベル幅（×10）
    let station_text_offset = 3000.0; // 測点名のオフセット（×10）

    let mut drawing = Drawing::new();
    drawing.header.version = dxf::enums::AcadVersion::R2000;

    // レイヤー定義
    for (name, color_idx) in [
        ("TENKAI_OUTLINE", 7),   // 外形線（黒）
        ("TENKAI_WIDTH", 7),     // 幅員線（黒）
        ("TENKAI_CENTER", 1),    // センターライン（赤）
        ("TENKAI_STATION", 5),   // 測点名（青）
        ("TENKAI_DIM", 7),       // 寸法テキスト（黒）
    ] {
        drawing.add_layer(dxf::tables::Layer {
            name: name.to_string(),
            color: Color::from_index(color_idx),
            ..Default::default()
        });
    }

    if sections.is_empty() {
        return drawing;
    }

    // 路線距離順にソートしたデータを準備
    let mut sorted_sections: Vec<&CrossSectionData> = sections.iter().collect();
    sorted_sections.sort_by(|a, b| {
        let dist_a = a.route_distance.unwrap_or_else(|| parse_station_distance(&a.survey_point_name));
        let dist_b = b.route_distance.unwrap_or_else(|| parse_station_distance(&b.survey_point_name));
        dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
    });

    // 基準距離（最小値）を取得 - 縦断図と同じ方式
    let base_distance = sorted_sections.iter()
        .map(|s| s.route_distance.unwrap_or_else(|| parse_station_distance(&s.survey_point_name)))
        .fold(f64::INFINITY, f64::min);

    // 各測点のX位置と幅員を収集
    struct StationInfo {
        x: f64,           // X座標（相対距離 * scale_x + label_width）
        wl: f64,          // 左幅員（正）
        wr: f64,          // 右幅員（正、描画時は負方向）
        name: String,     // 測点名
        dist: f64,        // 元の路線距離
    }

    let mut stations: Vec<StationInfo> = Vec::new();

    for section in &sorted_sections {
        let dist = section.route_distance.unwrap_or_else(|| parse_station_distance(&section.survey_point_name));
        // 縦断図と同じX計算方式（相対距離）
        let x = label_width + (dist - base_distance) * scale_x;

        // 左幅員: L端からCLまでの距離
        let wl = section.l_to_cl_distance;

        // 右幅員: survey_dataの最後の点のcumulative_distance（CLからの距離）
        let wr = if let Some(last) = section.survey_data.last() {
            last.cumulative_distance.abs()
        } else {
            0.0
        };

        stations.push(StationInfo {
            x,
            wl,
            wr,
            name: section.survey_point_name.clone(),
            dist,
        });
    }

    if stations.is_empty() {
        return drawing;
    }

    // ========== 描画 ==========

    // 各測点での垂直線（幅員線）
    for station in &stations {
        let x = station.x;
        let y_top = station.wl * scale_y;     // 左幅員（上方向）
        let y_bottom = -station.wr * scale_y; // 右幅員（下方向）

        // 幅員線（垂直）
        add_line(&mut drawing, x, y_bottom, x, y_top, 7, "TENKAI_WIDTH");
    }

    // センターライン（Y=0で測点間を接続）
    for i in 0..stations.len() - 1 {
        let x1 = stations[i].x;
        let x2 = stations[i + 1].x;
        add_line(&mut drawing, x1, 0.0, x2, 0.0, 1, "TENKAI_CENTER");
    }

    // 外形線（上端・下端を測点間で接続）
    for i in 0..stations.len() - 1 {
        let x1 = stations[i].x;
        let x2 = stations[i + 1].x;
        let y1_top = stations[i].wl * scale_y;
        let y2_top = stations[i + 1].wl * scale_y;
        let y1_bottom = -stations[i].wr * scale_y;
        let y2_bottom = -stations[i + 1].wr * scale_y;

        // 上端（左幅員側）
        add_line(&mut drawing, x1, y1_top, x2, y2_top, 7, "TENKAI_OUTLINE");
        // 下端（右幅員側）
        add_line(&mut drawing, x1, y1_bottom, x2, y2_bottom, 7, "TENKAI_OUTLINE");
    }

    // 延長寸法（測点間中央、Y=0）
    for i in 0..stations.len() - 1 {
        let x1 = stations[i].x;
        let x2 = stations[i + 1].x;
        let mid_x = (x1 + x2) / 2.0;
        let extension = (stations[i + 1].dist - stations[i].dist).abs();
        let text = format!("{:.2}", extension);
        add_text(&mut drawing, mid_x, text_height * 0.5, &text, text_height, 7, "TENKAI_DIM", TextAlign::Center);
    }

    // 幅員寸法（-90°回転、幅員線の外側）
    let marker_size = 200.0;  // デバッグ用マーカーサイズ
    for station in &stations {
        let x = station.x;

        // 左幅員（上側外側に配置）
        if station.wl > 0.0 {
            let y_text = station.wl * scale_y + station_text_offset;
            let text = format!("{:.2}", station.wl);
            // デバッグ: アンカーポイントに十字マーカー（赤）
            add_line(&mut drawing, x - marker_size, y_text, x + marker_size, y_text, 1, "DEBUG");
            add_line(&mut drawing, x, y_text - marker_size, x, y_text + marker_size, 1, "DEBUG");
            add_text_rotated(&mut drawing, x, y_text, &text, text_height,
                7, "TENKAI_DIM", TextAlign::Left, VerticalAlign::Top, -90.0);
        }

        // 右幅員（下側外側に配置）
        if station.wr > 0.0 {
            let y_text = -station.wr * scale_y - station_text_offset;
            let text = format!("{:.2}", station.wr);
            // デバッグ: アンカーポイントに十字マーカー（赤）
            add_line(&mut drawing, x - marker_size, y_text, x + marker_size, y_text, 1, "DEBUG");
            add_line(&mut drawing, x, y_text - marker_size, x, y_text + marker_size, 1, "DEBUG");
            add_text_rotated(&mut drawing, x, y_text, &text, text_height,
                7, "TENKAI_DIM", TextAlign::Right, VerticalAlign::Top, -90.0);
        }
    }

    // 測点名（-90°回転、青色、上端オフセット）
    // 最大文字数から高さを計算
    let max_name_len = stations.iter().map(|s| s.name.chars().count()).max().unwrap_or(6);
    let station_name_height = max_name_len as f64 * text_height * 0.8;
    let max_wl = stations.iter().map(|s| s.wl).fold(0.0_f64, f64::max);
    let y_name_base = max_wl * scale_y + station_text_offset + station_name_height;

    for station in &stations {
        let x = station.x;
        let name_len = station.name.chars().count() as f64;
        // アンダーライン（テキストの左側に縦線）
        let underline_y1 = y_name_base;
        let underline_y2 = y_name_base - name_len * text_height * 0.8;
        add_line(&mut drawing, x, underline_y1, x, underline_y2, 5, "TENKAI_STATION");
        // 測点名テキスト（アンダーラインの右側）
        add_text_rotated(&mut drawing, x, y_name_base, &station.name, text_height * 1.2,
            5, "TENKAI_STATION", TextAlign::Left, VerticalAlign::Bottom, -90.0);
    }

    drawing
}

/// 面積展開図のDXFバイト列を生成
pub fn generate_area_expansion_dxf_bytes(sections: &[CrossSectionData]) -> Vec<u8> {
    let drawing = generate_area_expansion_drawing(sections);
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
        4 => Color32::from_rgb(0, 255, 255),    // シアン
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
                let base_pos = view.dxf_to_screen(text.location.x, text.location.y);
                let font_size = (text.text_height as f32 * view.zoom).max(1.0);
                let font = egui::FontId::proportional(font_size);

                // Galleyを取得
                let galley = painter.layout_no_wrap(text.value.clone(), font, color);
                let text_width = galley.rect.width();
                let text_height = galley.rect.height();

                // アライメントに基づくオフセット計算（回転前の座標系で）
                let offset_x = match text.horizontal_text_justification {
                    HorizontalTextJustification::Left => 0.0,
                    HorizontalTextJustification::Center => -text_width / 2.0,
                    HorizontalTextJustification::Right => -text_width,
                    _ => 0.0,
                };
                let offset_y = match text.vertical_text_justification {
                    VerticalTextJustification::Baseline => -text_height,
                    VerticalTextJustification::Bottom => -text_height,
                    VerticalTextJustification::Middle => -text_height / 2.0,
                    VerticalTextJustification::Top => 0.0,
                };

                // 回転角度（テキストの頭が右向き=測点進行方向）
                let angle_rad = -(text.rotation as f32).to_radians();

                // オフセットを回転させる（回転後の描画位置を計算）
                let cos_a = angle_rad.cos();
                let sin_a = angle_rad.sin();
                let rotated_offset_x = offset_x * cos_a - offset_y * sin_a;
                let rotated_offset_y = offset_x * sin_a + offset_y * cos_a;

                let pos = Pos2::new(
                    base_pos.x + rotated_offset_x,
                    base_pos.y + rotated_offset_y
                );

                let text_shape = egui::epaint::TextShape {
                    pos,
                    galley,
                    underline: egui::Stroke::NONE,
                    fallback_color: color,
                    override_text_color: Some(color),
                    opacity_factor: 1.0,
                    angle: angle_rad,
                };
                painter.add(egui::Shape::Text(text_shape));
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

#[derive(Debug, Clone, PartialEq)]
pub struct CsvSection {
    pub name: String,
    pub unit_distances: Vec<f64>,
    pub elevations: Vec<f64>,
    pub planned_heights: Vec<f64>,
    pub cutting_depth: f64,
    pub cl_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrossSectionData {
    pub survey_point_name: String,
    pub dl: f64,
    pub cl_index: usize,
    pub l_to_cl_distance: f64,
    pub survey_data: Vec<SurveyRow>,
    /// 路線距離（m）- 測点の絶対位置。指定されていればparse_station_distanceより優先
    pub route_distance: Option<f64>,
    /// ルートID（route_1, route_2など）
    #[serde(default = "default_route_id")]
    pub route_id: String,
}

fn default_route_id() -> String {
    "route_1".to_string()
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
            dl, cl_index, l_to_cl_distance: l_to_cl, survey_data,
            route_distance: None, // 測点名からパースされる
            route_id: default_route_id(),
        }
    }


    pub fn from_json(json: &str) -> Result<Vec<Self>, String> {
        serde_json::from_str(json).map_err(|e| format!("JSON parse error: {e}"))
    }

    fn from_csv_section(section: &CsvSection) -> Result<Self, String> {
        let count = section.unit_distances.len();
        if count == 0 {
            return Err(format!("{}: no rows", section.name));
        }
        if section.elevations.len() != count || section.planned_heights.len() != count {
            return Err(format!("{}: column length mismatch", section.name));
        }
        let cl_index = section.cl_index.unwrap_or(0).min(count.saturating_sub(1));
        let cumulative = Self::calc_cumulative_distances(&section.unit_distances, cl_index);
        let cutting_bottoms: Vec<f64> = section
            .planned_heights
            .iter()
            .map(|&fh| fh - section.cutting_depth)
            .collect();
        let l_to_cl = cumulative[0].abs();
        let dl = section
            .elevations
            .iter()
            .cloned()
            .fold(f64::INFINITY, f64::min)
            .floor()
            - 1.0;

        let survey_data: Vec<SurveyRow> = (0..count)
            .map(|i| SurveyRow {
                unit_distance: section.unit_distances[i],
                elevation: section.elevations[i],
                planned_height: section.planned_heights[i],
                cumulative_distance: cumulative[i],
                cutting_bottom: cutting_bottoms[i],
            })
            .collect();

        Ok(CrossSectionData {
            survey_point_name: section.name.clone(),
            dl,
            cl_index,
            l_to_cl_distance: l_to_cl,
            survey_data,
            route_distance: None,
            route_id: default_route_id(),
        })
    }
}

// ============================================================================
// CSV Loaders (wasm only)
// ============================================================================

#[cfg(target_arch = "wasm32")]
fn parse_csv_sections(content: &str) -> Result<Vec<CsvSection>, String> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(content.as_bytes());
    let headers = reader
        .headers()
        .map_err(|e| format!("CSV header error: {e}"))?
        .clone();

    let get_idx = |name: &str| -> Option<usize> { headers.iter().position(|h| h.trim() == name) };

    let name_idx = get_idx("name")
        .or_else(|| get_idx("測点名"))
        .ok_or_else(|| "missing column: name/測点名".to_string())?;
    let unit_idx = get_idx("unit_distance")
        .or_else(|| get_idx("単延長"))
        .ok_or_else(|| "missing column: unit_distance/単延長".to_string())?;
    let elev_idx = get_idx("elevation")
        .or_else(|| get_idx("現地盤"))
        .ok_or_else(|| "missing column: elevation/現地盤".to_string())?;
    let plan_idx = get_idx("planned_height")
        .or_else(|| get_idx("計画高"))
        .ok_or_else(|| "missing column: planned_height/計画高".to_string())?;
    let cut_idx = get_idx("cutting_depth")
        .or_else(|| get_idx("切削厚"))
        .ok_or_else(|| "missing column: cutting_depth/切削厚".to_string())?;

    let section_idx = get_idx("section").or_else(|| get_idx("区間"));
    let cl_flag_idx = get_idx("cl")
        .or_else(|| get_idx("CL"))
        .or_else(|| get_idx("center"))
        .or_else(|| get_idx("中央"));
    let cl_index_idx = get_idx("cl_index").or_else(|| get_idx("CL_index"));
    let mut sections: Vec<CsvSection> = Vec::new();
    let mut current_name = String::new();
    let mut unit_distances = Vec::new();
    let mut elevations = Vec::new();
    let mut planned_heights = Vec::new();
    let mut cutting_depth: Option<f64> = None;
    let mut cl_index: Option<usize> = None;

    let mut flush = |name: &str,
                     unit_distances: &mut Vec<f64>,
                     elevations: &mut Vec<f64>,
                     planned_heights: &mut Vec<f64>,
                     cutting_depth: &mut Option<f64>,
                     cl_index: &mut Option<usize>,
                     out: &mut Vec<CsvSection>| {
        if unit_distances.is_empty() {
            return;
        }
        let cut = cutting_depth.unwrap_or(0.0);
        out.push(CsvSection {
            name: name.to_string(),
            unit_distances: std::mem::take(unit_distances),
            elevations: std::mem::take(elevations),
            planned_heights: std::mem::take(planned_heights),
            cutting_depth: cut,
            cl_index: *cl_index,
        });
        *cutting_depth = None;
        *cl_index = None;
    };

    for record in reader.records() {
        let record = record.map_err(|e| format!("CSV row error: {e}"))?;
        if record.iter().all(|v| v.trim().is_empty()) {
            continue;
        }

        let section_name = section_idx
            .and_then(|idx| record.get(idx))
            .map(|v| v.trim())
            .filter(|v| !v.is_empty());
        if let Some(section_name) = section_name {
            if current_name.is_empty() {
                current_name = section_name.to_string();
            } else if section_name != current_name {
                flush(
                    &current_name,
                    &mut unit_distances,
                    &mut elevations,
                    &mut planned_heights,
                    &mut cutting_depth,
                    &mut cl_index,
                    &mut sections,
                );
                current_name = section_name.to_string();
            }
        }

        let name = record.get(name_idx).unwrap_or("未設定").trim();
        if current_name.is_empty() {
            current_name = name.to_string();
        }

        unit_distances.push(parse_cell(record.get(unit_idx))?);
        elevations.push(parse_cell(record.get(elev_idx))?);
        planned_heights.push(parse_cell(record.get(plan_idx))?);
        let cut_value = parse_cell(record.get(cut_idx))?;
        cutting_depth = Some(cutting_depth.map_or(cut_value, |v| v.max(cut_value)));
        if cl_index.is_none() {
            if let Some(idx) = cl_index_idx.and_then(|idx| record.get(idx)) {
                if !idx.trim().is_empty() {
                    cl_index = idx.trim().parse::<usize>().ok();
                }
            } else if let Some(flag) = cl_flag_idx.and_then(|idx| record.get(idx)) {
                let flag = flag.trim();
                if matches!(flag, "1" | "true" | "TRUE" | "yes" | "YES" | "y" | "Y" | "中央") {
                    cl_index = Some(unit_distances.len().saturating_sub(1));
                }
            }
        }
    }

    if !current_name.is_empty() {
        flush(
            &current_name,
            &mut unit_distances,
            &mut elevations,
            &mut planned_heights,
            &mut cutting_depth,
            &mut cl_index,
            &mut sections,
        );
    }

    if sections.is_empty() {
        return Err("no valid sections in CSV".to_string());
    }
    Ok(sections)
}

#[cfg(target_arch = "wasm32")]
fn parse_cell(value: Option<&str>) -> Result<f64, String> {
    let raw = value.unwrap_or("").trim();
    if raw.is_empty() {
        return Err("empty numeric cell".to_string());
    }
    raw.parse::<f64>()
        .map_err(|_| format!("invalid number: {raw}"))
}

#[cfg(not(target_arch = "wasm32"))]
fn parse_csv_sections(_content: &str) -> Result<Vec<CsvSection>, String> {
    Err("CSV loader is only available in wasm builds".to_string())
}

// ============================================================================
// Application
// ============================================================================

#[derive(PartialEq, Clone, Copy)]
enum ViewMode {
    Single,      // 単一横断図
    AllGrid,     // 全横断図グリッド
    Longitudinal, // 縦断図
    Combo,       // 縦断図＋全横断図
    AreaExpansion, // 面積展開図
    AlignTest,   // アライメントテスト
}

pub struct CrossSectionApp {
    sections: Vec<CrossSectionData>,
    selected_index: Option<usize>,
    dxf_drawing: Option<Drawing>,
    dxf_view_state: DxfViewState,
    view_mode: ViewMode,
    grid_columns: usize,     // グリッドの列数
    column_gap: f64,         // 列間隔（メートル単位）
    status_message: Option<String>,
    needs_fit: bool,         // canvas_rect更新後にfit_to_dxfを呼ぶフラグ
    is_first_frame: bool,    // 初回フレームフラグ（モバイル判定用）
    loading_frame: usize,    // ローディングアニメーション用フレームカウンタ
    loading_stage: String,   // ローディング進捗メッセージ
    routes: Vec<String>,     // 利用可能なルートのリスト
    selected_route: String,  // 選択中のルート
    api_key: String,         // セッションのみのAPIキー
    api_key_set: bool,       // UI表示用フラグ
}

impl Default for CrossSectionApp {
    fn default() -> Self {
        Self {
            sections: Vec::new(),
            selected_index: None,
            dxf_drawing: None,
            dxf_view_state: DxfViewState::default(),
            view_mode: ViewMode::Single, // デフォルトで単一横断図（モバイル向け）
            grid_columns: 3,
            column_gap: 2.0,  // 列間隔2メートル（切削厚ラベル分）
            status_message: None,
            needs_fit: false,
            is_first_frame: true,
            loading_frame: 0,
            loading_stage: "JSONデータを取得中".to_string(),
            routes: vec!["route_1".to_string()],
            selected_route: "route_1".to_string(),
            api_key: String::new(),
            api_key_set: false,
        }
    }
}

impl CrossSectionApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // 日本語フォントを設定
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "NotoSansJP".to_owned(),
            egui::FontData::from_static(
                include_bytes!("../static/NotoSansJP-Subset.ttf")
            )
        );
        fonts.families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "NotoSansJP".to_owned());
        fonts.families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .insert(0, "NotoSansJP".to_owned());
        cc.egui_ctx.set_fonts(fonts);

        let mut app = Self::default();
        app.status_message = Some("読み込み中...".to_string());
        app
    }

    fn api_key_ui(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.label("Gemini API Key");
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.api_key)
                    .password(true)
                    .desired_width(160.0),
            );
            if ui.button("Set").clicked() {
                self.api_key_set = !self.api_key.is_empty();
            }
            if ui.button("Clear").clicked() {
                self.api_key.clear();
                self.api_key_set = false;
            }
        });
        ui.label(if self.api_key_set {
            "APIキー設定済み（セッションのみ）"
        } else {
            "APIキー未設定"
        });
    }

    #[cfg(target_arch = "wasm32")]
    fn gemini_excel_ui(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        if ui.button("Excelを読み込む (Gemini)").clicked() {
            trigger_xlsx_dialog();
        }
        ui.label("Excelはローカルに保存されず、APIに送信されます。");
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn gemini_excel_ui(&mut self, _ui: &mut egui::Ui) {}

    fn handle_csv_loaded(&mut self, csv_text: &str) {
        match parse_csv_sections(csv_text) {
            Ok(raw_sections) => {
                let mut sections = Vec::with_capacity(raw_sections.len());
                for section in raw_sections.iter() {
                    match CrossSectionData::from_csv_section(section) {
                        Ok(data) => sections.push(data),
                        Err(err) => {
                            self.status_message = Some(format!("CSV変換エラー: {err}"));
                            return;
                        }
                    }
                }
                self.sections = sections;
                self.selected_index = self.sections.first().map(|_| 0);
                self.status_message = Some(format!("CSVを読み込みました ({} 区間)", self.sections.len()));
                self.update_dxf_preview();
            }
            Err(err) => {
                self.status_message = Some(format!("CSV読み込みエラー: {err}"));
            }
        }
    }

    /// 選択中のルートでフィルターされたセクションを返す
    fn filtered_sections(&self) -> Vec<&CrossSectionData> {
        self.sections.iter()
            .filter(|s| s.route_id == self.selected_route)
            .collect()
    }

    fn update_dxf_preview(&mut self) {
        let filtered: Vec<CrossSectionData> = self.filtered_sections()
            .into_iter().cloned().collect();

        let drawing = match self.view_mode {
            ViewMode::AlignTest => {
                generate_alignment_test_drawing()
            }
            ViewMode::Combo if !filtered.is_empty() => {
                generate_combo_drawing(&filtered, self.grid_columns, self.column_gap)
            }
            ViewMode::AllGrid if !filtered.is_empty() => {
                generate_multi_drawing(&filtered, self.grid_columns, self.column_gap)
            }
            ViewMode::Longitudinal if !filtered.is_empty() => {
                generate_longitudinal_drawing(&filtered)
            }
            ViewMode::AreaExpansion if !filtered.is_empty() => {
                generate_area_expansion_drawing(&filtered)
            }
            _ => {
                if let Some(idx) = self.selected_index {
                    if let Some(section) = filtered.get(idx) {
                        generate_drawing(section)
                    } else { return; }
                } else { return; }
            }
        };

        self.dxf_drawing = Some(drawing);
        self.needs_fit = true; // canvas_rect更新後にfit_to_dxfを呼ぶ
    }
}

impl eframe::App for CrossSectionApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 起動時にsections.jsonをfetch開始
        start_json_fetch();

        // JSONのfetch結果を処理
        if let Some(json_text) = take_pending_json() {
            self.loading_stage = "JSONをパース中".to_string();
            match CrossSectionData::from_json(&json_text) {
                Ok(sections) => {
                    self.loading_stage = format!("{}測点のデータを処理中", sections.len());
                    let dump = if sections.is_empty() {
                        "JSONデータなし".to_string()
                    } else {
                        let first = &sections[0];
                        let last = &sections[sections.len() - 1];
                        format!(
                            "JSON: {}測点 {} ~ {} (DL={:.2}, {}点)",
                            sections.len(),
                            first.survey_point_name,
                            last.survey_point_name,
                            first.dl,
                            first.survey_data.len()
                        )
                    };
                    // ルート一覧を抽出
                    let mut route_set: std::collections::HashSet<String> = std::collections::HashSet::new();
                    for s in &sections {
                        route_set.insert(s.route_id.clone());
                    }
                    let mut routes: Vec<String> = route_set.into_iter().collect();
                    routes.sort();
                    self.routes = routes;
                    if !self.routes.is_empty() {
                        self.selected_route = self.routes[0].clone();
                    }

                    self.sections = sections;
                    self.selected_index = Some(0);
                    self.loading_stage = "DXFプレビューを生成中".to_string();
                    self.status_message = Some(dump);
                    self.update_dxf_preview();
                }
                Err(e) => {
                    self.status_message = Some(format!("JSONエラー: {e}"));
                }
            }
        }

        if let Some(csv_text) = take_pending_csv() {
            self.handle_csv_loaded(&csv_text);
        }
        if let Some(xlsx_bytes) = take_pending_xlsx() {
            if self.api_key.is_empty() {
                self.status_message = Some("APIキーが未設定です".to_string());
            } else {
                self.status_message = Some("Geminiで解析中...".to_string());
                self.loading_stage = "Gemini解析中".to_string();
                start_gemini_parse(self.api_key.clone(), xlsx_bytes);
            }
        }
        if let Some(err) = take_pending_error() {
            self.status_message = Some(format!("Geminiエラー: {err}"));
        }
        let screen_width = ctx.screen_rect().width();
        let is_mobile = screen_width < 600.0;

        // 初回フレームでデスクトップならComboモードに設定
        if self.is_first_frame {
            self.is_first_frame = false;
            if !is_mobile {
                self.view_mode = ViewMode::Combo;
                self.update_dxf_preview();
            }
        }

        if is_mobile {
            // モバイル: トップバー + フルスクリーン図面
            egui::TopBottomPanel::top("mobile_top").show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        // ルート選択（複数ルートがある場合のみ表示）
                        if self.routes.len() > 1 {
                            let response = egui::ComboBox::from_id_salt("route_select_mobile")
                                .selected_text(&self.selected_route)
                                .width(100.0)
                                .show_ui(ui, |ui| {
                                    let mut selected = None;
                                    for route in &self.routes {
                                        if ui.selectable_label(
                                            self.selected_route == *route,
                                            route
                                        ).clicked() {
                                            selected = Some(route.clone());
                                        }
                                    }
                                    selected
                                });
                            if let Some(route) = response.inner.flatten() {
                                self.selected_route = route;
                                self.selected_index = Some(0);
                                self.update_dxf_preview();
                            }
                        }

                        // 測点プルダウン（フィルター済みセクションから選択）
                        let filtered: Vec<_> = self.filtered_sections();
                        let current_name = self.selected_index
                            .and_then(|i| filtered.get(i))
                            .map(|s| s.survey_point_name.as_str())
                            .unwrap_or("--");

                        let response = egui::ComboBox::from_id_salt("station_select")
                            .selected_text(current_name)
                            .width(150.0)
                            .show_ui(ui, |ui| {
                                ui.set_min_width(140.0);
                                let mut selected = None;
                                for (i, section) in filtered.iter().enumerate() {
                                    let response = ui.selectable_label(
                                        self.selected_index == Some(i),
                                        &section.survey_point_name
                                    );
                                    if response.clicked() {
                                        selected = Some(i);
                                    }
                                }
                                selected
                            });
                        if let Some(idx) = response.inner.flatten() {
                            self.selected_index = Some(idx);
                            self.view_mode = ViewMode::Single;  // 単一モードに切り替え
                            self.update_dxf_preview();
                        }

                    });

                    ui.horizontal_wrapped(|ui| {
                        // 表示モード切替（タッチ操作しやすいよう幅を確保）
                        let mode_text = match self.view_mode {
                            ViewMode::Combo => "コンボ",
                            ViewMode::Single => "単一",
                            ViewMode::AllGrid => "全横断",
                            ViewMode::Longitudinal => "縦断",
                            ViewMode::AreaExpansion => "展開図",
                            ViewMode::AlignTest => "テスト",
                        };
                        let response = egui::ComboBox::from_id_salt("view_mode_select")
                            .selected_text(mode_text)
                            .width(120.0)
                            .show_ui(ui, |ui| {
                                ui.set_min_width(160.0);
                                let mut selected = None;
                                if ui.selectable_label(self.view_mode == ViewMode::Single, "単一横断").clicked() {
                                    selected = Some(ViewMode::Single);
                                }
                                if ui.selectable_label(self.view_mode == ViewMode::Combo, "コンボ (縦断+全横断)").clicked() {
                                    selected = Some(ViewMode::Combo);
                                }
                                if ui.selectable_label(self.view_mode == ViewMode::AllGrid, "全横断").clicked() {
                                    selected = Some(ViewMode::AllGrid);
                                }
                                if ui.selectable_label(self.view_mode == ViewMode::Longitudinal, "縦断図").clicked() {
                                    selected = Some(ViewMode::Longitudinal);
                                }
                                if ui.selectable_label(self.view_mode == ViewMode::AreaExpansion, "面積展開図").clicked() {
                                    selected = Some(ViewMode::AreaExpansion);
                                }
                                if ui.selectable_label(self.view_mode == ViewMode::AlignTest, "アライメントテスト").clicked() {
                                    selected = Some(ViewMode::AlignTest);
                                }
                                selected
                            });
                        if let Some(mode) = response.inner.flatten() {
                            if self.view_mode != mode {
                                self.view_mode = mode;
                                self.update_dxf_preview();
                            }
                        }
                    });

                    if self.view_mode == ViewMode::AllGrid || self.view_mode == ViewMode::Combo {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(format!("{}列", self.grid_columns));
                            if ui.small_button("+").clicked() && self.grid_columns < 10 {
                                self.grid_columns += 1;
                                self.update_dxf_preview();
                            }
                            if ui.small_button("-").clicked() && self.grid_columns > 1 {
                                self.grid_columns -= 1;
                                self.update_dxf_preview();
                            }
                            ui.label(format!("間隔{:.1}m", self.column_gap));
                            if ui.small_button("+").clicked() && self.column_gap < 5.0 {
                                self.column_gap += 0.5;
                                self.update_dxf_preview();
                            }
                            if ui.small_button("-").clicked() && self.column_gap > -5.0 {
                                self.column_gap -= 0.5;
                                self.update_dxf_preview();
                            }
                        });
                    }

                    ui.horizontal_wrapped(|ui| {
                        // DXFダウンロード（フィルター済みセクションを使用）
                        let filtered_for_dxf: Vec<CrossSectionData> = self.filtered_sections()
                            .into_iter().cloned().collect();
                        match self.view_mode {
                            ViewMode::Combo if !filtered_for_dxf.is_empty() => {
                                if ui.button("DXF").clicked() {
                                    let dxf_content = generate_combo_dxf_bytes(&filtered_for_dxf, self.grid_columns, self.column_gap);
                                    download_file("combo.dxf", &dxf_content);
                                }
                            }
                            ViewMode::AllGrid if !filtered_for_dxf.is_empty() => {
                                if ui.button("DXF").clicked() {
                                    let dxf_content = generate_multi_dxf_bytes(&filtered_for_dxf, self.grid_columns, self.column_gap);
                                    download_file("cross_sections_all.dxf", &dxf_content);
                                }
                            }
                            ViewMode::Longitudinal if !filtered_for_dxf.is_empty() => {
                                if ui.button("DXF").clicked() {
                                    let dxf_content = generate_longitudinal_dxf_bytes(&filtered_for_dxf);
                                    download_file("longitudinal.dxf", &dxf_content);
                                }
                            }
                            ViewMode::AreaExpansion if !filtered_for_dxf.is_empty() => {
                                if ui.button("DXF").clicked() {
                                    let dxf_content = generate_area_expansion_dxf_bytes(&filtered_for_dxf);
                                    download_file("area_expansion.dxf", &dxf_content);
                                }
                            }
                            ViewMode::Single => {
                                if self.selected_index.is_some() && ui.button("DXF").clicked() {
                                    if let Some(idx) = self.selected_index {
                                        if let Some(section) = filtered_for_dxf.get(idx) {
                                            let dxf_content = generate_dxf_bytes(section);
                                            let filename = format!("{}.dxf", section.survey_point_name);
                                            download_file(&filename, &dxf_content);
                                        }
                                    }
                                }
                            }
                            ViewMode::AlignTest => {
                                if ui.button("DXF").clicked() {
                                    let dxf_content = generate_alignment_test_dxf_bytes();
                                    download_file("alignment_test.dxf", &dxf_content);
                                }
                            }
                            _ => {}
                        }
                    });

                    ui.collapsing("API", |ui| {
                        self.api_key_ui(ui);
                        self.gemini_excel_ui(ui);
                    });
                });
            });
        } else {
            // デスクトップ: サイドパネル
            egui::SidePanel::left("side_panel").min_width(180.0).show(ctx, |ui| {
                ui.heading("Cross Section");
                ui.separator();

                #[cfg(target_arch = "wasm32")]
                if ui.button("CSVを読み込む").clicked() {
                    trigger_csv_dialog();
                }
                if let Some(message) = &self.status_message {
                    ui.label(message);
                }
                ui.collapsing("API", |ui| {
                    self.api_key_ui(ui);
                    self.gemini_excel_ui(ui);
                });

                // ルート選択（複数ルートがある場合のみ表示）
                if self.routes.len() > 1 {
                    ui.horizontal(|ui| {
                        ui.label("路線:");
                        let response = egui::ComboBox::from_id_salt("route_select_desktop")
                            .selected_text(&self.selected_route)
                            .show_ui(ui, |ui| {
                                let mut selected = None;
                                for route in &self.routes {
                                    if ui.selectable_label(
                                        self.selected_route == *route,
                                        route
                                    ).clicked() {
                                        selected = Some(route.clone());
                                    }
                                }
                                selected
                            });
                        if let Some(route) = response.inner.flatten() {
                            self.selected_route = route;
                            self.selected_index = Some(0);
                            self.update_dxf_preview();
                        }
                    });
                }

                // 表示モード切替
                ui.horizontal(|ui| {
                    ui.label("表示:");
                    if ui.selectable_label(self.view_mode == ViewMode::Combo, "コンボ").clicked() {
                        self.view_mode = ViewMode::Combo;
                        self.update_dxf_preview();
                    }
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
                    if ui.selectable_label(self.view_mode == ViewMode::AreaExpansion, "展開図").clicked() {
                        self.view_mode = ViewMode::AreaExpansion;
                        self.update_dxf_preview();
                    }
                });

                // AllGrid/Comboモード時の列数・間隔調整
                if self.view_mode == ViewMode::AllGrid || self.view_mode == ViewMode::Combo {
                    ui.horizontal(|ui| {
                        ui.label(format!("{}列", self.grid_columns));
                        if ui.small_button("+").clicked() && self.grid_columns < 10 {
                            self.grid_columns += 1;
                            self.update_dxf_preview();
                        }
                        if ui.small_button("-").clicked() && self.grid_columns > 1 {
                            self.grid_columns -= 1;
                            self.update_dxf_preview();
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label(format!("間隔{:.1}m", self.column_gap));
                        if ui.small_button("+").clicked() && self.column_gap < 5.0 {
                            self.column_gap += 0.5;
                            self.update_dxf_preview();
                        }
                        if ui.small_button("-").clicked() && self.column_gap > -5.0 {
                            self.column_gap -= 0.5;
                            self.update_dxf_preview();
                        }
                    });
                }

                // DXFダウンロード（フィルター済みセクションを使用）
                let filtered_for_download: Vec<CrossSectionData> = self.filtered_sections()
                    .into_iter().cloned().collect();

                match self.view_mode {
                    ViewMode::AlignTest => {
                        if ui.button("Download Test DXF").clicked() {
                            let dxf_content = generate_alignment_test_dxf_bytes();
                            download_file("alignment_test.dxf", &dxf_content);
                        }
                    }
                    _ if filtered_for_download.is_empty() => {}
                    ViewMode::Combo => {
                        if ui.button("Download Combo DXF").clicked() {
                            let dxf_content = generate_combo_dxf_bytes(&filtered_for_download, self.grid_columns, self.column_gap);
                            download_file("combo.dxf", &dxf_content);
                        }
                    }
                    ViewMode::AllGrid => {
                        if ui.button("Download All DXF").clicked() {
                            let dxf_content = generate_multi_dxf_bytes(&filtered_for_download, self.grid_columns, self.column_gap);
                            download_file("cross_sections_all.dxf", &dxf_content);
                        }
                    }
                    ViewMode::Longitudinal => {
                        if ui.button("Download 縦断 DXF").clicked() {
                            let dxf_content = generate_longitudinal_dxf_bytes(&filtered_for_download);
                            download_file("longitudinal.dxf", &dxf_content);
                        }
                    }
                    ViewMode::AreaExpansion => {
                        if ui.button("Download 展開図 DXF").clicked() {
                            let dxf_content = generate_area_expansion_dxf_bytes(&filtered_for_download);
                            download_file("area_expansion.dxf", &dxf_content);
                        }
                    }
                    ViewMode::Single => {
                        if let Some(idx) = self.selected_index {
                            if let Some(section) = filtered_for_download.get(idx) {
                                if ui.button("Download DXF").clicked() {
                                    let dxf_content = generate_dxf_bytes(section);
                                    let filename = format!("{}.dxf", section.survey_point_name);
                                    download_file(&filename, &dxf_content);
                                }
                            }
                        }
                    }
                }
                ui.separator();

                // 測点リスト（全モードで表示、選択時は単一モードに切替）
                // 借用の問題を避けるため、表示に必要なデータを先にコピー
                let filtered_names: Vec<String> = self.filtered_sections()
                    .iter().map(|s| s.survey_point_name.clone()).collect();
                let filtered_count = filtered_names.len();

                // モード別の情報表示
                match self.view_mode {
                    ViewMode::Single => {
                        ui.label(format!("単一横断 ({}測点)", filtered_count));
                    }
                    ViewMode::AllGrid => {
                        ui.label(format!("全横断グリッド ({}測点)", filtered_count));
                    }
                    ViewMode::Combo => {
                        ui.label(format!("縦断+全横断 ({}測点)", filtered_count));
                    }
                    ViewMode::Longitudinal => {
                        ui.label(format!("縦断図 ({}測点)", filtered_count));
                    }
                    ViewMode::AreaExpansion => {
                        ui.label(format!("面積展開図 ({}測点)", filtered_count));
                    }
                    ViewMode::AlignTest => {
                        ui.label("アライメントテスト");
                    }
                }

                ui.label("Stations:");
                let mut new_selection = None;
                egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                    for (i, name) in filtered_names.iter().enumerate() {
                        let selected = self.selected_index == Some(i);
                        if ui.selectable_label(selected, name).clicked() {
                            new_selection = Some(i);
                        }
                    }
                });
                if let Some(idx) = new_selection {
                    self.selected_index = Some(idx);
                    self.view_mode = ViewMode::Single;  // 単一モードに切り替え
                    self.update_dxf_preview();
                }

                // 単一モード時のみ詳細情報を表示
                if self.view_mode == ViewMode::Single {
                    ui.separator();
                    let filtered_list: Vec<_> = self.filtered_sections();
                    if let Some(idx) = self.selected_index {
                        if let Some(section) = filtered_list.get(idx) {
                            ui.label(format!("DL: {:.3}", section.dl));
                            ui.label(format!("L->CL: {:.2}m", section.l_to_cl_distance));
                        }
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
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let available = ui.available_size();
            let (response, painter) = ui.allocate_painter(available, egui::Sense::click_and_drag());

            // 白背景
            painter.rect_filled(response.rect, 0.0, Color32::from_rgb(250, 250, 250));
            self.dxf_view_state.canvas_rect = response.rect;

            // canvas_rect更新後にfit_to_dxfを実行
            if self.needs_fit {
                if let Some(ref drawing) = self.dxf_drawing {
                    let (min_x, min_y, max_x, max_y) = calc_dxf_bounds(drawing);
                    self.dxf_view_state.fit_to_dxf(min_x, min_y, max_x, max_y);
                }
                self.needs_fit = false;
            }

            if response.dragged() {
                self.dxf_view_state.pan += response.drag_delta();
            }

            // キャンバス上にホバー時のみズーム処理
            if response.hovered() {
                // マウスホイールズーム（マウス位置中心）
                let scroll = ui.input(|i| i.raw_scroll_delta.y);
                if scroll != 0.0 {
                    if let Some(mouse_pos) = response.hover_pos() {
                        let zoom_factor = if scroll > 0.0 { 1.1 } else { 0.9 };
                        let local = mouse_pos - self.dxf_view_state.canvas_rect.min;

                        // pan調整: マウス位置のDXF座標がズーム後も同じスクリーン位置に留まる
                        self.dxf_view_state.pan = local * (1.0 - zoom_factor)
                                                + self.dxf_view_state.pan * zoom_factor;
                        self.dxf_view_state.zoom *= zoom_factor;
                        self.dxf_view_state.zoom = self.dxf_view_state.zoom.clamp(0.001, 50.0);
                    }
                }

                // ピンチズーム（画面中心）
                let zoom_delta = ui.input(|i| i.zoom_delta());
                if zoom_delta != 1.0 {
                    let center = self.dxf_view_state.canvas_rect.center();
                    let local = center - self.dxf_view_state.canvas_rect.min;

                    self.dxf_view_state.pan = local * (1.0 - zoom_delta)
                                            + self.dxf_view_state.pan * zoom_delta;
                    self.dxf_view_state.zoom *= zoom_delta;
                    self.dxf_view_state.zoom = self.dxf_view_state.zoom.clamp(0.001, 50.0);
                }
            }

            if let Some(ref drawing) = self.dxf_drawing {
                render_dxf(&painter, drawing, &self.dxf_view_state);

                // ズームセンター（マウス位置）にレティクル描画
                if let Some(mouse_pos) = response.hover_pos() {
                    let reticle_size = 15.0;
                    let reticle_color = Color32::from_rgba_unmultiplied(255, 0, 0, 180);
                    // 横線
                    painter.line_segment(
                        [Pos2::new(mouse_pos.x - reticle_size, mouse_pos.y),
                         Pos2::new(mouse_pos.x + reticle_size, mouse_pos.y)],
                        egui::Stroke::new(1.5, reticle_color)
                    );
                    // 縦線
                    painter.line_segment(
                        [Pos2::new(mouse_pos.x, mouse_pos.y - reticle_size),
                         Pos2::new(mouse_pos.x, mouse_pos.y + reticle_size)],
                        egui::Stroke::new(1.5, reticle_color)
                    );
                    // 中心円
                    painter.circle_stroke(mouse_pos, 5.0, egui::Stroke::new(1.5, reticle_color));
                }
            } else {
                // ローディングアニメーション
                self.loading_frame += 1;
                let dots = match (self.loading_frame / 15) % 4 {
                    0 => "",
                    1 => ".",
                    2 => "..",
                    _ => "...",
                };
                let spinner = match (self.loading_frame / 5) % 4 {
                    0 => "◐",
                    1 => "◓",
                    2 => "◑",
                    _ => "◒",
                };
                let loading_text = format!("{} {} {}", spinner, self.loading_stage, dots);
                painter.text(
                    response.rect.center(),
                    egui::Align2::CENTER_CENTER,
                    &loading_text,
                    egui::FontId::proportional(18.0),
                    Color32::from_rgb(100, 100, 100)
                );
                // 再描画をリクエスト（アニメーション継続）
                ctx.request_repaint();
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
use web_sys::{console, Event, FileReader, HtmlCanvasElement, HtmlInputElement, Headers, Request, RequestInit, Response};

#[cfg(target_arch = "wasm32")]
thread_local! {
    static PENDING_CSV: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
    static PENDING_JSON: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
    static PENDING_XLSX: std::cell::RefCell<Option<Vec<u8>>> = std::cell::RefCell::new(None);
    static PENDING_ERROR: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
    static JSON_FETCH_STARTED: std::cell::Cell<bool> = std::cell::Cell::new(false);
    static GEMINI_IN_FLIGHT: std::cell::Cell<bool> = std::cell::Cell::new(false);
}

#[cfg(target_arch = "wasm32")]
fn trigger_csv_dialog() {
    let Some(window) = web_sys::window() else { return; };
    let Some(document) = window.document() else { return; };

    let Ok(input) = document.create_element("input") else { return; };
    let Ok(input) = input.dyn_into::<HtmlInputElement>() else { return; };
    input.set_type("file");
    input.set_accept(".csv,text/csv");

    let on_change: std::rc::Rc<std::cell::RefCell<Option<Closure<dyn FnMut(Event)>>>> =
        std::rc::Rc::new(std::cell::RefCell::new(None));
    let on_load: std::rc::Rc<std::cell::RefCell<Option<Closure<dyn FnMut(Event)>>>> =
        std::rc::Rc::new(std::cell::RefCell::new(None));

    let on_change_handle = on_change.clone();
    let on_load_handle = on_load.clone();

    *on_change.borrow_mut() = Some(Closure::<dyn FnMut(Event)>::new(move |event: Event| {
        let Some(target) = event.target() else { return; };
        let Ok(input) = target.dyn_into::<HtmlInputElement>() else { return; };
        let Some(files) = input.files() else { return; };
        let Some(file) = files.get(0) else { return; };

        let Ok(reader) = FileReader::new() else { return; };
        let reader_clone = reader.clone();
        let on_load_handle_clone = on_load_handle.clone();
        let on_change_handle = on_change_handle.clone();

        *on_load_handle.borrow_mut() = Some(Closure::<dyn FnMut(Event)>::new(move |_event: Event| {
            let result = reader_clone.result();
            if let Ok(result) = result {
                if let Some(text) = result.as_string() {
                    PENDING_CSV.with(|cell| {
                        *cell.borrow_mut() = Some(text);
                    });
                }
            }
            reader_clone.set_onload(None);
            on_load_handle_clone.borrow_mut().take();
        }));

        if let Some(handler) = on_load_handle.borrow().as_ref() {
            reader.set_onload(Some(handler.as_ref().unchecked_ref()));
        }

        let _ = reader.read_as_text(&file);
        input.set_onchange(None);
        on_change_handle.borrow_mut().take();
    }));

    if let Some(handler) = on_change.borrow().as_ref() {
        input.set_onchange(Some(handler.as_ref().unchecked_ref()));
    }
    input.click();
}

#[cfg(target_arch = "wasm32")]
fn trigger_xlsx_dialog() {
    let Some(window) = web_sys::window() else { return; };
    let Some(document) = window.document() else { return; };

    let Ok(input) = document.create_element("input") else { return; };
    let Ok(input) = input.dyn_into::<HtmlInputElement>() else { return; };
    input.set_type("file");
    input.set_accept(".xlsx,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet");

    let on_change: std::rc::Rc<std::cell::RefCell<Option<Closure<dyn FnMut(Event)>>>> =
        std::rc::Rc::new(std::cell::RefCell::new(None));
    let on_load: std::rc::Rc<std::cell::RefCell<Option<Closure<dyn FnMut(Event)>>>> =
        std::rc::Rc::new(std::cell::RefCell::new(None));

    let on_change_handle = on_change.clone();
    let on_load_handle = on_load.clone();

    *on_change.borrow_mut() = Some(Closure::<dyn FnMut(Event)>::new(move |event: Event| {
        let Some(target) = event.target() else { return; };
        let Ok(input) = target.dyn_into::<HtmlInputElement>() else { return; };
        let Some(files) = input.files() else { return; };
        let Some(file) = files.get(0) else { return; };

        let Ok(reader) = FileReader::new() else { return; };
        let reader_clone = reader.clone();
        let on_load_handle_clone = on_load_handle.clone();
        let on_change_handle = on_change_handle.clone();

        *on_load_handle.borrow_mut() = Some(Closure::<dyn FnMut(Event)>::new(move |_event: Event| {
            let result = reader_clone.result();
            if let Ok(result) = result {
                if let Some(buf) = result.dyn_ref::<js_sys::ArrayBuffer>() {
                    let array = js_sys::Uint8Array::new(buf);
                    let mut bytes = vec![0u8; array.length() as usize];
                    array.copy_to(&mut bytes);
                    PENDING_XLSX.with(|cell| {
                        *cell.borrow_mut() = Some(bytes);
                    });
                }
            }
            reader_clone.set_onload(None);
            on_load_handle_clone.borrow_mut().take();
        }));

        if let Some(handler) = on_load_handle.borrow().as_ref() {
            reader.set_onload(Some(handler.as_ref().unchecked_ref()));
        }

        let _ = reader.read_as_array_buffer(&file);
        input.set_onchange(None);
        on_change_handle.borrow_mut().take();
    }));

    if let Some(handler) = on_change.borrow().as_ref() {
        input.set_onchange(Some(handler.as_ref().unchecked_ref()));
    }
    input.click();
}

#[cfg(target_arch = "wasm32")]
fn take_pending_csv() -> Option<String> {
    PENDING_CSV.with(|cell| cell.borrow_mut().take())
}

#[cfg(not(target_arch = "wasm32"))]
fn take_pending_csv() -> Option<String> {
    None
}

#[cfg(target_arch = "wasm32")]
fn take_pending_json() -> Option<String> {
    PENDING_JSON.with(|cell| cell.borrow_mut().take())
}

#[cfg(not(target_arch = "wasm32"))]
fn take_pending_json() -> Option<String> {
    None
}

#[cfg(target_arch = "wasm32")]
fn take_pending_xlsx() -> Option<Vec<u8>> {
    PENDING_XLSX.with(|cell| cell.borrow_mut().take())
}

#[cfg(not(target_arch = "wasm32"))]
fn take_pending_xlsx() -> Option<Vec<u8>> {
    None
}

#[cfg(target_arch = "wasm32")]
fn take_pending_error() -> Option<String> {
    PENDING_ERROR.with(|cell| cell.borrow_mut().take())
}

#[cfg(not(target_arch = "wasm32"))]
fn take_pending_error() -> Option<String> {
    None
}

#[cfg(target_arch = "wasm32")]
fn extract_json_from_text(text: &str) -> String {
    if !text.contains("```") {
        return text.trim().to_string();
    }
    let parts: Vec<&str> = text.split("```").collect();
    if parts.len() >= 2 {
        let mut candidate = parts[1];
        if let Some(rest) = candidate.strip_prefix("json") {
            candidate = rest;
        }
        return candidate.trim().to_string();
    }
    text.trim().to_string()
}

#[cfg(target_arch = "wasm32")]
fn start_gemini_parse(api_key: String, xlsx_bytes: Vec<u8>) {
    use wasm_bindgen_futures::JsFuture;

    let already_started = GEMINI_IN_FLIGHT.with(|cell| {
        if cell.get() { return true; }
        cell.set(true);
        false
    });
    if already_started { return; }

    wasm_bindgen_futures::spawn_local(async move {
        let result = async {
            let window = web_sys::window().ok_or("No window")?;
            let base64_data = BASE64_STANDARD.encode(&xlsx_bytes);
            let prompt = [
                "You are given an Excel file. Extract road cross-section data and output ONLY JSON array matching schema:",
                "[{",
                "  \"survey_point_name\": string,",
                "  \"dl\": number,",
                "  \"cl_index\": number,",
                "  \"l_to_cl_distance\": number,",
                "  \"survey_data\": [{",
                "    \"unit_distance\": number,",
                "    \"elevation\": number,",
                "    \"planned_height\": number,",
                "    \"cumulative_distance\": number,",
                "    \"cutting_bottom\": number",
                "  }],",
                "  \"route_distance\": number | null,",
                "  \"route_id\": string",
                "}]",
                "Rules:",
                "- Output JSON only (no markdown).",
                "- If multiple routes exist, set route_id per route; otherwise use \"route_1\".",
                "- cl_index is the index of the center line point within survey_data.",
                "- l_to_cl_distance is the distance from left edge to CL (meters).",
                "- cumulative_distance is distance from CL (left is negative, right positive).",
                "- cutting_bottom is ground elevation minus cutting depth if explicit data is not present.",
            ].join("\n");

            let payload = serde_json::json!({
                "contents": [{
                    "role": "user",
                    "parts": [
                        { "text": prompt },
                        { "inlineData": {
                            "mimeType": "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                            "data": base64_data
                        }}
                    ]
                }],
                "generationConfig": {
                    "temperature": 0.2,
                    "topP": 0.9,
                    "maxOutputTokens": 8192
                }
            });

            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}",
                api_key
            );
            let mut opts = RequestInit::new();
            opts.method("POST");
            let body = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
            opts.body(Some(&JsValue::from_str(&body)));

            let headers = Headers::new().map_err(|_| "Failed to create headers")?;
            headers
                .set("Content-Type", "application/json")
                .map_err(|_| "Failed to set headers")?;
            opts.headers(&headers);

            let request = Request::new_with_str_and_init(&url, &opts)
                .map_err(|_| "Failed to create request")?;

            let resp_value = JsFuture::from(window.fetch_with_request(&request))
                .await
                .map_err(|_| "Failed to fetch")?;

            let resp: Response = resp_value.dyn_into().map_err(|_| "Invalid response")?;
            let text = JsFuture::from(resp.text().map_err(|_| "Failed to read response")?)
                .await
                .map_err(|_| "Failed to read response text")?;
            let text = text.as_string().ok_or("Response is not text")?;

            let value: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
            let content = value
                .get("candidates")
                .and_then(|c| c.get(0))
                .and_then(|c| c.get("content"))
                .and_then(|c| c.get("parts"))
                .and_then(|p| p.get(0))
                .and_then(|p| p.get("text"))
                .and_then(|t| t.as_str())
                .ok_or("Gemini response missing content")?;

            Ok::<String, String>(extract_json_from_text(content))
        }.await;

        match result {
            Ok(json_text) => {
                PENDING_JSON.with(|cell| {
                    *cell.borrow_mut() = Some(json_text);
                });
            }
            Err(err) => {
                PENDING_ERROR.with(|cell| {
                    *cell.borrow_mut() = Some(err);
                });
            }
        }
        GEMINI_IN_FLIGHT.with(|cell| cell.set(false));
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn start_gemini_parse(_api_key: String, _xlsx_bytes: Vec<u8>) {
    // ネイティブでは何もしない
}

#[cfg(target_arch = "wasm32")]
fn start_json_fetch() {
    use wasm_bindgen_futures::JsFuture;

    let already_started = JSON_FETCH_STARTED.with(|cell| {
        if cell.get() { return true; }
        cell.set(true);
        false
    });
    if already_started { return; }

    wasm_bindgen_futures::spawn_local(async {
        let window = match web_sys::window() {
            Some(w) => w,
            None => return,
        };

        // 相対パスでsections.jsonを取得
        let url = "sections.json";
        let mut opts = RequestInit::new();
        opts.method("GET");

        let request = match Request::new_with_str_and_init(url, &opts) {
            Ok(r) => r,
            Err(_) => return,
        };

        let resp_value = match JsFuture::from(window.fetch_with_request(&request)).await {
            Ok(r) => r,
            Err(_) => return,
        };

        let resp: Response = match resp_value.dyn_into() {
            Ok(r) => r,
            Err(_) => return,
        };

        let text = match resp.text() {
            Ok(t) => t,
            Err(_) => return,
        };

        let text: String = match JsFuture::from(text).await {
            Ok(t) => match t.as_string() {
                Some(s) => s,
                None => return,
            },
            Err(_) => return,
        };

        PENDING_JSON.with(|cell| {
            *cell.borrow_mut() = Some(text);
        });
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn start_json_fetch() {
    // ネイティブでは何もしない
}

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
