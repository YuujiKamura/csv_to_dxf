//! 複数横断図のグリッドレイアウト配置
//!
//! A3図枠内に複数の横断図を4列×N行で配置するためのモジュール
//!
//! # Module Structure (v2.0)
//!
//! This module has been refactored into smaller, focused submodules:
//!
//! - `grid_params`: GridLayoutParams struct and margin constants
//! - `grid_bounds`: GridBounds and calc_* functions for layout calculation
//! - `section_render`: build_section_content and draw_section_at_offset
//! - `multi_drawing`: generate_multi_drawing* functions
//! - `page_generation`: generate_all_pages_drawing* functions
//!
//! All public APIs are re-exported here for backward compatibility.

// ============================================================================
// Re-exports from submodules
// ============================================================================

// grid_params.rs
pub use crate::grid_params::{
    GridLayoutParams,
    DEFAULT_TOP_MARGIN_DXF,
    DEFAULT_BOTTOM_MARGIN_DXF,
};

// grid_bounds.rs
pub use crate::grid_bounds::{
    GridBounds,
    calc_grid_bounds,
    calc_grid_bounds_with_v_scale,
    calc_max_columns_for_frame,
    calc_optimal_columns_for_frame,
    calc_sections_per_page,
    calc_section_bounds_in_grid,
    calc_section_bounds_in_grid_with_v_scale,
    calc_column_widths,
    calc_column_x_offsets,
};

// section_render.rs
pub use crate::section_render::{
    build_section_content,
    draw_section_at_offset,
};

// multi_drawing.rs - Builder (recommended)
pub use crate::multi_drawing::MultiDrawingBuilder;

// multi_drawing_compat.rs - Legacy functions (deprecated, re-exported for backward compatibility)
#[allow(deprecated)]
pub use crate::multi_drawing_compat::{
    generate_multi_drawing,
    generate_multi_drawing_scaled,
    generate_multi_drawing_scaled_interpolated,
    generate_multi_drawing_with_frame,
    generate_multi_drawing_with_frame_at_scale,
    generate_multi_drawing_interpolated,
    generate_multi_drawing_with_frame_at_scale_interpolated,
    generate_multi_dxf_bytes,
    generate_multi_dxf_bytes_with_frame,
    generate_multi_dxf_bytes_with_frame_at_scale,
};

// page_generation.rs - Builder (recommended)
pub use crate::page_generation::AllPagesBuilder;

// page_generation_compat.rs - Legacy functions (deprecated, re-exported for backward compatibility)
#[allow(deprecated)]
pub use crate::page_generation_compat::{
    generate_all_pages_drawing,
    generate_all_pages_drawing_interpolated,
    generate_all_pages_dxf_bytes,
};
