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
```

## Architecture

### Module Structure (src/)

- **lib.rs** - Public exports, ViewMode enum, test utilities
- **app.rs** - `CrossSectionApp` struct with all UI state (view mode, scale, filters)
- **app_ui.rs** - `eframe::App` implementation, mobile/desktop responsive UI
- **data_model.rs** - `CrossSectionData`, `SurveyRow` structs, CSV parsing
- **drawing_ir.rs** - `DrawingContent` intermediate representation for DXF/GUI rendering
- **dxf_helpers.rs** - Low-level DXF primitives (`add_line`, `add_text`, `add_text_rotated`)
- **dxf_view.rs** - `DxfViewState` for pan/zoom, GUI rendering of DXF content
- **grid_layout.rs** - Multi-section grid layout, `build_section_content()`, frame fitting
- **title_block.rs** - A3 title block generation, `TitleBlockInfo`, `available_area` constants
- **dekigata.rs** - 出来形管理用紙 (construction quality control sheet) generation
- **longitudinal.rs** - Longitudinal profile drawing
- **combo_drawing.rs** - Combined longitudinal + cross-section view
- **area_expansion.rs** - Area expansion diagram
- **wasm_integration.rs** - WASM-specific code (file dialogs, downloads)
- **ui_chars.rs** - Japanese characters list for font subsetting

### Key Patterns

1. **DrawingContent IR**: Unified rendering via `DrawingContent` with `DrawPrimitive::Line/Text`. Use `content.add_to_dxf()` for DXF output, iterate primitives for GUI.

2. **Scale System**: `PlotScale = Option<u32>` where `Some(200)` = 1:200. DXF units are mm × scale (e.g., 1m = 1000 units at 1:1).

3. **View Modes**: `ViewMode` enum (Single, AllGrid, Combo, Longitudinal, AreaExpansion, Dekigata, etc.) controls which drawing function is called.

4. **Mobile/Desktop Detection**: `is_mobile = screen_width < 600.0` drives responsive layout in app_ui.rs.

## Font Subsetting

Japanese font is subsetted to reduce WASM size:

1. Add new characters to `src/ui_chars.rs`
2. Run `python scripts/subset_font.py` from project root
3. Regenerates `static/NotoSansJP-Subset.ttf`

Requires: `pip install fonttools`

## Key Constants (title_block.rs)

- A3 paper: 420mm × 297mm
- Frame margins: 10mm inner
- `available_area` module defines usable drawing regions per column

## WASM Conditional Compilation

Use `#[cfg(target_arch = "wasm32")]` for browser-specific code. Desktop-only features should use `#[cfg(not(target_arch = "wasm32"))]`.
