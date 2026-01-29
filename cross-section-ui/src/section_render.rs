//! 横断図の描画コンテンツ構築モジュール
//!
//! 単一の横断図（CrossSectionData）を中間描画表現（DrawingContent）に変換する機能を提供。
//! grid_layout.rsから分離された描画ロジックを含む。
//!
//! # 主要関数
//! - `build_section_content`: 横断図データから描画コンテンツを構築
//! - `draw_section_at_offset`: 後方互換性のためのラッパー（DrawingContentをDXFに追加）

use dxf::Drawing;

use crate::{
    CrossSectionData, round_dl, add_dimension_as_lines_to_content,
    DrawingContent, SectionStyle, HAlign, VAlign,
};

/// 横断図の描画コンテンツを構築（中間表現）
///
/// # 引数
/// - `section`: 横断図データ
/// - `offset_x`: DXF座標系でのX方向オフセット
/// - `offset_y`: DXF座標系でのY方向オフセット
/// - `scale`: 水平方向スケール（通常1000.0 = 1m = 1000 DXF units）
/// - `style`: 描画スタイル設定
/// - `v_scale_ratio`: 縦方向スケール倍率
///
/// # 戻り値
/// 描画コンテンツ（DrawingContent）
pub fn build_section_content(section: &CrossSectionData,
                         offset_x: f64, offset_y: f64, scale: f64,
                         style: &SectionStyle, v_scale_ratio: f64) -> DrawingContent {
    let mut content = DrawingContent::new();
    let data = &section.survey_data;
    let dl = round_dl(section.dl);
    let to_dxf_x = |d: f64| offset_x + d * scale;
    let y_scale = scale * v_scale_ratio;  // 縦方向スケール倍率適用
    let to_dxf_y = |h: f64| offset_y + (h - dl) * y_scale;

    let l_data = &data[0];
    let cl_data = &data[section.cl_index.min(data.len() - 1)];
    let r_data = &data[data.len() - 1];

    let left_dist = (cl_data.cumulative_distance - l_data.cumulative_distance).abs();
    let right_dist = (r_data.cumulative_distance - cl_data.cumulative_distance).abs();
    let left_slope = if left_dist > 0.0 { ((l_data.planned_height - cl_data.planned_height) / left_dist) * 100.0 } else { 0.0 };
    let right_slope = if right_dist > 0.0 { ((r_data.planned_height - cl_data.planned_height) / right_dist) * 100.0 } else { 0.0 };

    for i in 0..data.len() - 1 {
        content.add_line(to_dxf_x(data[i].cumulative_distance), to_dxf_y(data[i].elevation),
            to_dxf_x(data[i + 1].cumulative_distance), to_dxf_y(data[i + 1].elevation), 7, "GROUND");
        content.add_line(to_dxf_x(data[i].cumulative_distance), to_dxf_y(data[i].planned_height),
            to_dxf_x(data[i + 1].cumulative_distance), to_dxf_y(data[i + 1].planned_height), 1, "PLAN");
        content.add_line(to_dxf_x(data[i].cumulative_distance), to_dxf_y(data[i].cutting_bottom),
            to_dxf_x(data[i + 1].cumulative_distance), to_dxf_y(data[i + 1].cutting_bottom), 5, "CUTTING");
    }

    // ========== 測点ポインター（逆三角形 + Vラベル）==========
    let pointer_size = style.pointer_size;
    for (i, pt) in data.iter().enumerate() {
        let x = to_dxf_x(pt.cumulative_distance);
        let y = to_dxf_y(pt.planned_height);  // 計画高を基準
        let top_y = y + pointer_size;
        let half_w = pointer_size * 0.6;
        content.add_line(x - half_w, top_y, x + half_w, top_y, 5, "CUTTING");
        content.add_line(x - half_w, top_y, x, y, 5, "CUTTING");
        content.add_line(x + half_w, top_y, x, y, 5, "CUTTING");
        let label = format!("V{}", i + 1);
        content.add_text(x, top_y + style.pointer_label_gap, &label, style.text_height, 5, "CUTTING", HAlign::Center);
    }

    let cl_ground_y = to_dxf_y(cl_data.elevation);
    let flag_y = cl_ground_y + style.flag_base_offset;
    let l_x = to_dxf_x(l_data.cumulative_distance);
    let cl_x = to_dxf_x(cl_data.cumulative_distance);
    let r_x = to_dxf_x(r_data.cumulative_distance);

    content.add_text(cl_x, flag_y + style.label_title_offset, &section.survey_point_name, style.text_height * style.title_scale, 7, "TEXT", HAlign::Center);
    content.add_text(cl_x, flag_y + style.label_gh_offset, &format!("GH={:.3}", cl_data.elevation), style.text_height, 7, "TEXT", HAlign::Center);
    content.add_text(cl_x, flag_y + style.label_fh_offset, &format!("FH={:.3}", cl_data.planned_height), style.text_height, 1, "PLAN", HAlign::Center);

    let l_ground_y = to_dxf_y(l_data.elevation);
    content.add_text(l_x, l_ground_y + style.label_gh_offset + style.side_label_offset, &format!("GH={:.3}", l_data.elevation), style.text_height, 7, "TEXT", HAlign::Left);
    content.add_text(l_x, l_ground_y + style.label_fh_offset + style.side_label_offset, &format!("FH={:.3}", l_data.planned_height), style.text_height, 1, "PLAN", HAlign::Left);

    let r_ground_y = to_dxf_y(r_data.elevation);
    content.add_text(r_x, r_ground_y + style.label_gh_offset + style.side_label_offset, &format!("GH={:.3}", r_data.elevation), style.text_height, 7, "TEXT", HAlign::Right);
    content.add_text(r_x, r_ground_y + style.label_fh_offset + style.side_label_offset, &format!("FH={:.3}", r_data.planned_height), style.text_height, 1, "PLAN", HAlign::Right);

    let mid_l_x = (l_x + cl_x) / 2.0;
    let mid_r_x = (cl_x + r_x) / 2.0;
    let arrow_len = style.arrow_length;
    let arrow_drop = style.arrow_drop;
    let arrow_head = style.arrow_head;
    let arrow_offset = style.arrow_offset;
    let slope_text_y = flag_y + style.slope_text_offset;

    // 左側勾配
    content.add_text(mid_l_x, slope_text_y, &format!("{:.1}%", left_slope), style.text_height, 7, "TEXT", HAlign::Center);
    if left_slope.abs() > 0.01 {
        let arrow_y = slope_text_y - arrow_offset;
        let arrow_x = mid_l_x - arrow_len / 2.0;
        if left_slope < 0.0 {
            content.add_line(arrow_x + arrow_len, arrow_y + arrow_drop, arrow_x, arrow_y - arrow_drop, 7, "SLOPE");
            content.add_line(arrow_x, arrow_y - arrow_drop, arrow_x + arrow_head, arrow_y - arrow_drop + arrow_head * 0.4, 7, "SLOPE");
        } else {
            content.add_line(arrow_x, arrow_y + arrow_drop, arrow_x + arrow_len, arrow_y - arrow_drop, 7, "SLOPE");
            content.add_line(arrow_x + arrow_len, arrow_y - arrow_drop, arrow_x + arrow_len - arrow_head, arrow_y - arrow_drop + arrow_head * 0.4, 7, "SLOPE");
        }
    }

    // 右側勾配
    content.add_text(mid_r_x, slope_text_y, &format!("{:.1}%", right_slope), style.text_height, 7, "TEXT", HAlign::Center);
    if right_slope.abs() > 0.01 {
        let arrow_y = slope_text_y - arrow_offset;
        let arrow_x = mid_r_x - arrow_len / 2.0;
        if right_slope < 0.0 {
            content.add_line(arrow_x, arrow_y + arrow_drop, arrow_x + arrow_len, arrow_y - arrow_drop, 7, "SLOPE");
            content.add_line(arrow_x + arrow_len, arrow_y - arrow_drop, arrow_x + arrow_len - arrow_head, arrow_y - arrow_drop + arrow_head * 0.4, 7, "SLOPE");
        } else {
            content.add_line(arrow_x + arrow_len, arrow_y + arrow_drop, arrow_x, arrow_y - arrow_drop, 7, "SLOPE");
            content.add_line(arrow_x, arrow_y - arrow_drop, arrow_x + arrow_head, arrow_y - arrow_drop + arrow_head * 0.4, 7, "SLOPE");
        }
    }

    // 寸法線（幅員）- 線とテキストで描画
    let dim_base_y = flag_y;

    // 左幅員の寸法（線とテキストで描画）
    let left_width = (cl_data.cumulative_distance - l_data.cumulative_distance).abs();
    add_dimension_as_lines_to_content(&mut content, l_x, cl_x, dim_base_y,
        &format!("{:.2}", left_width), style.text_height, 7, "DIMENSION", style);

    // 右幅員の寸法（線とテキストで描画）
    let right_width = (r_data.cumulative_distance - cl_data.cumulative_distance).abs();
    add_dimension_as_lines_to_content(&mut content, cl_x, r_x, dim_base_y,
        &format!("{:.2}", right_width), style.text_height, 7, "DIMENSION", style);

    content.add_text_rotated(cl_x, to_dxf_y(dl),
        &format!("DL={:.3}  Scale:H1:V2", dl), style.text_height * style.small_text_scale, 8, "TEXT",
        HAlign::Left, VAlign::Bottom, 0.0);

    // ========== 切削厚表示（DLライン上部） ==========
    let cutting_text_height = style.text_height;  // GH等と同じサイズ
    let cutting_y = to_dxf_y(dl) - style.cutting_label_offset;
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
        content.add_text(x, cutting_y,
            &format!("{:.0}", cutting_thickness_mm), cutting_text_height, 5, "CUTTING", align);
    }
    // ラベル「切削厚」を中央下に表示
    content.add_text(cl_x, cutting_y - cutting_text_height - style.cutting_label_gap,
        "切削厚", cutting_text_height, 5, "CUTTING", HAlign::Center);

    // DLライン（幅員と同じ長さ）
    content.add_line(l_x, to_dxf_y(dl), r_x, to_dxf_y(dl), 8, "DIMENSION");
    let cl_cumulative = cl_data.cumulative_distance;
    content.add_line(to_dxf_x(cl_cumulative), to_dxf_y(dl), to_dxf_x(cl_cumulative), to_dxf_y(dl + 1.0), 8, "DIMENSION");

    content
}

/// 後方互換性のためのラッパー: DrawingContentをDXFに追加
///
/// `build_section_content`で構築した中間表現を直接DXF Drawingに追加する。
///
/// # 引数
/// - `drawing`: 追加先のDXF Drawing
/// - `section`: 横断図データ
/// - `offset_x`: DXF座標系でのX方向オフセット
/// - `offset_y`: DXF座標系でのY方向オフセット
/// - `scale`: 水平方向スケール
/// - `scale_multiplier`: スタイルのスケール倍率
/// - `v_scale_ratio`: 縦方向スケール倍率
pub fn draw_section_at_offset(drawing: &mut Drawing, section: &CrossSectionData,
                          offset_x: f64, offset_y: f64, scale: f64, scale_multiplier: f64, v_scale_ratio: f64) {
    let style = SectionStyle::default().scaled(scale_multiplier);
    let content = build_section_content(section, offset_x, offset_y, scale, &style, v_scale_ratio);
    content.add_to_dxf(drawing);
}
