//! 面積展開図 (Area Expansion Drawing)
//!
//! 横断面の幅員データを縦断方向に展開した図面を生成

use dxf::Drawing;
use dxf::Color;

use crate::data_model::CrossSectionData;
use crate::dxf_helpers::{new_drawing, add_line, add_text, add_text_rotated};
use crate::drawing_ir::{HAlign, VAlign};

const DEFAULT_STATION_INTERVAL: f64 = 20.0;

/// 測点名から路線距離を取得（"No.X" → X * DEFAULT_STATION_INTERVAL）
fn parse_station_distance(name: &str) -> f64 {
    if name.starts_with("No.") {
        name[3..].parse::<f64>().unwrap_or(0.0) * DEFAULT_STATION_INTERVAL
    } else {
        0.0
    }
}

// ============================================================================
// Area Expansion Drawing (面積展開図)
// ============================================================================

/// 面積展開図を生成
/// X軸: 縦断図と同じスケール（相対距離 * scale_x）
/// Y軸: 左幅員が正、右幅員が負（縦断図と同じscale_yで5:1）
pub fn generate_area_expansion_drawing(sections: &[CrossSectionData], show_station_names: bool) -> Drawing {
    // スケール設定（DXF単位: mm単位に統一、1m = 1000単位）
    let scale_x = 1000.0;     // 横方向スケール（1m = 1000単位）
    let scale_y = 1000.0;     // 縦方向スケール（1m = 1000単位）縦横比 H1:V1
    let text_height = 2250.0; // 基本テキスト高さ（×10）1.5倍拡大
    let label_width = 11250.0; // 左側ラベル幅（×10）1.5倍拡大
    let margin_x = 3000.0;    // 縦断図と同じマージン
    let station_text_offset = 4500.0; // 測点名のオフセット（×10）1.5倍拡大

    let mut drawing = new_drawing();

    // レイヤー定義
    for (name, color_idx) in [
        ("TENKAI_OUTLINE", 7),   // 外形線（黒）
        ("TENKAI_WIDTH", 7),     // 幅員線（黒）
        ("TENKAI_CENTER", 1),    // センターライン（赤）
        ("TENKAI_STATION", 5),   // 測点名（青）
        ("TENKAI_DIM", 7),       // 寸法テキスト（黒）
    ] {
        drawing.add_layer(dxf::tables::Layer {
            name: name.to_string(),
            color: Color::from_index(color_idx),
            ..Default::default()
        });
    }

    if sections.is_empty() {
        return drawing;
    }

    // 路線距離順にソートしたデータを準備
    let mut sorted_sections: Vec<&CrossSectionData> = sections.iter().collect();
    sorted_sections.sort_by(|a, b| {
        let dist_a = a.route_distance.unwrap_or_else(|| parse_station_distance(&a.survey_point_name));
        let dist_b = b.route_distance.unwrap_or_else(|| parse_station_distance(&b.survey_point_name));
        dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
    });

    // 基準距離（最小値）を取得 - 縦断図と同じ方式
    let base_distance = sorted_sections.iter()
        .map(|s| s.route_distance.unwrap_or_else(|| parse_station_distance(&s.survey_point_name)))
        .fold(f64::INFINITY, f64::min);

    // 各測点のX位置と幅員を収集
    struct StationInfo {
        x: f64,           // X座標（相対距離 * scale_x + label_width）
        wl: f64,          // 左幅員（正）
        wr: f64,          // 右幅員（正、描画時は負方向）
        name: String,     // 測点名
        dist: f64,        // 元の路線距離
    }

    let mut stations: Vec<StationInfo> = Vec::new();

    for section in &sorted_sections {
        let dist = section.route_distance.unwrap_or_else(|| parse_station_distance(&section.survey_point_name));
        // 縦断図と同じX計算方式（相対距離）
        let x = label_width + margin_x + (dist - base_distance) * scale_x;

        // 左幅員: L端からCLまでの距離
        let wl = section.l_to_cl_distance;

        // 右幅員: survey_dataの最後の点のcumulative_distance（CLからの距離）
        let wr = if let Some(last) = section.survey_data.last() {
            last.cumulative_distance.abs()
        } else {
            0.0
        };

        stations.push(StationInfo {
            x,
            wl,
            wr,
            name: section.survey_point_name.clone(),
            dist,
        });
    }

    if stations.is_empty() {
        return drawing;
    }

    // ========== 描画 ==========

    // 各測点での垂直線（幅員線）
    for station in &stations {
        let x = station.x;
        let y_top = station.wl * scale_y;     // 左幅員（上方向）
        let y_bottom = -station.wr * scale_y; // 右幅員（下方向）

        // 幅員線（垂直）
        add_line(&mut drawing, x, y_bottom, x, y_top, 7, "TENKAI_WIDTH");
    }

    // センターライン（Y=0で測点間を接続）
    for i in 0..stations.len() - 1 {
        let x1 = stations[i].x;
        let x2 = stations[i + 1].x;
        add_line(&mut drawing, x1, 0.0, x2, 0.0, 1, "TENKAI_CENTER");
    }

    // 外形線（上端・下端を測点間で接続）
    for i in 0..stations.len() - 1 {
        let x1 = stations[i].x;
        let x2 = stations[i + 1].x;
        let y1_top = stations[i].wl * scale_y;
        let y2_top = stations[i + 1].wl * scale_y;
        let y1_bottom = -stations[i].wr * scale_y;
        let y2_bottom = -stations[i + 1].wr * scale_y;

        // 上端（左幅員側）
        add_line(&mut drawing, x1, y1_top, x2, y2_top, 7, "TENKAI_OUTLINE");
        // 下端（右幅員側）
        add_line(&mut drawing, x1, y1_bottom, x2, y2_bottom, 7, "TENKAI_OUTLINE");
    }

    // 延長寸法（測点間中央、Y=0）
    for i in 0..stations.len() - 1 {
        let x1 = stations[i].x;
        let x2 = stations[i + 1].x;
        let mid_x = (x1 + x2) / 2.0;
        let extension = (stations[i + 1].dist - stations[i].dist).abs();
        let text = format!("{:.1}", extension);
        add_text(&mut drawing, mid_x, text_height * 0.5, &text, text_height, 7, "TENKAI_DIM", HAlign::Center);
    }

    // 幅員寸法（-90°回転、幅員線の外側）
    for station in &stations {
        let x = station.x;

        // 左幅員（上側外側に配置）
        if station.wl > 0.0 {
            let y_text = station.wl * scale_y + station_text_offset;
            let text = format!("{:.2}", station.wl);
            add_text_rotated(&mut drawing, x, y_text, &text, text_height,
                7, "TENKAI_DIM", HAlign::Left, VAlign::Top, -90.0);
        }

        // 右幅員（下側外側に配置）
        if station.wr > 0.0 {
            let y_text = -station.wr * scale_y - station_text_offset;
            let text = format!("{:.2}", station.wr);
            add_text_rotated(&mut drawing, x, y_text, &text, text_height,
                7, "TENKAI_DIM", HAlign::Right, VAlign::Top, -90.0);
        }
    }

    // 測点名（-90°回転、青色、上端オフセット）- オプションで非表示可能
    if show_station_names {
        // 最大文字数から高さを計算
        let max_name_len = stations.iter().map(|s| s.name.chars().count()).max().unwrap_or(6);
        let station_name_height = max_name_len as f64 * text_height * 0.8;
        let max_wl = stations.iter().map(|s| s.wl).fold(0.0_f64, f64::max);
        let y_name_base = max_wl * scale_y + station_text_offset + station_name_height;

        for station in &stations {
            let x = station.x;
            let name_len = station.name.chars().count() as f64;
            // アンダーライン（テキストの左側に縦線）
            let underline_y1 = y_name_base;
            let underline_y2 = y_name_base - name_len * text_height * 0.8;
            add_line(&mut drawing, x, underline_y1, x, underline_y2, 5, "TENKAI_STATION");
            // 測点名テキスト（アンダーラインの右側）
            add_text_rotated(&mut drawing, x, y_name_base, &station.name, text_height * 1.2,
                5, "TENKAI_STATION", HAlign::Left, VAlign::Bottom, -90.0);
        }
    }

    drawing
}

/// 面積展開図のDXFバイト列を生成
pub fn generate_area_expansion_dxf_bytes(sections: &[CrossSectionData]) -> Vec<u8> {
    let drawing = generate_area_expansion_drawing(sections, true);
    let mut output: Vec<u8> = Vec::new();
    drawing.save(&mut output).expect("Failed to save DXF");
    output
}
