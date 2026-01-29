# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Cross-section diagram and cutting calculation system built with Rust + egui, targeting WebAssembly for browser deployment. Generates DXF files for CAD viewing of road cross-section surveys.

## Build Commands

```bash
# Development build (native)
cargo check
cargo build

# WASM build for web deployment
trunk build --release

# Run tests
cargo test

# Run single test
cargo test test_name

# Run integration tests only
cargo test --test multi_drawing_tests
cargo test --test page_generation_tests
```

## Architecture

### Module Structure (src/)

**Core Data:**
- **data_model.rs** - `CrossSectionData`, `SurveyRow` structs, CSV parsing, `SectionData` trait impl
- **drawing_ir.rs** - `DrawingContent` IR for DXF/GUI rendering, `SectionStyle`, font metrics abstraction

**Grid Layout Submodules (v2.0 refactored):**
- **grid_layout.rs** - Re-exports all grid layout APIs for backward compatibility
- **grid_params.rs** - `GridLayoutParams` struct and margin constants
- **grid_bounds.rs** - `GridBounds` and `calc_*` functions for layout calculation
- **section_render.rs** - `build_section_content()` and `draw_section_at_offset()`
- **multi_drawing.rs** - `MultiDrawingBuilder` (recommended API)
- **page_generation.rs** - `AllPagesBuilder` (recommended API)
- **multi_drawing_compat.rs** - 10 deprecated legacy functions
- **page_generation_compat.rs** - 3 deprecated legacy functions

**DXF Output:**
- **dxf_helpers.rs** - Low-level DXF primitives (`add_line`, `add_text`, `round_baseline_with_margin`)
- **title_block.rs** - A3 title block generation, `TitleBlockInfo`, `available_area` constants

**Application:**
- **lib.rs** - Public exports, ViewMode enum, test utilities
- **app.rs** - `CrossSectionApp` struct with all UI state
- **app_ui.rs** - `eframe::App` implementation, mobile/desktop responsive UI
- **dxf_view.rs** - `DxfViewState` for pan/zoom, GUI rendering

**Specialized Views:**
- **dekigata.rs** - 出来形管理用紙 (construction quality control sheet)
- **combo_drawing.rs** - Combined longitudinal + cross-section view
- **longitudinal.rs** - Longitudinal section view

**Utilities:**
- **font_metrics.rs** - Font metrics constants (Noto Sans JP specific)
- **wasm_integration.rs** - WASM-specific code (file dialogs, downloads)
- **ui_chars.rs** - Japanese characters list for font subsetting

### Key Patterns

1. **Builder Pattern for Drawing Generation** (Recommended):
```rust
// Single page with frame
let drawing = MultiDrawingBuilder::new(&sections)
    .columns(4)
    .gaps(2.0, 1.0)
    .with_frame(500.0, &title_info)
    .interpolated()
    .v_scale(2.0)
    .build();

// Multi-page vertically stacked
let drawing = AllPagesBuilder::new(&sections, &title_info)
    .gaps(2.0, 1.0)
    .frame_scale(500.0)
    .drawing_number_base("001")
    .build();
```

2. **DrawingContent IR**: Unified rendering via `DrawingContent` with `DrawPrimitive::Line/Text`. Use `content.add_to_dxf()` for DXF output, iterate primitives for GUI.

3. **Scale System**: `PlotScale = Option<u32>` where `Some(200)` = 1:200. DXF units are mm × scale (e.g., 1m = 1000 units at 1:1).

4. **View Modes**: `ViewMode` enum (Single, AllGrid, Combo, Dekigata, etc.) controls which drawing function is called.

### Abstraction Layers

**Font Metrics (drawing_ir.rs)**:
- `FontMetrics` trait for cap height → DXF text height conversion
- `cap_height_to_dxf_height()` uses default Noto Sans JP metrics (1000/733 scale)

**Section Data Abstraction (drawing_ir.rs)**:
- `SectionPoint` trait: `elevation()`, `planned_height()`, `cumulative_distance()`, `cutting_bottom()`
- `SectionData` trait: `survey_points()`, `datum_level()`, `cl_index()`, `body_height()`
- Enables `GridLayoutParams` to work with any section data type

### GridLayoutParams (grid_params.rs)

Centralized cell height calculation for multi-section grids:

```rust
GridLayoutParams::new(column_gap, row_gap)
    .with_v_scale(2.0)           // Optional: vertical scale (default 2.0)
    .with_margins(top, bottom)   // Optional: custom margins
```

Cell height formula:
```
cell_height = body_height * scale * v_scale_ratio  // Section body (scaled)
            + top_margin (default 3350)            // Flag labels (fixed DXF units)
            + bottom_margin (default 700)          // Cutting labels (fixed)
            + row_gap * scale                      // Row gap (fixed)
```

Key constants:
- `DEFAULT_TOP_MARGIN_DXF = 3350` - Space for 旗揚げ (flag labels: No.xxx, GH=, FH=)
- `DEFAULT_BOTTOM_MARGIN_DXF = 700` - Space for 切削厚 labels below DL line

### SectionStyle (drawing_ir.rs)

Controls all label positioning in `build_section_content()`. Key values (DXF units):
- `flag_base_offset: 1600` - Flag base from ground level
- `label_title_offset: 1300` - Survey point name offset
- `cutting_label_offset: 300` - Cutting thickness label below DL

When modifying label positions, update both `SectionStyle` defaults AND margin constants in `grid_params.rs`.

## Integration Tests

Tests are organized in `tests/` directory:
- `tests/multi_drawing_tests.rs` - MultiDrawingBuilder and legacy function tests
- `tests/page_generation_tests.rs` - AllPagesBuilder and legacy function tests

## Font Subsetting

Japanese font is subsetted to reduce WASM size:

1. Add new characters to `src/ui_chars.rs`
2. Run `python scripts/subset_font.py` from project root
3. Regenerates `static/NotoSansJP-Subset.ttf`

Requires: `pip install fonttools`

## Key Constants

- A3 paper: 420mm × 297mm
- Frame margins: 10mm inner
- `available_area` module (title_block.rs) defines usable drawing regions per column

## WASM Conditional Compilation

Use `#[cfg(target_arch = "wasm32")]` for browser-specific code. Desktop-only features use `#[cfg(not(target_arch = "wasm32"))]`.

## GitHub Pages Deployment

Build output goes to `docs/` for GitHub Pages:
```bash
trunk build --release
cp -r dist/* ../docs/
```

`Trunk.toml` sets `public_url = "/csv_to_dxf/"` for correct asset paths.

## Deprecated APIs

Legacy functions in `*_compat.rs` modules are deprecated but re-exported via `grid_layout.rs` for backward compatibility:
- `generate_multi_drawing()` → use `MultiDrawingBuilder`
- `generate_all_pages_drawing()` → use `AllPagesBuilder`
- `round_dl()` → use `round_baseline_with_margin()`
- `font_metrics::cap_height_to_text_height()` → use `drawing_ir::cap_height_to_dxf_height()`
