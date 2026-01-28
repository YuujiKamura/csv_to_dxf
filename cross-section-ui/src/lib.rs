//! 横断図・切削計算システム - egui版
//!
//! PDF横断図に準拠した横断図表示と切削計算

use eframe::egui::{self, Color32, Painter, Pos2, Stroke, Vec2, Rect};
use serde::{Deserialize, Serialize};

mod font_metrics;
mod title_block;
mod ui_chars;

use font_metrics::cap_height_to_text_height;

// Re-export title block types
pub use title_block::{
    TitleBlockInfo,
    add_title_block_layer,
    draw_outer_frame,
    draw_top_title,
    draw_title_block,
    draw_drawing_frame,
    generate_title_block_test_drawing,
    generate_title_block_test_dxf_bytes,
    generate_title_block_dxf_for_download,
};

// dxf crate for proper DXF file generation
use dxf::Drawing;
use dxf::entities::{Entity, EntityType, Line, Solid, Text};
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

/// Drawing初期化（共通設定：バージョン、テキストスタイル）
fn new_drawing() -> Drawing {
    let mut drawing = Drawing::new();
    drawing.header.version = dxf::enums::AcadVersion::R2000;

    // テキストスタイル作成（Noto Sans JP - Web/DXF共通）
    let mut style = dxf::tables::Style::default();
    style.name = "NOTOSANSJP".to_string();
    style.primary_font_file_name = "Noto Sans JP".to_string();
    style.width_factor = 1.0;
    drawing.add_style(style);

    drawing
}

/// 水平アライメント
#[derive(Clone, Copy)]
enum TextAlign { Left, Center, Right }

/// TEXT追加ヘルパー（アライメント対応）
fn add_text(drawing: &mut Drawing, x: f64, y: f64, text: &str, height: f64, color: i16, layer: &str, align: TextAlign) {
    let mut t = Text::default();
    t.location = Point::new(x, y, 0.0);
    t.text_height = cap_height_to_text_height(height);  // フォントメトリクス補正
    t.value = text.to_string();
    t.text_style_name = "NOTOSANSJP".to_string();
    t.relative_x_scale_factor = 1.0;  // 幅が引き伸ばされるのを防止
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

/// テキスト幅を概算（文字数 × 高さ × 係数）
fn estimate_text_width(text: &str, height: f64) -> f64 {
    let char_count: f64 = text.chars()
        .map(|c| if c.is_ascii() { 0.6 } else { 1.0 })
        .sum();
    char_count * height
}

/// 白背景付きテキスト描画（マージンなし）
fn add_text_with_mask(
    drawing: &mut Drawing,
    x: f64, y: f64,
    text: &str,
    height: f64,
    color: i16,
    layer: &str,
    align: TextAlign
) {
    let width = estimate_text_width(text, height);

    // アライメントに応じたX座標調整
    let mask_x = match align {
        TextAlign::Left => x,
        TextAlign::Center => x - width / 2.0,
        TextAlign::Right => x - width,
    };

    // SOLID矩形（白、マージンなし）
    let mut solid = Solid::default();
    solid.first_corner = Point::new(mask_x, y - height / 2.0, 0.0);
    solid.second_corner = Point::new(mask_x + width, y - height / 2.0, 0.0);
    solid.third_corner = Point::new(mask_x, y + height / 2.0, 0.0);
    solid.fourth_corner = Point::new(mask_x + width, y + height / 2.0, 0.0);

    let mut entity = Entity::new(EntityType::Solid(solid));
    entity.common.layer = layer.to_string();
    entity.common.color = Color::from_index(7);  // 白
    drawing.add_entity(entity);

    // テキスト本体
    add_text(drawing, x, y, text, height, color, layer, align);
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
    t.text_height = cap_height_to_text_height(height);  // フォントメトリクス補正
    t.value = text.to_string();
    t.rotation = rotation;
    t.text_style_name = "NOTOSANSJP".to_string();
    t.relative_x_scale_factor = 1.0;  // 幅が引き伸ばされるのを防止

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
    let tick_down = 500.0; // 端点マークの下方向（アシを長く）

    // 水平線（寸法線本体）
    add_line(drawing, x1, y, x2, y, color, layer);

    // 左端の縦線（端点マーク）
    add_line(drawing, x1, y - tick_down, x1, y + tick_up, color, layer);

    // 右端の縦線（端点マーク）
    add_line(drawing, x2, y - tick_down, x2, y + tick_up, color, layer);

    // 中央にテキスト（寸法線の上にマージン、ボトムアラインメント）
    let mid_x = (x1 + x2) / 2.0;
    let text_y = y + 50.0;

    // 白背景マスク（テキストの下にSOLID矩形）
    let width = estimate_text_width(text, text_height);
    let mask_x = mid_x - width / 2.0;  // Center alignment
    let mut solid = Solid::default();
    // VerticalAlign::Bottom なので、テキストはyから上に伸びる
    solid.first_corner = Point::new(mask_x, text_y, 0.0);
    solid.second_corner = Point::new(mask_x + width, text_y, 0.0);
    solid.third_corner = Point::new(mask_x, text_y + text_height, 0.0);
    solid.fourth_corner = Point::new(mask_x + width, text_y + text_height, 0.0);

    let mut entity = Entity::new(EntityType::Solid(solid));
    entity.common.layer = layer.to_string();
    entity.common.color = Color::from_index(7);  // 白
    drawing.add_entity(entity);

    add_text_rotated(drawing, mid_x, text_y, text, text_height, color, layer,
        TextAlign::Center, VerticalAlign::Bottom, 0.0);
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
    let mut drawing = new_drawing();

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

    let mut drawing = new_drawing();

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
        let y = to_dxf_y(pt.planned_height);
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
    add_text(&mut drawing, cl_x, flag_y + 1600.0,
        &section.survey_point_name, text_height * 1.5, 7, "TEXT", TextAlign::Center);

    // ========== CL GH, FH ==========
    add_text_with_mask(&mut drawing, cl_x, flag_y + 800.0,
        &format!("GH={:.3}", cl_data.elevation), text_height, 7, "TEXT", TextAlign::Center);
    add_text_with_mask(&mut drawing, cl_x, flag_y + 400.0,
        &format!("FH={:.3}", cl_data.planned_height), text_height, 1, "PLAN", TextAlign::Center);

    // ========== L側 GH, FH（ポインター分上にオフセット）==========
    let l_ground_y = to_dxf_y(l_data.elevation);
    add_text_with_mask(&mut drawing, l_x, l_ground_y + 800.0 + pointer_offset,
        &format!("GH={:.3}", l_data.elevation), text_height, 7, "TEXT", TextAlign::Left);
    add_text_with_mask(&mut drawing, l_x, l_ground_y + 400.0 + pointer_offset,
        &format!("FH={:.3}", l_data.planned_height), text_height, 1, "PLAN", TextAlign::Left);

    // ========== R側 GH, FH（ポインター分上にオフセット）==========
    let r_ground_y = to_dxf_y(r_data.elevation);
    add_text_with_mask(&mut drawing, r_x, r_ground_y + 800.0 + pointer_offset,
        &format!("GH={:.3}", r_data.elevation), text_height, 7, "TEXT", TextAlign::Right);
    add_text_with_mask(&mut drawing, r_x, r_ground_y + 400.0 + pointer_offset,
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
    let arrow_len = 600.0;
    let arrow_drop = 20.0;  // 矢印の傾き（下がり量）
    let arrow_head = 180.0;
    let arrow_offset = 300.0;  // 矢印用のスペース
    let slope_text_y = flag_y + 500.0 + arrow_offset;  // 矢印分上げる

    // 左側勾配
    add_text_with_mask(&mut drawing, mid_l_x, slope_text_y,
        &format!("{:.1}%", left_slope), text_height, 7, "TEXT", TextAlign::Center);
    if left_slope.abs() > 0.01 {
        let arrow_y = slope_text_y - arrow_offset;
        let arrow_x = mid_l_x - arrow_len / 2.0;
        if left_slope < 0.0 {
            // 左下向き矢印（斜め）
            add_line(&mut drawing, arrow_x + arrow_len, arrow_y + arrow_drop, arrow_x, arrow_y - arrow_drop, 7, "SLOPE");
            add_line(&mut drawing, arrow_x, arrow_y - arrow_drop, arrow_x + arrow_head, arrow_y - arrow_drop + arrow_head * 0.4, 7, "SLOPE");
        } else {
            // 右下向き矢印（斜め）
            add_line(&mut drawing, arrow_x, arrow_y + arrow_drop, arrow_x + arrow_len, arrow_y - arrow_drop, 7, "SLOPE");
            add_line(&mut drawing, arrow_x + arrow_len, arrow_y - arrow_drop, arrow_x + arrow_len - arrow_head, arrow_y - arrow_drop + arrow_head * 0.4, 7, "SLOPE");
        }
    }

    // 右側勾配
    add_text_with_mask(&mut drawing, mid_r_x, slope_text_y,
        &format!("{:.1}%", right_slope), text_height, 7, "TEXT", TextAlign::Center);
    if right_slope.abs() > 0.01 {
        let arrow_y = slope_text_y - arrow_offset;
        let arrow_x = mid_r_x - arrow_len / 2.0;
        if right_slope < 0.0 {
            // 右下向き矢印（斜め）
            add_line(&mut drawing, arrow_x, arrow_y + arrow_drop, arrow_x + arrow_len, arrow_y - arrow_drop, 7, "SLOPE");
            add_line(&mut drawing, arrow_x + arrow_len, arrow_y - arrow_drop, arrow_x + arrow_len - arrow_head, arrow_y - arrow_drop + arrow_head * 0.4, 7, "SLOPE");
        } else {
            // 左下向き矢印（斜め）
            add_line(&mut drawing, arrow_x + arrow_len, arrow_y + arrow_drop, arrow_x, arrow_y - arrow_drop, 7, "SLOPE");
            add_line(&mut drawing, arrow_x, arrow_y - arrow_drop, arrow_x + arrow_head, arrow_y - arrow_drop + arrow_head * 0.4, 7, "SLOPE");
        }
    }

    // ========== DLラベル ==========
    add_text_rotated(&mut drawing, cl_x, to_dxf_y(dl),
        &format!("DL={:.3}  Scale:H1:V2", dl), text_height * 0.5, 8, "TEXT",
        TextAlign::Left, VerticalAlign::Bottom, 0.0);

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
        add_text_with_mask(&mut drawing, x, cutting_y,
            &format!("{:.0}", cutting_thickness_mm), cutting_text_height, 5, "CUTTING", align);
    }
    // ラベル「切削厚」を中央下に表示
    add_text_with_mask(&mut drawing, cl_x, cutting_y - cutting_text_height - 100.0,
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

// 配置可能エリア定数はtitle_block::available_areaを使用
pub use title_block::available_area;

/// グリッド配置のバウンディングボックス（DXF単位）
#[derive(Debug, Clone)]
pub struct GridBounds {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
    /// 列ごとの高さ（DXF単位）- 使用している列のみ有効
    pub column_heights: Vec<f64>,
}

impl GridBounds {
    pub fn width(&self) -> f64 { self.max_x - self.min_x }
    pub fn height(&self) -> f64 { self.max_y - self.min_y }

    /// 紙上サイズに換算（mm）
    pub fn to_paper_size(&self, plot_scale: f64) -> (f64, f64) {
        (self.width() / plot_scale, self.height() / plot_scale)
    }

    /// 図枠に収まるかチェック（列ごとの有効高さでチェック）
    pub fn fits_in_frame(&self, plot_scale: f64, _columns: usize) -> bool {
        let (w_mm, _) = self.to_paper_size(plot_scale);
        if w_mm > available_area::FRAME_USABLE_WIDTH_MM {
            return false;
        }
        // 列ごとに有効高さをチェック
        for (col, &col_height_dxf) in self.column_heights.iter().enumerate() {
            let col_height_mm = col_height_dxf / plot_scale;
            let max_height = available_area::height_for_column(col);
            if col_height_mm > max_height {
                return false;
            }
        }
        true
    }
}

/// グリッド配置時の全体バウンディングボックスを計算
/// 旗揚げやテキストを含む実際の描画範囲を返す
pub fn calc_grid_bounds(
    sections: &[CrossSectionData],
    columns: usize,
    column_gap: f64,
    row_gap: f64,
) -> Option<GridBounds> {
    if sections.is_empty() { return None; }

    let scale = 1000.0;
    let y_scale = scale * 2.0;
    // 図枠計算時は4列固定を使用（引数は互換性のため残す）
    let columns = available_area::COLUMN_COUNT.min(columns.max(1).min(sections.len()));
    let rows_per_column = (sections.len() + columns - 1) / columns;

    // 列ごとの最大幅と全体の最大高さを計算（generate_multi_drawing_internalと同じロジック）
    let mut col_max_left: Vec<f64> = vec![0.0; columns];
    let mut col_max_right: Vec<f64> = vec![0.0; columns];
    let mut max_height: f64 = 0.0;

    // 旗揚げ用の固定マージン（0.5m）
    const FLAG_MARGIN: f64 = 0.5;

    for (idx, section) in sections.iter().enumerate() {
        if section.survey_data.len() < 2 { continue; }
        let col = idx / rows_per_column;
        if col >= columns { break; }
        let data = &section.survey_data;
        let min_dist = data.first().unwrap().cumulative_distance;
        let max_dist = data.last().unwrap().cumulative_distance;
        col_max_left[col] = col_max_left[col].max(min_dist.abs());
        col_max_right[col] = col_max_right[col].max(max_dist);
        // 全体の最大高さ（旗揚げマージン込み）
        let max_elev = data.iter().map(|d| d.elevation.max(d.planned_height)).fold(f64::MIN, f64::max);
        max_height = max_height.max(max_elev - section.dl + row_gap + FLAG_MARGIN);
    }

    // セル高さ（全セクション共通、行間隔を含む）
    let cell_height = (max_height + row_gap) * y_scale;

    // 列ごとのCL位置を計算
    let mut col_x_offsets: Vec<f64> = Vec::with_capacity(columns);
    let mut cumulative_x = 0.0;

    for col in 0..columns {
        let cell_width = (col_max_left[col] + col_max_right[col] + column_gap) * scale;
        let cl_x = cumulative_x + (col_max_left[col] + column_gap / 2.0) * scale;
        col_x_offsets.push(cl_x);
        cumulative_x += cell_width;
    }

    // 各セクションのバウンディングボックスを計算して全体と列ごとを求める
    let mut global_min_x = f64::MAX;
    let mut global_min_y = f64::MAX;
    let mut global_max_x = f64::MIN;
    let mut global_max_y = f64::MIN;

    // 列ごとのmin/max Y
    let mut col_min_y: Vec<f64> = vec![f64::MAX; columns];
    let mut col_max_y: Vec<f64> = vec![f64::MIN; columns];

    for (idx, section) in sections.iter().enumerate() {
        if section.survey_data.len() < 2 { continue; }
        let col = idx / rows_per_column;
        if col >= columns { break; }
        let row_in_col = idx % rows_per_column;

        let data = &section.survey_data;
        let dl = round_dl(section.dl);

        // セクションの基準位置
        let offset_x = col_x_offsets[col];
        let offset_y = row_in_col as f64 * cell_height;

        // X方向の範囲（左端〜右端）
        let l_dist = data.first().unwrap().cumulative_distance;
        let r_dist = data.last().unwrap().cumulative_distance;
        let local_min_x = offset_x + l_dist * scale;
        let local_max_x = offset_x + r_dist * scale;

        // Y方向の範囲
        // 最低位置：cutting_bottomまたは切削厚ラベルの下端のいずれか低い方
        let min_cutting = data.iter().map(|d| d.cutting_bottom).fold(f64::MAX, f64::min);
        let cutting_bottom_y = offset_y + (min_cutting - dl) * y_scale;
        // 切削厚ラベルの下端（DL線から300+300+100=700mm下、さらにテキスト高さ300mm）
        let cutting_label_bottom_y = offset_y - 1000.0;  // DL基準なのでoffset_yから直接引く
        let local_min_y = cutting_bottom_y.min(cutting_label_bottom_y);

        // 最高位置：旗揚げの最上部
        // flag_y = cl_ground_y + 1600.0、その上にテキスト（text_height * 1.5 + 1300.0）
        let cl_idx = section.cl_index.min(data.len() - 1);
        let cl_elev = data[cl_idx].elevation;
        let cl_ground_y = offset_y + (cl_elev - dl) * y_scale;
        let flag_y = cl_ground_y + 1600.0;
        let text_height = 300.0;
        let local_max_y = flag_y + 1300.0 + text_height * 1.5 + 200.0;  // 余裕

        global_min_x = global_min_x.min(local_min_x);
        global_min_y = global_min_y.min(local_min_y);
        global_max_x = global_max_x.max(local_max_x);
        global_max_y = global_max_y.max(local_max_y);

        // 列ごとのY範囲を更新
        col_min_y[col] = col_min_y[col].min(local_min_y);
        col_max_y[col] = col_max_y[col].max(local_max_y);
    }

    if global_min_x == f64::MAX {
        return None;
    }

    // 列ごとの高さを計算
    let column_heights: Vec<f64> = (0..columns)
        .map(|col| {
            if col_min_y[col] == f64::MAX {
                0.0
            } else {
                col_max_y[col] - col_min_y[col]
            }
        })
        .collect();

    Some(GridBounds {
        min_x: global_min_x,
        min_y: global_min_y,
        max_x: global_max_x,
        max_y: global_max_y,
        column_heights,
    })
}

/// A3図枠に収まる最大列数を計算
pub fn calc_max_columns_for_frame(
    sections: &[CrossSectionData],
    column_gap: f64,
    row_gap: f64,
    plot_scale: f64,
) -> usize {
    if sections.is_empty() { return 1; }

    // 1列から順にチェックして収まる最大列数を探す
    let max_possible = sections.len();
    let mut max_fitting = 1;

    for cols in 1..=max_possible {
        if let Some(bounds) = calc_grid_bounds(sections, cols, column_gap, row_gap) {
            if bounds.fits_in_frame(plot_scale, cols) {
                max_fitting = cols;
            } else {
                // 列数が増えると幅は減るが高さは増える
                // 収まらなくなったらその前の列数が最大
                break;
            }
        }
    }

    max_fitting
}

/// A3図枠(1:500)に収まる最適列数を計算（後方互換性のため維持）
pub fn calc_optimal_columns_for_frame(
    sections: &[CrossSectionData],
    column_gap: f64,
    row_gap: f64,
    plot_scale: f64,
) -> usize {
    calc_max_columns_for_frame(sections, column_gap, row_gap, plot_scale)
}

/// 1ページに収まる最大セクション数を計算
/// 指定された列数とスケールで、A3図枠に収まる最大のセクション数を返す
pub fn calc_sections_per_page(
    sections: &[CrossSectionData],
    _columns: usize,  // 無視（4列固定のため）
    column_gap: f64,
    row_gap: f64,
    plot_scale: f64,
) -> usize {
    if sections.is_empty() { return 0; }

    // 図枠付きは4列固定
    let columns = available_area::COLUMN_COUNT;

    // 1個から順にチェックして収まる最大数を探す
    let mut max_fitting = 1;

    for n in 1..=sections.len() {
        let test_sections = &sections[0..n];
        if let Some(bounds) = calc_grid_bounds(test_sections, columns, column_gap, row_gap) {
            if bounds.fits_in_frame(plot_scale, columns) {
                max_fitting = n;
            } else {
                break;
            }
        }
    }

    max_fitting
}

/// 複数横断図をグリッド配置したDrawingを生成
/// 道路工事の配置ルール: 左下起点、列ごとに下から上へ、左から右へ
pub fn generate_multi_drawing(sections: &[CrossSectionData], columns: usize, column_gap: f64, row_gap: f64) -> Drawing {
    generate_multi_drawing_internal(sections, columns, column_gap, row_gap, None, None, 1.0, false)
}

/// スケール倍率指定で複数横断図をグリッド配置したDrawingを生成（コンボモード用）
pub fn generate_multi_drawing_scaled(sections: &[CrossSectionData], columns: usize, column_gap: f64, row_gap: f64, scale_multiplier: f64) -> Drawing {
    generate_multi_drawing_internal(sections, columns, column_gap, row_gap, None, None, scale_multiplier, false)
}

/// スケール倍率指定で複数横断図をグリッド配置（補完点を勾配補間）
pub fn generate_multi_drawing_scaled_interpolated(sections: &[CrossSectionData], columns: usize, column_gap: f64, row_gap: f64, scale_multiplier: f64) -> Drawing {
    generate_multi_drawing_internal(sections, columns, column_gap, row_gap, None, None, scale_multiplier, true)
}

/// 1:500図枠付きで複数横断図をグリッド配置したDrawingを生成
pub fn generate_multi_drawing_with_frame(
    sections: &[CrossSectionData],
    columns: usize,
    column_gap: f64,
    row_gap: f64,
    title_info: &TitleBlockInfo,
) -> Drawing {
    generate_multi_drawing_internal(sections, columns, column_gap, row_gap, Some(500.0), Some(title_info), 1.0, false)
}

/// 任意スケールの図枠付きで複数横断図をグリッド配置したDrawingを生成
pub fn generate_multi_drawing_with_frame_at_scale(
    sections: &[CrossSectionData],
    columns: usize,
    column_gap: f64,
    row_gap: f64,
    title_info: &TitleBlockInfo,
    frame_scale: f64,
) -> Drawing {
    generate_multi_drawing_internal(sections, columns, column_gap, row_gap, Some(frame_scale), Some(title_info), 1.0, false)
}

/// 複数横断図をグリッド配置したDrawingを生成（補完点を勾配補間）
/// 中間測点の補完点（V2, V4等）を採用点間の勾配で線形補間する
pub fn generate_multi_drawing_interpolated(sections: &[CrossSectionData], columns: usize, column_gap: f64, row_gap: f64) -> Drawing {
    generate_multi_drawing_internal(sections, columns, column_gap, row_gap, None, None, 1.0, true)
}

/// 任意スケールの図枠付きで複数横断図をグリッド配置（補完点を勾配補間）
pub fn generate_multi_drawing_with_frame_at_scale_interpolated(
    sections: &[CrossSectionData],
    columns: usize,
    column_gap: f64,
    row_gap: f64,
    title_info: &TitleBlockInfo,
    frame_scale: f64,
) -> Drawing {
    generate_multi_drawing_internal(sections, columns, column_gap, row_gap, Some(frame_scale), Some(title_info), 1.0, true)
}

/// 複数横断図をグリッド配置（内部実装）
/// frame_scale: None=図枠なし、Some(200.0)=1:200、Some(500.0)=1:500
/// scale_multiplier: 横断図全体のスケール倍率（コンボモード用）
/// interpolate_intermediate: 中間測点の補完点を勾配補間するか
fn generate_multi_drawing_internal(
    sections: &[CrossSectionData],
    columns: usize,
    column_gap: f64,
    row_gap: f64,
    frame_scale: Option<f64>,
    title_info: Option<&TitleBlockInfo>,
    scale_multiplier: f64,
    interpolate_intermediate: bool,
) -> Drawing {
    let scale = 1000.0 * scale_multiplier;

    let mut drawing = new_drawing();

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

    // 図枠付きの場合は4列固定、図枠なしの場合は指定列数を使用
    let columns = if frame_scale.is_some() {
        available_area::COLUMN_COUNT  // 4列固定
    } else {
        columns
    };

    // 旗揚げ用の固定マージン（0.5m）
    const FLAG_MARGIN: f64 = 0.5;

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
        max_height = max_height.max(max_elev - section.dl + row_gap + FLAG_MARGIN);
    }

    let cell_height = (max_height + row_gap) * scale * 2.0;  // 行間隔を含む（縦スケール2倍）

    // 図枠付きの場合: 4列均等分割の中心線を使用
    // 図枠なしの場合: 従来の動的計算
    let (col_x_offsets, col_y_offsets, grid_bounds) = if let Some(fs) = frame_scale {
        use available_area as area;

        // 列ごとのCL位置は固定（列の中心 * スケール）
        let col_centers: Vec<f64> = (0..columns)
            .map(|col| {
                if col < 4 {
                    area::COLUMN_CENTERS[col] * fs
                } else {
                    // 4列以上の場合は最後の列の中心を使用
                    area::COLUMN_CENTERS[3] * fs
                }
            })
            .collect();

        // グリッド全体の実際のバウンディングボックスを取得（Y配置とデバッグ用）
        let bounds = calc_grid_bounds(sections, columns, column_gap, row_gap);
        let content_height_mm = bounds.as_ref().map(|b| b.height() / fs).unwrap_or(0.0);

        // 列ごとのY方向オフセットを計算
        let mut col_offsets: Vec<f64> = Vec::with_capacity(columns);
        for col in 0..columns {
            // 列ごとの有効高さと下端位置
            let col_height = area::height_for_column(col);
            let col_bottom = area::bottom_for_column(col);

            // Y: 列ごとの利用可能領域でセンタリング
            let target_bottom = col_bottom + (col_height - content_height_mm) / 2.0;
            let offset_y = if let Some(ref b) = bounds {
                target_bottom * fs - b.min_y
            } else {
                0.0
            };
            col_offsets.push(offset_y);
        }

        (col_centers, col_offsets, bounds)
    } else {
        // 図枠なしの場合: 従来の動的計算（列ごとのセル幅と累積X位置を計算）
        let mut col_x_offsets: Vec<f64> = Vec::with_capacity(columns);
        let mut cumulative_x = 0.0;

        for col in 0..columns {
            let cell_width = (col_max_left[col] + col_max_right[col] + column_gap) * scale;
            // この列のCL位置: 累積X + 列内の左マージン
            let cl_x = cumulative_x + (col_max_left[col] + column_gap / 2.0) * scale;
            col_x_offsets.push(cl_x);
            cumulative_x += cell_width;
        }

        (col_x_offsets, vec![0.0; columns], None)
    };

    // セクションを描画
    for (idx, section) in sections.iter().enumerate() {
        if section.survey_data.len() < 2 { continue; }
        let col = idx / rows_per_column;
        let row_in_col = idx % rows_per_column;

        // X: 列の中心線を使用（図枠あり）または動的計算（図枠なし）
        let offset_x = if col < col_x_offsets.len() {
            col_x_offsets[col]
        } else {
            col_x_offsets.last().copied().unwrap_or(0.0)
        };

        // 列ごとのY方向オフセットを適用
        let col_y_offset = if col < col_y_offsets.len() {
            col_y_offsets[col]
        } else {
            col_y_offsets.last().copied().unwrap_or(0.0)
        };
        let offset_y = row_in_col as f64 * cell_height + col_y_offset;

        // 中間測点（起終点以外）の場合のみ、補完点を勾配補間
        let is_start_or_end = section.is_route_start || section.is_route_end;
        if interpolate_intermediate && !is_start_or_end {
            let mut section_to_draw = section.clone();
            section_to_draw.interpolate_planned_heights();
            draw_section_at_offset(&mut drawing, &section_to_draw, offset_x, offset_y, scale, scale_multiplier);
        } else {
            draw_section_at_offset(&mut drawing, section, offset_x, offset_y, scale, scale_multiplier);
        }
    }

    // 図枠を描画
    if let Some(fs) = frame_scale {
        const TEXT_SIZE_MM: f64 = 3.0;

        let info = title_info.cloned().unwrap_or_else(|| {
            TitleBlockInfo::new()
                .with_top_title("横断図")
                .with_scale(&format!("1:{} (A3)", fs as u32))
        });

        draw_drawing_frame(&mut drawing, &info, 0.0, 0.0, fs, TEXT_SIZE_MM);

        // グリッド外形を描画（デバッグ用）
        // 各列の中心線から推測したグリッド範囲を描画
        if info.show_debug_markers {
            if let Some(bounds) = &grid_bounds {
                const COLOR_RED: i16 = 1;

                // 列の左右端から計算（最初の列の左端〜最後の列の右端）
                let first_col = 0;
                let last_col = (columns - 1).min(3);
                let grid_min_x = available_area::COLUMN_LEFTS[first_col] * fs;
                let grid_max_x = available_area::COLUMN_RIGHTS[last_col] * fs;

                // 最小のY方向オフセット（左列が最も低い位置）を使用
                let min_y_offset = col_y_offsets.iter().copied().fold(f64::MAX, f64::min);
                let max_y_offset = col_y_offsets.iter().copied().fold(f64::MIN, f64::max);

                let grid_min_y = bounds.min_y + min_y_offset;
                let grid_max_y = bounds.max_y + max_y_offset;

                add_line(&mut drawing, grid_min_x, grid_min_y, grid_max_x, grid_min_y, COLOR_RED, "DEBUG");
                add_line(&mut drawing, grid_max_x, grid_min_y, grid_max_x, grid_max_y, COLOR_RED, "DEBUG");
                add_line(&mut drawing, grid_max_x, grid_max_y, grid_min_x, grid_max_y, COLOR_RED, "DEBUG");
                add_line(&mut drawing, grid_min_x, grid_max_y, grid_min_x, grid_min_y, COLOR_RED, "DEBUG");
            }
        }
    }

    drawing
}

/// 全ページを1つのDXFに垂直配置して生成
/// 各ページはA3図枠付きで、下から上に向かって配置される
pub fn generate_all_pages_drawing(
    all_sections: &[CrossSectionData],
    columns: usize,
    column_gap: f64,
    row_gap: f64,
    frame_scale: f64,
    base_title_info: &TitleBlockInfo,
    drawing_number_base: &str,
) -> Drawing {
    generate_all_pages_drawing_internal(
        all_sections, columns, column_gap, row_gap, frame_scale, base_title_info, drawing_number_base, false
    )
}

/// 全ページを1つのDXFに垂直配置して生成（補完点を勾配補間）
pub fn generate_all_pages_drawing_interpolated(
    all_sections: &[CrossSectionData],
    columns: usize,
    column_gap: f64,
    row_gap: f64,
    frame_scale: f64,
    base_title_info: &TitleBlockInfo,
    drawing_number_base: &str,
) -> Drawing {
    generate_all_pages_drawing_internal(
        all_sections, columns, column_gap, row_gap, frame_scale, base_title_info, drawing_number_base, true
    )
}

/// 全ページを1つのDXFに垂直配置して生成（内部実装）
fn generate_all_pages_drawing_internal(
    all_sections: &[CrossSectionData],
    columns: usize,
    column_gap: f64,
    row_gap: f64,
    frame_scale: f64,
    base_title_info: &TitleBlockInfo,
    drawing_number_base: &str,
    interpolate_intermediate: bool,
) -> Drawing {
    let mut drawing = new_drawing();

    for (name, color_idx) in [
        ("GROUND", 7), ("PLAN", 1), ("TEXT", 7),
        ("DIMENSION", 8), ("CUTTING", 5), ("FRAME", 9), ("TITLEBLOCK", 7), ("DEBUG", 1)
    ] {
        drawing.add_layer(dxf::tables::Layer {
            name: name.to_string(),
            color: Color::from_index(color_idx),
            ..Default::default()
        });
    }

    if all_sections.is_empty() { return drawing; }

    // ページあたりのセクション数を計算
    let sections_per_page = calc_sections_per_page(all_sections, columns, column_gap, row_gap, frame_scale);
    if sections_per_page == 0 { return drawing; }

    let total_pages = (all_sections.len() + sections_per_page - 1) / sections_per_page;

    // A3の高さ（mm）をDXF単位に変換
    const A3_HEIGHT_MM: f64 = 297.0;
    let page_height_dxf = A3_HEIGHT_MM * frame_scale;
    // ページ間のギャップ（10mm）
    const PAGE_GAP_MM: f64 = 10.0;
    let page_gap_dxf = PAGE_GAP_MM * frame_scale;

    // 各ページを描画
    for page in 0..total_pages {
        let start_idx = page * sections_per_page;
        let end_idx = (start_idx + sections_per_page).min(all_sections.len());
        let page_sections = &all_sections[start_idx..end_idx];

        // ページのY方向オフセット（下から上に配置）
        let page_offset_y = page as f64 * (page_height_dxf + page_gap_dxf);

        // ページ番号付きの図番
        let page_num = if total_pages > 1 {
            format!("{}-{}", drawing_number_base, page + 1)
        } else {
            drawing_number_base.to_string()
        };

        let info = base_title_info.clone()
            .with_drawing_number(&page_num);

        // このページのセクションを描画
        draw_page_content(
            &mut drawing, page_sections, columns, column_gap, row_gap, frame_scale, &info, page_offset_y,
            interpolate_intermediate
        );
    }

    drawing
}

/// 1ページ分のコンテンツを描画（オフセット付き）
/// interpolate_intermediate: 中間測点の補完点を勾配補間するか（起終点はフラグで判定）
fn draw_page_content(
    drawing: &mut Drawing,
    sections: &[CrossSectionData],
    _columns: usize,
    column_gap: f64,
    row_gap: f64,
    frame_scale: f64,
    title_info: &TitleBlockInfo,
    page_offset_y: f64,
    interpolate_intermediate: bool,
) {
    use available_area as area;

    let scale = 1000.0;
    let columns = area::COLUMN_COUNT;  // 4列固定

    if sections.is_empty() { return; }

    // 旗揚げ用の固定マージン（0.5m）
    const FLAG_MARGIN: f64 = 0.5;

    let rows_per_column = (sections.len() + columns - 1) / columns;

    // 全体の最大高さを計算
    let mut max_height: f64 = 0.0;
    for section in sections.iter() {
        if section.survey_data.len() < 2 { continue; }
        let data = &section.survey_data;
        let max_elev = data.iter().map(|d| d.elevation.max(d.planned_height)).fold(f64::MIN, f64::max);
        max_height = max_height.max(max_elev - section.dl + row_gap + FLAG_MARGIN);
    }

    let cell_height = (max_height + row_gap) * scale * 2.0;

    // 列ごとのCL位置（固定）
    let col_centers: Vec<f64> = (0..columns)
        .map(|col| area::COLUMN_CENTERS[col] * frame_scale)
        .collect();

    // グリッドバウンドを計算してY方向オフセットを決定
    let bounds = calc_grid_bounds(sections, columns, column_gap, row_gap);
    let content_height_mm = bounds.as_ref().map(|b| b.height() / frame_scale).unwrap_or(0.0);

    let mut col_y_offsets: Vec<f64> = Vec::with_capacity(columns);
    for col in 0..columns {
        let col_height = area::height_for_column(col);
        let col_bottom = area::bottom_for_column(col);
        let target_bottom = col_bottom + (col_height - content_height_mm) / 2.0;
        let offset_y = if let Some(ref b) = bounds {
            target_bottom * frame_scale - b.min_y
        } else {
            0.0
        };
        col_y_offsets.push(offset_y);
    }

    // セクションを描画
    for (idx, section) in sections.iter().enumerate() {
        if section.survey_data.len() < 2 { continue; }
        let col = idx / rows_per_column;
        let row_in_col = idx % rows_per_column;

        let offset_x = col_centers[col.min(columns - 1)];
        let col_y_offset = col_y_offsets[col.min(columns - 1)];
        let offset_y = row_in_col as f64 * cell_height + col_y_offset + page_offset_y;

        // 中間測点（起終点以外）の場合のみ、補完点を勾配補間
        let is_start_or_end = section.is_route_start || section.is_route_end;
        if interpolate_intermediate && !is_start_or_end {
            let mut section_to_draw = section.clone();
            section_to_draw.interpolate_planned_heights();
            draw_section_at_offset(drawing, &section_to_draw, offset_x, offset_y, scale, 1.0);
        } else {
            draw_section_at_offset(drawing, section, offset_x, offset_y, scale, 1.0);
        }
    }

    // 図枠を描画
    const TEXT_SIZE_MM: f64 = 3.0;
    draw_drawing_frame(drawing, title_info, 0.0, page_offset_y, frame_scale, TEXT_SIZE_MM);
}

/// 全ページを1つのDXFに垂直配置してバイト配列で返す
pub fn generate_all_pages_dxf_bytes(
    all_sections: &[CrossSectionData],
    columns: usize,
    column_gap: f64,
    row_gap: f64,
    frame_scale: f64,
    base_title_info: &TitleBlockInfo,
    drawing_number_base: &str,
) -> Vec<u8> {
    let drawing = generate_all_pages_drawing_interpolated(
        all_sections, columns, column_gap, row_gap, frame_scale, base_title_info, drawing_number_base
    );
    let mut output: Vec<u8> = Vec::new();
    drawing.save(&mut output).expect("Failed to save DXF");
    output
}

/// グリッド配置内の特定セクションのバウンディングボックスを計算
/// generate_multi_drawingと同じ配置ロジックを使用
fn calc_section_bounds_in_grid(
    sections: &[CrossSectionData], target_idx: usize, columns: usize, column_gap: f64, row_gap: f64
) -> Option<(f32, f32, f32, f32)> {
    if sections.is_empty() || target_idx >= sections.len() { return None; }

    let scale = 1000.0;
    let rows_per_column = (sections.len() + columns - 1) / columns;

    // 旗揚げ用の固定マージン（0.5m）
    const FLAG_MARGIN: f64 = 0.5;

    // 列ごとの最大幅と全体の最大高さを計算（generate_multi_drawingと同じ）
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
        max_height = max_height.max(max_elev - section.dl + row_gap + FLAG_MARGIN);
    }

    // 列ごとのCL位置を計算
    let mut col_x_offsets: Vec<f64> = Vec::with_capacity(columns);
    let mut cumulative_x = 0.0;

    for col in 0..columns {
        let cell_width = (col_max_left[col] + col_max_right[col] + column_gap) * scale;
        let cl_x = cumulative_x + (col_max_left[col] + column_gap / 2.0) * scale;
        col_x_offsets.push(cl_x);
        cumulative_x += cell_width;
    }

    let cell_height = (max_height + row_gap) * scale * 2.0;

    // ターゲットセクションの位置を計算
    let target_section = &sections[target_idx];
    if target_section.survey_data.len() < 2 { return None; }

    let col = target_idx / rows_per_column;
    let row_in_col = target_idx % rows_per_column;
    let offset_x = col_x_offsets[col];
    let offset_y = row_in_col as f64 * cell_height;

    // セクションのバウンディングボックスを計算
    let data = &target_section.survey_data;
    let dl = round_dl(target_section.dl);
    let y_scale = scale * 2.0;

    let min_dist = data.first().unwrap().cumulative_distance;
    let max_dist = data.last().unwrap().cumulative_distance;
    let min_elev = data.iter().map(|d| d.elevation.min(d.planned_height)).fold(f64::MAX, f64::min);
    let max_elev = data.iter().map(|d| d.elevation.max(d.planned_height)).fold(f64::MIN, f64::max);

    let min_x = (offset_x + min_dist * scale) as f32;
    let max_x = (offset_x + max_dist * scale) as f32;
    let min_y = (offset_y + (dl - dl) * y_scale) as f32;  // DL位置
    let max_y = (offset_y + (max_elev - dl + 2.0) * y_scale) as f32;  // 旗揚げ分のマージン

    Some((min_x, min_y, max_x, max_y))
}

fn draw_section_at_offset(drawing: &mut Drawing, section: &CrossSectionData,
                          offset_x: f64, offset_y: f64, scale: f64, scale_multiplier: f64) {
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

    // テキスト・スペーサー関連の値にscale_multiplierを適用
    let text_height = 300.0 * scale_multiplier;
    let pointer_offset = 500.0 * scale_multiplier;

    // ========== 測点ポインター（逆三角形 + Vラベル）==========
    let pointer_size = 150.0 * scale_multiplier;
    for (i, pt) in data.iter().enumerate() {
        let x = to_dxf_x(pt.cumulative_distance);
        let y = to_dxf_y(pt.planned_height);  // 計画高を基準
        let top_y = y + pointer_size;
        let half_w = pointer_size * 0.6;
        add_line(drawing, x - half_w, top_y, x + half_w, top_y, 5, "CUTTING");
        add_line(drawing, x - half_w, top_y, x, y, 5, "CUTTING");
        add_line(drawing, x + half_w, top_y, x, y, 5, "CUTTING");
        let label = format!("V{}", i + 1);
        add_text(drawing, x, top_y + 300.0 * scale_multiplier, &label, text_height, 5, "CUTTING", TextAlign::Center);
    }

    let cl_ground_y = to_dxf_y(cl_data.elevation);
    let flag_y = cl_ground_y + 1600.0 * scale_multiplier;
    let l_x = to_dxf_x(l_data.cumulative_distance);
    let cl_x = to_dxf_x(cl_data.cumulative_distance);
    let r_x = to_dxf_x(r_data.cumulative_distance);

    add_text(drawing, cl_x, flag_y + 1300.0 * scale_multiplier, &section.survey_point_name, text_height * 1.5, 7, "TEXT", TextAlign::Center);
    add_text_with_mask(drawing, cl_x, flag_y + 800.0 * scale_multiplier, &format!("GH={:.3}", cl_data.elevation), text_height, 7, "TEXT", TextAlign::Center);
    add_text_with_mask(drawing, cl_x, flag_y + 400.0 * scale_multiplier, &format!("FH={:.3}", cl_data.planned_height), text_height, 1, "PLAN", TextAlign::Center);

    let l_ground_y = to_dxf_y(l_data.elevation);
    add_text_with_mask(drawing, l_x, l_ground_y + 800.0 * scale_multiplier + pointer_offset, &format!("GH={:.3}", l_data.elevation), text_height, 7, "TEXT", TextAlign::Left);
    add_text_with_mask(drawing, l_x, l_ground_y + 400.0 * scale_multiplier + pointer_offset, &format!("FH={:.3}", l_data.planned_height), text_height, 1, "PLAN", TextAlign::Left);

    let r_ground_y = to_dxf_y(r_data.elevation);
    add_text_with_mask(drawing, r_x, r_ground_y + 800.0 * scale_multiplier + pointer_offset, &format!("GH={:.3}", r_data.elevation), text_height, 7, "TEXT", TextAlign::Right);
    add_text_with_mask(drawing, r_x, r_ground_y + 400.0 * scale_multiplier + pointer_offset, &format!("FH={:.3}", r_data.planned_height), text_height, 1, "PLAN", TextAlign::Right);

    let mid_l_x = (l_x + cl_x) / 2.0;
    let mid_r_x = (cl_x + r_x) / 2.0;
    let arrow_len = 600.0 * scale_multiplier;
    let arrow_drop = 20.0 * scale_multiplier;
    let arrow_head = 180.0 * scale_multiplier;
    let arrow_offset = 300.0 * scale_multiplier;
    let slope_text_y = flag_y + 500.0 * scale_multiplier + arrow_offset;

    // 左側勾配
    add_text_with_mask(drawing, mid_l_x, slope_text_y, &format!("{:.1}%", left_slope), text_height, 7, "TEXT", TextAlign::Center);
    if left_slope.abs() > 0.01 {
        let arrow_y = slope_text_y - arrow_offset;
        let arrow_x = mid_l_x - arrow_len / 2.0;
        if left_slope < 0.0 {
            add_line(drawing, arrow_x + arrow_len, arrow_y + arrow_drop, arrow_x, arrow_y - arrow_drop, 7, "SLOPE");
            add_line(drawing, arrow_x, arrow_y - arrow_drop, arrow_x + arrow_head, arrow_y - arrow_drop + arrow_head * 0.4, 7, "SLOPE");
        } else {
            add_line(drawing, arrow_x, arrow_y + arrow_drop, arrow_x + arrow_len, arrow_y - arrow_drop, 7, "SLOPE");
            add_line(drawing, arrow_x + arrow_len, arrow_y - arrow_drop, arrow_x + arrow_len - arrow_head, arrow_y - arrow_drop + arrow_head * 0.4, 7, "SLOPE");
        }
    }

    // 右側勾配
    add_text_with_mask(drawing, mid_r_x, slope_text_y, &format!("{:.1}%", right_slope), text_height, 7, "TEXT", TextAlign::Center);
    if right_slope.abs() > 0.01 {
        let arrow_y = slope_text_y - arrow_offset;
        let arrow_x = mid_r_x - arrow_len / 2.0;
        if right_slope < 0.0 {
            add_line(drawing, arrow_x, arrow_y + arrow_drop, arrow_x + arrow_len, arrow_y - arrow_drop, 7, "SLOPE");
            add_line(drawing, arrow_x + arrow_len, arrow_y - arrow_drop, arrow_x + arrow_len - arrow_head, arrow_y - arrow_drop + arrow_head * 0.4, 7, "SLOPE");
        } else {
            add_line(drawing, arrow_x + arrow_len, arrow_y + arrow_drop, arrow_x, arrow_y - arrow_drop, 7, "SLOPE");
            add_line(drawing, arrow_x, arrow_y - arrow_drop, arrow_x + arrow_head, arrow_y - arrow_drop + arrow_head * 0.4, 7, "SLOPE");
        }
    }

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

    add_text_rotated(drawing, cl_x, to_dxf_y(dl),
        &format!("DL={:.3}  Scale:H1:V2", dl), text_height * 0.5, 8, "TEXT",
        TextAlign::Left, VerticalAlign::Bottom, 0.0);

    // ========== 切削厚表示（DLライン上部） ==========
    let cutting_text_height = text_height;  // GH等と同じサイズ
    let cutting_y = to_dxf_y(dl) - 300.0 * scale_multiplier;
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
        add_text_with_mask(drawing, x, cutting_y,
            &format!("{:.0}", cutting_thickness_mm), cutting_text_height, 5, "CUTTING", align);
    }
    // ラベル「切削厚」を中央下に表示
    add_text_with_mask(drawing, cl_x, cutting_y - cutting_text_height - 100.0 * scale_multiplier,
        "切削厚", cutting_text_height, 5, "CUTTING", TextAlign::Center);

    // DLライン（幅員と同じ長さ）
    add_line(drawing, l_x, to_dxf_y(dl), r_x, to_dxf_y(dl), 8, "DIMENSION");
    let cl_cumulative = cl_data.cumulative_distance;
    add_line(drawing, to_dxf_x(cl_cumulative), to_dxf_y(dl), to_dxf_x(cl_cumulative), to_dxf_y(dl + 1.0), 8, "DIMENSION");
}

pub fn generate_multi_dxf_bytes(sections: &[CrossSectionData], columns: usize, column_gap: f64, row_gap: f64) -> Vec<u8> {
    let drawing = generate_multi_drawing(sections, columns, column_gap, row_gap);
    let mut output: Vec<u8> = Vec::new();
    drawing.save(&mut output).expect("Failed to save DXF");
    output
}

/// 1:500図枠付きDXFバイト列を生成
pub fn generate_multi_dxf_bytes_with_frame(
    sections: &[CrossSectionData],
    columns: usize,
    column_gap: f64,
    row_gap: f64,
    title_info: &TitleBlockInfo,
) -> Vec<u8> {
    let drawing = generate_multi_drawing_with_frame(sections, columns, column_gap, row_gap, title_info);
    let mut output: Vec<u8> = Vec::new();
    drawing.save(&mut output).expect("Failed to save DXF");
    output
}

/// 任意スケールの図枠付きDXFバイト列を生成
pub fn generate_multi_dxf_bytes_with_frame_at_scale(
    sections: &[CrossSectionData],
    columns: usize,
    column_gap: f64,
    row_gap: f64,
    title_info: &TitleBlockInfo,
    frame_scale: f64,
) -> Vec<u8> {
    let drawing = generate_multi_drawing_with_frame_at_scale_interpolated(sections, columns, column_gap, row_gap, title_info, frame_scale);
    let mut output: Vec<u8> = Vec::new();
    drawing.save(&mut output).expect("Failed to save DXF");
    output
}

// ============================================================================
// Dekigata Management Sheet (出来形管理用紙)
// ============================================================================

/// 出来形管理用紙のデフォルト切削厚（mm）
const DEKIGATA_DEFAULT_CUTTING_THICKNESS_MM: f64 = 50.0;

/// 出来形管理用紙の縮尺（1:50固定）
const DEKIGATA_SCALE: f64 = 50.0;

/// 出来形管理表のセル高さ（mm）
const DEKIGATA_TABLE_ROW_HEIGHT_MM: f64 = 16.0;  // 2倍に拡大

/// 出来形管理表のヘッダー列幅（mm）
const DEKIGATA_TABLE_HEADER_WIDTH_MM: f64 = 50.0;  // 2倍に拡大

/// 出来形管理表のデータ列幅（mm）
const DEKIGATA_TABLE_DATA_WIDTH_MM: f64 = 36.0;  // 2倍に拡大

/// 出来形管理表のテキストサイズ（mm）
const DEKIGATA_TABLE_TEXT_SIZE_MM: f64 = 5.0;  // 2倍に拡大

/// 単一測点の出来形管理用紙を描画
///
/// # Arguments
/// * `drawing` - DXFドローイング
/// * `section` - 横断データ
/// * `origin_x` - 原点X座標（DXF単位）
/// * `origin_y` - 原点Y座標（DXF単位）
/// * `frame_scale` - 図枠スケール（1:50なら50.0）
fn draw_dekigata_page(
    drawing: &mut Drawing,
    section: &CrossSectionData,
    origin_x: f64,
    origin_y: f64,
    frame_scale: f64,
) {
    let data = &section.survey_data;
    if data.len() < 2 { return; }

    // A3用紙の有効領域（mm単位）
    // 上部: 横断図、下部: 出来形管理表
    let table_height_mm = DEKIGATA_TABLE_ROW_HEIGHT_MM * 5.0;  // 5行（ヘッダー + 4データ行）
    let table_bottom_margin_mm = 60.0;  // タイトル枠の上

    // 横断図の描画領域（上部）
    let cross_section_bottom_mm = table_bottom_margin_mm + table_height_mm + 10.0;
    let cross_section_top_mm = 261.0;  // 上部タイトル下端
    let cross_section_height_mm = cross_section_top_mm - cross_section_bottom_mm;

    // 横断図の描画
    let scale = 1000.0;  // 1m = 1000 DXF単位
    let y_scale = scale * 2.0;  // 縦方向は2倍

    // 横断図のサイズを計算
    let l_data = &data[0];
    let r_data = &data[data.len() - 1];
    let cl_data = &data[section.cl_index.min(data.len() - 1)];
    let total_width_m = (r_data.cumulative_distance - l_data.cumulative_distance).abs();
    let max_elev = data.iter().map(|d| d.elevation.max(d.planned_height)).fold(f64::MIN, f64::max);
    let min_elev = data.iter().map(|d| d.elevation.min(d.planned_height).min(d.cutting_bottom)).fold(f64::MAX, f64::min);
    let dl = round_dl(section.dl);
    let height_range_m = (max_elev - dl + 2.0);  // 旗揚げ分のマージン

    // 横断図のDXF単位でのサイズ
    let cross_section_width_dxf = total_width_m * scale;
    let cross_section_height_dxf = height_range_m * y_scale;

    // 横断図を配置する中心位置（図枠内の中央上部）
    let frame_center_x_mm = 210.0;  // A3幅の中央
    let cross_section_center_y_mm = cross_section_bottom_mm + cross_section_height_mm / 2.0;

    // 横断図のCLを中心に配置
    let cl_offset = cl_data.cumulative_distance;
    let offset_x = origin_x + (frame_center_x_mm * frame_scale) - (cl_offset * scale);
    let offset_y = origin_y + (cross_section_bottom_mm * frame_scale);

    // 横断図を描画（draw_section_at_offsetと同様のロジック）
    draw_dekigata_cross_section(drawing, section, offset_x, offset_y, scale);

    // 出来形管理表を描画（センタリング）
    let num_points = data.len();
    let table_width_mm = DEKIGATA_TABLE_HEADER_WIDTH_MM + DEKIGATA_TABLE_DATA_WIDTH_MM * num_points as f64;
    let page_width_mm = 420.0;  // A3横幅
    let table_x = origin_x + ((page_width_mm - table_width_mm) / 2.0) * frame_scale;  // センタリング
    let table_y = origin_y + table_bottom_margin_mm * frame_scale;
    draw_dekigata_table(drawing, section, table_x, table_y, frame_scale);
}

/// 出来形管理用紙用の横断図を描画
fn draw_dekigata_cross_section(
    drawing: &mut Drawing,
    section: &CrossSectionData,
    offset_x: f64,
    offset_y: f64,
    scale: f64,
) {
    let data = &section.survey_data;
    let dl = round_dl(section.dl);
    let to_dxf_x = |d: f64| offset_x + d * scale;
    let y_scale = scale * 2.0;
    let to_dxf_y = |h: f64| offset_y + (h - dl) * y_scale;

    let l_data = &data[0];
    let cl_data = &data[section.cl_index.min(data.len() - 1)];
    let r_data = &data[data.len() - 1];

    // 地盤線・計画線・切削底面線を描画
    for i in 0..data.len() - 1 {
        add_line(drawing, to_dxf_x(data[i].cumulative_distance), to_dxf_y(data[i].elevation),
            to_dxf_x(data[i + 1].cumulative_distance), to_dxf_y(data[i + 1].elevation), 7, "GROUND");
        add_line(drawing, to_dxf_x(data[i].cumulative_distance), to_dxf_y(data[i].planned_height),
            to_dxf_x(data[i + 1].cumulative_distance), to_dxf_y(data[i + 1].planned_height), 1, "PLAN");
        add_line(drawing, to_dxf_x(data[i].cumulative_distance), to_dxf_y(data[i].cutting_bottom),
            to_dxf_x(data[i + 1].cumulative_distance), to_dxf_y(data[i + 1].cutting_bottom), 5, "CUTTING");
    }

    let text_height = 300.0;
    let pointer_size = 150.0;
    let pointer_offset = 500.0;

    // 測点ポインター（V1, V2...）
    for (i, pt) in data.iter().enumerate() {
        let x = to_dxf_x(pt.cumulative_distance);
        let y = to_dxf_y(pt.planned_height);
        let top_y = y + pointer_size;
        let half_w = pointer_size * 0.6;
        add_line(drawing, x - half_w, top_y, x + half_w, top_y, 5, "CUTTING");
        add_line(drawing, x - half_w, top_y, x, y, 5, "CUTTING");
        add_line(drawing, x + half_w, top_y, x, y, 5, "CUTTING");
        let label = format!("V{}", i + 1);
        add_text(drawing, x, top_y + 300.0, &label, text_height, 5, "CUTTING", TextAlign::Center);
    }

    // 測点名・GH・FH表示
    let cl_ground_y = to_dxf_y(cl_data.elevation);
    let flag_y = cl_ground_y + 1600.0;
    let l_x = to_dxf_x(l_data.cumulative_distance);
    let cl_x = to_dxf_x(cl_data.cumulative_distance);
    let r_x = to_dxf_x(r_data.cumulative_distance);

    add_text(drawing, cl_x, flag_y + 1300.0, &section.survey_point_name, text_height * 1.5, 7, "TEXT", TextAlign::Center);
    add_text_with_mask(drawing, cl_x, flag_y + 800.0, &format!("GH={:.3}", cl_data.elevation), text_height, 7, "TEXT", TextAlign::Center);
    add_text_with_mask(drawing, cl_x, flag_y + 400.0, &format!("FH={:.3}", cl_data.planned_height), text_height, 1, "PLAN", TextAlign::Center);

    let l_ground_y = to_dxf_y(l_data.elevation);
    add_text_with_mask(drawing, l_x, l_ground_y + 800.0 + pointer_offset, &format!("GH={:.3}", l_data.elevation), text_height, 7, "TEXT", TextAlign::Left);
    add_text_with_mask(drawing, l_x, l_ground_y + 400.0 + pointer_offset, &format!("FH={:.3}", l_data.planned_height), text_height, 1, "PLAN", TextAlign::Left);

    let r_ground_y = to_dxf_y(r_data.elevation);
    add_text_with_mask(drawing, r_x, r_ground_y + 800.0 + pointer_offset, &format!("GH={:.3}", r_data.elevation), text_height, 7, "TEXT", TextAlign::Right);
    add_text_with_mask(drawing, r_x, r_ground_y + 400.0 + pointer_offset, &format!("FH={:.3}", r_data.planned_height), text_height, 1, "PLAN", TextAlign::Right);

    // DLライン
    add_line(drawing, l_x, to_dxf_y(dl), r_x, to_dxf_y(dl), 8, "DIMENSION");
    add_text_rotated(drawing, cl_x, to_dxf_y(dl),
        &format!("DL={:.3}  Scale:H1:V2", dl), text_height * 0.5, 8, "TEXT",
        TextAlign::Left, VerticalAlign::Bottom, 0.0);
}

/// 出来形管理表を描画
fn draw_dekigata_table(
    drawing: &mut Drawing,
    section: &CrossSectionData,
    table_x: f64,
    table_y: f64,
    frame_scale: f64,
) {
    let data = &section.survey_data;
    let num_points = data.len();

    let row_height = DEKIGATA_TABLE_ROW_HEIGHT_MM * frame_scale;
    let header_width = DEKIGATA_TABLE_HEADER_WIDTH_MM * frame_scale;
    let data_width = DEKIGATA_TABLE_DATA_WIDTH_MM * frame_scale;
    let text_size = DEKIGATA_TABLE_TEXT_SIZE_MM * frame_scale;

    let total_width = header_width + data_width * num_points as f64;
    let total_height = row_height * 5.0;  // 5行

    // 外枠を描画
    add_line(drawing, table_x, table_y, table_x + total_width, table_y, 7, "DEKIGATA");
    add_line(drawing, table_x, table_y + total_height, table_x + total_width, table_y + total_height, 7, "DEKIGATA");
    add_line(drawing, table_x, table_y, table_x, table_y + total_height, 7, "DEKIGATA");
    add_line(drawing, table_x + total_width, table_y, table_x + total_width, table_y + total_height, 7, "DEKIGATA");

    // 横線（行の区切り）
    for i in 1..5 {
        let y = table_y + row_height * i as f64;
        add_line(drawing, table_x, y, table_x + total_width, y, 7, "DEKIGATA");
    }

    // ヘッダー列の縦線
    add_line(drawing, table_x + header_width, table_y, table_x + header_width, table_y + total_height, 7, "DEKIGATA");

    // データ列の縦線
    for i in 1..num_points {
        let x = table_x + header_width + data_width * i as f64;
        add_line(drawing, x, table_y, x, table_y + total_height, 7, "DEKIGATA");
    }

    // ヘッダーラベル（上から下へ: V1-Vn, 計画高(設計), 計画高(実測), 切削高(設計), 切削高(実測)）
    let row_labels = ["", "計画高\n（設計）", "計画高\n（実測）", "切削高\n（設計）", "切削高\n（実測）"];
    for (i, label) in row_labels.iter().enumerate() {
        if !label.is_empty() {
            let y = table_y + total_height - row_height * (i as f64 + 0.5);
            // 2行に分割されている場合は中央に1行目を表示
            let lines: Vec<&str> = label.split('\n').collect();
            if lines.len() == 2 {
                add_text(drawing, table_x + header_width / 2.0, y + text_size * 0.6, lines[0], text_size, 7, "DEKIGATA", TextAlign::Center);
                add_text(drawing, table_x + header_width / 2.0, y - text_size * 0.6, lines[1], text_size, 7, "DEKIGATA", TextAlign::Center);
            } else {
                add_text(drawing, table_x + header_width / 2.0, y, *label, text_size, 7, "DEKIGATA", TextAlign::Center);
            }
        }
    }

    // 測点ヘッダー（V1, V2, ...）とデータ
    for (col, pt) in data.iter().enumerate() {
        let col_center_x = table_x + header_width + data_width * (col as f64 + 0.5);

        // 測点ラベル（最上行）
        let v_label = format!("V{}", col + 1);
        let header_y = table_y + total_height - row_height * 0.5;
        add_text(drawing, col_center_x, header_y, &v_label, text_size, 5, "DEKIGATA", TextAlign::Center);

        // 計画高（設計）
        let design_fh_y = table_y + total_height - row_height * 1.5;
        add_text(drawing, col_center_x, design_fh_y, &format!("{:.3}", pt.planned_height), text_size, 7, "DEKIGATA", TextAlign::Center);

        // 計画高（実測）- 空欄 + 括弧（スペース広め）
        let actual_fh_y = table_y + total_height - row_height * 2.5;
        add_text(drawing, col_center_x + data_width * 0.35, actual_fh_y, "(        )", text_size * 0.8, 7, "DEKIGATA", TextAlign::Right);

        // 切削高（設計）= 計画高 - 切削厚
        let design_cutting = pt.planned_height - DEKIGATA_DEFAULT_CUTTING_THICKNESS_MM / 1000.0;
        let design_cut_y = table_y + total_height - row_height * 3.5;
        add_text(drawing, col_center_x, design_cut_y, &format!("{:.3}", design_cutting), text_size, 7, "DEKIGATA", TextAlign::Center);

        // 切削高（実測）- 空欄 + 括弧（スペース広め）
        let actual_cut_y = table_y + total_height - row_height * 4.5;
        add_text(drawing, col_center_x + data_width * 0.35, actual_cut_y, "(        )", text_size * 0.8, 7, "DEKIGATA", TextAlign::Right);
    }
}

/// 単一測点の出来形管理用紙のDrawingを生成
pub fn generate_dekigata_drawing(section: &CrossSectionData, title_info: &TitleBlockInfo) -> Drawing {
    let mut drawing = new_drawing();

    // レイヤー作成
    drawing.add_layer(dxf::tables::Layer {
        name: "GROUND".to_string(),
        color: Color::from_index(7),
        ..Default::default()
    });
    drawing.add_layer(dxf::tables::Layer {
        name: "PLAN".to_string(),
        color: Color::from_index(1),
        ..Default::default()
    });
    drawing.add_layer(dxf::tables::Layer {
        name: "TEXT".to_string(),
        color: Color::from_index(7),
        ..Default::default()
    });
    drawing.add_layer(dxf::tables::Layer {
        name: "DIMENSION".to_string(),
        color: Color::from_index(8),
        ..Default::default()
    });
    drawing.add_layer(dxf::tables::Layer {
        name: "CUTTING".to_string(),
        color: Color::from_index(5),
        ..Default::default()
    });
    drawing.add_layer(dxf::tables::Layer {
        name: "DEKIGATA".to_string(),
        color: Color::from_index(7),
        ..Default::default()
    });

    let frame_scale = DEKIGATA_SCALE;
    const TEXT_SIZE_MM: f64 = 3.0;

    // 図枠を描画
    draw_drawing_frame(&mut drawing, title_info, 0.0, 0.0, frame_scale, TEXT_SIZE_MM);

    // 出来形管理用紙の内容を描画
    draw_dekigata_page(&mut drawing, section, 0.0, 0.0, frame_scale);

    drawing
}

/// 整数測点かどうかを判定（"+"を含む中間測点を除外）
fn is_integer_station(name: &str) -> bool {
    !name.contains('+')
}

/// 全測点の出来形管理用紙を縦並びで生成
/// 中間測点（No.1+10など）は除外
pub fn generate_all_dekigata_pages(
    all_sections: &[CrossSectionData],
    base_title_info: &TitleBlockInfo,
    drawing_number_base: &str,
) -> Drawing {
    let mut drawing = new_drawing();

    // レイヤー作成
    drawing.add_layer(dxf::tables::Layer {
        name: "GROUND".to_string(),
        color: Color::from_index(7),
        ..Default::default()
    });
    drawing.add_layer(dxf::tables::Layer {
        name: "PLAN".to_string(),
        color: Color::from_index(1),
        ..Default::default()
    });
    drawing.add_layer(dxf::tables::Layer {
        name: "TEXT".to_string(),
        color: Color::from_index(7),
        ..Default::default()
    });
    drawing.add_layer(dxf::tables::Layer {
        name: "DIMENSION".to_string(),
        color: Color::from_index(8),
        ..Default::default()
    });
    drawing.add_layer(dxf::tables::Layer {
        name: "CUTTING".to_string(),
        color: Color::from_index(5),
        ..Default::default()
    });
    drawing.add_layer(dxf::tables::Layer {
        name: "DEKIGATA".to_string(),
        color: Color::from_index(7),
        ..Default::default()
    });

    // 整数測点のみをフィルタ
    let filtered_sections: Vec<&CrossSectionData> = all_sections
        .iter()
        .filter(|s| is_integer_station(&s.survey_point_name))
        .collect();

    if filtered_sections.is_empty() {
        return drawing;
    }

    let frame_scale = DEKIGATA_SCALE;
    let page_height_dxf = 297.0 * frame_scale;  // A3高さ
    const PAGE_GAP_MM: f64 = 10.0;
    let page_gap_dxf = PAGE_GAP_MM * frame_scale;
    const TEXT_SIZE_MM: f64 = 3.0;

    let total_pages = filtered_sections.len();

    for (page_idx, section) in filtered_sections.iter().enumerate() {
        let page_offset_y = (page_idx as f64) * (page_height_dxf + page_gap_dxf);

        // 図面番号を更新
        let drawing_number = if drawing_number_base.is_empty() {
            format!("{}/{}", page_idx + 1, total_pages)
        } else {
            format!("{}-{}/{}", drawing_number_base, page_idx + 1, total_pages)
        };

        let info = base_title_info.clone()
            .with_top_title("出来形管理用紙")
            .with_scale(&format!("1:{} (A3)", DEKIGATA_SCALE as u32))
            .with_drawing_number(&drawing_number);

        // 図枠を描画（タイトル枠なし - 外枠と上部タイトルのみ）
        title_block::add_title_block_layer(&mut drawing);
        title_block::draw_outer_frame(&mut drawing, 0.0, page_offset_y, frame_scale);
        title_block::draw_top_title(&mut drawing, &info, 0.0, page_offset_y, frame_scale, TEXT_SIZE_MM);
        // draw_title_block は呼ばない（出来形管理用紙では不要）

        // 出来形管理用紙の内容を描画
        draw_dekigata_page(&mut drawing, section, 0.0, page_offset_y, frame_scale);
    }

    drawing
}

/// 全測点の出来形管理用紙をDXFバイト配列として生成
pub fn generate_all_dekigata_dxf_bytes(
    all_sections: &[CrossSectionData],
    base_title_info: &TitleBlockInfo,
    drawing_number_base: &str,
) -> Vec<u8> {
    let drawing = generate_all_dekigata_pages(all_sections, base_title_info, drawing_number_base);
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
    let text_height = 2250.0; // 基本テキスト高さ（×10）1.5倍拡大
    let label_width = 11250.0; // 左側のラベル幅（×10）1.5倍拡大
    let _title_height = 0.0; // タイトルなし（ピッチリ）

    let mut drawing = new_drawing();

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
    let row_height = 8250.0;  // 1.5倍拡大（5500 * 1.5）
    // 測点名行の高さ（最大文字数に基づいて計算）
    let max_name_len = points.iter().map(|p| p.3.chars().count()).max().unwrap_or(6);
    let station_row_height = (max_name_len as f64 * text_height * 0.73).max(row_height);

    // 範囲計算
    let min_dist = points.first().map(|p| p.0).unwrap_or(0.0);
    let max_dist = points.last().map(|p| p.0).unwrap_or(100.0);
    let min_elev = points.iter().map(|p| p.1.min(p.2)).fold(f64::MAX, f64::min);
    let max_elev = points.iter().map(|p| p.1.max(p.2)).fold(f64::MIN, f64::max);

    // DL（基準高）を1m単位で切り下げ（上下2mマージン）
    let dl = (min_elev - 2.0).floor();
    let graph_top = (max_elev + 2.0).ceil();

    // 左右マージン（最初と最後の測点が表枠境界から離れるように）
    let margin_x = 3000.0;  // 3m分のマージン（テキスト1.5倍拡大に対応）

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
        let label_color = if is_dl_row { 5 } else { 7 };  // DL行は青
        let label_size = if is_dl_row { text_height * 0.9 } else { text_height * 0.9 };  // DL行は半分サイズ
        add_text_rotated(&mut drawing, label_width - 1200.0, y, &label_text,
            label_size, label_color, "TEXT", TextAlign::Right, VerticalAlign::Bottom, 0.0);

        // 標高ラベル（右側）
        add_text_rotated(&mut drawing, label_width + graph_width + 1200.0, y, &format!("{:.0}", elev),
            text_height * 0.9, 7, "TEXT", TextAlign::Left, VerticalAlign::Bottom, 0.0);
        elev += grid_step;
    }

    // 縮尺比率と単位の注釈（DL付近、標高ラベルの左側に配置）
    let dl_y = to_dxf_y(dl);
    let annotation_x = 500.0;  // 左端寄り（×10）
    // 縮尺比率: V:H=5:1
    add_text_rotated(&mut drawing, annotation_x, dl_y + scale_y * 0.8,
        "V:H=5:1", text_height * 0.8, 5, "ANNOTATION", TextAlign::Left, VerticalAlign::Bottom, 0.0);
    // 単位 (m)
    add_text_rotated(&mut drawing, annotation_x, dl_y + scale_y * 1.3,
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

/// コンボビュー（展開図＋縦断図、ルート2のみ横断図も追加）を生成
/// 展開図を上部に、縦断図を下部に配置し、図枠内に収める
pub fn generate_combo_drawing(sections: &[CrossSectionData], columns: usize, column_gap: f64) -> Drawing {
    if sections.is_empty() {
        return new_drawing();
    }

    // ルートIDを判定（ルート2かどうか）
    let is_route_2 = sections.first().map(|s| s.route_id == "route_2").unwrap_or(false);

    // 面積展開図を生成し、バウンディングボックスを取得（コンボでは測点名非表示）
    let area_expansion = generate_area_expansion_drawing(sections, false);
    let (_area_min_x, area_min_y, _area_max_x, _area_max_y) = calc_dxf_bounds(&area_expansion);

    // 縦断図を生成し、バウンディングボックスを取得
    let longitudinal = generate_longitudinal_drawing(sections);
    let (_long_min_x, _long_min_y, long_max_x, long_max_y) = calc_dxf_bounds(&longitudinal);

    // 横断図を生成（ルート2のみ、5倍拡大、補完点を勾配補間）
    let cross_sections = if is_route_2 {
        let row_gap = 1.0;
        Some(generate_multi_drawing_scaled_interpolated(sections, columns, column_gap, row_gap, 5.0))
    } else {
        None
    };

    // 新しいDrawingを作成
    let mut drawing = new_drawing();

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
    if let Some(ref cs) = cross_sections {
        for layer in cs.layers() {
            if drawing.layers().find(|l| l.name == layer.name).is_none() {
                let mut new_layer = dxf::tables::Layer::default();
                new_layer.name = layer.name.clone();
                new_layer.color = layer.color.clone();
                drawing.add_layer(new_layer);
            }
        }
    }

    // === レイアウト計算 ===
    let spacing = 2000.0; // 2m間隔（測点名非表示のため縮小）

    // 縦断図を原点に配置
    for entity in longitudinal.entities() {
        drawing.add_entity(entity.clone());
    }

    // 面積展開図: 縦断図の上に配置
    let area_x_offset = 0.0_f64;
    let area_y_offset = long_max_y as f64 + spacing - area_min_y as f64;
    for entity in area_expansion.entities() {
        let mut shifted_entity = entity.clone();
        shift_entity_xy(&mut shifted_entity, area_x_offset, area_y_offset);
        drawing.add_entity(shifted_entity);
    }

    // 横断図配置前に縦断図＋展開図のバウンディングボックスを取得
    let (_left_min_x, left_min_y, _left_max_x, left_max_y) = calc_dxf_bounds(&drawing);
    let left_center_y = (left_min_y as f64 + left_max_y as f64) / 2.0;

    // 横断図: ルート2のみ、縦断図・展開図の右側に配置（縦方向は中央揃え）
    if let Some(ref cs) = cross_sections {
        let (cs_min_x, cs_min_y, _cs_max_x, cs_max_y) = calc_dxf_bounds(cs);
        let cs_center_y = (cs_min_y as f64 + cs_max_y as f64) / 2.0;
        let cs_x_offset = long_max_x as f64 + spacing * 2.0 - cs_min_x as f64;
        let cs_y_offset = left_center_y - cs_center_y;  // 縦断図＋展開図の中央に合わせる
        for entity in cs.entities() {
            let mut shifted_entity = entity.clone();
            shift_entity_xy(&mut shifted_entity, cs_x_offset, cs_y_offset);
            drawing.add_entity(shifted_entity);
        }
    }

    // 配置後の全体バウンディングボックスを計算
    let (data_min_x, data_min_y, data_max_x, data_max_y) = calc_dxf_bounds(&drawing);
    let data_center_x = (data_min_x as f64 + data_max_x as f64) / 2.0;
    let data_center_y = (data_min_y as f64 + data_max_y as f64) / 2.0;

    // 図枠スケール（ルート2は1:500、それ以外は1:700）
    let frame_scale = if is_route_2 { 500.0 } else { 700.0 };
    let frame_text_size = 3.0;

    // 図枠の内枠中心（A3: 420x297mm, マージン10mm）
    let frame_center_x = 210.0;
    let frame_center_y = 148.5;

    // 図枠の原点（図枠中心とデータ中心を一致させる）
    let frame_origin_x = data_center_x - frame_center_x * frame_scale;
    let frame_origin_y = data_center_y - frame_center_y * frame_scale;

    // ルート2のときはタイトルに横断図も含める、路線名も変更
    let (drawing_type, top_title, scale_text, route_name) = if is_route_2 {
        ("縦断図　横断図", "縦断図　横断図", "H=1:500 V=1:100", "東側取付道路")
    } else {
        ("縦断図", "縦断図", "H=1:700 V=1:140", "市道南千反畑第１号線　本線")
    };

    let frame_info = TitleBlockInfo::new()
        .with_project_name("市道 南千反畑町第１号線舗装補修工事")
        .with_drawing_type(drawing_type)
        .with_route_name(route_name)
        .with_date("2026年1月")
        .with_scale(scale_text)
        .with_drawing_number("1/1")
        .with_author("有限会社　三雄建設")
        .with_top_title(top_title)
        .with_credit("")
        .with_debug_markers(false);

    draw_drawing_frame(&mut drawing, &frame_info, frame_origin_x, frame_origin_y, frame_scale, frame_text_size);

    // ルート2の場合、横断図範囲に縮尺を表示
    if let Some(ref cs) = cross_sections {
        let (cs_min_x, cs_min_y, _cs_max_x, cs_max_y) = calc_dxf_bounds(cs);
        let cs_center_y = (cs_min_y as f64 + cs_max_y as f64) / 2.0;
        let cs_y_offset = left_center_y - cs_center_y;
        // 横断図の左下に縮尺表示（配置後の位置）
        let scale_label_x = long_max_x as f64 + spacing * 2.0;
        let scale_label_y = cs_min_y as f64 + cs_y_offset - frame_text_size * frame_scale;
        add_text(&mut drawing, scale_label_x, scale_label_y, "横断図 Scale H=1:100 V=1:200",
            frame_text_size * frame_scale, 7, "TEXT", TextAlign::Left);
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

/// 全ルートを縦に並べたコンボDXFを生成（図枠ごと）
pub fn generate_all_routes_combo_dxf_bytes(sections: &[CrossSectionData], columns: usize, column_gap: f64) -> Vec<u8> {
    use std::collections::BTreeSet;

    // ルートIDを収集（ソート済み）
    let route_ids: BTreeSet<String> = sections.iter().map(|s| s.route_id.clone()).collect();

    if route_ids.is_empty() {
        return Vec::new();
    }

    let mut combined_drawing = new_drawing();
    let spacing = 10000.0; // ルート間の垂直スペース（10m）
    let mut current_y_offset = 0.0;

    for route_id in route_ids {
        // このルートのセクションをフィルター
        let route_sections: Vec<CrossSectionData> = sections.iter()
            .filter(|s| s.route_id == route_id)
            .cloned()
            .collect();

        if route_sections.is_empty() {
            continue;
        }

        // このルートのコンボ図面を生成
        let route_drawing = generate_combo_drawing(&route_sections, columns, column_gap);
        let (_, min_y, _, max_y) = calc_dxf_bounds(&route_drawing);
        let route_height = max_y as f64 - min_y as f64;

        // レイヤーをマージ（最初のルートのみ）
        if current_y_offset == 0.0 {
            for layer in route_drawing.layers() {
                let mut new_layer = dxf::tables::Layer::default();
                new_layer.name = layer.name.clone();
                new_layer.color = layer.color.clone();
                combined_drawing.add_layer(new_layer);
            }
        }

        // エンティティをY方向にシフトして追加
        let y_shift = current_y_offset - min_y as f64;
        for entity in route_drawing.entities() {
            let mut shifted_entity = entity.clone();
            shift_entity_xy(&mut shifted_entity, 0.0, y_shift);
            combined_drawing.add_entity(shifted_entity);
        }

        // 次のルートのY位置を更新
        current_y_offset += route_height + spacing;
    }

    let mut output: Vec<u8> = Vec::new();
    combined_drawing.save(&mut output).expect("Failed to save DXF");
    output
}

// ============================================================================
// Area Expansion Drawing (面積展開図)
// ============================================================================

/// 面積展開図を生成
/// X軸: 縦断図と同じスケール（相対距離 * scale_x）
/// Y軸: 左幅員が正、右幅員が負（縦断図と同じscale_yで5:1）
pub fn generate_area_expansion_drawing(sections: &[CrossSectionData], show_station_names: bool) -> Drawing {
    // スケール設定（DXF単位: mm単位に統一、1m = 1000単位）
    let scale_x = 1000.0;     // 横方向スケール（1m = 1000単位）
    let scale_y = 1000.0;     // 縦方向スケール（1m = 1000単位）縦横比 H1:V1
    let text_height = 2250.0; // 基本テキスト高さ（×10）1.5倍拡大
    let label_width = 11250.0; // 左側ラベル幅（×10）1.5倍拡大
    let margin_x = 3000.0;    // 縦断図と同じマージン
    let station_text_offset = 4500.0; // 測点名のオフセット（×10）1.5倍拡大

    let mut drawing = new_drawing();

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
        let x = label_width + margin_x + (dist - base_distance) * scale_x;

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
        let text = format!("{:.1}", extension);
        add_text(&mut drawing, mid_x, text_height * 0.5, &text, text_height, 7, "TENKAI_DIM", TextAlign::Center);
    }

    // 幅員寸法（-90°回転、幅員線の外側）
    for station in &stations {
        let x = station.x;

        // 左幅員（上側外側に配置）
        if station.wl > 0.0 {
            let y_text = station.wl * scale_y + station_text_offset;
            let text = format!("{:.2}", station.wl);
            add_text_rotated(&mut drawing, x, y_text, &text, text_height,
                7, "TENKAI_DIM", TextAlign::Left, VerticalAlign::Top, -90.0);
        }

        // 右幅員（下側外側に配置）
        if station.wr > 0.0 {
            let y_text = -station.wr * scale_y - station_text_offset;
            let text = format!("{:.2}", station.wr);
            add_text_rotated(&mut drawing, x, y_text, &text, text_height,
                7, "TENKAI_DIM", TextAlign::Right, VerticalAlign::Top, -90.0);
        }
    }

    // 測点名（-90°回転、青色、上端オフセット）- オプションで非表示可能
    if show_station_names {
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
    }

    drawing
}

/// 面積展開図のDXFバイト列を生成
pub fn generate_area_expansion_dxf_bytes(sections: &[CrossSectionData]) -> Vec<u8> {
    let drawing = generate_area_expansion_drawing(sections, true);
    let mut output: Vec<u8> = Vec::new();
    drawing.save(&mut output).expect("Failed to save DXF");
    output
}

// ============================================================================
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
                // DXF text_heightはCAD用に補正済み（×1.364）なので、egui表示用に逆補正
                let font_size = (text.text_height as f32 / font_metrics::SCALE_FOR_CAP_HEIGHT as f32 * view.zoom).max(1.0);
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
    /// 路線の起点かどうか（補間しない）
    #[serde(default)]
    pub is_route_start: bool,
    /// 路線の終点かどうか（補間しない）
    #[serde(default)]
    pub is_route_end: bool,
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
            is_route_start: false,
            is_route_end: false,
        }
    }


    pub fn from_json(json: &str) -> Result<Vec<Self>, String> {
        let mut sections: Vec<Self> = serde_json::from_str(json)
            .map_err(|e| format!("JSON parse error: {e}"))?;
        Self::set_route_start_end_flags(&mut sections);
        Ok(sections)
    }

    /// 新形式JSON（タイトルブロック情報付き）をパース
    pub fn from_json_with_title(json: &str) -> Result<SectionsData, String> {
        // まず新形式（オブジェクト）でパースを試みる
        if let Ok(mut data) = serde_json::from_str::<SectionsData>(json) {
            Self::set_route_start_end_flags(&mut data.sections);
            return Ok(data);
        }
        // 旧形式（配列）でパース
        let mut sections: Vec<Self> = serde_json::from_str(json)
            .map_err(|e| format!("JSON parse error: {e}"))?;
        Self::set_route_start_end_flags(&mut sections);
        Ok(SectionsData {
            title_block: None,
            sections,
        })
    }

    /// 各ルートごとに起終点フラグを設定
    fn set_route_start_end_flags(sections: &mut [Self]) {
        use std::collections::HashMap;

        // 各ルートの最初と最後のインデックスを収集
        let mut route_indices: HashMap<String, (usize, usize)> = HashMap::new();
        for (i, section) in sections.iter().enumerate() {
            let route_id = &section.route_id;
            route_indices
                .entry(route_id.clone())
                .and_modify(|(_, last)| *last = i)
                .or_insert((i, i));
        }

        // フラグを設定
        for (first, last) in route_indices.values() {
            sections[*first].is_route_start = true;
            sections[*last].is_route_end = true;
        }
    }

    /// 補完点の計画高を採用点間の勾配から線形補間する
    /// - 採用点: V1（左端）、CL（センター）、Vn（右端）は固定
    /// - 補完点: V2, V4等を勾配で補間
    pub fn interpolate_planned_heights(&mut self) {
        let n = self.survey_data.len();
        if n < 3 { return; }

        let cl = self.cl_index.min(n - 1);

        // 固定点の値を取得
        let fh_left = self.survey_data[0].planned_height;
        let fh_center = self.survey_data[cl].planned_height;
        let fh_right = self.survey_data[n - 1].planned_height;

        let dist_left = self.survey_data[0].cumulative_distance;
        let dist_center = self.survey_data[cl].cumulative_distance;
        let dist_right = self.survey_data[n - 1].cumulative_distance;

        // 左側（V1〜CL間）の補間
        if (dist_center - dist_left).abs() > 1e-9 {
            for i in 1..cl {
                // 舗装厚を保持（planned_height更新前に取得）
                let pavement_thickness = self.survey_data[i].pavement_thickness();

                let d = self.survey_data[i].cumulative_distance;
                let t = (d - dist_left) / (dist_center - dist_left);
                self.survey_data[i].planned_height = fh_left + t * (fh_center - fh_left);
                // cutting_bottomも再計算（舗装厚を維持）
                self.survey_data[i].cutting_bottom = self.survey_data[i].planned_height - pavement_thickness;
            }
        }

        // 右側（CL〜Vn間）の補間
        if (dist_right - dist_center).abs() > 1e-9 {
            for i in (cl + 1)..(n - 1) {
                // 舗装厚を保持（planned_height更新前に取得）
                let pavement_thickness = self.survey_data[i].pavement_thickness();

                let d = self.survey_data[i].cumulative_distance;
                let t = (d - dist_center) / (dist_right - dist_center);
                self.survey_data[i].planned_height = fh_center + t * (fh_right - fh_center);
                // cutting_bottomも再計算（舗装厚を維持）
                self.survey_data[i].cutting_bottom = self.survey_data[i].planned_height - pavement_thickness;
            }
        }
    }
}

/// タイトルブロック情報（JSON用）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TitleBlockJson {
    #[serde(default)]
    pub project_name: String,
    #[serde(default)]
    pub drawing_type: String,
    #[serde(default)]
    pub route_name: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub drawing_number: String,
}

/// セクションデータ（タイトルブロック情報付き）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionsData {
    #[serde(default)]
    pub title_block: Option<TitleBlockJson>,
    pub sections: Vec<CrossSectionData>,
}

impl CrossSectionData {
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
            is_route_start: false,
            is_route_end: false,
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
    Dekigata,    // 出来形管理用紙
    AlignTest,   // アライメントテスト
    TitleBlock,  // タイトル枠テスト
}

/// 図枠スケール（None=手動、Some(200)=1:200、Some(500)=1:500）
type PlotScale = Option<u32>;

pub struct CrossSectionApp {
    sections: Vec<CrossSectionData>,
    selected_index: Option<usize>,
    dxf_drawing: Option<Drawing>,
    dxf_view_state: DxfViewState,
    view_mode: ViewMode,
    grid_columns: usize,         // グリッドの列数
    column_gap: f64,             // 列間隔（メートル単位）
    row_gap: f64,                // 行間隔（メートル単位）
    plot_scale: PlotScale,       // 図枠スケール（None=手動、Some(200)=1:200等）
    max_columns: usize,          // 現在のスケールで収まる最大列数
    fits_in_frame: bool,         // 現在の設定で図枠に収まるか
    status_message: Option<String>,
    needs_fit: bool,             // canvas_rect更新後にfit_to_dxfを呼ぶフラグ
    fit_bounds: Option<(f32, f32, f32, f32)>,  // フィット先のバウンディングボックス（Singleモード用）
    is_first_frame: bool,        // 初回フレームフラグ（モバイル判定用）
    loading_frame: usize,        // ローディングアニメーション用フレームカウンタ
    loading_stage: String,       // ローディング進捗メッセージ
    routes: Vec<String>,         // 利用可能なルートのリスト
    selected_route: String,      // 選択中のルート
    api_key: String,             // セッションのみのAPIキー
    api_key_set: bool,           // UI表示用フラグ
    // タイトルブロック情報
    project_name: String,        // 工事名
    drawing_type: String,        // 図面名
    route_name: String,          // 路線名
    author: String,              // 施工者
    date: String,                // 作成日
    drawing_number: String,      // 図面番号
    show_debug_guides: bool,     // デバッグガイド表示
    // ページ分割
    current_page: usize,         // 現在のページ（0始まり）
    total_pages: usize,          // 総ページ数
    sections_per_page: usize,    // 1ページあたりのセクション数
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
            row_gap: 1.0,     // 行間隔1メートル
            plot_scale: None,  // デフォルトは手動列数指定
            max_columns: 99,   // 初期値（スケール未選択時）
            fits_in_frame: true,
            status_message: None,
            needs_fit: false,
            fit_bounds: None,
            is_first_frame: true,
            loading_frame: 0,
            loading_stage: "JSONデータを取得中".to_string(),
            routes: vec!["route_1".to_string()],
            selected_route: "route_1".to_string(),
            api_key: String::new(),
            api_key_set: false,
            // タイトルブロック情報（デフォルト）
            project_name: String::new(),
            drawing_type: "横断図".to_string(),
            route_name: String::new(),
            author: String::new(),
            date: String::new(),
            drawing_number: String::new(),
            show_debug_guides: true,  // デフォルトでON
            // ページ分割
            current_page: 0,
            total_pages: 1,
            sections_per_page: 6,  // デフォルト6個/ページ
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

    /// 出来形管理用紙用: 整数測点のみをフィルタ（「+」を含む中間測点は除外）
    fn dekigata_filtered_sections(&self) -> Vec<&CrossSectionData> {
        self.sections.iter()
            .filter(|s| s.route_id == self.selected_route)
            .filter(|s| !s.survey_point_name.contains('+'))
            .collect()
    }

    /// 現在のページに表示するセクションを返す
    fn current_page_sections(&self) -> Vec<&CrossSectionData> {
        let filtered = self.filtered_sections();
        if self.total_pages <= 1 || self.sections_per_page == 0 {
            return filtered;
        }
        let start = self.current_page * self.sections_per_page;
        let end = (start + self.sections_per_page).min(filtered.len());
        filtered[start..end].to_vec()
    }

    /// ページ数を再計算
    fn recalc_pages(&mut self) {
        let filtered: Vec<CrossSectionData> = self.filtered_sections()
            .into_iter().cloned().collect();

        if filtered.is_empty() {
            self.sections_per_page = 0;
            self.total_pages = 1;
            self.current_page = 0;
            return;
        }

        // スケールが指定されている場合のみページ分割
        if let Some(scale) = self.plot_scale {
            self.sections_per_page = calc_sections_per_page(
                &filtered,
                self.grid_columns,
                self.column_gap,
                self.row_gap,
                scale as f64,
            );

            if self.sections_per_page > 0 {
                self.total_pages = (filtered.len() + self.sections_per_page - 1) / self.sections_per_page;
            } else {
                self.total_pages = 1;
            }

            // 現在のページが範囲外なら調整
            if self.current_page >= self.total_pages {
                self.current_page = self.total_pages.saturating_sub(1);
            }
        } else {
            // 手動モードはページ分割なし
            self.sections_per_page = filtered.len();
            self.total_pages = 1;
            self.current_page = 0;
        }
    }

    fn update_dxf_preview(&mut self) {
        let filtered: Vec<CrossSectionData> = self.filtered_sections()
            .into_iter().cloned().collect();

        // fit_boundsをリセット（Singleモード以外はNone）
        self.fit_bounds = None;

        // 最大列数はセクション数（それ以上は意味がない）
        if !filtered.is_empty() {
            self.max_columns = filtered.len();
            // 列数がセクション数を超えていたら調整
            if self.grid_columns > self.max_columns {
                self.grid_columns = self.max_columns;
            }
        }

        // 図枠スケール選択時は収まるかチェック
        if let Some(scale) = self.plot_scale {
            if !filtered.is_empty() {
                if let Some(bounds) = calc_grid_bounds(&filtered, self.grid_columns, self.column_gap, self.row_gap) {
                    self.fits_in_frame = bounds.fits_in_frame(scale as f64, self.grid_columns);
                } else {
                    self.fits_in_frame = true;
                }
            }
        } else {
            self.fits_in_frame = true;
        }

        let columns = self.grid_columns;

        let drawing = match self.view_mode {
            ViewMode::AlignTest => {
                generate_alignment_test_drawing()
            }
            ViewMode::TitleBlock => {
                generate_title_block_test_drawing()
            }
            ViewMode::Combo if !filtered.is_empty() => {
                generate_combo_drawing(&filtered, columns, self.column_gap)
            }
            ViewMode::AllGrid if !filtered.is_empty() => {
                if let Some(scale) = self.plot_scale {
                    // 現在のページのセクションのみ使用
                    let page_sections: Vec<CrossSectionData> = self.current_page_sections()
                        .into_iter().cloned().collect();
                    // 図枠付きで描画
                    let page_num = if self.total_pages > 1 {
                        format!("{}-{}", self.drawing_number, self.current_page + 1)
                    } else {
                        self.drawing_number.clone()
                    };
                    let info = TitleBlockInfo::new()
                        .with_project_name(&self.project_name)
                        .with_drawing_type(&self.drawing_type)
                        .with_route_name(&self.route_name)
                        .with_author(&self.author)
                        .with_date(&self.date)
                        .with_drawing_number(&page_num)
                        .with_top_title(&self.drawing_type)
                        .with_scale(&format!("1:{} (A3)", scale))
                        .with_debug_markers(self.show_debug_guides);
                    generate_multi_drawing_with_frame_at_scale_interpolated(&page_sections, columns, self.column_gap, self.row_gap, &info, scale as f64)
                } else {
                    generate_multi_drawing_interpolated(&filtered, columns, self.column_gap, self.row_gap)
                }
            }
            ViewMode::Longitudinal if !filtered.is_empty() => {
                generate_longitudinal_drawing(&filtered)
            }
            ViewMode::AreaExpansion if !filtered.is_empty() => {
                generate_area_expansion_drawing(&filtered, true)
            }
            ViewMode::Dekigata => {
                // 整数測点のみフィルタ（中間測点は除外）
                let dekigata_sections: Vec<CrossSectionData> = self.dekigata_filtered_sections()
                    .into_iter().cloned().collect();
                if !dekigata_sections.is_empty() {
                    // selected_index を dekigata_sections 内のインデックスにマッピング
                    let display_idx = self.selected_index
                        .and_then(|sel_idx| {
                            // 全セクションの中から選択されたセクションを取得
                            self.sections.get(sel_idx)
                                .and_then(|selected| {
                                    // dekigata_sections 内での位置を探す
                                    dekigata_sections.iter().position(|s|
                                        s.survey_point_name == selected.survey_point_name
                                    )
                                })
                        })
                        .unwrap_or(0);
                    let info = TitleBlockInfo::new()
                        .with_project_name(&self.project_name)
                        .with_drawing_type("出来形管理用紙")
                        .with_route_name(&self.route_name)
                        .with_author(&self.author)
                        .with_date(&self.date)
                        .with_top_title("出来形管理用紙")
                        .with_scale("1:50 (A3)")
                        .with_debug_markers(self.show_debug_guides);
                    // プレビュー用: 選択された測点を表示
                    generate_dekigata_drawing(&dekigata_sections[display_idx], &info)
                } else {
                    return;
                }
            }
            ViewMode::Single if !filtered.is_empty() => {
                // 全横断図を生成し、選択した横断図の位置にフィット
                if let Some(idx) = self.selected_index {
                    self.fit_bounds = calc_section_bounds_in_grid(
                        &filtered, idx, columns, self.column_gap, self.row_gap
                    );
                }
                // 補間を適用（起終点以外の補完点を勾配補間）
                generate_multi_drawing_interpolated(&filtered, columns, self.column_gap, self.row_gap)
            }
            _ => { return; }
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
            match CrossSectionData::from_json_with_title(&json_text) {
                Ok(data) => {
                    let sections = data.sections;
                    // タイトルブロック情報を適用
                    if let Some(tb) = data.title_block {
                        if !tb.project_name.is_empty() { self.project_name = tb.project_name; }
                        if !tb.drawing_type.is_empty() { self.drawing_type = tb.drawing_type; }
                        if !tb.route_name.is_empty() { self.route_name = tb.route_name; }
                        if !tb.author.is_empty() { self.author = tb.author; }
                        if !tb.date.is_empty() { self.date = tb.date; }
                        if !tb.drawing_number.is_empty() { self.drawing_number = tb.drawing_number; }
                    }

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
                    self.recalc_pages();
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
                            // 出来形モード時はモード維持
                            if self.view_mode != ViewMode::Dekigata {
                                self.view_mode = ViewMode::Single;
                            }
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
                            ViewMode::Dekigata => "出来形",
                            ViewMode::AlignTest => "テスト",
                            ViewMode::TitleBlock => "図枠",
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
                                if ui.selectable_label(self.view_mode == ViewMode::Dekigata, "出来形管理用紙").clicked() {
                                    selected = Some(ViewMode::Dekigata);
                                }
                                if ui.selectable_label(self.view_mode == ViewMode::AlignTest, "アライメントテスト").clicked() {
                                    selected = Some(ViewMode::AlignTest);
                                }
                                if ui.selectable_label(self.view_mode == ViewMode::TitleBlock, "タイトル枠").clicked() {
                                    selected = Some(ViewMode::TitleBlock);
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
                            // 図枠スケール選択
                            let scale_label = match self.plot_scale {
                                None => "手動".to_string(),
                                Some(s) => format!("1:{}", s),
                            };
                            egui::ComboBox::from_id_salt("scale_select_mobile")
                                .selected_text(scale_label)
                                .width(60.0)
                                .show_ui(ui, |ui| {
                                    if ui.selectable_label(self.plot_scale.is_none(), "手動").clicked() {
                                        self.plot_scale = None;
                                        self.recalc_pages();
                                        self.update_dxf_preview();
                                    }
                                    if ui.selectable_label(self.plot_scale == Some(100), "1:100").clicked() {
                                        self.plot_scale = Some(100);
                                        self.recalc_pages();
                                        self.update_dxf_preview();
                                    }
                                    if ui.selectable_label(self.plot_scale == Some(200), "1:200").clicked() {
                                        self.plot_scale = Some(200);
                                        self.recalc_pages();
                                        self.update_dxf_preview();
                                    }
                                    if ui.selectable_label(self.plot_scale == Some(500), "1:500").clicked() {
                                        self.plot_scale = Some(500);
                                        self.recalc_pages();
                                        self.update_dxf_preview();
                                    }
                                });

                            // 列数選択（セクション数まで）
                            ui.label(format!("{}列", self.grid_columns));
                            if ui.small_button("+").clicked() && self.grid_columns < self.max_columns {
                                self.grid_columns += 1;
                                self.recalc_pages();
                                self.update_dxf_preview();
                            }
                            if ui.small_button("-").clicked() && self.grid_columns > 1 {
                                self.grid_columns -= 1;
                                self.recalc_pages();
                                self.update_dxf_preview();
                            }

                        });
                        ui.horizontal_wrapped(|ui| {
                            // 列間隔
                            ui.label(format!("列{:.1}m", self.column_gap));
                            if ui.small_button("+").clicked() && self.column_gap < 5.0 {
                                self.column_gap += 0.5;
                                self.recalc_pages();
                                self.update_dxf_preview();
                            }
                            if ui.small_button("-").clicked() && self.column_gap > -5.0 {
                                self.column_gap -= 0.5;
                                self.recalc_pages();
                                self.update_dxf_preview();
                            }
                            ui.separator();
                            // 行間隔
                            ui.label(format!("行{:.1}m", self.row_gap));
                            if ui.small_button("+").clicked() && self.row_gap < 3.0 {
                                self.row_gap += 0.5;
                                self.recalc_pages();
                                self.update_dxf_preview();
                            }
                            if ui.small_button("-").clicked() && self.row_gap > 0.0 {
                                self.row_gap -= 0.5;
                                self.recalc_pages();
                                self.update_dxf_preview();
                            }
                            // デバッグガイド表示切替
                            if self.plot_scale.is_some() {
                                if ui.checkbox(&mut self.show_debug_guides, "ガイド").changed() {
                                    self.update_dxf_preview();
                                }
                            }
                        });
                        // ページナビゲーション（複数ページある場合のみ）
                        if self.total_pages > 1 && self.plot_scale.is_some() {
                            ui.horizontal_wrapped(|ui| {
                                if ui.small_button("<").clicked() && self.current_page > 0 {
                                    self.current_page -= 1;
                                    self.update_dxf_preview();
                                }
                                ui.label(format!("{}/{}", self.current_page + 1, self.total_pages));
                                if ui.small_button(">").clicked() && self.current_page < self.total_pages - 1 {
                                    self.current_page += 1;
                                    self.update_dxf_preview();
                                }
                            });
                        }
                    }

                    ui.horizontal_wrapped(|ui| {
                        // DXFダウンロード（フィルター済みセクションを使用）
                        let filtered_for_dxf: Vec<CrossSectionData> = self.filtered_sections()
                            .into_iter().cloned().collect();
                        match self.view_mode {
                            ViewMode::Combo if !filtered_for_dxf.is_empty() => {
                                if ui.button("DXF").clicked() {
                                    // 全ルートを図枠ごと縦に並べたDXFを生成
                                    let dxf_content = generate_all_routes_combo_dxf_bytes(&self.sections, self.grid_columns, self.column_gap);
                                    download_file("combo.dxf", &dxf_content);
                                }
                            }
                            ViewMode::AllGrid if !filtered_for_dxf.is_empty() => {
                                if ui.button("DXF").clicked() {
                                    let (dxf_content, filename) = if let Some(scale) = self.plot_scale {
                                        // 全ページを1つのDXFに垂直配置
                                        let info = TitleBlockInfo::new()
                                            .with_project_name(&self.project_name)
                                            .with_drawing_type(&self.drawing_type)
                                            .with_route_name(&self.route_name)
                                            .with_author(&self.author)
                                            .with_date(&self.date)
                                            .with_top_title(&self.drawing_type)
                                            .with_scale(&format!("1:{} (A3)", scale))
                                            .with_debug_markers(false);
                                        (generate_all_pages_dxf_bytes(&filtered_for_dxf, self.grid_columns, self.column_gap, self.row_gap, scale as f64, &info, &self.drawing_number), "cross_sections.dxf".to_string())
                                    } else {
                                        (generate_multi_dxf_bytes(&filtered_for_dxf, self.grid_columns, self.column_gap, self.row_gap), "cross_sections_all.dxf".to_string())
                                    };
                                    download_file(&filename, &dxf_content);
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
                            ViewMode::Dekigata => {
                                // 整数測点のみフィルタ（中間測点は除外）
                                let dekigata_sections: Vec<CrossSectionData> = self.dekigata_filtered_sections()
                                    .into_iter().cloned().collect();
                                if !dekigata_sections.is_empty() && ui.button("DXF").clicked() {
                                    let info = TitleBlockInfo::new()
                                        .with_project_name(&self.project_name)
                                        .with_drawing_type("出来形管理用紙")
                                        .with_route_name(&self.route_name)
                                        .with_author(&self.author)
                                        .with_date(&self.date)
                                        .with_top_title("出来形管理用紙")
                                        .with_scale("1:50 (A3)")
                                        .with_debug_markers(false);
                                    let dxf_content = generate_all_dekigata_dxf_bytes(&dekigata_sections, &info, &self.drawing_number);
                                    download_file("dekigata.dxf", &dxf_content);
                                }
                            }
                            ViewMode::Single => {
                                if self.selected_index.is_some() && ui.button("DXF").clicked() {
                                    if let Some(idx) = self.selected_index {
                                        if let Some(section) = filtered_for_dxf.get(idx) {
                                            // 起終点以外は補間を適用
                                            let is_start_or_end = section.is_route_start || section.is_route_end;
                                            let mut section = section.clone();
                                            if !is_start_or_end {
                                                section.interpolate_planned_heights();
                                            }
                                            let dxf_content = generate_dxf_bytes(&section);
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
                            ViewMode::TitleBlock => {
                                if ui.button("DXF").clicked() {
                                    let dxf_content = generate_title_block_dxf_for_download();
                                    download_file("title_block.dxf", &dxf_content);
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
                    if ui.selectable_label(self.view_mode == ViewMode::Dekigata, "出来形").clicked() {
                        self.view_mode = ViewMode::Dekigata;
                        self.update_dxf_preview();
                    }
                    if ui.selectable_label(self.view_mode == ViewMode::TitleBlock, "図枠").clicked() {
                        self.view_mode = ViewMode::TitleBlock;
                        self.update_dxf_preview();
                    }
                });

                // AllGrid/Comboモード時の列数・間隔調整
                if self.view_mode == ViewMode::AllGrid || self.view_mode == ViewMode::Combo {
                    ui.horizontal(|ui| {
                        // 図枠スケール選択
                        let scale_label = match self.plot_scale {
                            None => "手動".to_string(),
                            Some(s) => format!("1:{}", s),
                        };
                        egui::ComboBox::from_id_salt("scale_select_desktop")
                            .selected_text(scale_label)
                            .width(70.0)
                            .show_ui(ui, |ui| {
                                if ui.selectable_label(self.plot_scale.is_none(), "手動").clicked() {
                                    self.plot_scale = None;
                                    self.recalc_pages();
                                    self.update_dxf_preview();
                                }
                                if ui.selectable_label(self.plot_scale == Some(100), "1:100").clicked() {
                                    self.plot_scale = Some(100);
                                    self.recalc_pages();
                                    self.update_dxf_preview();
                                }
                                if ui.selectable_label(self.plot_scale == Some(200), "1:200").clicked() {
                                    self.plot_scale = Some(200);
                                    self.recalc_pages();
                                    self.update_dxf_preview();
                                }
                                if ui.selectable_label(self.plot_scale == Some(500), "1:500").clicked() {
                                    self.plot_scale = Some(500);
                                    self.recalc_pages();
                                    self.update_dxf_preview();
                                }
                            });

                        // 列数選択（セクション数まで）
                        ui.label(format!("{}列", self.grid_columns));
                        if ui.small_button("+").clicked() && self.grid_columns < self.max_columns {
                            self.grid_columns += 1;
                            self.recalc_pages();
                            self.update_dxf_preview();
                        }
                        if ui.small_button("-").clicked() && self.grid_columns > 1 {
                            self.grid_columns -= 1;
                            self.recalc_pages();
                            self.update_dxf_preview();
                        }
                    });
                    ui.horizontal(|ui| {
                        // 列間隔
                        ui.label(format!("列{:.1}m", self.column_gap));
                        if ui.small_button("+").clicked() && self.column_gap < 5.0 {
                            self.column_gap += 0.5;
                            self.recalc_pages();
                            self.update_dxf_preview();
                        }
                        if ui.small_button("-").clicked() && self.column_gap > -5.0 {
                            self.column_gap -= 0.5;
                            self.recalc_pages();
                            self.update_dxf_preview();
                        }
                        ui.separator();
                        // 行間隔
                        ui.label(format!("行{:.1}m", self.row_gap));
                        if ui.small_button("+").clicked() && self.row_gap < 3.0 {
                            self.row_gap += 0.5;
                            self.recalc_pages();
                            self.update_dxf_preview();
                        }
                        if ui.small_button("-").clicked() && self.row_gap > 0.0 {
                            self.row_gap -= 0.5;
                            self.recalc_pages();
                            self.update_dxf_preview();
                        }
                        // デバッグガイド表示切替
                        if self.plot_scale.is_some() {
                            if ui.checkbox(&mut self.show_debug_guides, "ガイド").changed() {
                                self.update_dxf_preview();
                            }
                        }
                        // ページナビゲーション（複数ページある場合のみ）
                        if self.total_pages > 1 && self.plot_scale.is_some() {
                            ui.separator();
                            if ui.small_button("<").clicked() && self.current_page > 0 {
                                self.current_page -= 1;
                                self.update_dxf_preview();
                            }
                            ui.label(format!("{}/{}", self.current_page + 1, self.total_pages));
                            if ui.small_button(">").clicked() && self.current_page < self.total_pages - 1 {
                                self.current_page += 1;
                                self.update_dxf_preview();
                            }
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
                    ViewMode::TitleBlock => {
                        if ui.button("Download Title Block DXF").clicked() {
                            let dxf_content = generate_title_block_dxf_for_download();
                            download_file("title_block.dxf", &dxf_content);
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
                            let (dxf_content, filename) = if let Some(scale) = self.plot_scale {
                                // 全ページを1つのDXFに垂直配置
                                let info = TitleBlockInfo::new()
                                    .with_project_name(&self.project_name)
                                    .with_drawing_type(&self.drawing_type)
                                    .with_route_name(&self.route_name)
                                    .with_author(&self.author)
                                    .with_date(&self.date)
                                    .with_top_title(&self.drawing_type)
                                    .with_scale(&format!("1:{} (A3)", scale))
                                    .with_debug_markers(false);
                                (generate_all_pages_dxf_bytes(&filtered_for_download, self.grid_columns, self.column_gap, self.row_gap, scale as f64, &info, &self.drawing_number), "cross_sections.dxf".to_string())
                            } else {
                                (generate_multi_dxf_bytes(&filtered_for_download, self.grid_columns, self.column_gap, self.row_gap), "cross_sections_all.dxf".to_string())
                            };
                            download_file(&filename, &dxf_content);
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
                    ViewMode::Dekigata => {
                        // 整数測点のみフィルタ（中間測点は除外）
                        let dekigata_sections: Vec<CrossSectionData> = self.dekigata_filtered_sections()
                            .into_iter().cloned().collect();
                        if !dekigata_sections.is_empty() && ui.button("Download 出来形 DXF").clicked() {
                            let info = TitleBlockInfo::new()
                                .with_project_name(&self.project_name)
                                .with_drawing_type("出来形管理用紙")
                                .with_route_name(&self.route_name)
                                .with_author(&self.author)
                                .with_date(&self.date)
                                .with_top_title("出来形管理用紙")
                                .with_scale("1:50 (A3)")
                                .with_debug_markers(false);
                            let dxf_content = generate_all_dekigata_dxf_bytes(&dekigata_sections, &info, &self.drawing_number);
                            download_file("dekigata.dxf", &dxf_content);
                        }
                    }
                    ViewMode::Single => {
                        if let Some(idx) = self.selected_index {
                            if let Some(section) = filtered_for_download.get(idx) {
                                if ui.button("Download DXF").clicked() {
                                    // 起終点以外は補間を適用
                                    let is_start_or_end = section.is_route_start || section.is_route_end;
                                    let mut section = section.clone();
                                    if !is_start_or_end {
                                        section.interpolate_planned_heights();
                                    }
                                    let dxf_content = generate_dxf_bytes(&section);
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
                // 出来形モードでは整数測点のみ表示
                let filtered_names: Vec<String> = if self.view_mode == ViewMode::Dekigata {
                    self.dekigata_filtered_sections()
                        .iter().map(|s| s.survey_point_name.clone()).collect()
                } else {
                    self.filtered_sections()
                        .iter().map(|s| s.survey_point_name.clone()).collect()
                };
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
                    ViewMode::Dekigata => {
                        let dekigata_count = self.dekigata_filtered_sections().len();
                        ui.label(format!("出来形管理用紙 ({}測点/整数のみ)", dekigata_count));
                    }
                    ViewMode::AlignTest => {
                        ui.label("アライメントテスト");
                    }
                    ViewMode::TitleBlock => {
                        ui.label("タイトル枠テスト");
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
                    // 出来形モード時はモード維持
                    if self.view_mode != ViewMode::Dekigata {
                        self.view_mode = ViewMode::Single;
                    }
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
                // Singleモードでfit_boundsがある場合は選択した横断図にフィット
                if let Some((min_x, min_y, max_x, max_y)) = self.fit_bounds {
                    self.dxf_view_state.fit_to_dxf(min_x, min_y, max_x, max_y);
                } else if let Some(ref drawing) = self.dxf_drawing {
                    // それ以外は全体にフィット
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_section(name: &str) -> CrossSectionData {
        CrossSectionData {
            survey_point_name: name.to_string(),
            dl: 10.0,
            cl_index: 1,
            l_to_cl_distance: 3.0,
            survey_data: vec![
                SurveyRow {
                    unit_distance: 0.0,
                    elevation: 11.0,
                    planned_height: 11.0,
                    cumulative_distance: -3.0,  // 左端
                    cutting_bottom: 10.95,
                },
                SurveyRow {
                    unit_distance: 3.0,
                    elevation: 11.0,
                    planned_height: 11.0,
                    cumulative_distance: 0.0,  // CL
                    cutting_bottom: 10.95,
                },
                SurveyRow {
                    unit_distance: 3.0,
                    elevation: 11.0,
                    planned_height: 11.0,
                    cumulative_distance: 3.0,  // 右端
                    cutting_bottom: 10.95,
                },
            ],
            route_distance: None,
            route_id: "route_1".to_string(),
        }
    }

    #[test]
    fn test_calc_grid_bounds() {
        // 1セクションのバウンディングボックスを確認
        let sections = vec![make_test_section("NO.1")];

        let bounds = calc_grid_bounds(&sections, 1, 2.0, 1.0).unwrap();

        // 幅は6000 (左3m + 右3m = 6m × 1000)
        // X位置はグリッド配置後の値（CL位置 + 累積距離）
        assert!((bounds.width() - 6000.0).abs() < 1.0);

        // 高さの確認（旗揚げを含む）
        assert!(bounds.height() > 0.0);
    }

    #[test]
    fn test_calc_max_columns_for_frame() {
        // 空のセクションリストは1列を返す
        let empty: Vec<CrossSectionData> = vec![];
        assert_eq!(calc_max_columns_for_frame(&empty, 2.0, 1.0, 500.0), 1);

        // 20セクションのテストデータ
        let sections: Vec<CrossSectionData> = (1..=20)
            .map(|i| make_test_section(&format!("NO.{}", i)))
            .collect();

        // 1:500スケールで最大列数を計算
        // 各セクションのサイズ（幅8m × 高さ約5m紙上）を考慮
        let max_cols = calc_max_columns_for_frame(&sections, 2.0, 1.0, 500.0);

        // 20セクションあるので、少なくとも1列以上、最大20列以下
        assert!(max_cols >= 1);
        assert!(max_cols <= 20);

        // 紙上サイズで検証
        // 各セル幅: 8m = 8000 DXF単位 → 1:500で16mm
        // 有効幅: 300mm → 最大約18列程度（実際は高さ制約もある）
    }

    #[test]
    fn test_fits_in_frame() {
        let sections = vec![make_test_section("NO.1")];
        let bounds = calc_grid_bounds(&sections, 1, 2.0, 1.0).unwrap();

        // 1:500で収まるか確認（1列なので高さ制限は241mm）
        // 幅6000→12mm、高さは5550くらい→11mm程度
        assert!(bounds.fits_in_frame(500.0, 1));

        // 1:200でも収まる
        // 幅6000→30mm、高さ5550→27.75mm
        assert!(bounds.fits_in_frame(200.0, 1));

        // 4列の場合は高さ制限が203mmになる
        // 1列でテストしているので4列制限は適用されない
        assert!(bounds.fits_in_frame(500.0, 4));  // 4列でも収まる

        // 1:50だと収まらない（幅120mm、高さ111mm）
        // 有効高さ227mmあるので収まる可能性
        // 実際のテストはデータ次第
    }
}
