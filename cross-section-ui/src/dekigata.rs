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

    // A3用紙の有効領域（mm単位）
    // 上部: 横断図、下部: 出来形管理表
    let table_height_mm = DEKIGATA_TABLE_ROW_HEIGHT_MM * 5.0;  // 5行（ヘッダー + 4データ行）
    let table_bottom_margin_mm = 15.0;  // タイトル枠なしのため縮小

    // 横断図の描画領域（上部）
    // 工事名がある場合は上端を下げて被らないようにする
    let cross_section_bottom_mm = table_bottom_margin_mm + table_height_mm + 25.0;  // 表との間隔を広げる
    let cross_section_top_mm = if has_project_name { 235.0 } else { 255.0 };  // 工事名の有無で調整
    let cross_section_height_mm = cross_section_top_mm - cross_section_bottom_mm;

    // 横断図の描画
    let scale = 1000.0;  // 1m = 1000 DXF単位
    let y_scale = scale * 2.0;  // 縦方向は2倍

    // 横断図のサイズを計算
    let l_data = &data[0];
    let r_data = &data[data.len() - 1];
    let cl_data = &data[section.cl_index.min(data.len() - 1)];
    let total_width_m = (r_data.cumulative_distance - l_data.cumulative_distance).abs();
    let max_elev = data.iter().map(|d| d.elevation.max(d.planned_height)).fold(f64::MIN, f64::max);
    let _min_elev = data.iter().map(|d| d.elevation.min(d.planned_height).min(d.cutting_bottom)).fold(f64::MAX, f64::min);
    let dl = round_dl(section.dl);
    let height_range_m = max_elev - dl + 2.0;  // 旗揚げ分のマージン

    // 横断図のDXF単位でのサイズ
    let _cross_section_width_dxf = total_width_m * scale;
    let _cross_section_height_dxf = height_range_m * y_scale;

    // 横断図を配置する中心位置（図枠内の中央上部）
    let frame_center_x_mm = 210.0;  // A3幅の中央
    let _cross_section_center_y_mm = cross_section_bottom_mm + cross_section_height_mm / 2.0;

    // 横断図のCLを中心に配置
    let cl_offset = cl_data.cumulative_distance;
    let offset_x = origin_x + (frame_center_x_mm * frame_scale) - (cl_offset * scale);
    let offset_y = origin_y + (cross_section_bottom_mm * frame_scale);

    // 横断図を描画（共通関数を使用）
    draw_section_at_offset(drawing, section, offset_x, offset_y, scale, 1.0, v_scale_ratio);

    // 出来形管理表を描画（センタリング、スケールに応じたサイズ）
    let num_points = data.len();
    let table_scale = (frame_scale / 50.0).sqrt().clamp(0.7, 1.3);
    let table_width_mm = (DEKIGATA_TABLE_HEADER_WIDTH_MM + DEKIGATA_TABLE_DATA_WIDTH_MM * num_points as f64) * table_scale;
    let page_width_mm = 420.0;  // A3横幅
    let table_x = origin_x + ((page_width_mm - table_width_mm) / 2.0) * frame_scale;  // センタリング
    let table_y = origin_y + table_bottom_margin_mm * frame_scale;
    draw_dekigata_table(drawing, section, table_x, table_y, frame_scale);
}

/// 出来形管理表を描画
fn draw_dekigata_table(
    drawing: &mut Drawing,
    section: &CrossSectionData,
    table_x: f64,
    table_y: f64,
    frame_scale: f64,
) {
    let data = &section.survey_data;
    let num_points = data.len();

    // スケールに応じてテーブルサイズを可変（1:50基準、1:30で小さく、1:100で大きく）
    let table_scale = (frame_scale / 50.0).sqrt().clamp(0.7, 1.3);
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

