//! 図面タイトル表組み（図枠）
//!
//! trianglelistプロジェクト（Kotlin）から移植

use dxf::Drawing;
use dxf::entities::{Entity, EntityType, Line, Text};
use dxf::enums::{HorizontalTextJustification, VerticalTextJustification};
use dxf::{Color, Point};

use crate::font_metrics::cap_height_to_text_height;

// DXF color constants (ACI)
const WHITE: i16 = 8;  // Gray (visible on both light/dark backgrounds)

/// タイトルブロック情報
#[derive(Debug, Clone, Default)]
pub struct TitleBlockInfo {
    /// 工事名
    pub project_name: String,
    /// 工事種（図面種別: 横断図、縦断図など）
    pub work_type: String,
    /// 図面名（路線名など）
    pub drawing_name: String,
    /// 日付（例: "2024年1月1日"）
    pub date: String,
    /// 縮尺（例: "1:100"）
    pub scale: String,
    /// 図番（例: "1/3"）
    pub drawing_number: String,
    /// 作業者名
    pub author: String,
    /// 上部タイトル
    pub top_title: String,
    /// クレジット
    pub credit: String,
}

impl TitleBlockInfo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_project_name(mut self, name: &str) -> Self {
        self.project_name = name.to_string();
        self
    }

    pub fn with_work_type(mut self, work_type: &str) -> Self {
        self.work_type = work_type.to_string();
        self
    }

    pub fn with_drawing_name(mut self, name: &str) -> Self {
        self.drawing_name = name.to_string();
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
    layer.color = Color::from_index(WHITE as u8);
    drawing.add_layer(layer);
}

/// 外枠（用紙枠）を描画
///
/// A3用紙 (420×297mm) の外枠を描画
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
    // 外枠: 中心(21, 14.85) サイズ(40×27)cm → (400×270)mm相当
    // Kotlinでは writeRect(PointXY(21f, 14.85f, scale), 40f * scale, 27f * scale)
    // 21cm = 210mm, 14.85cm = 148.5mm を中心として描画
    add_rect_scaled(
        drawing,
        210.0, 148.5,  // 中心座標（mm）
        400.0, 270.0,  // サイズ（mm）
        WHITE,
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
    // 上のタイトル（2行）
    // Kotlinでは (21f, 27.1f), (21f, 26f) → 210mm, 271mm と 210mm, 260mm
    add_text_scaled(
        drawing, 210.0, 271.0, &info.top_title,
        text_size, WHITE, "TITLEBLOCK",
        HorizontalTextJustification::Left,
        VerticalTextJustification::Middle,
        scale, origin_x, origin_y,
    );
    add_text_scaled(
        drawing, 210.0, 260.0, &info.drawing_name,
        text_size, WHITE, "TITLEBLOCK",
        HorizontalTextJustification::Left,
        VerticalTextJustification::Middle,
        scale, origin_x, origin_y,
    );

    // タイトル下の二重線
    // Kotlinでは (19f, 27f)-(23f, 27f) と (19f, 26.9f)-(23f, 26.9f)
    // → 190-230mm, 270mm と 190-230mm, 269mm
    add_line_scaled(drawing, 190.0, 270.0, 230.0, 270.0, WHITE, "TITLEBLOCK", scale, origin_x, origin_y);
    add_line_scaled(drawing, 190.0, 269.0, 230.0, 269.0, WHITE, "TITLEBLOCK", scale, origin_x, origin_y);
}

/// 右下タイトル枠を描画
///
/// レイアウト（Kotlinからの移植）:
/// - 右下表組み: X=310〜410mm, Y=13.5〜73.5mm
/// - 行高さ: 10mm
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
    // 座標変換: Kotlinの座標系はcm単位(×10でmm)
    // 31f→310mm, 41f→410mm, 1.35f→13.5mm, 7.35f→73.5mm

    // === 外枠線 ===
    // 上辺
    add_line_scaled(drawing, 310.0, 73.5, 410.0, 73.5, WHITE, "TITLEBLOCK", scale, origin_x, origin_y);
    // 左辺（外側）
    add_line_scaled(drawing, 310.0, 13.5, 310.0, 73.5, WHITE, "TITLEBLOCK", scale, origin_x, origin_y);
    // 左辺（内側、ラベル列）: 33f→330mm
    add_line_scaled(drawing, 330.0, 13.5, 330.0, 73.5, WHITE, "TITLEBLOCK", scale, origin_x, origin_y);

    // === 横線（各行の境界）===
    // 行: 6.35f, 5.35f, 4.35f, 3.35f, 2.35f → 63.5, 53.5, 43.5, 33.5, 23.5mm
    add_line_scaled(drawing, 310.0, 63.5, 410.0, 63.5, WHITE, "TITLEBLOCK", scale, origin_x, origin_y);
    add_line_scaled(drawing, 310.0, 53.5, 410.0, 53.5, WHITE, "TITLEBLOCK", scale, origin_x, origin_y);
    add_line_scaled(drawing, 310.0, 43.5, 410.0, 43.5, WHITE, "TITLEBLOCK", scale, origin_x, origin_y);
    add_line_scaled(drawing, 310.0, 33.5, 410.0, 33.5, WHITE, "TITLEBLOCK", scale, origin_x, origin_y);
    add_line_scaled(drawing, 310.0, 23.5, 410.0, 23.5, WHITE, "TITLEBLOCK", scale, origin_x, origin_y);

    // 縮尺/図番の縦仕切り: 36f, 38f → 360mm, 380mm (Y: 23.5-33.5mm)
    add_line_scaled(drawing, 360.0, 23.5, 360.0, 33.5, WHITE, "TITLEBLOCK", scale, origin_x, origin_y);
    add_line_scaled(drawing, 380.0, 23.5, 380.0, 33.5, WHITE, "TITLEBLOCK", scale, origin_x, origin_y);

    // === ラベルテキスト（左列）===
    // 32f, Y.7f → 320mm, (Y*10+7)mm に左揃え
    add_text_scaled(drawing, 320.0, 67.0, "工事名", text_size, WHITE, "TITLEBLOCK",
                    HorizontalTextJustification::Left, VerticalTextJustification::Middle,
                    scale, origin_x, origin_y);
    add_text_scaled(drawing, 320.0, 57.0, "工種", text_size, WHITE, "TITLEBLOCK",
                    HorizontalTextJustification::Left, VerticalTextJustification::Middle,
                    scale, origin_x, origin_y);
    add_text_scaled(drawing, 320.0, 47.0, "路線", text_size, WHITE, "TITLEBLOCK",
                    HorizontalTextJustification::Left, VerticalTextJustification::Middle,
                    scale, origin_x, origin_y);
    add_text_scaled(drawing, 320.0, 37.0, "日付", text_size, WHITE, "TITLEBLOCK",
                    HorizontalTextJustification::Left, VerticalTextJustification::Middle,
                    scale, origin_x, origin_y);
    add_text_scaled(drawing, 320.0, 27.0, "縮尺", text_size, WHITE, "TITLEBLOCK",
                    HorizontalTextJustification::Left, VerticalTextJustification::Middle,
                    scale, origin_x, origin_y);
    add_text_scaled(drawing, 370.0, 27.0, "図番", text_size, WHITE, "TITLEBLOCK",
                    HorizontalTextJustification::Left, VerticalTextJustification::Middle,
                    scale, origin_x, origin_y);
    add_text_scaled(drawing, 320.0, 17.0, "作成", text_size, WHITE, "TITLEBLOCK",
                    HorizontalTextJustification::Left, VerticalTextJustification::Middle,
                    scale, origin_x, origin_y);

    // === 内容テキスト（右列）===
    // 33.5f → 335mm（データ列の中央）
    let data_x = 335.0;

    // 工事名（長い場合は改行処理）
    let project_name_display = truncate_or_split(&info.project_name, 25);
    add_text_scaled(drawing, data_x, 67.0, &project_name_display, text_size, WHITE, "TITLEBLOCK",
                    HorizontalTextJustification::Left, VerticalTextJustification::Middle,
                    scale, origin_x, origin_y);

    // 工種
    add_text_scaled(drawing, data_x, 57.0, &info.work_type, text_size, WHITE, "TITLEBLOCK",
                    HorizontalTextJustification::Left, VerticalTextJustification::Middle,
                    scale, origin_x, origin_y);

    // 路線名
    add_text_scaled(drawing, data_x, 47.0, &info.drawing_name, text_size, WHITE, "TITLEBLOCK",
                    HorizontalTextJustification::Left, VerticalTextJustification::Middle,
                    scale, origin_x, origin_y);

    // 日付
    add_text_scaled(drawing, data_x, 37.0, &info.date, text_size, WHITE, "TITLEBLOCK",
                    HorizontalTextJustification::Left, VerticalTextJustification::Middle,
                    scale, origin_x, origin_y);

    // 縮尺（345mm位置）
    add_text_scaled(drawing, 345.0, 27.0, &info.scale, text_size, WHITE, "TITLEBLOCK",
                    HorizontalTextJustification::Left, VerticalTextJustification::Middle,
                    scale, origin_x, origin_y);

    // 図番（395mm位置）
    add_text_scaled(drawing, 395.0, 27.0, &info.drawing_number, text_size, WHITE, "TITLEBLOCK",
                    HorizontalTextJustification::Left, VerticalTextJustification::Middle,
                    scale, origin_x, origin_y);

    // 作成者
    add_text_scaled(drawing, data_x, 17.0, &info.author, text_size, WHITE, "TITLEBLOCK",
                    HorizontalTextJustification::Left, VerticalTextJustification::Middle,
                    scale, origin_x, origin_y);

    // クレジット（左下: 80mm, 10mm）
    if !info.credit.is_empty() {
        add_text_scaled(drawing, 80.0, 10.0, &info.credit, text_size, WHITE, "TITLEBLOCK",
                        HorizontalTextJustification::Left, VerticalTextJustification::Middle,
                        scale, origin_x, origin_y);
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

/// 図枠全体を描画（外枠＋上部タイトル＋右下タイトル枠）
///
/// # Arguments
/// * `drawing` - DXFドローイング
/// * `info` - タイトルブロック情報
/// * `origin_x` - 原点X座標
/// * `origin_y` - 原点Y座標
/// * `scale` - スケール
/// * `text_size` - テキストサイズ（mm）
pub fn draw_drawing_frame(
    drawing: &mut Drawing,
    info: &TitleBlockInfo,
    origin_x: f64,
    origin_y: f64,
    scale: f64,
    text_size: f64,
) {
    add_title_block_layer(drawing);
    draw_outer_frame(drawing, origin_x, origin_y, scale);
    draw_top_title(drawing, info, origin_x, origin_y, scale, text_size);
    draw_title_block(drawing, info, origin_x, origin_y, scale, text_size);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_title_block_info_builder() {
        let info = TitleBlockInfo::new()
            .with_project_name("テスト工事")
            .with_work_type("横断図")
            .with_drawing_name("市道1号線")
            .with_date("2024年1月1日")
            .with_scale("1:100")
            .with_drawing_number("1/3")
            .with_author("テスト太郎");

        assert_eq!(info.project_name, "テスト工事");
        assert_eq!(info.work_type, "横断図");
        assert_eq!(info.drawing_name, "市道1号線");
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
            .with_work_type("横断図");

        draw_drawing_frame(&mut drawing, &info, 0.0, 0.0, 1.0, 3.0);

        // レイヤーが追加されていることを確認
        assert!(drawing.layers().any(|l| l.name == "TITLEBLOCK"));

        // エンティティが追加されていることを確認
        let entity_count = drawing.entities().count();
        assert!(entity_count > 0, "エンティティが追加されていない");
    }
}
