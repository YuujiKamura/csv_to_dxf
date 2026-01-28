//! Longitudinal Profile (縦断図)
//!
//! 縦断図の生成機能を提供

use dxf::Drawing;
use dxf::Color;

use crate::{
    CrossSectionData,
    new_drawing,
    add_line,
    add_text,
    add_text_rotated,
    HAlign,
    VAlign,
};

/// 測点間隔のデフォルト値（メートル）
/// 土木工事では測点間隔は通常20mが標準
const DEFAULT_STATION_INTERVAL: f64 = 20.0;

/// 測点名から路線距離を取得（"No.X" → X * DEFAULT_STATION_INTERVAL）
fn parse_station_distance(name: &str) -> f64 {
    if name.starts_with("No.") {
        name[3..].parse::<f64>().unwrap_or(0.0) * DEFAULT_STATION_INTERVAL
    } else {
        0.0
    }
}

/// 測点名がプラス杭かどうか判定（例: "No.1+5" はプラス杭）
#[allow(dead_code)]
fn is_plus_stake(name: &str) -> bool {
    name.contains('+')
}

/// 縦断図を生成（土木標準形式）
pub fn generate_longitudinal_drawing(sections: &[CrossSectionData]) -> Drawing {
    // スケール設定（DXF単位: mm単位に統一、1m = 1000単位）
    let scale_x = 1000.0;     // 横方向スケール（1m = 1000単位）
    let scale_y = 5000.0;     // 縦方向スケール（1m = 5000単位）縦横比 H1:V5
    let text_height = 2250.0; // 基本テキスト高さ（×10）1.5倍拡大
    let label_width = 11250.0; // 左側のラベル幅（×10）1.5倍拡大
    let _title_height = 0.0; // タイトルなし（ピッチリ）

    let mut drawing = new_drawing();

    // レイヤー作成
    for (name, color_idx) in [
        ("GROUND", 7),    // 黒 - 現地盤高
        ("PLAN", 1),      // 赤 - 計画高
        ("GRID", 8),      // グレー - グリッド
        ("TABLE", 7),     // 黒 - 表枠
        ("TEXT", 7),      // 黒 - テキスト
        ("TITLE", 7),     // 黒 - タイトル
        ("ANNOTATION", 5), // 青 - 注釈
    ] {
        let mut layer = dxf::tables::Layer::default();
        layer.name = name.to_string();
        layer.color = Color::from_index(color_idx);
        drawing.add_layer(layer);
    }

    // データ収集（路線距離順にソート）
    // route_distanceがあればそれを優先、なければ測点名からパース
    let mut points: Vec<(f64, f64, f64, String)> = sections.iter().map(|s| {
        let cl = &s.survey_data[s.cl_index.min(s.survey_data.len() - 1)];
        let dist = s.route_distance.unwrap_or_else(|| parse_station_distance(&s.survey_point_name));
        (dist, cl.elevation, cl.planned_height, s.survey_point_name.clone())
    }).collect();
    points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    if points.is_empty() { return drawing; }

    // 通常行の高さ（固定）（×10）
    let row_height = 8250.0;  // 1.5倍拡大（5500 * 1.5）
    // 測点名行の高さ（最大文字数に基づいて計算）
    let max_name_len = points.iter().map(|p| p.3.chars().count()).max().unwrap_or(6);
    let station_row_height = (max_name_len as f64 * text_height * 0.73).max(row_height);

    // 範囲計算
    let min_dist = points.first().map(|p| p.0).unwrap_or(0.0);
    let max_dist = points.last().map(|p| p.0).unwrap_or(100.0);
    let min_elev = points.iter().map(|p| p.1.min(p.2)).fold(f64::MAX, f64::min);
    let max_elev = points.iter().map(|p| p.1.max(p.2)).fold(f64::MIN, f64::max);

    // DL（基準高）を1m単位で切り下げ（上下2mマージン）
    let dl = (min_elev - 2.0).floor();
    let graph_top = (max_elev + 2.0).ceil();

    // 左右マージン（最初と最後の測点が表枠境界から離れるように）
    let margin_x = 3000.0;  // 3m分のマージン（テキスト1.5倍拡大に対応）

    // 座標変換
    let to_dxf_x = |d: f64| label_width + margin_x + (d - min_dist) * scale_x;
    let to_dxf_y = |h: f64| (h - dl) * scale_y;

    let graph_width = (max_dist - min_dist) * scale_x + margin_x * 2.0;
    let graph_height = (graph_top - dl) * scale_y;

    // データ表の行定義（上から下へ）
    // 行0-6は通常高さ、行7（測点名）は独立した高さ
    let table_rows = [
        ("勾配", 0),
        ("盛土", 1),
        ("切土", 2),
        ("計画高", 3),
        ("地盤高", 4),
        ("追加距離", 5),
        ("単距離", 6),
        ("測点名", 7),
    ];
    let table_top = 0.0;
    let normal_rows_bottom = table_top - 7.0 * row_height;  // 行0-6
    let table_bottom = normal_rows_bottom - station_row_height;  // 行7（測点名）

    // タイトルは描画しない（ピッチリ）

    // ===================
    // グラフ部分の描画
    // ===================

    // 横グリッド線（標高ライン）- 1m間隔
    let grid_step = 1.0;
    let mut elev = dl;
    while elev <= graph_top {
        let y = to_dxf_y(elev);
        add_line(&mut drawing, label_width, y, label_width + graph_width, y, 8, "GRID");
        // 標高ラベル（左側）- DL行は特別表示、ボトムアライメント
        let is_dl_row = (elev - dl).abs() < 0.01;
        let label_text = if is_dl_row {
            format!("DL={:.0}", elev)
        } else {
            format!("{:.0}", elev)
        };
        let label_color = if is_dl_row { 5 } else { 7 };  // DL行は青
        let label_size = if is_dl_row { text_height * 0.9 } else { text_height * 0.9 };  // DL行は半分サイズ
        add_text_rotated(&mut drawing, label_width - 1200.0, y, &label_text,
            label_size, label_color, "TEXT", HAlign::Right, VAlign::Bottom, 0.0);

        // 標高ラベル（右側）
        add_text_rotated(&mut drawing, label_width + graph_width + 1200.0, y, &format!("{:.0}", elev),
            text_height * 0.9, 7, "TEXT", HAlign::Left, VAlign::Bottom, 0.0);
        elev += grid_step;
    }

    // 縮尺比率と単位の注釈（DL付近、標高ラベルの左側に配置）
    let dl_y = to_dxf_y(dl);
    let annotation_x = 500.0;  // 左端寄り（×10）
    // 縮尺比率: V:H=5:1
    add_text_rotated(&mut drawing, annotation_x, dl_y + scale_y * 0.8,
        "V:H=5:1", text_height * 0.8, 5, "ANNOTATION", HAlign::Left, VAlign::Bottom, 0.0);
    // 単位 (m)
    add_text_rotated(&mut drawing, annotation_x, dl_y + scale_y * 1.3,
        "単位(m)", text_height * 0.8, 7, "TEXT", HAlign::Left, VAlign::Bottom, 0.0);

    // グラフ枠線
    add_line(&mut drawing, label_width, 0.0, label_width + graph_width, 0.0, 7, "TABLE");
    add_line(&mut drawing, label_width, graph_height, label_width + graph_width, graph_height, 7, "TABLE");
    add_line(&mut drawing, label_width, 0.0, label_width, graph_height, 7, "TABLE");
    add_line(&mut drawing, label_width + graph_width, 0.0, label_width + graph_width, graph_height, 7, "TABLE");

    // 現地盤高線（黒）
    for i in 0..points.len() - 1 {
        add_line(&mut drawing,
            to_dxf_x(points[i].0), to_dxf_y(points[i].1),
            to_dxf_x(points[i + 1].0), to_dxf_y(points[i + 1].1),
            7, "GROUND");
    }

    // 計画高線（赤）
    for i in 0..points.len() - 1 {
        add_line(&mut drawing,
            to_dxf_x(points[i].0), to_dxf_y(points[i].2),
            to_dxf_x(points[i + 1].0), to_dxf_y(points[i + 1].2),
            1, "PLAN");
    }

    // ===================
    // データ表の描画
    // ===================

    // 表の横線（行0-6は通常高さ、行7は測点名行の高さ）
    for i in 0..=7 {
        let y = table_top - (i as f64) * row_height;
        add_line(&mut drawing, 0.0, y, label_width + graph_width, y, 7, "TABLE");
    }
    // 最下行（測点名行の下端）
    add_line(&mut drawing, 0.0, table_bottom, label_width + graph_width, table_bottom, 7, "TABLE");

    // 左端・ラベル列の縦線
    add_line(&mut drawing, 0.0, table_top, 0.0, table_bottom, 7, "TABLE");
    add_line(&mut drawing, label_width, table_top, label_width, table_bottom, 7, "TABLE");

    // 行ラベル（行7は測点名行の高さを使用）
    for (label, idx) in &table_rows {
        let y = if *idx < 7 {
            table_top - (*idx as f64) * row_height - row_height / 2.0
        } else {
            normal_rows_bottom - station_row_height / 2.0
        };
        add_text(&mut drawing, label_width / 2.0, y, label, text_height, 7, "TEXT", HAlign::Center);
    }

    // 各測点のデータ
    let mut prev_dist = min_dist;
    let mut cum_dist = 0.0;

    for (i, (dist, gh, fh, name)) in points.iter().enumerate() {
        let x = to_dxf_x(*dist);

        // 単距離・累積距離計算
        let unit_dist = if i == 0 { 0.0 } else { dist - prev_dist };
        cum_dist += unit_dist;

        // 盛土・切土計算
        let fill = if fh > gh { fh - gh } else { 0.0 };
        let cut = if gh > fh { gh - fh } else { 0.0 };

        // 勾配計算（次の点との区間）- 勾配はセル中央X座標に配置
        if i < points.len() - 1 {
            let next = &points[i + 1];
            let d_dist = next.0 - dist;
            let d_elev = next.2 - fh;
            if d_dist.abs() > 0.001 {
                let slope_pct = (d_elev / d_dist) * 100.0;
                let slope_str = format!("{:+.3}%", slope_pct);  // 符号を明示
                // 勾配: -90°回転（他と同じ）
                let slope_y = table_top - 0.0 * row_height - 300.0;
                let slope_mid_x = (x + to_dxf_x(next.0)) / 2.0;
                add_text_rotated(&mut drawing, slope_mid_x, slope_y, &slope_str,
                    text_height * 0.9, 7, "TEXT", HAlign::Left, VAlign::Bottom, -90.0);
            }
        }

        let cell_text_height = text_height * 0.9;

        // 各フィールドの左上を基準に配置
        // -90°回転テキストでは、HAlign::Left = テキストが上から下に伸びる
        // VAlign::Bottom = テキストの下端が挿入点
        let top_margin = 300.0;  // セル上端からのマージン（×10）

        // 盛土: 行1の上端基準
        if fill > 0.001 {
            let y = table_top - 1.0 * row_height - top_margin;
            add_text_rotated(&mut drawing, x, y, &format!("{:.3}", fill),
                cell_text_height, 7, "TEXT", HAlign::Left, VAlign::Bottom, -90.0);
        }

        // 切土: 行2の上端基準
        if cut > 0.001 {
            let y = table_top - 2.0 * row_height - top_margin;
            add_text_rotated(&mut drawing, x, y, &format!("{:.3}", cut),
                cell_text_height, 7, "TEXT", HAlign::Left, VAlign::Bottom, -90.0);
        }

        // FH/GH/追加距離/単距離: 各行の上端基準
        let row_data: [(usize, String); 4] = [
            (3, format!("{:.3}", fh)),      // 計画高(FH)
            (4, format!("{:.3}", gh)),      // 地盤高(GH)
            (5, format!("{:.2}", cum_dist)), // 追加距離
            (6, format!("{:.2}", unit_dist)), // 単距離
        ];
        for (row_idx, text) in row_data {
            let y = table_top - row_idx as f64 * row_height - top_margin;
            add_text_rotated(&mut drawing, x, y, &text,
                cell_text_height, 7, "TEXT", HAlign::Left, VAlign::Bottom, -90.0);
        }

        // 測点名: 測点名行の上端基準
        let station_y = normal_rows_bottom - top_margin;
        add_text_rotated(&mut drawing, x, station_y, name,
            cell_text_height, 7, "TEXT", HAlign::Left, VAlign::Bottom, -90.0);

        prev_dist = *dist;
    }

    // 右端の縦線
    add_line(&mut drawing, label_width + graph_width, table_top, label_width + graph_width, table_bottom, 7, "TABLE");

    drawing
}

pub fn generate_longitudinal_dxf_bytes(sections: &[CrossSectionData]) -> Vec<u8> {
    let drawing = generate_longitudinal_drawing(sections);
    let mut output: Vec<u8> = Vec::new();
    drawing.save(&mut output).expect("Failed to save DXF");
    output
}
