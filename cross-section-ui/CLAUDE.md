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
```

## Architecture

### Module Structure (src/)

- **lib.rs** - Public exports, ViewMode enum, test utilities
- **app.rs** - `CrossSectionApp` struct with all UI state (view mode, scale, filters)
- **app_ui.rs** - `eframe::App` implementation, mobile/desktop responsive UI
- **data_model.rs** - `CrossSectionData`, `SurveyRow` structs, CSV parsing
- **drawing_ir.rs** - `DrawingContent` IR for DXF/GUI rendering, `SectionStyle` for label positioning
- **dxf_helpers.rs** - Low-level DXF primitives (`add_line`, `add_text`, `add_text_rotated`)
- **dxf_view.rs** - `DxfViewState` for pan/zoom, GUI rendering of DXF content
- **grid_layout.rs** - Multi-section grid layout, `GridLayoutParams`, `build_section_content()`
- **title_block.rs** - A3 title block generation, `TitleBlockInfo`, `available_area` constants
- **dekigata.rs** - 出来形管理用紙 (construction quality control sheet) generation
- **combo_drawing.rs** - Combined longitudinal + cross-section view
- **wasm_integration.rs** - WASM-specific code (file dialogs, downloads)
- **ui_chars.rs** - Japanese characters list for font subsetting

### Key Patterns

1. **DrawingContent IR**: Unified rendering via `DrawingContent` with `DrawPrimitive::Line/Text`. Use `content.add_to_dxf()` for DXF output, iterate primitives for GUI.

2. **Scale System**: `PlotScale = Option<u32>` where `Some(200)` = 1:200. DXF units are mm × scale (e.g., 1m = 1000 units at 1:1).

3. **View Modes**: `ViewMode` enum (Single, AllGrid, Combo, Dekigata, etc.) controls which drawing function is called.

4. **Mobile/Desktop Detection**: `is_mobile = screen_width < 600.0` drives responsive layout.

### GridLayoutParams (grid_layout.rs)

Centralized cell height calculation for multi-section grids. Critical for avoiding overlap:

```
cell_height = body_height * scale * v_scale_ratio  // Section body (scaled)
            + TOP_MARGIN_DXF (3350)                // Flag labels (fixed)
            + BOTTOM_MARGIN_DXF (700)              // Cutting labels (fixed)
            + row_gap * scale                      // Row gap (fixed)
```

- **v_scale_ratio**: Vertical scale multiplier (1.0-5.0), only affects section body
- **TOP_MARGIN_DXF**: Space for 旗揚げ (flag labels: No.xxx, GH=, FH=)
- **BOTTOM_MARGIN_DXF**: Space for 切削厚 labels below DL line
- **offset_y**: Must add `BOTTOM_MARGIN_DXF` to prevent cutting labels extending below cell

### SectionStyle (drawing_ir.rs)

Controls all label positioning in `build_section_content()`. Key values (DXF units):
- `flag_base_offset: 1600` - Flag base from ground level
- `label_title_offset: 1300` - Survey point name offset
- `cutting_label_offset: 300` - Cutting thickness label below DL

When modifying label positions, update both `SectionStyle` defaults AND margin constants in `grid_layout.rs`.

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
