//! Deprecated multi-drawing API compatibility layer
//!
//! This module contains deprecated functions from the original multi_drawing module.
//! Use `MultiDrawingBuilder` instead for new code.

use dxf::Drawing;

use crate::multi_drawing::MultiDrawingBuilder;
use crate::{CrossSectionData, TitleBlockInfo};

// ============================================================================
// Basic Drawing Generation (Deprecated)
// ============================================================================

/// Generate a multi-section drawing with basic grid layout.
///
/// **Deprecated**: Use `MultiDrawingBuilder::new(sections).columns(columns).gaps(column_gap, row_gap).build()` instead.
#[deprecated(since = "2.0.0", note = "Use MultiDrawingBuilder instead")]
pub fn generate_multi_drawing(
    sections: &[CrossSectionData],
    columns: usize,
    column_gap: f64,
    row_gap: f64,
) -> Drawing {
    MultiDrawingBuilder::new(sections).columns(columns).gaps(column_gap, row_gap).build()
}

/// Generate a multi-section drawing with scale multiplier.
///
/// **Deprecated**: Use `MultiDrawingBuilder` instead.
#[deprecated(since = "2.0.0", note = "Use MultiDrawingBuilder instead")]
pub fn generate_multi_drawing_scaled(
    sections: &[CrossSectionData],
    columns: usize,
    column_gap: f64,
    row_gap: f64,
    scale_multiplier: f64,
) -> Drawing {
    MultiDrawingBuilder::new(sections)
        .columns(columns)
        .gaps(column_gap, row_gap)
        .scale_multiplier(scale_multiplier)
        .build()
}

/// Generate a multi-section drawing with scale multiplier and interpolation.
///
/// **Deprecated**: Use `MultiDrawingBuilder` instead.
#[deprecated(since = "2.0.0", note = "Use MultiDrawingBuilder instead")]
pub fn generate_multi_drawing_scaled_interpolated(
    sections: &[CrossSectionData],
    columns: usize,
    column_gap: f64,
    row_gap: f64,
    scale_multiplier: f64,
) -> Drawing {
    MultiDrawingBuilder::new(sections)
        .columns(columns)
        .gaps(column_gap, row_gap)
        .scale_multiplier(scale_multiplier)
        .interpolated()
        .build()
}

// ============================================================================
// Framed Drawing Generation (Deprecated)
// ============================================================================

/// Generate a multi-section drawing with 1:500 A3 frame.
///
/// **Deprecated**: Use `MultiDrawingBuilder` instead.
#[deprecated(since = "2.0.0", note = "Use MultiDrawingBuilder instead")]
pub fn generate_multi_drawing_with_frame(
    sections: &[CrossSectionData],
    _columns: usize,
    column_gap: f64,
    row_gap: f64,
    title_info: &TitleBlockInfo,
) -> Drawing {
    MultiDrawingBuilder::new(sections)
        .gaps(column_gap, row_gap)
        .with_frame(500.0, title_info)
        .build()
}

/// Generate a multi-section drawing with A3 frame at custom scale.
///
/// **Deprecated**: Use `MultiDrawingBuilder` instead.
#[deprecated(since = "2.0.0", note = "Use MultiDrawingBuilder instead")]
pub fn generate_multi_drawing_with_frame_at_scale(
    sections: &[CrossSectionData],
    _columns: usize,
    column_gap: f64,
    row_gap: f64,
    title_info: &TitleBlockInfo,
    frame_scale: f64,
) -> Drawing {
    MultiDrawingBuilder::new(sections)
        .gaps(column_gap, row_gap)
        .with_frame(frame_scale, title_info)
        .build()
}

// ============================================================================
// Interpolated Drawing Generation (Deprecated)
// ============================================================================

/// Generate a multi-section drawing with interpolation and custom v_scale.
///
/// **Deprecated**: Use `MultiDrawingBuilder` instead.
#[deprecated(since = "2.0.0", note = "Use MultiDrawingBuilder instead")]
pub fn generate_multi_drawing_interpolated(
    sections: &[CrossSectionData],
    columns: usize,
    column_gap: f64,
    row_gap: f64,
    v_scale_ratio: f64,
) -> Drawing {
    MultiDrawingBuilder::new(sections)
        .columns(columns)
        .gaps(column_gap, row_gap)
        .interpolated()
        .v_scale(v_scale_ratio)
        .build()
}

/// Generate a multi-section drawing with frame, interpolation, and custom v_scale.
///
/// **Deprecated**: Use `MultiDrawingBuilder` instead.
#[deprecated(since = "2.0.0", note = "Use MultiDrawingBuilder instead")]
pub fn generate_multi_drawing_with_frame_at_scale_interpolated(
    sections: &[CrossSectionData],
    _columns: usize,
    column_gap: f64,
    row_gap: f64,
    title_info: &TitleBlockInfo,
    frame_scale: f64,
    v_scale_ratio: f64,
) -> Drawing {
    MultiDrawingBuilder::new(sections)
        .gaps(column_gap, row_gap)
        .with_frame(frame_scale, title_info)
        .interpolated()
        .v_scale(v_scale_ratio)
        .build()
}

// ============================================================================
// Byte Output (Deprecated)
// ============================================================================

/// Generate multi-section DXF as byte array.
///
/// **Deprecated**: Use `MultiDrawingBuilder::new(sections)...to_dxf_bytes()` instead.
#[deprecated(since = "2.0.0", note = "Use MultiDrawingBuilder::to_dxf_bytes() instead")]
pub fn generate_multi_dxf_bytes(
    sections: &[CrossSectionData],
    columns: usize,
    column_gap: f64,
    row_gap: f64,
) -> Vec<u8> {
    MultiDrawingBuilder::new(sections).columns(columns).gaps(column_gap, row_gap).to_dxf_bytes()
}

/// Generate multi-section DXF with 1:500 frame as byte array.
///
/// **Deprecated**: Use `MultiDrawingBuilder` instead.
#[deprecated(since = "2.0.0", note = "Use MultiDrawingBuilder::to_dxf_bytes() instead")]
pub fn generate_multi_dxf_bytes_with_frame(
    sections: &[CrossSectionData],
    _columns: usize,
    column_gap: f64,
    row_gap: f64,
    title_info: &TitleBlockInfo,
) -> Vec<u8> {
    MultiDrawingBuilder::new(sections)
        .gaps(column_gap, row_gap)
        .with_frame(500.0, title_info)
        .to_dxf_bytes()
}

/// Generate multi-section DXF with custom scale frame as byte array.
///
/// **Deprecated**: Use `MultiDrawingBuilder` instead.
#[deprecated(since = "2.0.0", note = "Use MultiDrawingBuilder::to_dxf_bytes() instead")]
pub fn generate_multi_dxf_bytes_with_frame_at_scale(
    sections: &[CrossSectionData],
    _columns: usize,
    column_gap: f64,
    row_gap: f64,
    title_info: &TitleBlockInfo,
    frame_scale: f64,
    v_scale_ratio: f64,
) -> Vec<u8> {
    MultiDrawingBuilder::new(sections)
        .gaps(column_gap, row_gap)
        .with_frame(frame_scale, title_info)
        .interpolated()
        .v_scale(v_scale_ratio)
        .to_dxf_bytes()
}
