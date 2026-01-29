//! 横断図・切削計算システム - egui版
//!
//! PDF横断図に準拠した横断図表示と切削計算
//! hook test

mod font_metrics;
mod title_block;
mod ui_chars;
mod drawing_ir;
mod data_model;
mod dxf_view;

// grid_layout submodules (must be declared before grid_layout)
mod grid_params;
mod grid_bounds;
mod section_render;
mod multi_drawing;
mod page_generation;
mod multi_drawing_compat;
mod page_generation_compat;

mod grid_layout;
mod dekigata;
mod combo_drawing;
mod area_expansion;
mod wasm_integration;
mod app;
mod app_ui;

pub use app::CrossSectionApp;
pub use dxf_view::*;
mod dxf_helpers;

pub use combo_drawing::*;
pub use area_expansion::*;

pub use dxf_helpers::*;
pub use dekigata::*;

pub use data_model::*;

pub use drawing_ir::{
    DrawingContent, DrawPrimitive, DrawLine, DrawText, HAlign, VAlign, ViewState as IrViewState, SectionStyle,
    // Font metrics abstraction
    FontMetrics, DefaultFontMetrics, default_font_metrics, cap_height_to_dxf_height, scale_for_cap_height,
    // Section data abstraction
    SectionData, SectionPoint,
};

pub use grid_layout::*;

// Re-export title block types
pub use title_block::{
    TitleBlockInfo,
    add_title_block_layer,
    draw_outer_frame,
    draw_top_title,
    draw_title_block,
    draw_drawing_frame,
    generate_title_block_test_drawing,
    generate_title_block_test_dxf_bytes,
    generate_title_block_dxf_for_download,
};

// dxf crate for proper DXF file generation
use dxf::Drawing;
use dxf::Color;

// wasm_integration exports
#[cfg(target_arch = "wasm32")]
pub use wasm_integration::*;


/// アライメントテスト用DXF生成
/// 回転テキスト(-90°)の全アライメント組み合わせを表示
pub fn generate_alignment_test_drawing() -> Drawing {
    let mut drawing = new_drawing();

    let mut layer = dxf::tables::Layer::default();
    layer.name = "TEST".to_string();
    layer.color = Color::from_index(7);
    drawing.add_layer(layer);

    let text_height = 100.0;
    let cell_width = 600.0;
    let cell_height = 400.0;

    let h_aligns = [
        (HAlign::Left, "H:Left"),
        (HAlign::Center, "H:Center"),
        (HAlign::Right, "H:Right"),
    ];
    let v_aligns = [
        (VAlign::Top, "V:Top"),
        (VAlign::Middle, "V:Mid"),
        (VAlign::Bottom, "V:Bot"),
    ];

    // グリッド描画
    for col in 0..=3 {
        let x = col as f64 * cell_width;
        add_line(&mut drawing, x, 0.0, x, -3.0 * cell_height, 8, "TEST");
    }
    for row in 0..=3 {
        let y = -(row as f64) * cell_height;
        add_line(&mut drawing, 0.0, y, 3.0 * cell_width, y, 8, "TEST");
    }

    // 各セルにテスト表示
    for (col, (h_align, h_name)) in h_aligns.iter().enumerate() {
        for (row, (v_align, v_name)) in v_aligns.iter().enumerate() {
            let x = col as f64 * cell_width + cell_width / 2.0;
            let y = -(row as f64) * cell_height - cell_height / 2.0;

            // アンカーポイントにマーカー（赤十字）
            add_line(&mut drawing, x - 30.0, y, x + 30.0, y, 1, "TEST");
            add_line(&mut drawing, x, y - 30.0, x, y + 30.0, 1, "TEST");

            // テスト文字列（-90°回転）
            let label = format!("{} {}", h_name, v_name);
            add_text_rotated(&mut drawing, x, y, &label,
                text_height, 7, "TEST", *h_align, *v_align, -90.0);
        }
    }

    // 説明テキスト
    add_text(&mut drawing, 0.0, 200.0, "Alignment Test: -90deg rotation", text_height, 7, "TEST", HAlign::Left);
    add_text(&mut drawing, 0.0, 50.0, "Red cross = anchor point", text_height * 0.8, 1, "TEST", HAlign::Left);

    drawing
}

/// アライメントテストDXFをバイト列で返す
pub fn generate_alignment_test_dxf_bytes() -> Vec<u8> {
    let drawing = generate_alignment_test_drawing();
    let mut buf = Vec::new();
    drawing.save(&mut buf).unwrap();
    buf
}

/// Drawingオブジェクトを生成する
pub fn generate_drawing(section: &CrossSectionData) -> Drawing {
    let scale = 1000.0; // mm単位
    let data = &section.survey_data;

    let mut drawing = new_drawing();

    // レイヤー作成
    drawing.add_layer(dxf::tables::Layer {
        name: "GROUND".to_string(),
        color: Color::from_index(7),  // 白/黒
        ..Default::default()
    });
    drawing.add_layer(dxf::tables::Layer {
        name: "PLAN".to_string(),
        color: Color::from_index(1),  // 赤
        ..Default::default()
    });
    drawing.add_layer(dxf::tables::Layer {
        name: "TEXT".to_string(),
        color: Color::from_index(7),  // 白/黒
        ..Default::default()
    });
    drawing.add_layer(dxf::tables::Layer {
        name: "DIMENSION".to_string(),
        color: Color::from_index(8),  // グレー
        ..Default::default()
    });
    drawing.add_layer(dxf::tables::Layer {
        name: "CUTTING".to_string(),
        color: Color::from_index(5),  // 青
        ..Default::default()
    });

    if data.len() < 2 {
        return drawing;
    }

    let dl = round_dl(section.dl);

    // CLを原点（x=0）にするためのオフセット
    let cl_data = &data[section.cl_index.min(data.len() - 1)];
    let cl_offset = cl_data.cumulative_distance;

    let to_dxf_x = |cumulative_distance: f64| -> f64 {
        (cumulative_distance - cl_offset) * scale  // CLが原点に来るようにオフセット
    };
    let y_scale = scale * 2.0;  // 縦方向スケールを2倍（縦断図と同様）
    let to_dxf_y = |height: f64| -> f64 {
        (height - dl) * y_scale
    };

    let l_data = &data[0];
    let r_data = &data[data.len() - 1];

    let left_distance = (cl_data.cumulative_distance - l_data.cumulative_distance).abs();
    let right_distance = (r_data.cumulative_distance - cl_data.cumulative_distance).abs();

    let left_slope = if left_distance > 0.0 {
        ((l_data.planned_height - cl_data.planned_height) / left_distance) * 100.0
    } else { 0.0 };
    let right_slope = if right_distance > 0.0 {
        ((r_data.planned_height - cl_data.planned_height) / right_distance) * 100.0
    } else { 0.0 };

    // ========== 地盤線（黒）==========
    for i in 0..data.len() - 1 {
        let p1_x = to_dxf_x(data[i].cumulative_distance);
        let p1_y = to_dxf_y(data[i].elevation);
        let p2_x = to_dxf_x(data[i + 1].cumulative_distance);
        let p2_y = to_dxf_y(data[i + 1].elevation);
        add_line(&mut drawing, p1_x, p1_y, p2_x, p2_y, 7, "GROUND");
    }

    // ========== 計画高線（赤）==========
    for i in 0..data.len() - 1 {
        let p1_x = to_dxf_x(data[i].cumulative_distance);
        let p1_y = to_dxf_y(data[i].planned_height);
        let p2_x = to_dxf_x(data[i + 1].cumulative_distance);
        let p2_y = to_dxf_y(data[i + 1].planned_height);
        add_line(&mut drawing, p1_x, p1_y, p2_x, p2_y, 1, "PLAN");
    }

    // ========== 切削底面線（青）==========
    for i in 0..data.len() - 1 {
        let p1_x = to_dxf_x(data[i].cumulative_distance);
        let p1_y = to_dxf_y(data[i].cutting_bottom);
        let p2_x = to_dxf_x(data[i + 1].cumulative_distance);
        let p2_y = to_dxf_y(data[i + 1].cutting_bottom);
        add_line(&mut drawing, p1_x, p1_y, p2_x, p2_y, 5, "CUTTING");
    }

    let text_height = 300.0;  // モバイル表示用に2倍
    let pointer_offset = 500.0;  // ポインター分の上方オフセット

    // ========== 測点ポインター（逆三角形 + Vラベル）==========
    let pointer_size = 150.0;  // 逆三角形のサイズ
    for (i, pt) in data.iter().enumerate() {
        let x = to_dxf_x(pt.cumulative_distance);
        let y = to_dxf_y(pt.planned_height);
        // 逆三角形（▽）を描画 - 青色
        let top_y = y + pointer_size;
        let half_w = pointer_size * 0.6;
        add_line(&mut drawing, x - half_w, top_y, x + half_w, top_y, 5, "CUTTING");
        add_line(&mut drawing, x - half_w, top_y, x, y, 5, "CUTTING");
        add_line(&mut drawing, x + half_w, top_y, x, y, 5, "CUTTING");
        // ラベル（V1, V2, ...）- 文字サイズ統一
        let label = format!("V{}", i + 1);
        add_text(&mut drawing, x, top_y + 300.0, &label, text_height, 5, "CUTTING", HAlign::Center);
    }

    let cl_ground_y = to_dxf_y(cl_data.elevation);
    let flag_height_mm = 1600.0;  // 2倍
    let flag_y = cl_ground_y + flag_height_mm;

    let l_x = to_dxf_x(l_data.cumulative_distance);
    let cl_x = to_dxf_x(cl_data.cumulative_distance);
    let r_x = to_dxf_x(r_data.cumulative_distance);

    // ========== 測点名（CL上）==========
    add_text(&mut drawing, cl_x, flag_y + 1600.0,
        &section.survey_point_name, text_height * 1.5, 7, "TEXT", HAlign::Center);

    // ========== CL GH, FH ==========
    add_text_with_mask(&mut drawing, cl_x, flag_y + 800.0,
        &format!("GH={:.3}", cl_data.elevation), text_height, 7, "TEXT", HAlign::Center);
    add_text_with_mask(&mut drawing, cl_x, flag_y + 400.0,
        &format!("FH={:.3}", cl_data.planned_height), text_height, 1, "PLAN", HAlign::Center);

    // ========== L側 GH, FH（ポインター分上にオフセット）==========
    let l_ground_y = to_dxf_y(l_data.elevation);
    add_text_with_mask(&mut drawing, l_x, l_ground_y + 800.0 + pointer_offset,
        &format!("GH={:.3}", l_data.elevation), text_height, 7, "TEXT", HAlign::Left);
    add_text_with_mask(&mut drawing, l_x, l_ground_y + 400.0 + pointer_offset,
        &format!("FH={:.3}", l_data.planned_height), text_height, 1, "PLAN", HAlign::Left);

    // ========== R側 GH, FH（ポインター分上にオフセット）==========
    let r_ground_y = to_dxf_y(r_data.elevation);
    add_text_with_mask(&mut drawing, r_x, r_ground_y + 800.0 + pointer_offset,
        &format!("GH={:.3}", r_data.elevation), text_height, 7, "TEXT", HAlign::Right);
    add_text_with_mask(&mut drawing, r_x, r_ground_y + 400.0 + pointer_offset,
        &format!("FH={:.3}", r_data.planned_height), text_height, 1, "PLAN", HAlign::Right);

    // ========== 寸法線による旗揚げ（幅員）==========
    let dim_base_y = cl_ground_y + flag_height_mm;
    let mid_l_x = (l_x + cl_x) / 2.0;
    let mid_r_x = (cl_x + r_x) / 2.0;

    // 左幅員の寸法（線とテキストで描画）
    let left_width = (cl_x - l_x).abs() / scale;  // メートル単位
    add_dimension_as_lines(&mut drawing, l_x, cl_x, dim_base_y,
        &format!("{:.2}", left_width), text_height, 7, "DIMENSION");

    // 右幅員の寸法（線とテキストで描画）
    let right_width = (r_x - cl_x).abs() / scale;  // メートル単位
    add_dimension_as_lines(&mut drawing, cl_x, r_x, dim_base_y,
        &format!("{:.2}", right_width), text_height, 7, "DIMENSION");

    // ========== 勾配テキスト（寸法線の上）==========
    let arrow_len = 600.0;
    let arrow_drop = 20.0;  // 矢印の傾き（下がり量）
    let arrow_head = 180.0;
    let arrow_offset = 300.0;  // 矢印用のスペース
    let slope_text_y = flag_y + 500.0 + arrow_offset;  // 矢印分上げる

    // 左側勾配
    add_text_with_mask(&mut drawing, mid_l_x, slope_text_y,
        &format!("{:.1}%", left_slope), text_height, 7, "TEXT", HAlign::Center);
    if left_slope.abs() > 0.01 {
        let arrow_y = slope_text_y - arrow_offset;
        let arrow_x = mid_l_x - arrow_len / 2.0;
        if left_slope < 0.0 {
            // 左下向き矢印（斜め）
            add_line(&mut drawing, arrow_x + arrow_len, arrow_y + arrow_drop, arrow_x, arrow_y - arrow_drop, 7, "SLOPE");
            add_line(&mut drawing, arrow_x, arrow_y - arrow_drop, arrow_x + arrow_head, arrow_y - arrow_drop + arrow_head * 0.4, 7, "SLOPE");
        } else {
            // 右下向き矢印（斜め）
            add_line(&mut drawing, arrow_x, arrow_y + arrow_drop, arrow_x + arrow_len, arrow_y - arrow_drop, 7, "SLOPE");
            add_line(&mut drawing, arrow_x + arrow_len, arrow_y - arrow_drop, arrow_x + arrow_len - arrow_head, arrow_y - arrow_drop + arrow_head * 0.4, 7, "SLOPE");
        }
    }

    // 右側勾配
    add_text_with_mask(&mut drawing, mid_r_x, slope_text_y,
        &format!("{:.1}%", right_slope), text_height, 7, "TEXT", HAlign::Center);
    if right_slope.abs() > 0.01 {
        let arrow_y = slope_text_y - arrow_offset;
        let arrow_x = mid_r_x - arrow_len / 2.0;
        if right_slope < 0.0 {
            // 右下向き矢印（斜め）
            add_line(&mut drawing, arrow_x, arrow_y + arrow_drop, arrow_x + arrow_len, arrow_y - arrow_drop, 7, "SLOPE");
            add_line(&mut drawing, arrow_x + arrow_len, arrow_y - arrow_drop, arrow_x + arrow_len - arrow_head, arrow_y - arrow_drop + arrow_head * 0.4, 7, "SLOPE");
        } else {
            // 左下向き矢印（斜め）
            add_line(&mut drawing, arrow_x + arrow_len, arrow_y + arrow_drop, arrow_x, arrow_y - arrow_drop, 7, "SLOPE");
            add_line(&mut drawing, arrow_x, arrow_y - arrow_drop, arrow_x + arrow_head, arrow_y - arrow_drop + arrow_head * 0.4, 7, "SLOPE");
        }
    }

    // ========== DLラベル ==========
    add_text_rotated(&mut drawing, cl_x, to_dxf_y(dl),
        &format!("DL={:.3}  Scale:H1:V2", dl), text_height * 0.5, 8, "TEXT",
        HAlign::Left, VAlign::Bottom, 0.0);

    // ========== 切削厚表示（DLライン上部） ==========
    let cutting_text_height = text_height;  // GH等と同じサイズ
    let cutting_y = to_dxf_y(dl) - 300.0;  // 2倍
    for (i, pt) in data.iter().enumerate() {
        let x = to_dxf_x(pt.cumulative_distance);
        let cutting_thickness_mm = (pt.elevation - pt.cutting_bottom) * 1000.0;
        let align = if i == 0 {
            HAlign::Left
        } else if i == data.len() - 1 {
            HAlign::Right
        } else {
            HAlign::Center
        };
        add_text_with_mask(&mut drawing, x, cutting_y,
            &format!("{:.0}", cutting_thickness_mm), cutting_text_height, 5, "CUTTING", align);
    }
    // ラベル「切削厚」を中央下に表示
    add_text_with_mask(&mut drawing, cl_x, cutting_y - cutting_text_height - 100.0,
        "切削厚", cutting_text_height, 5, "CUTTING", HAlign::Center);

    // ========== ガイド線 ==========
    let guide_v_length_mm = 1000.0;
    let cl_cumulative = cl_data.cumulative_distance;

    // DLライン（幅員と同じ長さ）
    add_line(&mut drawing, l_x, to_dxf_y(dl), r_x, to_dxf_y(dl), 8, "DIMENSION");

    add_line(&mut drawing,
        to_dxf_x(cl_cumulative), to_dxf_y(dl),
        to_dxf_x(cl_cumulative), to_dxf_y(dl + guide_v_length_mm / 1000.0),
        8, "DIMENSION");

    drawing
}

/// DXFバイト列を生成する（ダウンロード用）
pub fn generate_dxf_bytes(section: &CrossSectionData) -> Vec<u8> {
    let drawing = generate_drawing(section);
    let mut output: Vec<u8> = Vec::new();
    drawing.save(&mut output).expect("Failed to save DXF");
    output
}

// 配置可能エリア定数はtitle_block::available_areaを使用
pub use title_block::available_area;

mod longitudinal;
pub use longitudinal::*;

// ============================================================================
// Application
// ============================================================================

#[derive(PartialEq, Clone, Copy)]
enum ViewMode {
    Single,      // 単一横断図
    AllGrid,     // 全横断図グリッド
    Longitudinal, // 縦断図
    Combo,       // 縦断図＋全横断図
    AreaExpansion, // 面積展開図
    Dekigata,    // 出来形管理用紙
    AlignTest,   // アライメントテスト
    TitleBlock,  // タイトル枠テスト
}

/// 図枠スケール（None=手動、Some(200)=1:200、Some(500)=1:500）
pub(crate) type PlotScale = Option<u32>;

// CrossSectionApp構造体とDefault/メソッド実装は app.rs に移動
// eframe::App トレイト実装は app_ui.rs に移動

// (impl eframe::App ブロックは app_ui.rs に移動)

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_section(name: &str) -> CrossSectionData {
        CrossSectionData {
            survey_point_name: name.to_string(),
            dl: 10.0,
            cl_index: 1,
            l_to_cl_distance: 3.0,
            survey_data: vec![
                SurveyRow {
                    unit_distance: 0.0,
                    elevation: 11.0,
                    planned_height: 11.0,
                    cumulative_distance: -3.0,  // 左端
                    cutting_bottom: 10.95,
                },
                SurveyRow {
                    unit_distance: 3.0,
                    elevation: 11.0,
                    planned_height: 11.0,
                    cumulative_distance: 0.0,  // CL
                    cutting_bottom: 10.95,
                },
                SurveyRow {
                    unit_distance: 3.0,
                    elevation: 11.0,
                    planned_height: 11.0,
                    cumulative_distance: 3.0,  // 右端
                    cutting_bottom: 10.95,
                },
            ],
            route_distance: None,
            route_id: "route_1".to_string(),
            is_route_start: false,
            is_route_end: false,
        }
    }

    #[test]
    fn test_calc_grid_bounds() {
        // 1セクションのバウンディングボックスを確認
        let sections = vec![make_test_section("NO.1")];

        let bounds = calc_grid_bounds(&sections, 1, 2.0, 1.0).unwrap();

        // 幅は6000 (左3m + 右3m = 6m × 1000)
        // X位置はグリッド配置後の値（CL位置 + 累積距離）
        assert!((bounds.width() - 6000.0).abs() < 1.0);

        // 高さの確認（旗揚げを含む）
        assert!(bounds.height() > 0.0);
    }

    #[test]
    fn test_calc_max_columns_for_frame() {
        // 空のセクションリストは1列を返す
        let empty: Vec<CrossSectionData> = vec![];
        assert_eq!(calc_max_columns_for_frame(&empty, 2.0, 1.0, 500.0), 1);

        // 20セクションのテストデータ
        let sections: Vec<CrossSectionData> = (1..=20)
            .map(|i| make_test_section(&format!("NO.{}", i)))
            .collect();

        // 1:500スケールで最大列数を計算
        // 各セクションのサイズ（幅8m × 高さ約5m紙上）を考慮
        let max_cols = calc_max_columns_for_frame(&sections, 2.0, 1.0, 500.0);

        // 20セクションあるので、少なくとも1列以上、最大20列以下
        assert!(max_cols >= 1);
        assert!(max_cols <= 20);

        // 紙上サイズで検証
        // 各セル幅: 8m = 8000 DXF単位 → 1:500で16mm
        // 有効幅: 300mm → 最大約18列程度（実際は高さ制約もある）
    }

    #[test]
    fn test_fits_in_frame() {
        let sections = vec![make_test_section("NO.1")];
        let bounds = calc_grid_bounds(&sections, 1, 2.0, 1.0).unwrap();

        // 1:500で収まるか確認（1列なので高さ制限は241mm）
        // 幅6000→12mm、高さは5550くらい→11mm程度
        assert!(bounds.fits_in_frame(500.0, 1));

        // 1:200でも収まる
        // 幅6000→30mm、高さ5550→27.75mm
        assert!(bounds.fits_in_frame(200.0, 1));

        // 4列の場合は高さ制限が203mmになる
        // 1列でテストしているので4列制限は適用されない
        assert!(bounds.fits_in_frame(500.0, 4));  // 4列でも収まる

        // 1:50だと収まらない（幅120mm、高さ111mm）
        // 有効高さ227mmあるので収まる可能性
        // 実際のテストはデータ次第
    }

    // ========================================================================
    // build_section_content tests
    // ========================================================================

    /// Helper to create test section data for build_section_content tests
    fn make_test_section_for_content() -> CrossSectionData {
        let survey_data = vec![
            SurveyRow {
                unit_distance: 0.0,
                elevation: 10.0,
                planned_height: 10.05,
                cumulative_distance: -3.0,
                cutting_bottom: 9.95,
            },
            SurveyRow {
                unit_distance: 3.0,
                elevation: 10.1,
                planned_height: 10.1,
                cumulative_distance: 0.0,
                cutting_bottom: 10.0,
            },
            SurveyRow {
                unit_distance: 3.0,
                elevation: 10.0,
                planned_height: 10.05,
                cumulative_distance: 3.0,
                cutting_bottom: 9.95,
            },
        ];

        CrossSectionData {
            survey_point_name: "No.1".to_string(),
            dl: 9.0,
            cl_index: 1,
            l_to_cl_distance: 3.0,
            survey_data,
            route_distance: None,
            route_id: "route_1".to_string(),
            is_route_start: false,
            is_route_end: false,
        }
    }

    #[test]
    fn test_build_section_content_basic() {
        // Create a simple CrossSectionData and verify build_section_content returns a non-empty DrawingContent
        let section = make_test_section_for_content();
        let offset_x = 0.0;
        let offset_y = 0.0;
        let scale = 1000.0;  // 1m = 1000 DXF units
        let style = SectionStyle::default();

        let content = build_section_content(&section, offset_x, offset_y, scale, &style, 2.0);

        // Check that primitives.len() > 0
        assert!(content.primitives.len() > 0, "DrawingContent should have primitives");

        // Check that there are both Line and Text primitives
        let has_lines = content.primitives.iter().any(|p| matches!(p, DrawPrimitive::Line(_)));
        let has_texts = content.primitives.iter().any(|p| matches!(p, DrawPrimitive::Text(_)));

        assert!(has_lines, "DrawingContent should contain Line primitives");
        assert!(has_texts, "DrawingContent should contain Text primitives");
    }

    #[test]
    fn test_build_section_content_has_lines() {
        // Verify the content includes ground, plan, and cutting lines
        let section = make_test_section_for_content();
        let content = build_section_content(&section, 0.0, 0.0, 1000.0, &SectionStyle::default(), 2.0);

        // Count Line primitives
        let line_count = content.primitives.iter()
            .filter(|p| matches!(p, DrawPrimitive::Line(_)))
            .count();

        // Verify at least 3 lines exist (GROUND, PLAN, CUTTING)
        // In practice, there will be many more lines (grid lines, dimension lines, etc.)
        assert!(line_count >= 3,
            "Expected at least 3 lines (GROUND, PLAN, CUTTING), found {}", line_count);

        // Check that lines exist on expected layers
        let layers: Vec<&str> = content.primitives.iter()
            .filter_map(|p| match p {
                DrawPrimitive::Line(line) => Some(line.layer.as_str()),
                _ => None,
            })
            .collect();

        // GROUND, PLAN, CUTTING layers should be present
        assert!(layers.iter().any(|&l| l == "GROUND"), "Should have GROUND layer lines");
        assert!(layers.iter().any(|&l| l == "PLAN"), "Should have PLAN layer lines");
        assert!(layers.iter().any(|&l| l == "CUTTING"), "Should have CUTTING layer lines");
    }

    #[test]
    fn test_build_section_content_has_labels() {
        // Verify the content includes text labels
        let section = make_test_section_for_content();
        let content = build_section_content(&section, 0.0, 0.0, 1000.0, &SectionStyle::default(), 2.0);

        // Count Text primitives
        let text_count = content.primitives.iter()
            .filter(|p| matches!(p, DrawPrimitive::Text(_)))
            .count();

        // Verify there are text labels
        assert!(text_count > 0, "Expected text labels in DrawingContent, found none");

        // Collect all text values
        let texts: Vec<&str> = content.primitives.iter()
            .filter_map(|p| match p {
                DrawPrimitive::Text(text) => Some(text.text.as_str()),
                _ => None,
            })
            .collect();

        // Verify survey point name exists in the labels
        let has_survey_point_name = texts.iter().any(|t| t.contains("No.1"));
        assert!(has_survey_point_name,
            "Expected survey point name 'No.1' in text labels. Found texts: {:?}", texts);
    }
}
