//! 図面タイトル表組み（図枠）
//!
//! trianglelistプロジェクト（Kotlin）から移植

use dxf::Drawing;
use dxf::entities::{Entity, EntityType, Line, Text};
use dxf::enums::{HorizontalTextJustification, VerticalTextJustification};
use dxf::{Color, Point};

use crate::font_metrics::cap_height_to_text_height;
use crate::drawing_ir::{DrawingContent, HAlign, VAlign};

// DXF color constants (ACI)
const COLOR_WHITE: i16 = 7;  // White (displays as black on white background, white on black)

// A3用紙サイズと図面枠マージン
const A3_WIDTH: f64 = 420.0;   // A3横幅 (mm)
const A3_HEIGHT: f64 = 297.0;  // A3縦幅 (mm)
const FRAME_MARGIN: f64 = 10.0; // 図面枠マージン (mm)

// 内枠座標（マージン後）
const FRAME_LEFT: f64 = FRAME_MARGIN;                    // 10mm
const FRAME_RIGHT: f64 = A3_WIDTH - FRAME_MARGIN;        // 410mm
const FRAME_BOTTOM: f64 = FRAME_MARGIN;                  // 10mm
const FRAME_TOP: f64 = A3_HEIGHT - FRAME_MARGIN;         // 287mm
const FRAME_WIDTH: f64 = FRAME_RIGHT - FRAME_LEFT;       // 400mm
const FRAME_HEIGHT: f64 = FRAME_TOP - FRAME_BOTTOM;      // 277mm
const FRAME_CENTER_X: f64 = (FRAME_LEFT + FRAME_RIGHT) / 2.0;   // 210mm
const FRAME_CENTER_Y: f64 = (FRAME_BOTTOM + FRAME_TOP) / 2.0;   // 148.5mm

/// 配置可能エリア定数（横断図などのコンテンツを配置できる領域）
/// title_block.rsの定数を元に正確に計算
pub mod available_area {
    use super::*;

    /// 配置可能エリアの左端（内枠左端 = 10mm）
    pub const LEFT_MM: f64 = FRAME_LEFT;
    /// 配置可能エリアの右端（内枠右端 - タイトルブロック幅80mm = 330mm）
    pub const RIGHT_MM: f64 = FRAME_RIGHT - 80.0;
    /// 配置可能エリアの下端（内枠下端 = 10mm）
    pub const BOTTOM_MM: f64 = FRAME_BOTTOM;
    /// 配置可能エリアの上端（上部タイトル下端 = 内枠上端 - 26mm = 261mm）
    pub const TOP_MM: f64 = FRAME_TOP - 26.0;
    /// 配置可能エリアの幅（330mm - 10mm = 320mm）緑エリアのみ
    pub const WIDTH_MM: f64 = RIGHT_MM - LEFT_MM;
    /// 配置可能エリアの高さ（261mm - 10mm = 251mm）
    pub const HEIGHT_MM: f64 = TOP_MM - BOTTOM_MM;
    /// 配置エリア内のマージン
    pub const MARGIN_MM: f64 = 5.0;

    // === 内枠全体（緑+水色エリア）の定数 ===
    /// 内枠全体の幅（410mm - 10mm = 400mm）
    pub const FRAME_FULL_WIDTH_MM: f64 = FRAME_RIGHT - FRAME_LEFT;
    /// 内枠全体の有効幅（マージン込み）
    pub const FRAME_USABLE_WIDTH_MM: f64 = FRAME_FULL_WIDTH_MM - MARGIN_MM * 2.0;  // 390mm
    /// タイトル枠の高さ（グリッド配置時の下端マージン）
    pub const TITLE_BLOCK_HEIGHT_MM: f64 = 48.0;
    /// タイトル枠上端のY座標（内枠下端 + タイトル枠高さ = 58mm）
    pub const TITLE_BLOCK_TOP_MM: f64 = FRAME_BOTTOM + TITLE_BLOCK_HEIGHT_MM;
    /// 水色エリアの高さをフルに使用（上部タイトル下端 - タイトル枠上端 = 203mm）
    pub const FRAME_USABLE_HEIGHT_MM: f64 = TOP_MM - TITLE_BLOCK_TOP_MM;  // 203mm

    // === 4列均等分割の定数 ===
    /// 列数
    pub const COLUMN_COUNT: usize = 4;
    /// 列幅（100mm = 400mm / 4）
    pub const COLUMN_WIDTH_MM: f64 = FRAME_FULL_WIDTH_MM / 4.0;

    /// 列ごとの左端X座標
    pub const COLUMN_LEFTS: [f64; 4] = [
        FRAME_LEFT,                          // 10mm
        FRAME_LEFT + COLUMN_WIDTH_MM,        // 110mm
        FRAME_LEFT + COLUMN_WIDTH_MM * 2.0,  // 210mm
        FRAME_LEFT + COLUMN_WIDTH_MM * 3.0,  // 310mm
    ];

    /// 列ごとの右端X座標
    pub const COLUMN_RIGHTS: [f64; 4] = [
        FRAME_LEFT + COLUMN_WIDTH_MM,        // 110mm
        FRAME_LEFT + COLUMN_WIDTH_MM * 2.0,  // 210mm
        FRAME_LEFT + COLUMN_WIDTH_MM * 3.0,  // 310mm
        FRAME_RIGHT,                         // 410mm
    ];

    /// 列の中心X座標（横断図CL配置基準）
    pub const COLUMN_CENTERS: [f64; 4] = [
        FRAME_LEFT + COLUMN_WIDTH_MM * 0.5,  // 60mm
        FRAME_LEFT + COLUMN_WIDTH_MM * 1.5,  // 160mm
        FRAME_LEFT + COLUMN_WIDTH_MM * 2.5,  // 260mm
        FRAME_LEFT + COLUMN_WIDTH_MM * 3.5,  // 360mm
    ];

    /// 列ごとの上端Y座標（全列共通 = 261mm）
    pub const COLUMN_TOP: f64 = TOP_MM;

    /// 列ごとの下端Y座標（ガイド枠用、マージンなし）
    /// 列0-2: 内枠下端 = 10mm
    /// 列3: タイトル枠上端 = 58mm
    pub const COLUMN_BOTTOMS: [f64; 4] = [
        BOTTOM_MM,                           // 列0: 10mm
        BOTTOM_MM,                           // 列1: 10mm
        BOTTOM_MM,                           // 列2: 10mm
        TITLE_BLOCK_TOP_MM,                  // 列3: 58mm
    ];

    /// 列ごとの有効高さ（ガイド枠用、マージンなし）
    /// 列0-2: フル高さ（251mm）
    /// 列3: タイトル枠上のみ（203mm）
    pub const COLUMN_HEIGHTS: [f64; 4] = [
        HEIGHT_MM,                           // 列0: 251mm
        HEIGHT_MM,                           // 列1: 251mm
        HEIGHT_MM,                           // 列2: 251mm
        FRAME_USABLE_HEIGHT_MM,              // 列3: 203mm
    ];

    // === 有効描画領域（マージン込み）の関数 ===

    /// 指定列数に対する最大使用可能高さを返す
    /// 4列以上の場合は最も制限が厳しい列3の高さ（203mm）
    /// 3列以下の場合は列0-2の高さ（241mm）マージン込み
    #[inline]
    pub fn max_height_for_columns(columns: usize) -> f64 {
        if columns >= 4 {
            FRAME_USABLE_HEIGHT_MM  // 203mm
        } else {
            HEIGHT_MM - MARGIN_MM * 2.0  // 241mm
        }
    }

    /// 指定列に対する下端Y座標を返す（マージン込み）
    #[inline]
    pub fn bottom_for_column(col: usize) -> f64 {
        if col >= 3 {
            TITLE_BLOCK_TOP_MM  // 58mm
        } else {
            BOTTOM_MM + MARGIN_MM  // 15mm
        }
    }

    /// 指定列に対する有効高さを返す（マージン込み）
    #[inline]
    pub fn height_for_column(col: usize) -> f64 {
        if col >= 3 {
            FRAME_USABLE_HEIGHT_MM  // 203mm
        } else {
            HEIGHT_MM - MARGIN_MM * 2.0  // 241mm
        }
    }

    // === 旧定数（互換性維持）===
    /// 有効描画幅（マージン込み）- 緑エリアのみ
    pub const USABLE_WIDTH_MM: f64 = WIDTH_MM - MARGIN_MM * 2.0;  // 310mm
    /// 有効描画高さ（マージン込み）
    pub const USABLE_HEIGHT_MM: f64 = HEIGHT_MM - MARGIN_MM * 2.0;  // 241mm
}

/// タイトルブロック情報
#[derive(Debug, Clone, Default)]
pub struct TitleBlockInfo {
    /// 工事名
    pub project_name: String,
    /// 図面名（横断図、縦断図など）
    pub drawing_type: String,
    /// 路線名
    pub route_name: String,
    /// 作成日（例: "2024年1月1日"）
    pub date: String,
    /// 縮尺（例: "1:100 (A3)"）
    pub scale: String,
    /// 図面番号（例: "1/3"）
    pub drawing_number: String,
    /// 施工者名
    pub author: String,
    /// 上部タイトル（図面種別）
    pub top_title: String,
    /// クレジット
    pub credit: String,
    /// デバッグマーク表示（アンカー十字・ラベル名）
    pub show_debug_markers: bool,
}

impl TitleBlockInfo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_project_name(mut self, name: &str) -> Self {
        self.project_name = name.to_string();
        self
    }

    pub fn with_drawing_type(mut self, drawing_type: &str) -> Self {
        self.drawing_type = drawing_type.to_string();
        self
    }

    pub fn with_route_name(mut self, name: &str) -> Self {
        self.route_name = name.to_string();
        self
    }

    pub fn with_date(mut self, date: &str) -> Self {
        self.date = date.to_string();
        self
    }

    pub fn with_scale(mut self, scale: &str) -> Self {
        self.scale = scale.to_string();
        self
    }

    pub fn with_drawing_number(mut self, number: &str) -> Self {
        self.drawing_number = number.to_string();
        self
    }

    pub fn with_author(mut self, author: &str) -> Self {
        self.author = author.to_string();
        self
    }

    pub fn with_top_title(mut self, title: &str) -> Self {
        self.top_title = title.to_string();
        self
    }

    pub fn with_credit(mut self, credit: &str) -> Self {
        self.credit = credit.to_string();
        self
    }

    pub fn with_debug_markers(mut self, show: bool) -> Self {
        self.show_debug_markers = show;
        self
    }
}

/// LINE追加ヘルパー（座標変換付き）
fn add_line_scaled(
    drawing: &mut Drawing,
    x1: f64, y1: f64,
    x2: f64, y2: f64,
    color: i16,
    layer: &str,
    scale: f64,
    origin_x: f64,
    origin_y: f64,
) {
    let mut line = Line::default();
    line.p1 = Point::new(origin_x + x1 * scale, origin_y + y1 * scale, 0.0);
    line.p2 = Point::new(origin_x + x2 * scale, origin_y + y2 * scale, 0.0);
    let mut entity = Entity::new(EntityType::Line(line));
    entity.common.layer = layer.to_string();
    entity.common.color = Color::from_index(color as u8);
    drawing.add_entity(entity);
}

/// TEXT追加ヘルパー（座標変換付き）
fn add_text_scaled(
    drawing: &mut Drawing,
    x: f64, y: f64,
    text: &str,
    height: f64,
    color: i16,
    layer: &str,
    align_h: HorizontalTextJustification,
    align_v: VerticalTextJustification,
    scale: f64,
    origin_x: f64,
    origin_y: f64,
) {
    add_text_scaled_named(drawing, x, y, text, height, color, layer, align_h, align_v, scale, origin_x, origin_y, None, false);
}

/// TEXT追加ヘルパー（名前付き、座標変換付き）
fn add_text_scaled_named(
    drawing: &mut Drawing,
    x: f64, y: f64,
    text: &str,
    height: f64,
    color: i16,
    layer: &str,
    align_h: HorizontalTextJustification,
    align_v: VerticalTextJustification,
    scale: f64,
    origin_x: f64,
    origin_y: f64,
    name: Option<&str>,
    show_debug: bool,
) {
    let px = origin_x + x * scale;
    let py = origin_y + y * scale;
    let mut t = Text::default();
    t.location = Point::new(px, py, 0.0);
    t.text_height = cap_height_to_text_height(height * scale);
    t.value = text.to_string();
    t.text_style_name = "NOTOSANSJP".to_string();
    t.relative_x_scale_factor = 1.0;
    t.horizontal_text_justification = align_h;
    t.second_alignment_point = Point::new(px, py, 0.0);
    t.vertical_text_justification = align_v;
    let mut entity = Entity::new(EntityType::Text(t));
    entity.common.layer = layer.to_string();
    entity.common.color = Color::from_index(color as u8);
    drawing.add_entity(entity);

    // アンカーポイントを十字で描画（デバッグ用、show_debugがtrueの場合のみ）
    if show_debug {
        let mark_size = height * scale * 0.5;
        add_anchor_mark(drawing, px, py, mark_size, layer, name);
    }
}

/// アンカーポイントを十字マークで描画（名前付き）
fn add_anchor_mark(drawing: &mut Drawing, x: f64, y: f64, size: f64, layer: &str, name: Option<&str>) {
    // 横線
    let mut h_line = Line::default();
    h_line.p1 = Point::new(x - size, y, 0.0);
    h_line.p2 = Point::new(x + size, y, 0.0);
    let mut entity = Entity::new(EntityType::Line(h_line));
    entity.common.layer = layer.to_string();
    entity.common.color = Color::from_index(1); // 赤
    drawing.add_entity(entity);

    // 縦線
    let mut v_line = Line::default();
    v_line.p1 = Point::new(x, y - size, 0.0);
    v_line.p2 = Point::new(x, y + size, 0.0);
    let mut entity = Entity::new(EntityType::Line(v_line));
    entity.common.layer = layer.to_string();
    entity.common.color = Color::from_index(1); // 赤
    drawing.add_entity(entity);

    // 名前ラベル（あれば）
    if let Some(n) = name {
        let mut t = Text::default();
        t.location = Point::new(x + size * 1.2, y + size * 0.5, 0.0);
        t.text_height = size * 0.8;
        t.value = n.to_string();
        t.horizontal_text_justification = HorizontalTextJustification::Left;
        t.vertical_text_justification = VerticalTextJustification::Bottom;
        t.second_alignment_point = Point::new(x + size * 1.2, y + size * 0.5, 0.0);
        let mut entity = Entity::new(EntityType::Text(t));
        entity.common.layer = layer.to_string();
        entity.common.color = Color::from_index(3); // 緑
        drawing.add_entity(entity);
    }
}

/// 矩形を描画
fn add_rect_scaled(
    drawing: &mut Drawing,
    center_x: f64, center_y: f64,
    size_x: f64, size_y: f64,
    color: i16,
    layer: &str,
    scale: f64,
    origin_x: f64,
    origin_y: f64,
) {
    let half_x = size_x / 2.0;
    let half_y = size_y / 2.0;
    // 下辺
    add_line_scaled(drawing, center_x - half_x, center_y - half_y,
                    center_x + half_x, center_y - half_y, color, layer, scale, origin_x, origin_y);
    // 上辺
    add_line_scaled(drawing, center_x - half_x, center_y + half_y,
                    center_x + half_x, center_y + half_y, color, layer, scale, origin_x, origin_y);
    // 左辺
    add_line_scaled(drawing, center_x - half_x, center_y - half_y,
                    center_x - half_x, center_y + half_y, color, layer, scale, origin_x, origin_y);
    // 右辺
    add_line_scaled(drawing, center_x + half_x, center_y - half_y,
                    center_x + half_x, center_y + half_y, color, layer, scale, origin_x, origin_y);
}

/// タイトルブロック用レイヤーを追加
pub fn add_title_block_layer(drawing: &mut Drawing) {
    let mut layer = dxf::tables::Layer::default();
    layer.name = "TITLEBLOCK".to_string();
    layer.color = Color::from_index(COLOR_WHITE as u8);
    drawing.add_layer(layer);
}

/// 外枠（用紙枠）と内枠（図面枠）を描画
///
/// - 外枠: A3用紙サイズ (420×297mm)
/// - 内枠: 20mmマージン後 (380mm × 257mm, X: 20〜400mm, Y: 20〜277mm)
///
/// # Arguments
/// * `drawing` - DXFドローイング
/// * `origin_x` - 原点X座標
/// * `origin_y` - 原点Y座標
/// * `scale` - スケール（1.0 = 1mm = 1単位）
pub fn draw_outer_frame(
    drawing: &mut Drawing,
    origin_x: f64,
    origin_y: f64,
    scale: f64,
) {
    // 外枠（A3用紙端）
    add_rect_scaled(
        drawing,
        A3_WIDTH / 2.0, A3_HEIGHT / 2.0,  // 中心座標（210mm, 148.5mm）
        A3_WIDTH, A3_HEIGHT,              // サイズ（420mm × 297mm）
        COLOR_WHITE,
        "TITLEBLOCK",
        scale,
        origin_x,
        origin_y,
    );

    // 内枠（20mmマージン後の図面枠）
    add_rect_scaled(
        drawing,
        FRAME_CENTER_X, FRAME_CENTER_Y,  // 中心座標（210mm, 148.5mm）
        FRAME_WIDTH, FRAME_HEIGHT,       // サイズ（380mm × 257mm）
        COLOR_WHITE,
        "TITLEBLOCK",
        scale,
        origin_x,
        origin_y,
    );
}

/// 上部タイトルを描画
///
/// # Arguments
/// * `drawing` - DXFドローイング
/// * `info` - タイトルブロック情報
/// * `origin_x` - 原点X座標
/// * `origin_y` - 原点Y座標
/// * `scale` - スケール
/// * `text_size` - テキストサイズ（mm）
pub fn draw_top_title(
    drawing: &mut Drawing,
    info: &TitleBlockInfo,
    origin_x: f64,
    origin_y: f64,
    scale: f64,
    text_size: f64,
) {
    // 上のタイトル（2行）- 中央揃え、下揃え、テキストサイズ2倍
    // 内枠上端から内側（下方向）に配置
    let show_debug = info.show_debug_markers;
    let title_y = FRAME_TOP - 15.0;     // 262mm (内枠上端から15mm下)
    let name_y = FRAME_TOP - 26.0;      // 251mm (タイトルの下)
    let line_y1 = FRAME_TOP - 16.0;     // 261mm (二重線上)
    let line_y2 = FRAME_TOP - 17.0;     // 260mm (二重線下)

    add_text_scaled_named(
        drawing, FRAME_CENTER_X, title_y, &info.top_title,
        text_size * 2.0, COLOR_WHITE, "TITLEBLOCK",
        HorizontalTextJustification::Center,
        VerticalTextJustification::Bottom,
        scale, origin_x, origin_y,
        Some("TOP_TITLE"), show_debug,
    );
    add_text_scaled_named(
        drawing, FRAME_CENTER_X, name_y, &info.project_name,
        text_size * 2.0, COLOR_WHITE, "TITLEBLOCK",
        HorizontalTextJustification::Center,
        VerticalTextJustification::Bottom,
        scale, origin_x, origin_y,
        Some("TOP_NAME"), show_debug,
    );

    // タイトル下の二重線（中央に40mm幅）
    let line_half_width = 20.0;
    add_line_scaled(drawing, FRAME_CENTER_X - line_half_width, line_y1,
                    FRAME_CENTER_X + line_half_width, line_y1, COLOR_WHITE, "TITLEBLOCK", scale, origin_x, origin_y);
    add_line_scaled(drawing, FRAME_CENTER_X - line_half_width, line_y2,
                    FRAME_CENTER_X + line_half_width, line_y2, COLOR_WHITE, "TITLEBLOCK", scale, origin_x, origin_y);
}

/// 右下タイトル枠を描画
///
/// レイアウト:
/// - 右下表組み: 内枠右端から左に100mm、内枠下端から上に60mm
/// - 表組みサイズ: 100mm × 60mm
/// - 行高さ: 10mm × 6行
///
/// # Arguments
/// * `drawing` - DXFドローイング
/// * `info` - タイトルブロック情報
/// * `origin_x` - 原点X座標
/// * `origin_y` - 原点Y座標
/// * `scale` - スケール
/// * `text_size` - テキストサイズ（mm）
pub fn draw_title_block(
    drawing: &mut Drawing,
    info: &TitleBlockInfo,
    origin_x: f64,
    origin_y: f64,
    scale: f64,
    text_size: f64,
) {
    // タイトル枠の基準座標（内枠基準）- 0.8倍スケール
    let tb_scale = 0.8;
    let tb_width = 100.0 * tb_scale;      // 80mm
    let tb_height = 60.0 * tb_scale;      // 48mm
    let tb_left = FRAME_RIGHT - tb_width; // 330mm (内枠右端から左に80mm)
    let tb_right = FRAME_RIGHT;           // 410mm (内枠右端)
    let tb_bottom = FRAME_BOTTOM;         // 10mm (内枠下端)
    let tb_top = FRAME_BOTTOM + tb_height;// 58mm (内枠下端から48mm上)
    let label_col = tb_left + 20.0 * tb_scale;  // ラベル列の右端（幅16mm）
    let row_height = 10.0 * tb_scale;     // 行高さ 8mm

    // 各行のY座標（下から上へ、行の下端座標）
    let row_author_y = tb_bottom;                     // 20mm (施工者行の下端)
    let row_scale_y = tb_bottom + row_height;         // 30mm (縮尺/図番行の下端)
    let row_date_y = tb_bottom + row_height * 2.0;    // 40mm (作成日行の下端)
    let row_route_y = tb_bottom + row_height * 3.0;   // 50mm (路線名行の下端)
    let row_type_y = tb_bottom + row_height * 4.0;    // 60mm (図面名行の下端)
    let row_proj_y = tb_bottom + row_height * 5.0;    // 70mm (工事名行の下端)

    // === 外枠線 ===
    // 上辺
    add_line_scaled(drawing, tb_left, tb_top, tb_right, tb_top, COLOR_WHITE, "TITLEBLOCK", scale, origin_x, origin_y);
    // 左辺（外側）
    add_line_scaled(drawing, tb_left, tb_bottom, tb_left, tb_top, COLOR_WHITE, "TITLEBLOCK", scale, origin_x, origin_y);
    // 左辺（内側、ラベル列）
    add_line_scaled(drawing, label_col, tb_bottom, label_col, tb_top, COLOR_WHITE, "TITLEBLOCK", scale, origin_x, origin_y);

    // === 横線（各行の境界）===
    add_line_scaled(drawing, tb_left, row_proj_y, tb_right, row_proj_y, COLOR_WHITE, "TITLEBLOCK", scale, origin_x, origin_y);
    add_line_scaled(drawing, tb_left, row_type_y, tb_right, row_type_y, COLOR_WHITE, "TITLEBLOCK", scale, origin_x, origin_y);
    add_line_scaled(drawing, tb_left, row_route_y, tb_right, row_route_y, COLOR_WHITE, "TITLEBLOCK", scale, origin_x, origin_y);
    add_line_scaled(drawing, tb_left, row_date_y, tb_right, row_date_y, COLOR_WHITE, "TITLEBLOCK", scale, origin_x, origin_y);
    add_line_scaled(drawing, tb_left, row_scale_y, tb_right, row_scale_y, COLOR_WHITE, "TITLEBLOCK", scale, origin_x, origin_y);

    // 縮尺/図番の縦仕切り（row_scale行内）
    let scale_col = label_col + 30.0 * tb_scale;     // 縮尺列の右端
    let num_lbl_col = scale_col + 20.0 * tb_scale;   // 図番ラベル列の右端
    add_line_scaled(drawing, scale_col, row_scale_y, scale_col, row_date_y, COLOR_WHITE, "TITLEBLOCK", scale, origin_x, origin_y);
    add_line_scaled(drawing, num_lbl_col, row_scale_y, num_lbl_col, row_date_y, COLOR_WHITE, "TITLEBLOCK", scale, origin_x, origin_y);

    // === ラベルテキスト（左列）=== 中央揃え
    let show_debug = info.show_debug_markers;
    let label_x = (tb_left + label_col) / 2.0;  // 310mm (ラベル列中央)

    add_text_scaled_named(drawing, label_x, row_proj_y + row_height / 2.0, "工事名", text_size, COLOR_WHITE, "TITLEBLOCK",
                    HorizontalTextJustification::Center, VerticalTextJustification::Middle,
                    scale, origin_x, origin_y, Some("LBL_PROJECT"), show_debug);
    add_text_scaled_named(drawing, label_x, row_type_y + row_height / 2.0, "図面名", text_size, COLOR_WHITE, "TITLEBLOCK",
                    HorizontalTextJustification::Center, VerticalTextJustification::Middle,
                    scale, origin_x, origin_y, Some("LBL_TYPE"), show_debug);
    add_text_scaled_named(drawing, label_x, row_route_y + row_height / 2.0, "路線名", text_size, COLOR_WHITE, "TITLEBLOCK",
                    HorizontalTextJustification::Center, VerticalTextJustification::Middle,
                    scale, origin_x, origin_y, Some("LBL_ROUTE"), show_debug);
    add_text_scaled_named(drawing, label_x, row_date_y + row_height / 2.0, "作成日", text_size, COLOR_WHITE, "TITLEBLOCK",
                    HorizontalTextJustification::Center, VerticalTextJustification::Middle,
                    scale, origin_x, origin_y, Some("LBL_DATE"), show_debug);
    // 縮尺ラベル
    add_text_scaled_named(drawing, label_x, row_scale_y + row_height / 2.0, "縮尺", text_size, COLOR_WHITE, "TITLEBLOCK",
                    HorizontalTextJustification::Center, VerticalTextJustification::Middle,
                    scale, origin_x, origin_y, Some("LBL_SCALE"), show_debug);
    // 図面番号ラベル
    let num_lbl_x = (scale_col + num_lbl_col) / 2.0;  // 360mm
    add_text_scaled_named(drawing, num_lbl_x, row_scale_y + row_height / 2.0, "図面番号", text_size, COLOR_WHITE, "TITLEBLOCK",
                    HorizontalTextJustification::Center, VerticalTextJustification::Middle,
                    scale, origin_x, origin_y, Some("LBL_NUM"), show_debug);
    // 施工者ラベル
    add_text_scaled_named(drawing, label_x, row_author_y + row_height / 2.0, "施工者", text_size, COLOR_WHITE, "TITLEBLOCK",
                    HorizontalTextJustification::Center, VerticalTextJustification::Middle,
                    scale, origin_x, origin_y, Some("LBL_AUTHOR"), show_debug);

    // === 内容テキスト（右列）===
    let data_x = label_col + 5.0 * tb_scale;  // データ列の左端、少し内側

    // 工事名（長い場合は改行処理）
    let project_name_display = truncate_or_split(&info.project_name, 25);
    add_text_scaled_named(drawing, data_x, row_proj_y + row_height / 2.0, &project_name_display, text_size, COLOR_WHITE, "TITLEBLOCK",
                    HorizontalTextJustification::Left, VerticalTextJustification::Middle,
                    scale, origin_x, origin_y, Some("VAL_PROJECT"), show_debug);

    // 図面名
    add_text_scaled_named(drawing, data_x, row_type_y + row_height / 2.0, &info.drawing_type, text_size, COLOR_WHITE, "TITLEBLOCK",
                    HorizontalTextJustification::Left, VerticalTextJustification::Middle,
                    scale, origin_x, origin_y, Some("VAL_TYPE"), show_debug);

    // 路線名
    add_text_scaled_named(drawing, data_x, row_route_y + row_height / 2.0, &info.route_name, text_size, COLOR_WHITE, "TITLEBLOCK",
                    HorizontalTextJustification::Left, VerticalTextJustification::Middle,
                    scale, origin_x, origin_y, Some("VAL_ROUTE"), show_debug);

    // 作成日
    add_text_scaled_named(drawing, data_x, row_date_y + row_height / 2.0, &info.date, text_size, COLOR_WHITE, "TITLEBLOCK",
                    HorizontalTextJustification::Left, VerticalTextJustification::Middle,
                    scale, origin_x, origin_y, Some("VAL_DATE"), show_debug);

    // 縮尺値（中央揃え）
    let scale_val_x = (label_col + scale_col) / 2.0;  // 335mm
    add_text_scaled_named(drawing, scale_val_x, row_scale_y + row_height / 2.0, &info.scale, text_size, COLOR_WHITE, "TITLEBLOCK",
                    HorizontalTextJustification::Center, VerticalTextJustification::Middle,
                    scale, origin_x, origin_y, Some("VAL_SCALE"), show_debug);

    // 図面番号値（中央揃え）
    let num_val_x = (num_lbl_col + tb_right) / 2.0;  // 385mm
    add_text_scaled_named(drawing, num_val_x, row_scale_y + row_height / 2.0, &info.drawing_number, text_size, COLOR_WHITE, "TITLEBLOCK",
                    HorizontalTextJustification::Center, VerticalTextJustification::Middle,
                    scale, origin_x, origin_y, Some("VAL_NUM"), show_debug);

    // 施工者
    add_text_scaled_named(drawing, data_x, row_author_y + row_height / 2.0, &info.author, text_size, COLOR_WHITE, "TITLEBLOCK",
                    HorizontalTextJustification::Left, VerticalTextJustification::Middle,
                    scale, origin_x, origin_y, Some("VAL_AUTHOR"), show_debug);

    // クレジット（内枠左下）
    if !info.credit.is_empty() {
        add_text_scaled_named(drawing, FRAME_LEFT + 60.0, FRAME_BOTTOM - 10.0, &info.credit, text_size, COLOR_WHITE, "TITLEBLOCK",
                        HorizontalTextJustification::Left, VerticalTextJustification::Middle,
                        scale, origin_x, origin_y, Some("CREDIT"), show_debug);
    }
}

/// 文字列を指定文字数で切り詰めまたは分割
fn truncate_or_split(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else if s.contains(' ') {
        // スペースで分割して最初の部分を返す
        s.split(' ').next().unwrap_or(s).to_string()
    } else {
        // 指定文字数で切り詰め
        s.chars().take(max_len).collect()
    }
}

/// 配置可能エリアのガイド矩形を描画
///
/// 4列均等分割のガイド枠を描画（各列100mm幅）
fn draw_available_area_guide(
    drawing: &mut Drawing,
    origin_x: f64,
    origin_y: f64,
    scale: f64,
    text_size: f64,
    show_debug: bool,
) {
    // ガイド矩形を緑色で描画（デバッグ時のみ）
    if show_debug {
        const COLOR_GREEN: i16 = 3;

        // 4列の緑色ガイド枠を描画
        for col in 0..4 {
            let left = available_area::COLUMN_LEFTS[col];
            let right = available_area::COLUMN_RIGHTS[col];
            let bottom = available_area::COLUMN_BOTTOMS[col];
            let top = available_area::COLUMN_TOP;
            let width = right - left;
            let height = top - bottom;

            // 4辺を描画
            add_line_scaled(drawing, left, bottom, right, bottom,
                            COLOR_GREEN, "TITLEBLOCK", scale, origin_x, origin_y);
            add_line_scaled(drawing, left, top, right, top,
                            COLOR_GREEN, "TITLEBLOCK", scale, origin_x, origin_y);
            add_line_scaled(drawing, left, bottom, left, top,
                            COLOR_GREEN, "TITLEBLOCK", scale, origin_x, origin_y);
            add_line_scaled(drawing, right, bottom, right, top,
                            COLOR_GREEN, "TITLEBLOCK", scale, origin_x, origin_y);

            // 列番号とサイズを中央に表示
            let label = format!("列{} {}x{}", col, width as i32, height as i32);
            add_text_scaled_named(
                drawing, (left + right) / 2.0, (bottom + top) / 2.0,
                &label, text_size, COLOR_GREEN, "TITLEBLOCK",
                HorizontalTextJustification::Center, VerticalTextJustification::Middle,
                scale, origin_x, origin_y, Some(&format!("COL{}_SIZE", col)), show_debug,
            );
        }
    }
}

// ============================================================================
// DrawingContent IR-based Title Block Generation
// ============================================================================

/// TitleBlockInfoからDrawingContentを生成
pub fn build_title_block_content(info: &TitleBlockInfo) -> DrawingContent {
    let mut content = DrawingContent::new();
    let text_size = 4.0; // 固定テキストサイズ (mm)

    // 外枠（A3用紙端）と内枠
    build_outer_frame_content(&mut content);

    // 上部タイトル
    build_top_title_content(&mut content, info, text_size);

    // 右下タイトル枠
    build_title_table_content(&mut content, info, text_size);

    // 配置可能エリアのガイド（デバッグ時のみ）
    if info.show_debug_markers {
        build_debug_guide_content(&mut content, text_size);
    }

    content
}

/// 外枠（A3用紙端）と内枠をDrawingContentに追加
fn build_outer_frame_content(content: &mut DrawingContent) {
    // 外枠（A3用紙端）
    content.add_rect(
        A3_WIDTH / 2.0, A3_HEIGHT / 2.0,  // 中心座標（210mm, 148.5mm）
        A3_WIDTH, A3_HEIGHT,              // サイズ（420mm × 297mm）
        COLOR_WHITE,
        "TITLEBLOCK",
    );

    // 内枠（10mmマージン後の図面枠）
    content.add_rect(
        FRAME_CENTER_X, FRAME_CENTER_Y,  // 中心座標（210mm, 148.5mm）
        FRAME_WIDTH, FRAME_HEIGHT,       // サイズ（400mm × 277mm）
        COLOR_WHITE,
        "TITLEBLOCK",
    );
}

/// 上部タイトルをDrawingContentに追加
fn build_top_title_content(content: &mut DrawingContent, info: &TitleBlockInfo, text_size: f64) {
    // 上のタイトル（2行）- 中央揃え、下揃え、テキストサイズ2倍
    // text_size=4.0mm → 実際のテキスト高さ8.0mm に対応したスペース配分
    let title_text_size = text_size * 2.0;  // 8.0mm
    let title_y = FRAME_TOP - 12.0;         // 275mm (内枠上端から12mm下)
    let line_y1 = FRAME_TOP - 14.0;         // 273mm (タイトル下の二重線上)
    let line_y2 = FRAME_TOP - 15.5;         // 271.5mm (二重線下)
    let name_y = FRAME_TOP - 30.0;          // 257mm (二重線の下、十分なスペース確保)

    content.add_text_with_v_align(
        FRAME_CENTER_X, title_y, &info.top_title,
        title_text_size, COLOR_WHITE, "TITLEBLOCK",
        HAlign::Center, VAlign::Bottom,
    );

    // タイトル下の二重線（中央に40mm幅）
    let line_half_width = 20.0;
    content.add_line(FRAME_CENTER_X - line_half_width, line_y1,
                     FRAME_CENTER_X + line_half_width, line_y1, COLOR_WHITE, "TITLEBLOCK");
    content.add_line(FRAME_CENTER_X - line_half_width, line_y2,
                     FRAME_CENTER_X + line_half_width, line_y2, COLOR_WHITE, "TITLEBLOCK");

    // 工事名（二重線の下）
    content.add_text_with_v_align(
        FRAME_CENTER_X, name_y, &info.project_name,
        title_text_size, COLOR_WHITE, "TITLEBLOCK",
        HAlign::Center, VAlign::Bottom,
    );
}

/// 右下タイトル表組みをDrawingContentに追加
fn build_title_table_content(content: &mut DrawingContent, info: &TitleBlockInfo, text_size: f64) {
    // タイトル枠の基準座標（内枠基準）- 0.8倍スケール
    let tb_scale = 0.8;
    let tb_width = 100.0 * tb_scale;      // 80mm
    let tb_height = 60.0 * tb_scale;      // 48mm
    let tb_left = FRAME_RIGHT - tb_width; // 330mm (内枠右端から左に80mm)
    let tb_right = FRAME_RIGHT;           // 410mm (内枠右端)
    let tb_bottom = FRAME_BOTTOM;         // 10mm (内枠下端)
    let tb_top = FRAME_BOTTOM + tb_height;// 58mm (内枠下端から48mm上)
    let label_col = tb_left + 20.0 * tb_scale;  // ラベル列の右端（幅16mm）
    let row_height = 10.0 * tb_scale;     // 行高さ 8mm

    // 各行のY座標（下から上へ、行の下端座標）
    let row_author_y = tb_bottom;                     // 10mm (施工者行の下端)
    let row_scale_y = tb_bottom + row_height;         // 18mm (縮尺/図番行の下端)
    let row_date_y = tb_bottom + row_height * 2.0;    // 26mm (作成日行の下端)
    let row_route_y = tb_bottom + row_height * 3.0;   // 34mm (路線名行の下端)
    let row_type_y = tb_bottom + row_height * 4.0;    // 42mm (図面名行の下端)
    let row_proj_y = tb_bottom + row_height * 5.0;    // 50mm (工事名行の下端)

    // === 外枠線 ===
    // 上辺
    content.add_line(tb_left, tb_top, tb_right, tb_top, COLOR_WHITE, "TITLEBLOCK");
    // 左辺（外側）
    content.add_line(tb_left, tb_bottom, tb_left, tb_top, COLOR_WHITE, "TITLEBLOCK");
    // 左辺（内側、ラベル列）
    content.add_line(label_col, tb_bottom, label_col, tb_top, COLOR_WHITE, "TITLEBLOCK");

    // === 横線（各行の境界）===
    content.add_line(tb_left, row_proj_y, tb_right, row_proj_y, COLOR_WHITE, "TITLEBLOCK");
    content.add_line(tb_left, row_type_y, tb_right, row_type_y, COLOR_WHITE, "TITLEBLOCK");
    content.add_line(tb_left, row_route_y, tb_right, row_route_y, COLOR_WHITE, "TITLEBLOCK");
    content.add_line(tb_left, row_date_y, tb_right, row_date_y, COLOR_WHITE, "TITLEBLOCK");
    content.add_line(tb_left, row_scale_y, tb_right, row_scale_y, COLOR_WHITE, "TITLEBLOCK");

    // 縮尺/図番の縦仕切り（row_scale行内）
    let scale_col = label_col + 30.0 * tb_scale;     // 縮尺列の右端
    let num_lbl_col = scale_col + 20.0 * tb_scale;   // 図番ラベル列の右端
    content.add_line(scale_col, row_scale_y, scale_col, row_date_y, COLOR_WHITE, "TITLEBLOCK");
    content.add_line(num_lbl_col, row_scale_y, num_lbl_col, row_date_y, COLOR_WHITE, "TITLEBLOCK");

    // === ラベルテキスト（左列）=== 中央揃え
    let label_x = (tb_left + label_col) / 2.0;  // ラベル列中央

    content.add_text(label_x, row_proj_y + row_height / 2.0, "工事名", text_size, COLOR_WHITE, "TITLEBLOCK", HAlign::Center);
    content.add_text(label_x, row_type_y + row_height / 2.0, "図面名", text_size, COLOR_WHITE, "TITLEBLOCK", HAlign::Center);
    content.add_text(label_x, row_route_y + row_height / 2.0, "路線名", text_size, COLOR_WHITE, "TITLEBLOCK", HAlign::Center);
    content.add_text(label_x, row_date_y + row_height / 2.0, "作成日", text_size, COLOR_WHITE, "TITLEBLOCK", HAlign::Center);
    // 縮尺ラベル
    content.add_text(label_x, row_scale_y + row_height / 2.0, "縮尺", text_size, COLOR_WHITE, "TITLEBLOCK", HAlign::Center);
    // 図面番号ラベル
    let num_lbl_x = (scale_col + num_lbl_col) / 2.0;
    content.add_text(num_lbl_x, row_scale_y + row_height / 2.0, "図面番号", text_size, COLOR_WHITE, "TITLEBLOCK", HAlign::Center);
    // 施工者ラベル
    content.add_text(label_x, row_author_y + row_height / 2.0, "施工者", text_size, COLOR_WHITE, "TITLEBLOCK", HAlign::Center);

    // === 内容テキスト（右列）===
    let data_x = label_col + 5.0 * tb_scale;  // データ列の左端、少し内側

    // 工事名（長い場合は改行処理）
    let project_name_display = truncate_or_split(&info.project_name, 25);
    content.add_text(data_x, row_proj_y + row_height / 2.0, &project_name_display, text_size, COLOR_WHITE, "TITLEBLOCK", HAlign::Left);

    // 図面名
    content.add_text(data_x, row_type_y + row_height / 2.0, &info.drawing_type, text_size, COLOR_WHITE, "TITLEBLOCK", HAlign::Left);

    // 路線名
    content.add_text(data_x, row_route_y + row_height / 2.0, &info.route_name, text_size, COLOR_WHITE, "TITLEBLOCK", HAlign::Left);

    // 作成日
    content.add_text(data_x, row_date_y + row_height / 2.0, &info.date, text_size, COLOR_WHITE, "TITLEBLOCK", HAlign::Left);

    // 縮尺値（中央揃え）
    let scale_val_x = (label_col + scale_col) / 2.0;
    content.add_text(scale_val_x, row_scale_y + row_height / 2.0, &info.scale, text_size, COLOR_WHITE, "TITLEBLOCK", HAlign::Center);

    // 図面番号値（中央揃え）
    let num_val_x = (num_lbl_col + tb_right) / 2.0;
    content.add_text(num_val_x, row_scale_y + row_height / 2.0, &info.drawing_number, text_size, COLOR_WHITE, "TITLEBLOCK", HAlign::Center);

    // 施工者
    content.add_text(data_x, row_author_y + row_height / 2.0, &info.author, text_size, COLOR_WHITE, "TITLEBLOCK", HAlign::Left);

    // クレジット（内枠左下）
    if !info.credit.is_empty() {
        content.add_text(FRAME_LEFT + 60.0, FRAME_BOTTOM - 10.0, &info.credit, text_size, COLOR_WHITE, "TITLEBLOCK", HAlign::Left);
    }
}

/// デバッグ用ガイド矩形をDrawingContentに追加（4列均等分割のガイド枠）
fn build_debug_guide_content(content: &mut DrawingContent, text_size: f64) {
    const COLOR_GREEN: i16 = 3;

    // 4列の緑色ガイド枠を描画
    for col in 0..4 {
        let left = available_area::COLUMN_LEFTS[col];
        let right = available_area::COLUMN_RIGHTS[col];
        let bottom = available_area::COLUMN_BOTTOMS[col];
        let top = available_area::COLUMN_TOP;
        let width = right - left;
        let height = top - bottom;

        // 4辺を描画
        content.add_line(left, bottom, right, bottom, COLOR_GREEN, "TITLEBLOCK");
        content.add_line(left, top, right, top, COLOR_GREEN, "TITLEBLOCK");
        content.add_line(left, bottom, left, top, COLOR_GREEN, "TITLEBLOCK");
        content.add_line(right, bottom, right, top, COLOR_GREEN, "TITLEBLOCK");

        // 列番号とサイズを中央に表示
        let label = format!("列{} {}x{}", col, width as i32, height as i32);
        content.add_text(
            (left + right) / 2.0, (bottom + top) / 2.0,
            &label, text_size, COLOR_GREEN, "TITLEBLOCK",
            HAlign::Center,
        );
    }
}

/// 図枠全体を描画（外枠＋上部タイトル＋右下タイトル枠＋配置ガイド）
///
/// # Arguments
/// * `drawing` - DXFドローイング
/// * `info` - タイトルブロック情報
/// * `origin_x` - 原点X座標
/// * `origin_y` - 原点Y座標
/// * `scale` - スケール
/// * `_text_size` - テキストサイズ（mm）- 廃止予定、内部で固定値を使用
pub fn draw_drawing_frame(
    drawing: &mut Drawing,
    info: &TitleBlockInfo,
    origin_x: f64,
    origin_y: f64,
    scale: f64,
    _text_size: f64,
) {
    add_title_block_layer(drawing);
    let content = build_title_block_content(info);
    content.add_to_dxf_transformed(drawing, origin_x, origin_y, scale);
}

/// タイトル枠テスト用Drawingを生成
/// サンプルデータで図枠全体を描画
pub fn generate_title_block_test_drawing() -> Drawing {
    let mut drawing = Drawing::new();
    drawing.header.version = dxf::enums::AcadVersion::R2000;

    // テキストスタイル作成
    let mut style = dxf::tables::Style::default();
    style.name = "NOTOSANSJP".to_string();
    style.primary_font_file_name = "Noto Sans JP".to_string();
    style.width_factor = 1.0;
    drawing.add_style(style);

    let info = TitleBlockInfo::new()
        .with_project_name("市道 南千反畑町第１号線舗装補修工事")
        .with_drawing_type("横断図")
        .with_route_name("熊本市中央区南千反畑町外地内")
        .with_date("2026年1月9日")
        .with_scale("1:100 (A3)")
        .with_drawing_number("1/1")
        .with_author("有限会社　三雄建設")
        .with_top_title("横断図")
        .with_credit("")
        .with_debug_markers(true);  // プレビュー用にデバッグマーカー表示

    // scale=1.0で1mm=1単位、text_size=3.0mmで描画
    draw_drawing_frame(&mut drawing, &info, 0.0, 0.0, 1.0, 3.0);

    drawing
}

/// タイトル枠テストDXFをバイト列で返す（デバッグマーカー付き）
pub fn generate_title_block_test_dxf_bytes() -> Vec<u8> {
    let drawing = generate_title_block_test_drawing();
    let mut buf = Vec::new();
    drawing.save(&mut buf).unwrap();
    buf
}

/// タイトル枠DXFをバイト列で返す（デバッグマーカーなし、ダウンロード用）
pub fn generate_title_block_dxf_for_download() -> Vec<u8> {
    let mut drawing = Drawing::new();
    drawing.header.version = dxf::enums::AcadVersion::R2000;

    // テキストスタイル作成
    let mut style = dxf::tables::Style::default();
    style.name = "NOTOSANSJP".to_string();
    style.primary_font_file_name = "Noto Sans JP".to_string();
    style.width_factor = 1.0;
    drawing.add_style(style);

    let info = TitleBlockInfo::new()
        .with_project_name("市道 南千反畑町第１号線舗装補修工事")
        .with_drawing_type("横断図")
        .with_route_name("熊本市中央区南千反畑町外地内")
        .with_date("2026年1月9日")
        .with_scale("1:100 (A3)")
        .with_drawing_number("1/1")
        .with_author("有限会社　三雄建設")
        .with_top_title("横断図")
        .with_credit("")
        .with_debug_markers(false);  // ダウンロード用にデバッグマーカー非表示

    draw_drawing_frame(&mut drawing, &info, 0.0, 0.0, 1.0, 3.0);

    let mut buf = Vec::new();
    drawing.save(&mut buf).unwrap();
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_title_block_info_builder() {
        let info = TitleBlockInfo::new()
            .with_project_name("テスト工事")
            .with_drawing_type("横断図")
            .with_route_name("市道1号線")
            .with_date("2024年1月1日")
            .with_scale("1:100")
            .with_drawing_number("1/3")
            .with_author("テスト太郎");

        assert_eq!(info.project_name, "テスト工事");
        assert_eq!(info.drawing_type, "横断図");
        assert_eq!(info.route_name, "市道1号線");
        assert_eq!(info.date, "2024年1月1日");
        assert_eq!(info.scale, "1:100");
        assert_eq!(info.drawing_number, "1/3");
        assert_eq!(info.author, "テスト太郎");
    }

    #[test]
    fn test_truncate_or_split() {
        // 短い文字列はそのまま
        assert_eq!(truncate_or_split("短い文字列", 25), "短い文字列");
        // スペースがある場合は最初の部分を返す
        assert_eq!(truncate_or_split("これは非常に長い工事名です 副題部分", 10), "これは非常に長い工事名です");
        // スペースがない場合は指定文字数で切り詰め
        let result = truncate_or_split("スペースなし長い文字列テスト", 10);
        assert_eq!(result.chars().count(), 10);
    }

    #[test]
    fn test_draw_drawing_frame() {
        let mut drawing = dxf::Drawing::new();
        drawing.header.version = dxf::enums::AcadVersion::R2000;

        let info = TitleBlockInfo::new()
            .with_project_name("テスト工事")
            .with_drawing_type("横断図");

        draw_drawing_frame(&mut drawing, &info, 0.0, 0.0, 1.0, 3.0);

        // レイヤーが追加されていることを確認
        assert!(drawing.layers().any(|l| l.name == "TITLEBLOCK"));

        // エンティティが追加されていることを確認
        let entity_count = drawing.entities().count();
        assert!(entity_count > 0, "エンティティが追加されていない");
    }
}
