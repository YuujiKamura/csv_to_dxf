//! Intermediate Representation for Drawing Primitives
//!
//! This module provides a platform-agnostic representation of drawing elements
//! that can be converted to both DXF format and rendered to egui.
//!
//! # Font Metrics
//!
//! DXF output requires font metrics conversion for proper text sizing.
//! This module provides a configurable `FontMetrics` trait with a default
//! implementation. Projects can provide custom implementations for specific fonts.

use eframe::egui::{self, Color32, Painter, Pos2, Stroke, Rect, Vec2};
use dxf::Drawing;
use dxf::entities::{Entity, EntityType, Line, Text};
use dxf::enums::{HorizontalTextJustification, VerticalTextJustification};
use dxf::{Color, Point};

// ============================================================================
// Font Metrics Abstraction
// ============================================================================

/// Font metrics configuration for DXF text height conversion
///
/// DXF text height is based on cap height (height of capital letters),
/// while egui uses em-square for font sizing. This trait provides
/// the conversion factor between them.
pub trait FontMetrics {
    /// Convert logical cap height to DXF text_height
    ///
    /// Default implementation uses a scale factor of 1.364 (1000/733),
    /// which matches Noto Sans JP metrics.
    fn cap_height_to_text_height(&self, cap_height: f64) -> f64 {
        cap_height * self.scale_for_cap_height()
    }

    /// Get the scale factor for cap height conversion
    ///
    /// Default: 1.364 (em-square 1000 / cap-height 733 for Noto Sans JP)
    fn scale_for_cap_height(&self) -> f64 {
        1000.0 / 733.0  // ≈ 1.364
    }
}

/// Default font metrics implementation using Noto Sans JP values
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultFontMetrics;

impl FontMetrics for DefaultFontMetrics {}

/// Global font metrics instance (can be replaced for custom fonts)
static DEFAULT_FONT_METRICS: DefaultFontMetrics = DefaultFontMetrics;

/// Get the default font metrics
pub fn default_font_metrics() -> &'static impl FontMetrics {
    &DEFAULT_FONT_METRICS
}

/// Convert cap height to DXF text height using default font metrics
///
/// This is a convenience function that uses `DefaultFontMetrics`.
/// For custom fonts, use `FontMetrics::cap_height_to_text_height` directly.
#[inline]
pub fn cap_height_to_dxf_height(cap_height: f64) -> f64 {
    DEFAULT_FONT_METRICS.cap_height_to_text_height(cap_height)
}

/// Scale factor for cap height (inverse conversion from DXF to logical)
#[inline]
pub fn scale_for_cap_height() -> f64 {
    DEFAULT_FONT_METRICS.scale_for_cap_height()
}

// ============================================================================
// Section Data Abstraction
// ============================================================================

/// Data point within a section (survey point)
///
/// This trait abstracts the data needed to render a single point
/// in a cross-section drawing.
pub trait SectionPoint {
    /// Elevation (ground height) at this point
    fn elevation(&self) -> f64;

    /// Planned height (design height) at this point
    fn planned_height(&self) -> f64;

    /// Cumulative distance from section start (can be negative for left side)
    fn cumulative_distance(&self) -> f64;

    /// Bottom of cutting layer
    fn cutting_bottom(&self) -> f64;
}

/// Section data abstraction for grid layout calculations
///
/// This trait provides the minimal interface needed by `GridLayoutParams`
/// to calculate layout dimensions without depending on specific data structures.
pub trait SectionData {
    /// Type of points in this section
    type Point: SectionPoint;

    /// Get the data points for this section
    fn survey_points(&self) -> &[Self::Point];

    /// Get the datum/baseline level (DL)
    fn datum_level(&self) -> f64;

    /// Get the center line index
    fn cl_index(&self) -> usize;

    /// Get the name of this section/survey point
    fn name(&self) -> &str;

    /// Calculate the body height (max elevation - datum level)
    fn body_height(&self) -> f64 {
        let points = self.survey_points();
        if points.len() < 2 {
            return 0.0;
        }
        let max_elev = points
            .iter()
            .map(|p| p.elevation().max(p.planned_height()))
            .fold(f64::MIN, f64::max);
        max_elev - self.datum_level()
    }

    /// Get leftmost distance (typically negative)
    fn left_distance(&self) -> f64 {
        self.survey_points()
            .first()
            .map(|p| p.cumulative_distance())
            .unwrap_or(0.0)
    }

    /// Get rightmost distance
    fn right_distance(&self) -> f64 {
        self.survey_points()
            .last()
            .map(|p| p.cumulative_distance())
            .unwrap_or(0.0)
    }
}

// ============================================================================
// ViewState - local definition to avoid circular dependency with lib.rs
// ============================================================================

/// View state for coordinate transformation (DXF to screen)
/// This is a simplified local version to avoid circular dependency with DxfViewState in lib.rs
#[derive(Clone)]
pub struct ViewState {
    pub zoom: f32,
    pub pan: Vec2,
    pub canvas_rect: Rect,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: Vec2::ZERO,
            canvas_rect: Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 400.0)),
        }
    }
}

impl ViewState {
    /// Convert DXF coordinates to screen coordinates
    pub fn dxf_to_screen(&self, x: f64, y: f64) -> Pos2 {
        Pos2::new(
            self.canvas_rect.min.x + self.pan.x + (x as f32) * self.zoom,
            self.canvas_rect.min.y + self.pan.y - (y as f32) * self.zoom,
        )
    }
}

// ============================================================================
// Alignment Enums
// ============================================================================

/// Horizontal text alignment
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HAlign {
    Left,
    Center,
    Right,
}

/// Vertical text alignment
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VAlign {
    Top,
    Middle,
    Bottom,
}

// ============================================================================
// Section Drawing Style
// ============================================================================

/// 横断図描画スタイル設定
#[derive(Clone, Debug)]
pub struct SectionStyle {
    // テキスト
    pub text_height: f64,           // 基本テキスト高さ
    pub title_scale: f64,           // タイトル倍率
    pub small_text_scale: f64,      // 小テキスト倍率

    // ラベル配置（旗揚げベースからの相対Y座標）
    pub label_title_offset: f64,    // 測点名オフセット
    pub label_gh_offset: f64,       // GHオフセット
    pub label_fh_offset: f64,       // FHオフセット
    pub side_label_offset: f64,     // 左右端点のポインターオフセット

    // 旗揚げ
    pub flag_base_offset: f64,      // 旗揚げベース位置（地盤高からのオフセット）

    // ポインター（逆三角形）
    pub pointer_size: f64,          // サイズ
    pub pointer_label_gap: f64,     // ラベル間隔

    // 勾配矢印
    pub arrow_length: f64,          // 長さ
    pub arrow_drop: f64,            // 傾き
    pub arrow_head: f64,            // ヘッド長
    pub arrow_offset: f64,          // オフセット
    pub slope_text_offset: f64,     // 勾配テキストY位置（flag_yからのオフセット）

    // 切削厚
    pub cutting_label_offset: f64,  // DLからのオフセット
    pub cutting_label_gap: f64,     // ラベル間隔

    // 寸法線
    pub dim_tick_up: f64,           // 上方ティック
    pub dim_tick_down: f64,         // 下方ティック
    pub dim_text_offset: f64,       // テキストY位置オフセット
    pub dim_offset: f64,            // 寸法線の高さオフセット
    pub dim_arrow_size: f64,        // 矢印のサイズ
}

impl Default for SectionStyle {
    fn default() -> Self {
        Self {
            text_height: 300.0,
            title_scale: 1.5,
            small_text_scale: 0.5,
            label_title_offset: 1300.0,
            label_gh_offset: 800.0,
            label_fh_offset: 400.0,
            side_label_offset: 500.0,
            flag_base_offset: 1600.0,
            pointer_size: 150.0,
            pointer_label_gap: 300.0,
            arrow_length: 600.0,
            arrow_drop: 20.0,
            arrow_head: 180.0,
            arrow_offset: 300.0,
            slope_text_offset: 800.0,
            cutting_label_offset: 300.0,
            cutting_label_gap: 100.0,
            dim_tick_up: 50.0,
            dim_tick_down: 500.0,
            dim_text_offset: 50.0,
            dim_offset: 100.0,
            dim_arrow_size: 50.0,
        }
    }
}

impl SectionStyle {
    /// scale_multiplierを適用した新しいスタイルを返す
    pub fn scaled(&self, scale_multiplier: f64) -> Self {
        Self {
            text_height: self.text_height * scale_multiplier,
            title_scale: self.title_scale,  // 倍率はそのまま
            small_text_scale: self.small_text_scale,  // 倍率はそのまま
            label_title_offset: self.label_title_offset * scale_multiplier,
            label_gh_offset: self.label_gh_offset * scale_multiplier,
            label_fh_offset: self.label_fh_offset * scale_multiplier,
            side_label_offset: self.side_label_offset * scale_multiplier,
            flag_base_offset: self.flag_base_offset * scale_multiplier,
            pointer_size: self.pointer_size * scale_multiplier,
            pointer_label_gap: self.pointer_label_gap * scale_multiplier,
            arrow_length: self.arrow_length * scale_multiplier,
            arrow_drop: self.arrow_drop * scale_multiplier,
            arrow_head: self.arrow_head * scale_multiplier,
            arrow_offset: self.arrow_offset * scale_multiplier,
            slope_text_offset: self.slope_text_offset * scale_multiplier,
            cutting_label_offset: self.cutting_label_offset * scale_multiplier,
            cutting_label_gap: self.cutting_label_gap * scale_multiplier,
            dim_tick_up: self.dim_tick_up,  // 寸法線ティックはスケールしない
            dim_tick_down: self.dim_tick_down,  // 寸法線ティックはスケールしない
            dim_text_offset: self.dim_text_offset,  // 寸法線テキストはスケールしない
            dim_offset: self.dim_offset * scale_multiplier,
            dim_arrow_size: self.dim_arrow_size * scale_multiplier,
        }
    }
}

// ============================================================================
// Drawing Primitives
// ============================================================================

/// Line primitive with start/end points, color, and layer
#[derive(Clone, Debug)]
pub struct DrawLine {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub color: i16,   // ACI color index
    pub layer: String,
}

/// Text primitive with position, content, styling, and alignment
#[derive(Clone, Debug)]
pub struct DrawText {
    pub x: f64,
    pub y: f64,
    pub text: String,
    pub height: f64,
    pub rotation: f64,  // degrees
    pub color: i16,
    pub layer: String,
    pub h_align: HAlign,
    pub v_align: VAlign,
}

/// Enum representing all drawing primitive types
#[derive(Clone, Debug)]
pub enum DrawPrimitive {
    Line(DrawLine),
    Text(DrawText),
}

// ============================================================================
// Drawing Content Container
// ============================================================================

/// Container for drawing primitives with methods for adding and converting
#[derive(Clone, Debug, Default)]
pub struct DrawingContent {
    pub primitives: Vec<DrawPrimitive>,
}

impl DrawingContent {
    /// Create a new empty DrawingContent
    pub fn new() -> Self {
        Self {
            primitives: Vec::new(),
        }
    }

    /// Add a line primitive
    pub fn add_line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, color: i16, layer: &str) {
        self.primitives.push(DrawPrimitive::Line(DrawLine {
            x1,
            y1,
            x2,
            y2,
            color,
            layer: layer.to_string(),
        }));
    }

    /// Add a text primitive with horizontal alignment (vertical defaults to Middle)
    pub fn add_text(
        &mut self,
        x: f64,
        y: f64,
        text: &str,
        height: f64,
        color: i16,
        layer: &str,
        align: HAlign,
    ) {
        self.primitives.push(DrawPrimitive::Text(DrawText {
            x,
            y,
            text: text.to_string(),
            height,
            rotation: 0.0,
            color,
            layer: layer.to_string(),
            h_align: align,
            v_align: VAlign::Middle,
        }));
    }

    /// Add a rotated text primitive with full alignment control
    pub fn add_text_rotated(
        &mut self,
        x: f64,
        y: f64,
        text: &str,
        height: f64,
        color: i16,
        layer: &str,
        h_align: HAlign,
        v_align: VAlign,
        rotation: f64,
    ) {
        self.primitives.push(DrawPrimitive::Text(DrawText {
            x,
            y,
            text: text.to_string(),
            height,
            rotation,
            color,
            layer: layer.to_string(),
            h_align,
            v_align,
        }));
    }

    /// Add a rectangle primitive (4 lines forming a box)
    pub fn add_rect(
        &mut self,
        center_x: f64,
        center_y: f64,
        width: f64,
        height: f64,
        color: i16,
        layer: &str,
    ) {
        let half_w = width / 2.0;
        let half_h = height / 2.0;
        // Bottom line
        self.add_line(
            center_x - half_w,
            center_y - half_h,
            center_x + half_w,
            center_y - half_h,
            color,
            layer,
        );
        // Right line
        self.add_line(
            center_x + half_w,
            center_y - half_h,
            center_x + half_w,
            center_y + half_h,
            color,
            layer,
        );
        // Top line
        self.add_line(
            center_x + half_w,
            center_y + half_h,
            center_x - half_w,
            center_y + half_h,
            color,
            layer,
        );
        // Left line
        self.add_line(
            center_x - half_w,
            center_y + half_h,
            center_x - half_w,
            center_y - half_h,
            color,
            layer,
        );
    }

    /// Add a text primitive with explicit vertical alignment
    pub fn add_text_with_v_align(
        &mut self,
        x: f64,
        y: f64,
        text: &str,
        height: f64,
        color: i16,
        layer: &str,
        h_align: HAlign,
        v_align: VAlign,
    ) {
        self.primitives.push(DrawPrimitive::Text(DrawText {
            x,
            y,
            text: text.to_string(),
            height,
            rotation: 0.0,
            color,
            layer: layer.to_string(),
            h_align,
            v_align,
        }));
    }

    /// Merge another DrawingContent into this one
    pub fn merge(&mut self, other: DrawingContent) {
        self.primitives.extend(other.primitives);
    }

    /// Calculate bounding box of all primitives
    pub fn calc_bounds(&self) -> (f32, f32, f32, f32) {
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for prim in &self.primitives {
            match prim {
                DrawPrimitive::Line(line) => {
                    min_x = min_x.min(line.x1 as f32).min(line.x2 as f32);
                    min_y = min_y.min(line.y1 as f32).min(line.y2 as f32);
                    max_x = max_x.max(line.x1 as f32).max(line.x2 as f32);
                    max_y = max_y.max(line.y1 as f32).max(line.y2 as f32);
                }
                DrawPrimitive::Text(text) => {
                    let base_x = text.x as f32;
                    let base_y = text.y as f32;
                    let text_height = text.height as f32;
                    let estimated_width = estimate_text_width_f32(&text.text, text.height);

                    let (text_left, text_right) = match text.h_align {
                        HAlign::Left => (base_x, base_x + estimated_width),
                        HAlign::Center => (base_x - estimated_width / 2.0, base_x + estimated_width / 2.0),
                        HAlign::Right => (base_x - estimated_width, base_x),
                    };

                    let (text_bottom, text_top) = match text.v_align {
                        VAlign::Bottom => (base_y - text_height, base_y),
                        VAlign::Middle => (base_y - text_height / 2.0, base_y + text_height / 2.0),
                        VAlign::Top => (base_y, base_y + text_height),
                    };

                    min_x = min_x.min(text_left);
                    min_y = min_y.min(text_bottom);
                    max_x = max_x.max(text_right);
                    max_y = max_y.max(text_top);
                }
            }
        }

        if min_x == f32::MAX {
            (0.0, 0.0, 100.0, 100.0)
        } else {
            (min_x, min_y, max_x, max_y)
        }
    }
}

// ============================================================================
// DXF Output
// ============================================================================

impl DrawingContent {
    /// Convert to DXF Drawing
    pub fn to_dxf(&self) -> Drawing {
        let mut drawing = new_drawing();
        self.add_to_dxf(&mut drawing);
        drawing
    }

    /// Add primitives to an existing DXF Drawing
    pub fn add_to_dxf(&self, drawing: &mut Drawing) {
        for prim in &self.primitives {
            match prim {
                DrawPrimitive::Line(line) => {
                    add_line_to_dxf(drawing, line);
                }
                DrawPrimitive::Text(text) => {
                    add_text_to_dxf(drawing, text);
                }
            }
        }
    }

    /// Add primitives to DXF with origin and scale transformation
    ///
    /// origin_x, origin_y: DXF座標系での配置原点
    /// scale: 座標とテキスト高さに乗じるスケール
    pub fn add_to_dxf_transformed(
        &self,
        drawing: &mut Drawing,
        origin_x: f64,
        origin_y: f64,
        scale: f64,
    ) {
        for prim in &self.primitives {
            match prim {
                DrawPrimitive::Line(line) => {
                    // 座標変換: origin + coord * scale
                    let mut dxf_line = Line::default();
                    dxf_line.p1 = Point::new(
                        origin_x + line.x1 * scale,
                        origin_y + line.y1 * scale,
                        0.0,
                    );
                    dxf_line.p2 = Point::new(
                        origin_x + line.x2 * scale,
                        origin_y + line.y2 * scale,
                        0.0,
                    );
                    let mut entity = Entity::new(EntityType::Line(dxf_line));
                    entity.common.layer = line.layer.clone();
                    entity.common.color = Color::from_index(line.color as u8);
                    drawing.add_entity(entity);
                }
                DrawPrimitive::Text(text) => {
                    // テキスト座標とサイズも変換
                    let transformed_x = origin_x + text.x * scale;
                    let transformed_y = origin_y + text.y * scale;
                    let scaled_height = cap_height_to_dxf_height(text.height * scale);

                    let mut t = Text::default();
                    t.location = Point::new(transformed_x, transformed_y, 0.0);
                    t.text_height = scaled_height;
                    t.value = text.text.clone();
                    t.rotation = text.rotation;
                    t.text_style_name = "NOTOSANSJP".to_string();
                    t.relative_x_scale_factor = 1.0;

                    t.horizontal_text_justification = match text.h_align {
                        HAlign::Left => HorizontalTextJustification::Left,
                        HAlign::Center => HorizontalTextJustification::Center,
                        HAlign::Right => HorizontalTextJustification::Right,
                    };

                    // Second alignment point is required for non-baseline vertical alignment
                    t.second_alignment_point = Point::new(transformed_x, transformed_y, 0.0);

                    t.vertical_text_justification = match text.v_align {
                        VAlign::Top => VerticalTextJustification::Top,
                        VAlign::Middle => VerticalTextJustification::Middle,
                        VAlign::Bottom => VerticalTextJustification::Bottom,
                    };

                    let mut entity = Entity::new(EntityType::Text(t));
                    entity.common.layer = text.layer.clone();
                    entity.common.color = Color::from_index(text.color as u8);
                    drawing.add_entity(entity);
                }
            }
        }
    }
}

/// Initialize Drawing (common settings: version, text style)
fn new_drawing() -> Drawing {
    let mut drawing = Drawing::new();
    drawing.header.version = dxf::enums::AcadVersion::R2000;

    // Create text style (Noto Sans JP - Web/DXF common)
    let mut style = dxf::tables::Style::default();
    style.name = "NOTOSANSJP".to_string();
    style.primary_font_file_name = "Noto Sans JP".to_string();
    style.width_factor = 1.0;
    drawing.add_style(style);

    drawing
}

fn add_line_to_dxf(drawing: &mut Drawing, line: &DrawLine) {
    let mut dxf_line = Line::default();
    dxf_line.p1 = Point::new(line.x1, line.y1, 0.0);
    dxf_line.p2 = Point::new(line.x2, line.y2, 0.0);
    let mut entity = Entity::new(EntityType::Line(dxf_line));
    entity.common.layer = line.layer.clone();
    entity.common.color = Color::from_index(line.color as u8);
    drawing.add_entity(entity);
}

fn add_text_to_dxf(drawing: &mut Drawing, text: &DrawText) {
    let mut t = Text::default();
    t.location = Point::new(text.x, text.y, 0.0);
    t.text_height = cap_height_to_dxf_height(text.height);
    t.value = text.text.clone();
    t.rotation = text.rotation;
    t.text_style_name = "NOTOSANSJP".to_string();
    t.relative_x_scale_factor = 1.0;

    t.horizontal_text_justification = match text.h_align {
        HAlign::Left => HorizontalTextJustification::Left,
        HAlign::Center => HorizontalTextJustification::Center,
        HAlign::Right => HorizontalTextJustification::Right,
    };

    // Second alignment point is required for non-baseline vertical alignment
    t.second_alignment_point = Point::new(text.x, text.y, 0.0);

    t.vertical_text_justification = match text.v_align {
        VAlign::Top => VerticalTextJustification::Top,
        VAlign::Middle => VerticalTextJustification::Middle,
        VAlign::Bottom => VerticalTextJustification::Bottom,
    };

    let mut entity = Entity::new(EntityType::Text(t));
    entity.common.layer = text.layer.clone();
    entity.common.color = Color::from_index(text.color as u8);
    drawing.add_entity(entity);
}

// ============================================================================
// egui Rendering
// ============================================================================

impl DrawingContent {
    /// Render to egui Painter using ViewState for coordinate transformation
    pub fn render_egui(&self, painter: &Painter, view: &ViewState) {
        for prim in &self.primitives {
            match prim {
                DrawPrimitive::Line(line) => {
                    let p1 = view.dxf_to_screen(line.x1, line.y1);
                    let p2 = view.dxf_to_screen(line.x2, line.y2);
                    let color = aci_to_color32(line.color);
                    painter.line_segment([p1, p2], Stroke::new(1.5, color));
                }
                DrawPrimitive::Text(text) => {
                    render_text_egui(painter, view, text);
                }
            }
        }
    }
}

fn render_text_egui(painter: &Painter, view: &ViewState, text: &DrawText) {
    let base_pos = view.dxf_to_screen(text.x, text.y);
    let color = aci_to_color32(text.color);

    // text.height is the logical cap height, scale by zoom for screen display
    let font_size = (text.height as f32 * view.zoom).max(1.0);
    let font = egui::FontId::proportional(font_size);

    // Get Galley
    let galley = painter.layout_no_wrap(text.text.clone(), font, color);
    let text_width = galley.rect.width();
    let text_height = galley.rect.height();

    // Calculate offset based on alignment (in pre-rotation coordinate system)
    let offset_x = match text.h_align {
        HAlign::Left => 0.0,
        HAlign::Center => -text_width / 2.0,
        HAlign::Right => -text_width,
    };
    let offset_y = match text.v_align {
        VAlign::Bottom => -text_height,
        VAlign::Middle => -text_height / 2.0,
        VAlign::Top => 0.0,
    };

    // Rotation angle (text head facing right = station progression direction)
    let angle_rad = -(text.rotation as f32).to_radians();

    // Rotate the offset (calculate drawing position after rotation)
    let cos_a = angle_rad.cos();
    let sin_a = angle_rad.sin();
    let rotated_offset_x = offset_x * cos_a - offset_y * sin_a;
    let rotated_offset_y = offset_x * sin_a + offset_y * cos_a;

    let pos = Pos2::new(
        base_pos.x + rotated_offset_x,
        base_pos.y + rotated_offset_y,
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

// ============================================================================
// Utility Functions
// ============================================================================

/// Convert ACI (AutoCAD Color Index) to egui Color32 (for white background)
pub fn aci_to_color32(aci: i16) -> Color32 {
    match aci {
        1 => Color32::from_rgb(255, 0, 0),      // red
        2 => Color32::from_rgb(255, 255, 0),    // yellow
        3 => Color32::from_rgb(0, 255, 0),      // green
        4 => Color32::from_rgb(0, 255, 255),    // cyan
        5 => Color32::from_rgb(0, 0, 255),      // blue
        6 => Color32::from_rgb(255, 0, 255),    // magenta
        7 => Color32::from_rgb(0, 0, 0),        // black (for white background)
        8 => Color32::from_rgb(128, 128, 128),  // gray
        9 => Color32::from_rgb(192, 192, 192),  // light gray
        _ => Color32::from_rgb(100, 100, 100),
    }
}

/// Estimate text width (character count * height * coefficient)
fn estimate_text_width_f32(text: &str, height: f64) -> f32 {
    let char_count: f64 = text
        .chars()
        .map(|c| if c.is_ascii() { 0.6 } else { 1.0 })
        .sum();
    (char_count * height) as f32
}

// ============================================================================
// Conversion from DXF Drawing
// ============================================================================

impl DrawingContent {
    /// Create DrawingContent from a DXF Drawing
    /// Converts Line, Text, and RotatedDimension entities to primitives
    pub fn from_drawing(drawing: &Drawing) -> Self {
        use dxf::entities::EntityType;
        let scale_factor = scale_for_cap_height();

        let mut content = DrawingContent::new();

        for entity in drawing.entities() {
            let color_index = match entity.common.color.index() {
                Some(idx) => idx as i16,
                None => 7,
            };
            let layer = entity.common.layer.clone();

            match &entity.specific {
                EntityType::Line(line) => {
                    content.add_line(
                        line.p1.x,
                        line.p1.y,
                        line.p2.x,
                        line.p2.y,
                        color_index,
                        &layer,
                    );
                }
                EntityType::Text(text) => {
                    // DXF text_height is scaled for CAD (x scale_factor), reverse it for IR
                    let logical_height = text.text_height / scale_factor;

                    let h_align = match text.horizontal_text_justification {
                        HorizontalTextJustification::Left => HAlign::Left,
                        HorizontalTextJustification::Center => HAlign::Center,
                        HorizontalTextJustification::Right => HAlign::Right,
                        _ => HAlign::Left,
                    };
                    let v_align = match text.vertical_text_justification {
                        VerticalTextJustification::Baseline => VAlign::Bottom,
                        VerticalTextJustification::Bottom => VAlign::Bottom,
                        VerticalTextJustification::Middle => VAlign::Middle,
                        VerticalTextJustification::Top => VAlign::Top,
                    };

                    content.add_text_rotated(
                        text.location.x,
                        text.location.y,
                        &text.value,
                        logical_height,
                        color_index,
                        &layer,
                        h_align,
                        v_align,
                        text.rotation,
                    );
                }
                EntityType::RotatedDimension(dim) => {
                    // Convert RotatedDimension to lines and text
                    let p2 = &dim.definition_point_2;
                    let p3 = &dim.definition_point_3;
                    let ins = &dim.insertion_point;
                    let text_pt = &dim.dimension_base.text_mid_point;

                    let dim_y = ins.y;
                    let left_x = p2.x.min(p3.x);
                    let right_x = p2.x.max(p3.x);

                    // Dimension line
                    content.add_line(left_x, dim_y, right_x, dim_y, color_index, &layer);

                    // Extension lines
                    content.add_line(p2.x, p2.y, p2.x, dim_y + 50.0, color_index, &layer);
                    content.add_line(p3.x, p3.y, p3.x, dim_y + 50.0, color_index, &layer);

                    // Dimension text
                    let distance = (p3.x - p2.x).abs();
                    content.add_text(
                        text_pt.x,
                        text_pt.y,
                        &format!("{:.2}", distance / 1000.0),
                        150.0, // Same as render_dxf uses
                        color_index,
                        &layer,
                        HAlign::Center,
                    );
                }
                _ => {}
            }
        }

        content
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drawing_content_new() {
        let content = DrawingContent::new();
        assert!(content.primitives.is_empty());
    }

    #[test]
    fn test_add_line() {
        let mut content = DrawingContent::new();
        content.add_line(0.0, 0.0, 100.0, 100.0, 7, "0");
        assert_eq!(content.primitives.len(), 1);

        if let DrawPrimitive::Line(line) = &content.primitives[0] {
            assert_eq!(line.x1, 0.0);
            assert_eq!(line.y1, 0.0);
            assert_eq!(line.x2, 100.0);
            assert_eq!(line.y2, 100.0);
            assert_eq!(line.color, 7);
            assert_eq!(line.layer, "0");
        } else {
            panic!("Expected Line primitive");
        }
    }

    #[test]
    fn test_add_text() {
        let mut content = DrawingContent::new();
        content.add_text(50.0, 50.0, "Test", 10.0, 7, "0", HAlign::Center);
        assert_eq!(content.primitives.len(), 1);

        if let DrawPrimitive::Text(text) = &content.primitives[0] {
            assert_eq!(text.x, 50.0);
            assert_eq!(text.y, 50.0);
            assert_eq!(text.text, "Test");
            assert_eq!(text.height, 10.0);
            assert_eq!(text.rotation, 0.0);
            assert_eq!(text.color, 7);
            assert_eq!(text.layer, "0");
            assert_eq!(text.h_align, HAlign::Center);
            assert_eq!(text.v_align, VAlign::Middle);
        } else {
            panic!("Expected Text primitive");
        }
    }

    #[test]
    fn test_add_text_rotated() {
        let mut content = DrawingContent::new();
        content.add_text_rotated(50.0, 50.0, "Rotated", 10.0, 1, "LABEL", HAlign::Left, VAlign::Bottom, 90.0);
        assert_eq!(content.primitives.len(), 1);

        if let DrawPrimitive::Text(text) = &content.primitives[0] {
            assert_eq!(text.rotation, 90.0);
            assert_eq!(text.h_align, HAlign::Left);
            assert_eq!(text.v_align, VAlign::Bottom);
        } else {
            panic!("Expected Text primitive");
        }
    }

    #[test]
    fn test_merge() {
        let mut content1 = DrawingContent::new();
        content1.add_line(0.0, 0.0, 100.0, 100.0, 7, "0");

        let mut content2 = DrawingContent::new();
        content2.add_text(50.0, 50.0, "Test", 10.0, 7, "0", HAlign::Center);

        content1.merge(content2);
        assert_eq!(content1.primitives.len(), 2);
    }

    #[test]
    fn test_aci_to_color32() {
        assert_eq!(aci_to_color32(1), Color32::from_rgb(255, 0, 0));
        assert_eq!(aci_to_color32(7), Color32::from_rgb(0, 0, 0));
        assert_eq!(aci_to_color32(999), Color32::from_rgb(100, 100, 100));
    }

    #[test]
    fn test_calc_bounds() {
        let mut content = DrawingContent::new();
        content.add_line(0.0, 0.0, 100.0, 50.0, 7, "0");
        content.add_line(200.0, 100.0, 300.0, 150.0, 7, "0");

        let (min_x, min_y, max_x, max_y) = content.calc_bounds();
        assert_eq!(min_x, 0.0);
        assert_eq!(min_y, 0.0);
        assert_eq!(max_x, 300.0);
        assert_eq!(max_y, 150.0);
    }

    #[test]
    fn test_to_dxf() {
        let mut content = DrawingContent::new();
        content.add_line(0.0, 0.0, 100.0, 100.0, 7, "0");
        content.add_text(50.0, 50.0, "Test", 10.0, 7, "0", HAlign::Center);

        let drawing = content.to_dxf();

        // Verify the drawing has entities
        let entities: Vec<_> = drawing.entities().collect();
        assert_eq!(entities.len(), 2);
    }

    #[test]
    fn test_add_to_dxf() {
        // Create an existing drawing with one line
        let mut drawing = Drawing::new();
        let mut existing_line = Line::default();
        existing_line.p1 = Point::new(0.0, 0.0, 0.0);
        existing_line.p2 = Point::new(50.0, 50.0, 0.0);
        drawing.add_entity(Entity::new(EntityType::Line(existing_line)));

        // Verify initial state
        let initial_count = drawing.entities().count();
        assert_eq!(initial_count, 1);

        // Add primitives to existing drawing
        let mut content = DrawingContent::new();
        content.add_line(100.0, 100.0, 200.0, 200.0, 7, "0");
        content.add_text(150.0, 150.0, "Added", 10.0, 1, "TEXT", HAlign::Left);
        content.add_to_dxf(&mut drawing);

        // Verify primitives were added
        let final_count = drawing.entities().count();
        assert_eq!(final_count, 3);
    }

    #[test]
    fn test_calc_bounds_with_text() {
        let mut content = DrawingContent::new();
        // Add text at position (100, 100) with height 10, left-aligned
        // Text "Test" has 4 ASCII chars, estimated width = 4 * 0.6 * 10 = 24
        content.add_text(100.0, 100.0, "Test", 10.0, 7, "0", HAlign::Left);

        let (min_x, min_y, max_x, max_y) = content.calc_bounds();

        // For HAlign::Left, VAlign::Middle (default):
        // text_left = 100, text_right = 100 + 24 = 124
        // text_bottom = 100 - 5, text_top = 100 + 5
        assert_eq!(min_x, 100.0);
        assert_eq!(max_x, 124.0);
        assert_eq!(min_y, 95.0);
        assert_eq!(max_y, 105.0);
    }

    #[test]
    fn test_calc_bounds_empty() {
        let content = DrawingContent::new();
        let (min_x, min_y, max_x, max_y) = content.calc_bounds();

        // Empty content returns default bounds
        assert_eq!(min_x, 0.0);
        assert_eq!(min_y, 0.0);
        assert_eq!(max_x, 100.0);
        assert_eq!(max_y, 100.0);
    }

    #[test]
    fn test_view_state_dxf_to_screen() {
        let view = ViewState {
            zoom: 2.0,
            pan: Vec2::new(10.0, 20.0),
            canvas_rect: Rect::from_min_size(Pos2::new(100.0, 50.0), Vec2::new(400.0, 400.0)),
        };

        // DXF coords (50, 30) should transform to:
        // screen_x = canvas_min_x + pan_x + x * zoom = 100 + 10 + 50 * 2 = 210
        // screen_y = canvas_min_y + pan_y - y * zoom = 50 + 20 - 30 * 2 = 10
        let screen_pos = view.dxf_to_screen(50.0, 30.0);
        assert_eq!(screen_pos.x, 210.0);
        assert_eq!(screen_pos.y, 10.0);
    }

    #[test]
    fn test_halign_variants() {
        // Test all HAlign variants can be created and compared
        assert_eq!(HAlign::Left, HAlign::Left);
        assert_eq!(HAlign::Center, HAlign::Center);
        assert_eq!(HAlign::Right, HAlign::Right);
        assert_ne!(HAlign::Left, HAlign::Center);
        assert_ne!(HAlign::Center, HAlign::Right);
        assert_ne!(HAlign::Left, HAlign::Right);

        // Test HAlign affects bounds calculation correctly
        let mut content_left = DrawingContent::new();
        content_left.add_text(100.0, 100.0, "Test", 10.0, 7, "0", HAlign::Left);
        let (left_min_x, _, left_max_x, _) = content_left.calc_bounds();

        let mut content_center = DrawingContent::new();
        content_center.add_text(100.0, 100.0, "Test", 10.0, 7, "0", HAlign::Center);
        let (center_min_x, _, center_max_x, _) = content_center.calc_bounds();

        let mut content_right = DrawingContent::new();
        content_right.add_text(100.0, 100.0, "Test", 10.0, 7, "0", HAlign::Right);
        let (right_min_x, _, right_max_x, _) = content_right.calc_bounds();

        // Left-aligned: starts at x, extends right
        assert_eq!(left_min_x, 100.0);
        assert!(left_max_x > 100.0);

        // Center-aligned: centered around x
        assert!(center_min_x < 100.0);
        assert!(center_max_x > 100.0);

        // Right-aligned: ends at x, extends left
        assert!(right_min_x < 100.0);
        assert_eq!(right_max_x, 100.0);
    }

    #[test]
    fn test_valign_variants() {
        // Test all VAlign variants can be created and compared
        assert_eq!(VAlign::Top, VAlign::Top);
        assert_eq!(VAlign::Middle, VAlign::Middle);
        assert_eq!(VAlign::Bottom, VAlign::Bottom);
        assert_ne!(VAlign::Top, VAlign::Middle);
        assert_ne!(VAlign::Middle, VAlign::Bottom);
        assert_ne!(VAlign::Top, VAlign::Bottom);

        // Test VAlign affects bounds calculation correctly
        let mut content_top = DrawingContent::new();
        content_top.add_text_rotated(100.0, 100.0, "Test", 10.0, 7, "0", HAlign::Left, VAlign::Top, 0.0);
        let (_, top_min_y, _, top_max_y) = content_top.calc_bounds();

        let mut content_middle = DrawingContent::new();
        content_middle.add_text_rotated(100.0, 100.0, "Test", 10.0, 7, "0", HAlign::Left, VAlign::Middle, 0.0);
        let (_, middle_min_y, _, middle_max_y) = content_middle.calc_bounds();

        let mut content_bottom = DrawingContent::new();
        content_bottom.add_text_rotated(100.0, 100.0, "Test", 10.0, 7, "0", HAlign::Left, VAlign::Bottom, 0.0);
        let (_, bottom_min_y, _, bottom_max_y) = content_bottom.calc_bounds();

        // Top-aligned: text_bottom = y, text_top = y + height
        assert_eq!(top_min_y, 100.0);
        assert_eq!(top_max_y, 110.0);

        // Middle-aligned: text_bottom = y - height/2, text_top = y + height/2
        assert_eq!(middle_min_y, 95.0);
        assert_eq!(middle_max_y, 105.0);

        // Bottom-aligned: text_bottom = y - height, text_top = y
        assert_eq!(bottom_min_y, 90.0);
        assert_eq!(bottom_max_y, 100.0);
    }

    // =========================================================================
    // DXF Output Integration Tests
    // =========================================================================

    #[test]
    fn test_drawing_content_to_dxf_entities() {
        // Create DrawingContent with multiple primitives
        let mut content = DrawingContent::new();
        content.add_line(0.0, 0.0, 100.0, 100.0, 7, "0");
        content.add_text(50.0, 50.0, "Test", 10.0, 7, "0", HAlign::Center);

        // Convert to DXF and verify entity count
        let drawing = content.to_dxf();
        let entities: Vec<_> = drawing.entities().collect();
        assert_eq!(entities.len(), 2, "Expected 2 entities (1 line + 1 text)");

        // Verify entity types
        let mut line_count = 0;
        let mut text_count = 0;
        for entity in drawing.entities() {
            match &entity.specific {
                EntityType::Line(_) => line_count += 1,
                EntityType::Text(_) => text_count += 1,
                _ => {}
            }
        }
        assert_eq!(line_count, 1, "Expected 1 line entity");
        assert_eq!(text_count, 1, "Expected 1 text entity");
    }

    #[test]
    fn test_drawing_content_line_coordinates() {
        // Create DrawingContent with a line
        let mut content = DrawingContent::new();
        content.add_line(10.0, 20.0, 30.0, 40.0, 7, "LAYER1");

        // Convert to DXF
        let drawing = content.to_dxf();

        // Extract and verify line coordinates
        for entity in drawing.entities() {
            if let EntityType::Line(line) = &entity.specific {
                // Verify p1 coordinates
                assert!((line.p1.x - 10.0).abs() < 1e-10, "Line p1.x should be 10.0");
                assert!((line.p1.y - 20.0).abs() < 1e-10, "Line p1.y should be 20.0");
                assert!((line.p1.z - 0.0).abs() < 1e-10, "Line p1.z should be 0.0");

                // Verify p2 coordinates
                assert!((line.p2.x - 30.0).abs() < 1e-10, "Line p2.x should be 30.0");
                assert!((line.p2.y - 40.0).abs() < 1e-10, "Line p2.y should be 40.0");
                assert!((line.p2.z - 0.0).abs() < 1e-10, "Line p2.z should be 0.0");

                // Verify layer
                assert_eq!(entity.common.layer, "LAYER1", "Layer should be LAYER1");

                return;
            }
        }
        panic!("No line entity found in DXF");
    }

    #[test]
    fn test_drawing_content_text_properties() {
        // Create DrawingContent with rotated text and full alignment
        let mut content = DrawingContent::new();
        content.add_text_rotated(
            100.0, 200.0,
            "TestText",
            15.0,   // height
            1,      // color (red)
            "TEXT_LAYER",
            HAlign::Right,
            VAlign::Top,
            45.0,   // rotation
        );

        // Convert to DXF
        let drawing = content.to_dxf();

        // Extract and verify text properties
        for entity in drawing.entities() {
            if let EntityType::Text(text) = &entity.specific {
                // Verify text value
                assert_eq!(text.value, "TestText", "Text value should match");

                // Verify height (scaled by scale_for_cap_height)
                let expected_height = 15.0 * scale_for_cap_height();
                assert!(
                    (text.text_height - expected_height).abs() < 0.01,
                    "Text height should be {} (got {})", expected_height, text.text_height
                );

                // Verify rotation
                assert!(
                    (text.rotation - 45.0).abs() < 1e-10,
                    "Text rotation should be 45.0"
                );

                // Verify horizontal alignment
                assert_eq!(
                    text.horizontal_text_justification,
                    HorizontalTextJustification::Right,
                    "Horizontal alignment should be Right"
                );

                // Verify vertical alignment
                assert_eq!(
                    text.vertical_text_justification,
                    VerticalTextJustification::Top,
                    "Vertical alignment should be Top"
                );

                // Verify location
                assert!((text.location.x - 100.0).abs() < 1e-10, "Text location.x should be 100.0");
                assert!((text.location.y - 200.0).abs() < 1e-10, "Text location.y should be 200.0");

                // Verify layer
                assert_eq!(entity.common.layer, "TEXT_LAYER", "Layer should be TEXT_LAYER");

                return;
            }
        }
        panic!("No text entity found in DXF");
    }

    #[test]
    fn test_dxf_serialization() {
        // Create DrawingContent with some content
        let mut content = DrawingContent::new();
        content.add_line(0.0, 0.0, 100.0, 100.0, 7, "0");
        content.add_text(50.0, 50.0, "Serialization Test", 10.0, 7, "0", HAlign::Center);

        // Convert to DXF
        let drawing = content.to_dxf();

        // Serialize to bytes
        let mut output: Vec<u8> = Vec::new();
        drawing.save(&mut output).expect("Failed to save DXF to bytes");

        // Verify output is not empty
        assert!(!output.is_empty(), "DXF output should not be empty");

        // Verify output contains DXF content markers
        let output_str = String::from_utf8_lossy(&output);
        assert!(output_str.contains("SECTION"), "DXF should contain SECTION");
        assert!(output_str.contains("ENTITIES"), "DXF should contain ENTITIES");
        assert!(output_str.contains("EOF"), "DXF should contain EOF");
    }

    #[test]
    fn test_empty_drawing_content_to_dxf() {
        // Create empty DrawingContent
        let content = DrawingContent::new();

        // Convert to DXF
        let drawing = content.to_dxf();

        // Verify no entities
        let entities: Vec<_> = drawing.entities().collect();
        assert_eq!(entities.len(), 0, "Empty DrawingContent should produce 0 entities");

        // Verify can still serialize
        let mut output: Vec<u8> = Vec::new();
        drawing.save(&mut output).expect("Failed to save empty DXF");
        assert!(!output.is_empty(), "Even empty DXF should produce output");
    }

    #[test]
    fn test_multiple_lines_to_dxf() {
        let mut content = DrawingContent::new();
        content.add_line(0.0, 0.0, 10.0, 10.0, 1, "RED");
        content.add_line(20.0, 20.0, 30.0, 30.0, 2, "YELLOW");
        content.add_line(40.0, 40.0, 50.0, 50.0, 3, "GREEN");

        let drawing = content.to_dxf();
        let entities: Vec<_> = drawing.entities().collect();
        assert_eq!(entities.len(), 3, "Expected 3 line entities");

        // Verify each line has correct layer
        let mut layers: Vec<String> = Vec::new();
        for entity in drawing.entities() {
            layers.push(entity.common.layer.clone());
        }
        assert!(layers.contains(&"RED".to_string()));
        assert!(layers.contains(&"YELLOW".to_string()));
        assert!(layers.contains(&"GREEN".to_string()));
    }

    #[test]
    fn test_text_alignment_variants_in_dxf() {
        let mut content = DrawingContent::new();

        // Add texts with different alignments
        content.add_text(0.0, 0.0, "Left", 10.0, 7, "0", HAlign::Left);
        content.add_text(50.0, 0.0, "Center", 10.0, 7, "0", HAlign::Center);
        content.add_text(100.0, 0.0, "Right", 10.0, 7, "0", HAlign::Right);

        let drawing = content.to_dxf();

        let mut left_found = false;
        let mut center_found = false;
        let mut right_found = false;

        for entity in drawing.entities() {
            if let EntityType::Text(text) = &entity.specific {
                match text.value.as_str() {
                    "Left" => {
                        assert_eq!(text.horizontal_text_justification, HorizontalTextJustification::Left);
                        left_found = true;
                    }
                    "Center" => {
                        assert_eq!(text.horizontal_text_justification, HorizontalTextJustification::Center);
                        center_found = true;
                    }
                    "Right" => {
                        assert_eq!(text.horizontal_text_justification, HorizontalTextJustification::Right);
                        right_found = true;
                    }
                    _ => {}
                }
            }
        }

        assert!(left_found, "Left-aligned text not found");
        assert!(center_found, "Center-aligned text not found");
        assert!(right_found, "Right-aligned text not found");
    }

    // =========================================================================
    // SectionStyle Tests
    // =========================================================================

    #[test]
    fn test_section_style_default() {
        let style = SectionStyle::default();
        assert_eq!(style.text_height, 300.0);
        assert_eq!(style.title_scale, 1.5);
        assert_eq!(style.pointer_size, 150.0);
        assert_eq!(style.flag_base_offset, 1600.0);
    }

    #[test]
    fn test_section_style_scaled() {
        let style = SectionStyle::default();
        let scaled = style.scaled(2.0);

        // Scaled values
        assert_eq!(scaled.text_height, 600.0);
        assert_eq!(scaled.pointer_size, 300.0);
        assert_eq!(scaled.flag_base_offset, 3200.0);

        // Unscaled values (ratios)
        assert_eq!(scaled.title_scale, 1.5);
        assert_eq!(scaled.small_text_scale, 0.5);
    }

    #[test]
    fn test_section_style_scaled_half() {
        let style = SectionStyle::default();
        let scaled = style.scaled(0.5);

        assert_eq!(scaled.text_height, 150.0);
        assert_eq!(scaled.pointer_size, 75.0);
    }

    #[test]
    fn test_section_style_clone() {
        let style = SectionStyle::default();
        let cloned = style.clone();
        assert_eq!(style.text_height, cloned.text_height);
        assert_eq!(style.arrow_length, cloned.arrow_length);
    }
}
