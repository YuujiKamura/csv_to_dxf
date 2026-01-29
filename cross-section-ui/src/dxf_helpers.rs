//! DXF生成ヘルパー関数

use dxf::Drawing;
use dxf::entities::{Entity, EntityType, Line, Text, Wipeout};
use dxf::enums::{HorizontalTextJustification, VerticalTextJustification};
use dxf::{Color, Point};

use crate::drawing_ir::{DrawingContent, HAlign, VAlign, SectionStyle, cap_height_to_dxf_height};

// 後方互換性のためのエイリアス
type TextAlign = HAlign;

/// LINE追加ヘルパー
pub fn add_line(drawing: &mut Drawing, x1: f64, y1: f64, x2: f64, y2: f64, color: i16, layer: &str) {
    let mut line = Line::default();
    line.p1 = Point::new(x1, y1, 0.0);
    line.p2 = Point::new(x2, y2, 0.0);
    let mut entity = Entity::new(EntityType::Line(line));
    entity.common.layer = layer.to_string();
    entity.common.color = Color::from_index(color as u8);
    drawing.add_entity(entity);
}

/// Drawing初期化（共通設定：バージョン、テキストスタイル）
pub fn new_drawing() -> Drawing {
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

/// TEXT追加ヘルパー（アライメント対応）
pub fn add_text(drawing: &mut Drawing, x: f64, y: f64, text: &str, height: f64, color: i16, layer: &str, align: HAlign) {
    let mut t = Text::default();
    t.location = Point::new(x, y, 0.0);
    t.text_height = cap_height_to_dxf_height(height);  // フォントメトリクス補正
    t.value = text.to_string();
    t.text_style_name = "NOTOSANSJP".to_string();
    t.relative_x_scale_factor = 1.0;  // 幅が引き伸ばされるのを防止
    t.horizontal_text_justification = match align {
        HAlign::Left => HorizontalTextJustification::Left,
        HAlign::Center => HorizontalTextJustification::Center,
        HAlign::Right => HorizontalTextJustification::Right,
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
pub fn estimate_text_width(text: &str, height: f64) -> f64 {
    let char_count: f64 = text.chars()
        .map(|c| if c.is_ascii() { 0.6 } else { 1.0 })
        .sum();
    char_count * height
}

/// テキスト描画（マスクなし、デフォルト）
pub fn add_text_with_mask(
    drawing: &mut Drawing,
    x: f64, y: f64,
    text: &str,
    height: f64,
    color: i16,
    layer: &str,
    align: HAlign
) {
    // マスクなしでテキストのみ描画
    add_text(drawing, x, y, text, height, color, layer, align);
}

/// 白背景付きテキスト描画（WIPEOUTエンティティ使用）
/// 線と重なるテキストを読みやすくしたい場合に使用
#[allow(dead_code)]
pub fn add_text_with_wipeout(
    drawing: &mut Drawing,
    x: f64, y: f64,
    text: &str,
    height: f64,
    color: i16,
    layer: &str,
    align: HAlign
) {
    let width = estimate_text_width(text, height);
    let mask_x = match align {
        HAlign::Left => x,
        HAlign::Center => x - width / 2.0,
        HAlign::Right => x - width,
    };

    // WIPEOUT矩形（背景マスク）
    let mut wipeout = Wipeout::default();
    // クリッピング頂点で矩形を定義
    wipeout.clipping_vertices = vec![
        Point::new(mask_x, y - height / 2.0, 0.0),
        Point::new(mask_x + width, y - height / 2.0, 0.0),
        Point::new(mask_x + width, y + height / 2.0, 0.0),
        Point::new(mask_x, y + height / 2.0, 0.0),
    ];
    wipeout.use_clipping = true;
    wipeout.clipping_vertex_count = 4;

    let mut entity = Entity::new(EntityType::Wipeout(wipeout));
    entity.common.layer = layer.to_string();
    drawing.add_entity(entity);

    // テキスト本体
    add_text(drawing, x, y, text, height, color, layer, align);
}

/// TEXT追加ヘルパー（回転・垂直アライメント対応）
/// rotation: 度数法（0=水平、90=下から上に読む方向）
pub fn add_text_rotated(
    drawing: &mut Drawing, x: f64, y: f64, text: &str, height: f64,
    color: i16, layer: &str, align: HAlign, v_align: VAlign, rotation: f64
) {
    let mut t = Text::default();
    t.location = Point::new(x, y, 0.0);
    t.text_height = cap_height_to_dxf_height(height);  // フォントメトリクス補正
    t.value = text.to_string();
    t.rotation = rotation;
    t.text_style_name = "NOTOSANSJP".to_string();
    t.relative_x_scale_factor = 1.0;  // 幅が引き伸ばされるのを防止

    t.horizontal_text_justification = match align {
        HAlign::Left => HorizontalTextJustification::Left,
        HAlign::Center => HorizontalTextJustification::Center,
        HAlign::Right => HorizontalTextJustification::Right,
    };
    // 垂直配置がBaseline以外の場合、second_alignment_pointが必要
    t.second_alignment_point = Point::new(x, y, 0.0);

    t.vertical_text_justification = match v_align {
        VAlign::Top => VerticalTextJustification::Top,
        VAlign::Middle => VerticalTextJustification::Middle,
        VAlign::Bottom => VerticalTextJustification::Bottom,
    };

    let mut entity = Entity::new(EntityType::Text(t));
    entity.common.layer = layer.to_string();
    entity.common.color = Color::from_index(color as u8);
    drawing.add_entity(entity);
}

/// 寸法線を線とテキストで描画（Dimensionエンティティを使わない）
pub fn add_dimension_as_lines(
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

    add_text_rotated(drawing, mid_x, text_y, text, text_height, color, layer,
        TextAlign::Center, VAlign::Bottom, 0.0);
}

/// 寸法線をDrawingContentに追加
pub fn add_dimension_as_lines_to_content(
    content: &mut DrawingContent,
    x1: f64, x2: f64, y: f64,
    text: &str, text_height: f64,
    color: i16, layer: &str,
    style: &SectionStyle,
) {
    let tick_up = style.dim_tick_up;
    let tick_down = style.dim_tick_down;

    content.add_line(x1, y, x2, y, color, layer);
    content.add_line(x1, y - tick_down, x1, y + tick_up, color, layer);
    content.add_line(x2, y - tick_down, x2, y + tick_up, color, layer);

    let mid_x = (x1 + x2) / 2.0;
    let text_y = y + style.dim_text_offset;

    content.add_text_rotated(mid_x, text_y, text, text_height, color, layer,
        HAlign::Center, VAlign::Bottom, 0.0);
}

/// Round a baseline value to integer grid with margin adjustment
///
/// Rounds up to the nearest integer, but if the difference to the reference
/// value (baseline + 1.0) is less than the margin threshold, steps down by 1.
///
/// This is useful for datum/baseline calculations where you want clean integer
/// values but need to maintain adequate margin from the minimum value.
///
/// # Arguments
/// * `baseline` - The baseline value to round (e.g., minimum elevation - 1.0)
///
/// # Returns
/// Rounded baseline value, adjusted to ensure margin
///
/// # Example
/// ```ignore
/// let min_elev = 10.1;
/// let baseline = min_elev - 1.0;  // 9.1
/// let rounded = round_baseline_with_margin(baseline);  // 10.0
/// ```
pub fn round_baseline_with_margin(baseline: f64) -> f64 {
    round_baseline_with_margin_threshold(baseline, 0.2)
}

/// Round a baseline value to integer grid with configurable margin threshold
///
/// # Arguments
/// * `baseline` - The baseline value to round
/// * `margin_threshold` - If difference to reference is less than this, step down by 1
pub fn round_baseline_with_margin_threshold(baseline: f64, margin_threshold: f64) -> f64 {
    let reference = baseline + 1.0;  // Original reference value (baseline = ref - 1.0)
    let rounded = baseline.ceil();
    let diff = reference - rounded;
    if diff < margin_threshold {
        rounded - 1.0
    } else {
        rounded
    }
}

/// DL値を1m刻みに丸める（最小標高との差が0.2以下なら1m下げる）
///
/// Deprecated: Use `round_baseline_with_margin` for new code
#[deprecated(since = "0.2.0", note = "Use round_baseline_with_margin instead")]
#[inline]
pub fn round_dl(dl: f64) -> f64 {
    round_baseline_with_margin(dl)
}
