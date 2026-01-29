//! Multi-page DXF output generation
//!
//! This module provides functions for generating multi-page DXF drawings
//! where each page is vertically stacked in a single DXF file.
//!
//! ## Page Layout
//!
//! - Each page is an A3 size (420mm x 297mm) with a drawing frame
//! - Pages are stacked vertically from bottom to top
//! - Page gap is 10mm between consecutive pages
//! - Uses 4-column fixed layout (available_area::COLUMN_COUNT)
//!
//! ## Builder Pattern (Recommended)
//!
//! Use `AllPagesBuilder` for a fluent, composable API:
//!
//! ```ignore
//! let drawing = AllPagesBuilder::new(&sections, &title_info)
//!     .gaps(2.0, 1.0)
//!     .frame_scale(500.0)
//!     .drawing_number_base("001")
//!     .interpolated()
//!     .v_scale(2.0)
//!     .build();
//! ```
//!
//! ## Legacy Functions (Deprecated)
//!
//! Deprecated functions are available in `page_generation_compat` module
//! and re-exported via `grid_layout` for backward compatibility.

use dxf::{Drawing, Color};

use crate::{
    CrossSectionData, TitleBlockInfo,
    new_drawing, draw_drawing_frame,
    title_block::available_area,
};

// Import from grid_params module (assumes mod grid_params is declared in lib.rs or parent)
use crate::grid_params::{GridLayoutParams, DEFAULT_BOTTOM_MARGIN_DXF};
// Import from grid_bounds module (assumes mod grid_bounds is declared in lib.rs or parent)
use crate::grid_bounds::{calc_grid_bounds, calc_sections_per_page};
// Import from section_render module (assumes mod section_render is declared in lib.rs or parent)
use crate::section_render::draw_section_at_offset;

// ============================================================================
// Builder Pattern
// ============================================================================

/// Builder for creating multi-page DXF drawings with a fluent API.
///
/// # Example
///
/// ```ignore
/// let drawing = AllPagesBuilder::new(&sections, &title_info)
///     .gaps(2.0, 1.0)
///     .frame_scale(500.0)
///     .drawing_number_base("001")
///     .interpolated()
///     .v_scale(2.0)
///     .build();
/// ```
pub struct AllPagesBuilder<'a> {
    sections: &'a [CrossSectionData],
    columns: usize,
    column_gap: f64,
    row_gap: f64,
    frame_scale: f64,
    title_info: &'a TitleBlockInfo,
    drawing_number_base: String,
    interpolate: bool,
    v_scale_ratio: f64,
}

impl<'a> AllPagesBuilder<'a> {
    /// Create a new builder with sections and title info.
    ///
    /// Default values:
    /// - columns: 4 (fixed for A3 layout)
    /// - column_gap: 2.0
    /// - row_gap: 1.0
    /// - frame_scale: 500.0 (1:500)
    /// - drawing_number_base: "001"
    /// - interpolate: false
    /// - v_scale_ratio: 2.0
    pub fn new(sections: &'a [CrossSectionData], title_info: &'a TitleBlockInfo) -> Self {
        Self {
            sections,
            columns: 4,
            column_gap: 2.0,
            row_gap: 1.0,
            frame_scale: 500.0,
            title_info,
            drawing_number_base: "001".to_string(),
            interpolate: false,
            v_scale_ratio: 2.0,
        }
    }

    /// Set column and row gaps in meters.
    pub fn gaps(mut self, column_gap: f64, row_gap: f64) -> Self {
        self.column_gap = column_gap;
        self.row_gap = row_gap;
        self
    }

    /// Set the frame scale (e.g., 200.0 for 1:200, 500.0 for 1:500).
    pub fn frame_scale(mut self, scale: f64) -> Self {
        self.frame_scale = scale;
        self
    }

    /// Set the base drawing number (e.g., "001" becomes "001-1", "001-2", etc.).
    pub fn drawing_number_base(mut self, base: &str) -> Self {
        self.drawing_number_base = base.to_string();
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

    /// Build the DXF Drawing with all pages vertically stacked.
    pub fn build(self) -> Drawing {
        generate_all_pages_drawing_internal(
            self.sections,
            self.columns,
            self.column_gap,
            self.row_gap,
            self.frame_scale,
            self.title_info,
            &self.drawing_number_base,
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
// Constants
// ============================================================================

/// A3 paper height in mm
const A3_HEIGHT_MM: f64 = 297.0;

/// Gap between pages in mm
const PAGE_GAP_MM: f64 = 10.0;

/// Default text size for frame labels in mm
const TEXT_SIZE_MM: f64 = 3.0;

// ============================================================================
// Internal Implementation
// ============================================================================

/// Internal implementation for multi-page DXF generation.
///
/// # Arguments
///
/// * `all_sections` - All cross-section data to render
/// * `columns` - Number of columns (ignored, uses 4-column fixed layout)
/// * `column_gap` - Gap between columns in meters
/// * `row_gap` - Gap between rows in meters
/// * `frame_scale` - Drawing frame scale (e.g., 500.0 for 1:500)
/// * `base_title_info` - Base title block information
/// * `drawing_number_base` - Base drawing number
/// * `interpolate_intermediate` - Whether to interpolate intermediate sections
/// * `v_scale_ratio` - Vertical scale ratio
fn generate_all_pages_drawing_internal(
    all_sections: &[CrossSectionData],
    columns: usize,
    column_gap: f64,
    row_gap: f64,
    frame_scale: f64,
    base_title_info: &TitleBlockInfo,
    drawing_number_base: &str,
    interpolate_intermediate: bool,
    v_scale_ratio: f64,
) -> Drawing {
    let mut drawing = new_drawing();

    // Add required layers
    for (name, color_idx) in [
        ("GROUND", 7),
        ("PLAN", 1),
        ("TEXT", 7),
        ("DIMENSION", 8),
        ("CUTTING", 5),
        ("FRAME", 9),
        ("TITLEBLOCK", 7),
        ("DEBUG", 1),
        ("SLOPE", 7),
    ] {
        drawing.add_layer(dxf::tables::Layer {
            name: name.to_string(),
            color: Color::from_index(color_idx),
            ..Default::default()
        });
    }

    if all_sections.is_empty() {
        return drawing;
    }

    // Calculate sections per page using the grid_bounds function
    let sections_per_page = calc_sections_per_page(
        all_sections,
        columns,
        column_gap,
        row_gap,
        frame_scale,
    );

    if sections_per_page == 0 {
        return drawing;
    }

    let total_pages = (all_sections.len() + sections_per_page - 1) / sections_per_page;

    // Convert A3 height to DXF units
    let page_height_dxf = A3_HEIGHT_MM * frame_scale;
    let page_gap_dxf = PAGE_GAP_MM * frame_scale;

    // Draw each page
    for page in 0..total_pages {
        let start_idx = page * sections_per_page;
        let end_idx = (start_idx + sections_per_page).min(all_sections.len());
        let page_sections = &all_sections[start_idx..end_idx];

        // Page Y offset (pages stack from bottom to top)
        let page_offset_y = page as f64 * (page_height_dxf + page_gap_dxf);

        // Generate page-specific drawing number
        let page_num = if total_pages > 1 {
            format!("{}-{}", drawing_number_base, page + 1)
        } else {
            drawing_number_base.to_string()
        };

        let info = base_title_info.clone().with_drawing_number(&page_num);

        // Draw this page's content
        draw_page_content(
            &mut drawing,
            page_sections,
            columns,
            column_gap,
            row_gap,
            frame_scale,
            &info,
            page_offset_y,
            interpolate_intermediate,
            v_scale_ratio,
        );
    }

    drawing
}

/// Draw content for a single page at the given Y offset.
///
/// This function renders all sections for one page, including:
/// - Cross-section drawings in a 4-column grid layout
/// - Drawing frame with title block
///
/// # Arguments
///
/// * `drawing` - The DXF drawing to add entities to
/// * `sections` - Sections to render on this page
/// * `_columns` - Number of columns (ignored, uses 4-column fixed layout)
/// * `column_gap` - Gap between columns in meters
/// * `row_gap` - Gap between rows in meters
/// * `frame_scale` - Drawing frame scale
/// * `title_info` - Title block information for this page
/// * `page_offset_y` - Y offset for this page in DXF units
/// * `interpolate_intermediate` - Whether to interpolate intermediate sections
/// * `v_scale_ratio` - Vertical scale ratio
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
    v_scale_ratio: f64,
) {
    let scale = 1000.0;
    // Use 4-column fixed layout
    let columns = available_area::COLUMN_COUNT;

    if sections.is_empty() {
        // Still draw the empty frame
        draw_drawing_frame(drawing, title_info, 0.0, page_offset_y, frame_scale, TEXT_SIZE_MM);
        return;
    }

    // Use GridLayoutParams for consistent calculations
    let params = GridLayoutParams::new(column_gap, row_gap).with_v_scale(v_scale_ratio);

    let rows_per_column = (sections.len() + columns - 1) / columns;

    // Calculate cell height using GridLayoutParams
    let cell_height = params.cell_height_for_sections(sections);

    // Column center positions (fixed 4-column layout)
    let col_centers: Vec<f64> = (0..columns)
        .map(|col| available_area::COLUMN_CENTERS[col] * frame_scale)
        .collect();

    // Calculate grid bounds for Y offset determination
    let bounds = calc_grid_bounds(sections, columns, column_gap, row_gap);
    let content_height_mm = bounds.as_ref().map(|b| b.height() / frame_scale).unwrap_or(0.0);

    // Calculate Y offset for each column to center content vertically
    let mut col_y_offsets: Vec<f64> = Vec::with_capacity(columns);
    for col in 0..columns {
        let col_height = available_area::height_for_column(col);
        let col_bottom = available_area::bottom_for_column(col);
        let target_bottom = col_bottom + (col_height - content_height_mm) / 2.0;
        let offset_y = if let Some(ref b) = bounds {
            target_bottom * frame_scale - b.min_y
        } else {
            0.0
        };
        col_y_offsets.push(offset_y);
    }

    // Draw each section
    for (idx, section) in sections.iter().enumerate() {
        if section.survey_data.len() < 2 {
            continue;
        }

        let col = idx / rows_per_column;
        let row_in_col = idx % rows_per_column;

        let offset_x = col_centers[col.min(columns - 1)];
        let col_y_offset = col_y_offsets[col.min(columns - 1)];

        // Calculate Y offset including page offset and bottom margin
        let offset_y = row_in_col as f64 * cell_height
            + col_y_offset
            + page_offset_y
            + DEFAULT_BOTTOM_MARGIN_DXF;

        // Apply interpolation for intermediate sections (not route start/end)
        let is_start_or_end = section.is_route_start || section.is_route_end;

        if interpolate_intermediate && !is_start_or_end {
            let mut section_to_draw = section.clone();
            section_to_draw.interpolate_planned_heights();
            draw_section_at_offset(
                drawing,
                &section_to_draw,
                offset_x,
                offset_y,
                scale,
                1.0,
                v_scale_ratio,
            );
        } else {
            draw_section_at_offset(
                drawing,
                section,
                offset_x,
                offset_y,
                scale,
                1.0,
                v_scale_ratio,
            );
        }
    }

    // Draw the frame for this page
    draw_drawing_frame(drawing, title_info, 0.0, page_offset_y, frame_scale, TEXT_SIZE_MM);
}

