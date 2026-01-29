//! Deprecated page-generation API compatibility layer
//!
//! This module contains deprecated functions from the original page_generation module.
//! Use `AllPagesBuilder` instead for new code.

use dxf::Drawing;

use crate::page_generation::AllPagesBuilder;
use crate::{CrossSectionData, TitleBlockInfo};

// ============================================================================
// Legacy Functions (Deprecated)
// ============================================================================

/// Generate all pages as a single DXF drawing with pages vertically stacked.
///
/// **Deprecated**: Use `AllPagesBuilder` instead.
#[deprecated(since = "2.0.0", note = "Use AllPagesBuilder instead")]
pub fn generate_all_pages_drawing(
    all_sections: &[CrossSectionData],
    _columns: usize,
    column_gap: f64,
    row_gap: f64,
    frame_scale: f64,
    base_title_info: &TitleBlockInfo,
    drawing_number_base: &str,
    v_scale_ratio: f64,
) -> Drawing {
    AllPagesBuilder::new(all_sections, base_title_info)
        .gaps(column_gap, row_gap)
        .frame_scale(frame_scale)
        .drawing_number_base(drawing_number_base)
        .v_scale(v_scale_ratio)
        .build()
}

/// Generate all pages with slope interpolation for intermediate sections.
///
/// **Deprecated**: Use `AllPagesBuilder` instead.
#[deprecated(since = "2.0.0", note = "Use AllPagesBuilder instead")]
pub fn generate_all_pages_drawing_interpolated(
    all_sections: &[CrossSectionData],
    _columns: usize,
    column_gap: f64,
    row_gap: f64,
    frame_scale: f64,
    base_title_info: &TitleBlockInfo,
    drawing_number_base: &str,
    v_scale_ratio: f64,
) -> Drawing {
    AllPagesBuilder::new(all_sections, base_title_info)
        .gaps(column_gap, row_gap)
        .frame_scale(frame_scale)
        .drawing_number_base(drawing_number_base)
        .interpolated()
        .v_scale(v_scale_ratio)
        .build()
}

/// Generate all pages and return as DXF byte array.
///
/// **Deprecated**: Use `AllPagesBuilder::to_dxf_bytes()` instead.
#[deprecated(since = "2.0.0", note = "Use AllPagesBuilder::to_dxf_bytes() instead")]
pub fn generate_all_pages_dxf_bytes(
    all_sections: &[CrossSectionData],
    _columns: usize,
    column_gap: f64,
    row_gap: f64,
    frame_scale: f64,
    base_title_info: &TitleBlockInfo,
    drawing_number_base: &str,
    v_scale_ratio: f64,
) -> Vec<u8> {
    AllPagesBuilder::new(all_sections, base_title_info)
        .gaps(column_gap, row_gap)
        .frame_scale(frame_scale)
        .drawing_number_base(drawing_number_base)
        .interpolated()
        .v_scale(v_scale_ratio)
        .to_dxf_bytes()
}
