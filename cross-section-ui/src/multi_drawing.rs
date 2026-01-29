//! Multi-Section Drawing Generation
//!
//! This module provides functions for generating DXF drawings containing multiple
//! cross-sections arranged in a grid layout. It supports various configurations:
//!
//! - Basic grid layout without frame
//! - Grid layout with A3 drawing frame at various scales (1:200, 1:500, etc.)
//! - Scale multiplier for combined drawings
//! - Interpolation of intermediate section planned heights
//! - Configurable vertical scale ratio
//!
//! The grid layout follows road construction conventions:
//! - Origin at bottom-left
//! - Sections arranged column by column, bottom to top
//! - Columns arranged left to right
//!
//! For framed drawings, a fixed 4-column layout is used with column centers
//! aligned to the available drawing area.
//!
//! ## Builder Pattern (Recommended)
//!
//! Use `MultiDrawingBuilder` for a fluent, composable API:
//!
//! ```ignore
//! let drawing = MultiDrawingBuilder::new(&sections)
//!     .columns(4)
//!     .gaps(2.0, 1.0)
//!     .with_frame(500.0, &title_info)
//!     .interpolated()
//!     .v_scale(2.0)
//!     .build();
//! ```

use dxf::{Drawing, Color};

use crate::{
    CrossSectionData, TitleBlockInfo,
    new_drawing, add_line, draw_drawing_frame,
    title_block::available_area,
};
// Note: These imports use the current module structure.
// When grid_params, grid_bounds, and section_render modules are created,
// update imports to use crate::grid_params, crate::grid_bounds, crate::section_render
use crate::grid_params::{GridLayoutParams, DEFAULT_BOTTOM_MARGIN_DXF};
use crate::grid_bounds::{GridBounds, calc_grid_bounds, calc_column_widths, calc_column_x_offsets};
use crate::section_render::draw_section_at_offset;

// ============================================================================
// Builder Pattern
// ============================================================================

/// Builder for creating multi-section drawings with a fluent API.
///
/// This provides a more flexible and readable way to configure drawing generation
/// compared to the many individual functions.
///
/// # Example
///
/// ```ignore
/// let drawing = MultiDrawingBuilder::new(&sections)
///     .columns(4)
///     .gaps(2.0, 1.0)
///     .with_frame(500.0, &title_info)
///     .interpolated()
///     .v_scale(2.0)
///     .build();
/// ```
pub struct MultiDrawingBuilder<'a> {
    sections: &'a [CrossSectionData],
    columns: usize,
    column_gap: f64,
    row_gap: f64,
    frame_scale: Option<f64>,
    title_info: Option<&'a TitleBlockInfo>,
    scale_multiplier: f64,
    interpolate: bool,
    v_scale_ratio: f64,
}

impl<'a> MultiDrawingBuilder<'a> {
    /// Create a new builder with the given sections.
    ///
    /// Default values:
    /// - columns: 4
    /// - column_gap: 2.0
    /// - row_gap: 1.0
    /// - scale_multiplier: 1.0
    /// - interpolate: false
    /// - v_scale_ratio: 2.0
    pub fn new(sections: &'a [CrossSectionData]) -> Self {
        Self {
            sections,
            columns: 4,
            column_gap: 2.0,
            row_gap: 1.0,
            frame_scale: None,
            title_info: None,
            scale_multiplier: 1.0,
            interpolate: false,
            v_scale_ratio: 2.0,
        }
    }

    /// Set the number of columns in the grid layout.
    /// Note: Ignored for framed drawings (uses fixed 4 columns).
    pub fn columns(mut self, n: usize) -> Self {
        self.columns = n;
        self
    }

    /// Set column and row gaps in meters.
    pub fn gaps(mut self, column_gap: f64, row_gap: f64) -> Self {
        self.column_gap = column_gap;
        self.row_gap = row_gap;
        self
    }

    /// Add an A3 drawing frame at the specified scale.
    ///
    /// # Arguments
    /// * `scale` - Plot scale (e.g., 200.0 for 1:200, 500.0 for 1:500)
    /// * `title_info` - Title block information
    pub fn with_frame(mut self, scale: f64, title_info: &'a TitleBlockInfo) -> Self {
        self.frame_scale = Some(scale);
        self.title_info = Some(title_info);
        self
    }

    /// Enable interpolation of planned heights for intermediate sections.
    pub fn interpolated(mut self) -> Self {
        self.interpolate = true;
        self
    }

    /// Set the vertical scale ratio (1.0 to 5.0, default 2.0).
    pub fn v_scale(mut self, ratio: f64) -> Self {
        self.v_scale_ratio = ratio;
        self
    }

    /// Set the scale multiplier for combined drawings.
    pub fn scale_multiplier(mut self, s: f64) -> Self {
        self.scale_multiplier = s;
        self
    }

    /// Build the DXF Drawing.
    pub fn build(self) -> Drawing {
        generate_multi_drawing_internal(
            self.sections,
            self.columns,
            self.column_gap,
            self.row_gap,
            self.frame_scale,
            self.title_info,
            self.scale_multiplier,
            self.interpolate,
            self.v_scale_ratio,
        )
    }

    /// Build and return as DXF byte array.
    pub fn to_dxf_bytes(self) -> Vec<u8> {
        let drawing = self.build();
        let mut output: Vec<u8> = Vec::new();
        drawing.save(&mut output).expect("Failed to save DXF");
        output
    }
}

// ============================================================================
// Layer Setup
// ============================================================================

/// Setup standard layers for cross-section drawings
///
/// Adds the following layers with standard colors:
/// - GROUND (7 - white/black): Ground surface lines
/// - PLAN (1 - red): Planned height lines
/// - TEXT (7 - white/black): Text labels
/// - DIMENSION (8 - gray): Dimension lines
/// - CUTTING (5 - blue): Cutting bottom lines
/// - FRAME (9 - light gray): Frame elements
fn setup_layers(drawing: &mut Drawing) {
    for (name, color_idx) in [
        ("GROUND", 7),
        ("PLAN", 1),
        ("TEXT", 7),
        ("DIMENSION", 8),
        ("CUTTING", 5),
        ("FRAME", 9),
    ] {
        drawing.add_layer(dxf::tables::Layer {
            name: name.to_string(),
            color: Color::from_index(color_idx),
            ..Default::default()
        });
    }
}

// ============================================================================
// Internal Implementation
// ============================================================================

/// Internal implementation for multi-section drawing generation
///
/// All public drawing functions delegate to this implementation.
///
/// # Arguments
/// * `sections` - Cross-section data to draw
/// * `columns` - Number of columns (ignored for framed drawings)
/// * `column_gap` - Gap between columns in meters
/// * `row_gap` - Gap between rows in meters
/// * `frame_scale` - Optional plot scale for framed drawings
/// * `title_info` - Optional title block information
/// * `scale_multiplier` - Scale multiplier for combined drawings
/// * `interpolate_intermediate` - Whether to interpolate intermediate sections
/// * `v_scale_ratio` - Vertical scale ratio (1.0 to 5.0)
fn generate_multi_drawing_internal(
    sections: &[CrossSectionData],
    columns: usize,
    column_gap: f64,
    row_gap: f64,
    frame_scale: Option<f64>,
    title_info: Option<&TitleBlockInfo>,
    scale_multiplier: f64,
    interpolate_intermediate: bool,
    v_scale_ratio: f64,
) -> Drawing {
    let scale = 1000.0 * scale_multiplier;

    let mut drawing = new_drawing();
    setup_layers(&mut drawing);

    if sections.is_empty() {
        return drawing;
    }

    // For framed drawings, use fixed 4-column layout
    let columns = if frame_scale.is_some() {
        available_area::COLUMN_COUNT
    } else {
        columns
    };

    // Use GridLayoutParams for consistent calculations
    let params = GridLayoutParams::new(column_gap, row_gap).with_v_scale(v_scale_ratio);

    // Road construction layout: bottom-left origin, columns bottom to top
    let rows_per_column = (sections.len() + columns - 1) / columns;

    // Calculate column widths
    let (col_max_left, col_max_right) = calc_column_widths(sections, columns, rows_per_column);

    // Cell height from GridLayoutParams (scaled by scale_multiplier for combo drawings)
    let cell_height = params.cell_height_for_sections(sections) * scale_multiplier;

    // Calculate X and Y offsets based on frame mode
    let (col_x_offsets, col_y_offsets, grid_bounds) = if let Some(fs) = frame_scale {
        // Framed: use fixed column centers
        let col_centers: Vec<f64> = (0..columns)
            .map(|col| {
                if col < 4 {
                    available_area::COLUMN_CENTERS[col] * fs
                } else {
                    available_area::COLUMN_CENTERS[3] * fs
                }
            })
            .collect();

        // Get grid bounds for Y positioning
        let bounds = calc_grid_bounds(sections, columns, column_gap, row_gap);
        let content_height_mm = bounds.as_ref().map(|b| b.height() / fs).unwrap_or(0.0);

        // Calculate per-column Y offsets for centering
        let mut col_offsets: Vec<f64> = Vec::with_capacity(columns);
        for col in 0..columns {
            let col_height = available_area::height_for_column(col);
            let col_bottom = available_area::bottom_for_column(col);
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
        // Frameless: dynamic calculation based on content widths
        let col_x_offsets = calc_column_x_offsets(
            &col_max_left,
            &col_max_right,
            column_gap,
            scale,
        );
        (col_x_offsets, vec![0.0; columns], None)
    };

    // Draw each section
    for (idx, section) in sections.iter().enumerate() {
        if section.survey_data.len() < 2 {
            continue;
        }
        let col = idx / rows_per_column;
        let row_in_col = idx % rows_per_column;

        // X position from column centers
        let offset_x = if col < col_x_offsets.len() {
            col_x_offsets[col]
        } else {
            col_x_offsets.last().copied().unwrap_or(0.0)
        };

        // Y position with per-column offset and bottom margin
        let col_y_offset = if col < col_y_offsets.len() {
            col_y_offsets[col]
        } else {
            col_y_offsets.last().copied().unwrap_or(0.0)
        };
        let offset_y = row_in_col as f64 * cell_height + col_y_offset + DEFAULT_BOTTOM_MARGIN_DXF * scale_multiplier;

        // Interpolate intermediate sections (not route start/end)
        let is_start_or_end = section.is_route_start || section.is_route_end;
        if interpolate_intermediate && !is_start_or_end {
            let mut section_to_draw = section.clone();
            section_to_draw.interpolate_planned_heights();
            draw_section_at_offset(
                &mut drawing,
                &section_to_draw,
                offset_x,
                offset_y,
                scale,
                scale_multiplier,
                v_scale_ratio,
            );
        } else {
            draw_section_at_offset(
                &mut drawing,
                section,
                offset_x,
                offset_y,
                scale,
                scale_multiplier,
                v_scale_ratio,
            );
        }
    }

    // Draw frame if specified
    if let Some(fs) = frame_scale {
        const TEXT_SIZE_MM: f64 = 3.0;

        let info = title_info.cloned().unwrap_or_else(|| {
            TitleBlockInfo::new()
                .with_top_title("横断図")
                .with_scale(&format!("1:{} (A3)", fs as u32))
        });

        draw_drawing_frame(&mut drawing, &info, 0.0, 0.0, fs, TEXT_SIZE_MM);

        // Draw debug grid bounds if debug markers enabled
        if info.show_debug_markers {
            if let Some(bounds) = &grid_bounds {
                draw_debug_grid_bounds(
                    &mut drawing,
                    bounds,
                    &col_y_offsets,
                    columns,
                    fs,
                );
            }
        }
    }

    drawing
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Draw debug grid bounds rectangle
fn draw_debug_grid_bounds(
    drawing: &mut Drawing,
    bounds: &GridBounds,
    col_y_offsets: &[f64],
    columns: usize,
    frame_scale: f64,
) {
    const COLOR_RED: i16 = 1;

    let first_col = 0;
    let last_col = (columns - 1).min(3);
    let grid_min_x = available_area::COLUMN_LEFTS[first_col] * frame_scale;
    let grid_max_x = available_area::COLUMN_RIGHTS[last_col] * frame_scale;

    let min_y_offset = col_y_offsets.iter().copied().fold(f64::MAX, f64::min);
    let max_y_offset = col_y_offsets.iter().copied().fold(f64::MIN, f64::max);

    let grid_min_y = bounds.min_y + min_y_offset;
    let grid_max_y = bounds.max_y + max_y_offset;

    add_line(drawing, grid_min_x, grid_min_y, grid_max_x, grid_min_y, COLOR_RED, "DEBUG");
    add_line(drawing, grid_max_x, grid_min_y, grid_max_x, grid_max_y, COLOR_RED, "DEBUG");
    add_line(drawing, grid_max_x, grid_max_y, grid_min_x, grid_max_y, COLOR_RED, "DEBUG");
    add_line(drawing, grid_min_x, grid_max_y, grid_min_x, grid_min_y, COLOR_RED, "DEBUG");
}

