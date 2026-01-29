//! 出来形管理用紙の生成モジュール
//!
//! 横断図と出来形管理表を組み合わせた管理用紙のDXF生成

use dxf::Drawing;
use dxf::Color;

use crate::title_block::{self, TitleBlockInfo};
use crate::data_model::CrossSectionData;
use crate::dxf_helpers::{new_drawing, add_line, add_text, round_dl};
use crate::drawing_ir::HAlign;
use crate::draw_section_at_offset;

// ============================================================================
// 出来形管理用紙 (Dekigata)
// ============================================================================

/// 出来形管理用紙のデフォルト切削厚（mm）
const DEKIGATA_DEFAULT_CUTTING_THICKNESS_MM: f64 = 50.0;

/// 出来形管理表のセル高さ（mm）
const DEKIGATA_TABLE_ROW_HEIGHT_MM: f64 = 20.0;  // 2.5倍に拡大

/// 出来形管理表のヘッダー列幅（mm）
const DEKIGATA_TABLE_HEADER_WIDTH_MM: f64 = 60.0;  // 2.5倍に拡大

/// 出来形管理表のデータ列幅（mm）
const DEKIGATA_TABLE_DATA_WIDTH_MM: f64 = 45.0;  // 2.5倍に拡大

/// 出来形管理表のテキストサイズ（mm）
const DEKIGATA_TABLE_TEXT_SIZE_MM: f64 = 10.0;  // さらに拡大

/// 幅員表のヘッダー列幅（mm）
const FUKUIN_TABLE_HEADER_WIDTH_MM: f64 = 60.0;

/// 幅員表のデータ列幅（mm）
const FUKUIN_TABLE_DATA_WIDTH_MM: f64 = 50.0;

/// A3用紙の高さ（mm）
const A3_HEIGHT_MM: f64 = 297.0;

/// 内枠マージン（mm）
const FRAME_MARGIN_MM: f64 = 10.0;

/// 工事名の上マージン（mm）
const PROJECT_NAME_TOP_MARGIN_MM: f64 = 15.0;

/// 工事名のテキスト高さ（mm）
const PROJECT_NAME_TEXT_HEIGHT_MM: f64 = 12.0;

/// 横断図の旗揚げ高さ（DXF単位、SectionStyleのデフォルト値から計算）
/// flag_base_offset(1600) + label_title_offset(1300) + text_height*title_scale(450) = 3350
const CROSS_SECTION_FLAG_HEIGHT_DXF: f64 = 3350.0;

/// 横断図の下部マージン（DXF単位、切削厚ラベル分）
const CROSS_SECTION_BOTTOM_MARGIN_DXF: f64 = 700.0;

/// 横断図の高さを計算（mm単位）
fn calc_cross_section_height_mm(
    section: &CrossSectionData,
    frame_scale: f64,
    v_scale_ratio: f64,
) -> f64 {
    let data = &section.survey_data;
    if data.len() < 2 { return 0.0; }

    let dl = round_dl(section.dl);
    let max_elev = data.iter()
        .map(|d| d.elevation.max(d.planned_height))
        .fold(f64::MIN, f64::max);

    let scale = 1000.0;  // 1m = 1000 DXF単位
    let y_scale = scale * v_scale_ratio;

    // 横断図の本体高さ（地盤高〜最高点）
    let body_height_dxf = (max_elev - dl) * y_scale;

    // 旗揚げ高さを加算
    let total_height_dxf = body_height_dxf + CROSS_SECTION_FLAG_HEIGHT_DXF + CROSS_SECTION_BOTTOM_MARGIN_DXF;

    // mm単位に変換
    total_height_dxf / frame_scale
}

/// レイアウトパラメータを計算（重なり判定付き）
struct DekigataLayout {
    table_scale: f64,
    cross_section_bottom_mm: f64,
    project_name_bottom_mm: f64,
}

fn calc_dekigata_layout(
    section: &CrossSectionData,
    frame_scale: f64,
    v_scale_ratio: f64,
    has_project_name: bool,
) -> DekigataLayout {
    // 利用可能な高さ（mm）
    let usable_top_mm = A3_HEIGHT_MM - FRAME_MARGIN_MM;
    let usable_bottom_mm = FRAME_MARGIN_MM;

    // 工事名の位置（上端から）
    let project_name_y_mm = if has_project_name {
        usable_top_mm - PROJECT_NAME_TOP_MARGIN_MM
    } else {
        usable_top_mm
    };
    // 工事名の下端（テキスト高さを考慮）
    let project_name_bottom_mm = if has_project_name {
        project_name_y_mm - PROJECT_NAME_TEXT_HEIGHT_MM - 5.0  // 5mmマージン
    } else {
        usable_top_mm
    };

    // 初期テーブルスケール
    let mut table_scale = (frame_scale / 50.0).sqrt().clamp(0.7, 1.3);

    // 反復して適切なスケールを見つける
    for _ in 0..10 {
        // テーブルの合計高さ
        let kijunko_table_height_mm = DEKIGATA_TABLE_ROW_HEIGHT_MM * 5.0 * table_scale;
        let fukuin_table_height_mm = DEKIGATA_TABLE_ROW_HEIGHT_MM * 3.0 * table_scale;
        let table_gap_mm = 5.0;
        let table_bottom_margin_mm = 15.0;
        let table_to_diagram_gap_mm = 15.0;
        let total_tables_height_mm = fukuin_table_height_mm + table_gap_mm + kijunko_table_height_mm;

        // 横断図の配置
        let cross_section_bottom_mm = usable_bottom_mm + table_bottom_margin_mm + total_tables_height_mm + table_to_diagram_gap_mm;
        let cross_section_height_mm = calc_cross_section_height_mm(section, frame_scale, v_scale_ratio);
        let cross_section_top_mm = cross_section_bottom_mm + cross_section_height_mm;

        // 重なり判定
        let overlap = cross_section_top_mm - project_name_bottom_mm;

        if overlap <= 0.0 {
            // 重なりなし
            return DekigataLayout {
                table_scale,
                cross_section_bottom_mm,
                project_name_bottom_mm,
            };
        }

        // 重なりあり：テーブルスケールを縮小
        // 必要な縮小量を計算
        let available_height = project_name_bottom_mm - usable_bottom_mm - table_bottom_margin_mm - table_to_diagram_gap_mm - cross_section_height_mm;
        if available_height <= 0.0 {
            // 横断図自体が大きすぎる場合は最小スケールを使用
            table_scale = 0.5;
            break;
        }

        // 新しいテーブルスケールを計算
        let required_table_height = available_height - table_gap_mm;
        let base_table_height = DEKIGATA_TABLE_ROW_HEIGHT_MM * 8.0;  // 5行 + 3行
        let new_scale = (required_table_height / base_table_height).clamp(0.5, table_scale - 0.05);

        if (new_scale - table_scale).abs() < 0.01 {
            break;
        }
        table_scale = new_scale;
    }

    // 最終レイアウト計算
    let kijunko_table_height_mm = DEKIGATA_TABLE_ROW_HEIGHT_MM * 5.0 * table_scale;
    let fukuin_table_height_mm = DEKIGATA_TABLE_ROW_HEIGHT_MM * 3.0 * table_scale;
    let table_gap_mm = 5.0;
    let table_bottom_margin_mm = 15.0;
    let table_to_diagram_gap_mm = 15.0;
    let total_tables_height_mm = fukuin_table_height_mm + table_gap_mm + kijunko_table_height_mm;
    let cross_section_bottom_mm = usable_bottom_mm + table_bottom_margin_mm + total_tables_height_mm + table_to_diagram_gap_mm;

    DekigataLayout {
        table_scale,
        cross_section_bottom_mm,
        project_name_bottom_mm,
    }
}

/// 単一測点の出来形管理用紙を描画
///
/// # Arguments
/// * `drawing` - DXFドローイング
/// * `section` - 横断データ
/// * `origin_x` - 原点X座標（DXF単位）
/// * `origin_y` - 原点Y座標（DXF単位）
/// * `frame_scale` - 図枠スケール（1:50なら50.0）
/// * `v_scale_ratio` - 縦方向スケール倍率
/// * `has_project_name` - 工事名が表示されるかどうか
fn draw_dekigata_page(
    drawing: &mut Drawing,
    section: &CrossSectionData,
    origin_x: f64,
    origin_y: f64,
    frame_scale: f64,
    v_scale_ratio: f64,
    has_project_name: bool,
) {
    let data = &section.survey_data;
    if data.len() < 2 { return; }

    // レイアウト計算（重なり判定付き）
    let layout = calc_dekigata_layout(section, frame_scale, v_scale_ratio, has_project_name);
    let table_scale = layout.table_scale;

    // テーブル高さ（スケール適用後）
    let kijunko_table_height_mm = DEKIGATA_TABLE_ROW_HEIGHT_MM * 5.0 * table_scale;
    let fukuin_table_height_mm = DEKIGATA_TABLE_ROW_HEIGHT_MM * 3.0 * table_scale;
    let table_gap_mm = 5.0;
    let table_bottom_margin_mm = FRAME_MARGIN_MM + 5.0;  // 内枠マージン + 追加マージン

    // 横断図の描画
    let scale = 1000.0;  // 1m = 1000 DXF単位
    let cl_data = &data[section.cl_index.min(data.len() - 1)];
    let frame_center_x_mm = 210.0;  // A3幅の中央

    // 横断図のCLを中心に配置
    let cl_offset = cl_data.cumulative_distance;
    let offset_x = origin_x + (frame_center_x_mm * frame_scale) - (cl_offset * scale);
    let offset_y = origin_y + (layout.cross_section_bottom_mm * frame_scale);

    // 横断図を描画（共通関数を使用）
    draw_section_at_offset(drawing, section, offset_x, offset_y, scale, 1.0, v_scale_ratio);

    // 出来形管理表を描画（センタリング）
    let num_points = data.len();
    let kijunko_table_width_mm = (DEKIGATA_TABLE_HEADER_WIDTH_MM + DEKIGATA_TABLE_DATA_WIDTH_MM * num_points as f64) * table_scale;
    let page_width_mm = 420.0;  // A3横幅

    // 幅員表を下部に描画（3行×2列）
    let fukuin_table_width_mm = (FUKUIN_TABLE_HEADER_WIDTH_MM + FUKUIN_TABLE_DATA_WIDTH_MM * 2.0) * table_scale;
    let fukuin_table_x = origin_x + ((page_width_mm - fukuin_table_width_mm) / 2.0) * frame_scale;
    let fukuin_table_y = origin_y + table_bottom_margin_mm * frame_scale;
    draw_fukuin_table_with_scale(drawing, section, fukuin_table_x, fukuin_table_y, frame_scale, table_scale);

    // 基準高表を幅員表の上に描画
    let kijunko_table_x = origin_x + ((page_width_mm - kijunko_table_width_mm) / 2.0) * frame_scale;
    let kijunko_table_y = fukuin_table_y + (fukuin_table_height_mm + table_gap_mm) * frame_scale;
    draw_dekigata_table_with_scale(drawing, section, kijunko_table_x, kijunko_table_y, frame_scale, table_scale);
}

/// 幅員表を描画（3行×2列: 左幅員・右幅員）
fn draw_fukuin_table(
    drawing: &mut Drawing,
    section: &CrossSectionData,
    table_x: f64,
    table_y: f64,
    frame_scale: f64,
) {
    let table_scale = (frame_scale / 50.0).sqrt().clamp(0.7, 1.3);
    draw_fukuin_table_with_scale(drawing, section, table_x, table_y, frame_scale, table_scale);
}

/// 幅員表を描画（スケール指定版）
fn draw_fukuin_table_with_scale(
    drawing: &mut Drawing,
    section: &CrossSectionData,
    table_x: f64,
    table_y: f64,
    frame_scale: f64,
    table_scale: f64,
) {
    let data = &section.survey_data;
    if data.len() < 2 { return; }

    let row_height = DEKIGATA_TABLE_ROW_HEIGHT_MM * frame_scale * table_scale;
    let header_width = FUKUIN_TABLE_HEADER_WIDTH_MM * frame_scale * table_scale;
    let data_width = FUKUIN_TABLE_DATA_WIDTH_MM * frame_scale * table_scale;
    let text_size = DEKIGATA_TABLE_TEXT_SIZE_MM * frame_scale * table_scale;

    let total_width = header_width + data_width * 2.0;  // 2列（左幅員・右幅員）
    let total_height = row_height * 3.0;  // 3行

    // 外枠を描画
    add_line(drawing, table_x, table_y, table_x + total_width, table_y, 7, "DEKIGATA");
    add_line(drawing, table_x, table_y + total_height, table_x + total_width, table_y + total_height, 7, "DEKIGATA");
    add_line(drawing, table_x, table_y, table_x, table_y + total_height, 7, "DEKIGATA");
    add_line(drawing, table_x + total_width, table_y, table_x + total_width, table_y + total_height, 7, "DEKIGATA");

    // 横線（行の区切り）
    for i in 1..3 {
        let y = table_y + row_height * i as f64;
        add_line(drawing, table_x, y, table_x + total_width, y, 7, "DEKIGATA");
    }

    // ヘッダー列の縦線
    add_line(drawing, table_x + header_width, table_y, table_x + header_width, table_y + total_height, 7, "DEKIGATA");

    // データ列の縦線（左幅員と右幅員の境界）
    let mid_x = table_x + header_width + data_width;
    add_line(drawing, mid_x, table_y, mid_x, table_y + total_height, 7, "DEKIGATA");

    // ヘッダーラベル（上から: ヘッダー, 設計, 実測）
    let row_labels = ["", "設計", "実測"];
    let row_colors: [i16; 3] = [7, 7, 1];  // 実測行は赤
    for (i, label) in row_labels.iter().enumerate() {
        if !label.is_empty() {
            let y = table_y + total_height - row_height * (i as f64 + 0.5);
            let color = row_colors[i];
            add_text(drawing, table_x + header_width / 2.0, y, *label, text_size, color, "DEKIGATA", HAlign::Center);
        }
    }

    // 幅員の計算
    let cl_data = &data[section.cl_index.min(data.len() - 1)];
    let l_data = &data[0];
    let r_data = &data[data.len() - 1];

    let left_width = (cl_data.cumulative_distance - l_data.cumulative_distance).abs();
    let right_width = (r_data.cumulative_distance - cl_data.cumulative_distance).abs();

    // 列ヘッダー（左幅員・右幅員）
    let left_col_x = table_x + header_width + data_width * 0.5;
    let right_col_x = table_x + header_width + data_width * 1.5;
    let header_y = table_y + total_height - row_height * 0.5;

    add_text(drawing, left_col_x, header_y, "左幅員", text_size, 7, "DEKIGATA", HAlign::Center);
    add_text(drawing, right_col_x, header_y, "右幅員", text_size, 7, "DEKIGATA", HAlign::Center);

    // 設計値
    let design_y = table_y + total_height - row_height * 1.5;
    add_text(drawing, left_col_x, design_y, &format!("{:.2}", left_width), text_size, 7, "DEKIGATA", HAlign::Center);
    add_text(drawing, right_col_x, design_y, &format!("{:.2}", right_width), text_size, 7, "DEKIGATA", HAlign::Center);

    // 実測値は空欄（手書き用）
    // let actual_y = table_y + total_height - row_height * 2.5;
}

/// 出来形管理表を描画
fn draw_dekigata_table(
    drawing: &mut Drawing,
    section: &CrossSectionData,
    table_x: f64,
    table_y: f64,
    frame_scale: f64,
) {
    let table_scale = (frame_scale / 50.0).sqrt().clamp(0.7, 1.3);
    draw_dekigata_table_with_scale(drawing, section, table_x, table_y, frame_scale, table_scale);
}

/// 出来形管理表を描画（スケール指定版）
fn draw_dekigata_table_with_scale(
    drawing: &mut Drawing,
    section: &CrossSectionData,
    table_x: f64,
    table_y: f64,
    frame_scale: f64,
    table_scale: f64,
) {
    let data = &section.survey_data;
    let num_points = data.len();

    let row_height = DEKIGATA_TABLE_ROW_HEIGHT_MM * frame_scale * table_scale;
    let header_width = DEKIGATA_TABLE_HEADER_WIDTH_MM * frame_scale * table_scale;
    let data_width = DEKIGATA_TABLE_DATA_WIDTH_MM * frame_scale * table_scale;
    let text_size = DEKIGATA_TABLE_TEXT_SIZE_MM * frame_scale * table_scale;

    let total_width = header_width + data_width * num_points as f64;
    let total_height = row_height * 5.0;  // 5行

    // 外枠を描画
    add_line(drawing, table_x, table_y, table_x + total_width, table_y, 7, "DEKIGATA");
    add_line(drawing, table_x, table_y + total_height, table_x + total_width, table_y + total_height, 7, "DEKIGATA");
    add_line(drawing, table_x, table_y, table_x, table_y + total_height, 7, "DEKIGATA");
    add_line(drawing, table_x + total_width, table_y, table_x + total_width, table_y + total_height, 7, "DEKIGATA");

    // 横線（行の区切り）
    for i in 1..5 {
        let y = table_y + row_height * i as f64;
        add_line(drawing, table_x, y, table_x + total_width, y, 7, "DEKIGATA");
    }

    // ヘッダー列の縦線
    add_line(drawing, table_x + header_width, table_y, table_x + header_width, table_y + total_height, 7, "DEKIGATA");

    // データ列の縦線
    for i in 1..num_points {
        let x = table_x + header_width + data_width * i as f64;
        add_line(drawing, x, table_y, x, table_y + total_height, 7, "DEKIGATA");
    }

    // ヘッダーラベル（上から下へ: V1-Vn, 計画高(設計), 計画高(実施), 切削高(設計), 切削高(実施)）
    // 実施行は赤色(1)、ラベルは1行で表示
    let row_labels = ["", "計画高(設計)", "計画高(実施)", "切削高(設計)", "切削高(実施)"];
    let row_colors: [i16; 5] = [7, 7, 1, 7, 1];  // 実施行は赤
    for (i, label) in row_labels.iter().enumerate() {
        if !label.is_empty() {
            let y = table_y + total_height - row_height * (i as f64 + 0.5);
            let color = row_colors[i];
            add_text(drawing, table_x + header_width / 2.0, y, *label, text_size, color, "DEKIGATA", HAlign::Center);
        }
    }

    // 測点ヘッダー（V1, V2, ...）とデータ
    for (col, pt) in data.iter().enumerate() {
        let col_center_x = table_x + header_width + data_width * (col as f64 + 0.5);

        // 測点ラベル（最上行）
        let v_label = format!("V{}", col + 1);
        let header_y = table_y + total_height - row_height * 0.5;
        add_text(drawing, col_center_x, header_y, &v_label, text_size, 5, "DEKIGATA", HAlign::Center);

        // 計画高（設計）
        let design_fh_y = table_y + total_height - row_height * 1.5;
        add_text(drawing, col_center_x, design_fh_y, &format!("{:.3}", pt.planned_height), text_size, 7, "DEKIGATA", HAlign::Center);

        // 計画高（実施）- 空欄（赤）
        let _actual_fh_y = table_y + total_height - row_height * 2.5;
        // 実施行は手書き用に空欄

        // 切削高（設計）= 計画高 - 切削厚
        let design_cutting = pt.planned_height - DEKIGATA_DEFAULT_CUTTING_THICKNESS_MM / 1000.0;
        let design_cut_y = table_y + total_height - row_height * 3.5;
        add_text(drawing, col_center_x, design_cut_y, &format!("{:.3}", design_cutting), text_size, 7, "DEKIGATA", HAlign::Center);

        // 切削高（実施）- 空欄（赤）
        let _actual_cut_y = table_y + total_height - row_height * 4.5;
        // 実施行は手書き用に空欄
    }
}

/// 単一測点の出来形管理用紙のDrawingを生成
pub fn generate_dekigata_drawing(section: &CrossSectionData, title_info: &TitleBlockInfo, v_scale_ratio: f64, dekigata_scale: f64) -> Drawing {
    let mut drawing = new_drawing();

    // レイヤー作成
    drawing.add_layer(dxf::tables::Layer {
        name: "GROUND".to_string(),
        color: Color::from_index(7),
        ..Default::default()
    });
    drawing.add_layer(dxf::tables::Layer {
        name: "PLAN".to_string(),
        color: Color::from_index(1),
        ..Default::default()
    });
    drawing.add_layer(dxf::tables::Layer {
        name: "TEXT".to_string(),
        color: Color::from_index(7),
        ..Default::default()
    });
    drawing.add_layer(dxf::tables::Layer {
        name: "DIMENSION".to_string(),
        color: Color::from_index(8),
        ..Default::default()
    });
    drawing.add_layer(dxf::tables::Layer {
        name: "CUTTING".to_string(),
        color: Color::from_index(5),
        ..Default::default()
    });
    drawing.add_layer(dxf::tables::Layer {
        name: "DEKIGATA".to_string(),
        color: Color::from_index(7),
        ..Default::default()
    });

    let frame_scale = dekigata_scale;

    // 図枠を描画（外枠のみ）
    title_block::add_title_block_layer(&mut drawing);
    title_block::draw_outer_frame(&mut drawing, 0.0, 0.0, frame_scale);

    // 工事名のみ表示（タイトル・横棒は不要）
    let text_size = 12.0 * frame_scale;  // 4倍サイズ（元の2倍×さらに2倍）
    let name_y = (277.0 - 15.0) * frame_scale;  // 内枠上端から15mm下
    let center_x = 210.0 * frame_scale;  // A3中央
    add_text(&mut drawing, center_x, name_y, &title_info.project_name, text_size, 7, "TITLEBLOCK", HAlign::Center);

    // 出来形管理用紙の内容を描画
    draw_dekigata_page(&mut drawing, section, 0.0, 0.0, frame_scale, v_scale_ratio, title_info.has_project_name());

    drawing
}

/// 整数測点かどうかを判定（"+"を含む中間測点を除外）
fn is_integer_station(name: &str) -> bool {
    !name.contains('+')
}

/// 全測点の出来形管理用紙を縦並びで生成
/// 中間測点（No.1+10など）は除外
pub fn generate_all_dekigata_pages(
    all_sections: &[CrossSectionData],
    base_title_info: &TitleBlockInfo,
    drawing_number_base: &str,
    v_scale_ratio: f64,
    dekigata_scale: f64,
) -> Drawing {
    let mut drawing = new_drawing();

    // レイヤー作成
    drawing.add_layer(dxf::tables::Layer {
        name: "GROUND".to_string(),
        color: Color::from_index(7),
        ..Default::default()
    });
    drawing.add_layer(dxf::tables::Layer {
        name: "PLAN".to_string(),
        color: Color::from_index(1),
        ..Default::default()
    });
    drawing.add_layer(dxf::tables::Layer {
        name: "TEXT".to_string(),
        color: Color::from_index(7),
        ..Default::default()
    });
    drawing.add_layer(dxf::tables::Layer {
        name: "DIMENSION".to_string(),
        color: Color::from_index(8),
        ..Default::default()
    });
    drawing.add_layer(dxf::tables::Layer {
        name: "CUTTING".to_string(),
        color: Color::from_index(5),
        ..Default::default()
    });
    drawing.add_layer(dxf::tables::Layer {
        name: "DEKIGATA".to_string(),
        color: Color::from_index(7),
        ..Default::default()
    });

    // 整数測点のみをフィルタ
    let filtered_sections: Vec<&CrossSectionData> = all_sections
        .iter()
        .filter(|s| is_integer_station(&s.survey_point_name))
        .collect();

    if filtered_sections.is_empty() {
        return drawing;
    }

    let frame_scale = dekigata_scale;
    let page_height_dxf = 297.0 * frame_scale;  // A3高さ
    const PAGE_GAP_MM: f64 = 10.0;
    let page_gap_dxf = PAGE_GAP_MM * frame_scale;
    const TEXT_SIZE_MM: f64 = 3.0;

    let total_pages = filtered_sections.len();

    for (page_idx, section) in filtered_sections.iter().enumerate() {
        let page_offset_y = (page_idx as f64) * (page_height_dxf + page_gap_dxf);

        // 図面番号を更新
        let drawing_number = if drawing_number_base.is_empty() {
            format!("{}/{}", page_idx + 1, total_pages)
        } else {
            format!("{}-{}/{}", drawing_number_base, page_idx + 1, total_pages)
        };

        let info = base_title_info.clone()
            .with_top_title("出来形管理用紙")
            .with_scale(&format!("1:{} (A3)", dekigata_scale as u32))
            .with_drawing_number(&drawing_number);

        // 図枠を描画（タイトル枠なし - 外枠と上部タイトルのみ）
        title_block::add_title_block_layer(&mut drawing);
        title_block::draw_outer_frame(&mut drawing, 0.0, page_offset_y, frame_scale);
        title_block::draw_top_title(&mut drawing, &info, 0.0, page_offset_y, frame_scale, TEXT_SIZE_MM);
        // draw_title_block は呼ばない（出来形管理用紙では不要）

        // 出来形管理用紙の内容を描画
        draw_dekigata_page(&mut drawing, section, 0.0, page_offset_y, frame_scale, v_scale_ratio, info.has_project_name());
    }

    drawing
}

/// 全測点の出来形管理用紙をDXFバイト配列として生成
pub fn generate_all_dekigata_dxf_bytes(
    all_sections: &[CrossSectionData],
    base_title_info: &TitleBlockInfo,
    drawing_number_base: &str,
    v_scale_ratio: f64,
    dekigata_scale: f64,
) -> Vec<u8> {
    let drawing = generate_all_dekigata_pages(all_sections, base_title_info, drawing_number_base, v_scale_ratio, dekigata_scale);
    let mut output: Vec<u8> = Vec::new();
    drawing.save(&mut output).expect("Failed to save DXF");
    output
}

