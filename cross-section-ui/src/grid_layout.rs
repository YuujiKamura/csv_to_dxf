//! 複数横断図のグリッドレイアウト配置
//!
//! A3図枠内に複数の横断図を4列×N行で配置するためのモジュール

use dxf::Drawing;
use dxf::Color;

use crate::{
    CrossSectionData, TitleBlockInfo,
    new_drawing, add_line, round_dl,
    draw_drawing_frame, add_dimension_as_lines_to_content,
    DrawingContent, SectionStyle, HAlign, VAlign,
};

// SectionStyleのデフォルト値から計算された固定マージン（DXF単位）
// 上方向: flag_base_offset + label_title_offset + text_height * title_scale
const TOP_MARGIN_DXF: f64 = 1600.0 + 1300.0 + 300.0 * 1.5;  // = 3350.0
// 下方向: cutting_label_offset + text_height + cutting_label_gap
const BOTTOM_MARGIN_DXF: f64 = 300.0 + 300.0 + 100.0;  // = 700.0
use crate::title_block::available_area;

// ============================================================================
// Multi-Section Grid Layout
// ============================================================================

/// グリッドレイアウトのパラメータと計算ロジック
///
/// セル高さの計算を一元管理し、v_scale_ratioと行間隔の関係を正しく処理する。
/// - セクション本体高さ: v_scale_ratio適用（縦に伸縮）
/// - ラベルマージン（旗揚げ・切削厚）: 固定DXF単位
/// - 行間隔: 固定（v_scale_ratioに依存しない）
#[derive(Debug, Clone, Copy)]
pub struct GridLayoutParams {
    pub scale: f64,           // 基本スケール（通常1000.0 = 1m）
    pub v_scale_ratio: f64,   // 縦方向スケール倍率（1.0〜5.0）
    pub column_gap: f64,      // 列間隔（メートル）
    pub row_gap: f64,         // 行間隔（メートル）
}

impl GridLayoutParams {
    /// デフォルトパラメータ（scale=1000, v_scale=2.0）
    pub fn new(column_gap: f64, row_gap: f64) -> Self {
        Self {
            scale: 1000.0,
            v_scale_ratio: 2.0,
            column_gap,
            row_gap,
        }
    }

    /// v_scale_ratioを指定
    pub fn with_v_scale(mut self, v_scale_ratio: f64) -> Self {
        self.v_scale_ratio = v_scale_ratio;
        self
    }

    /// セクションの本体高さを計算（メートル単位、max_elev - dl）
    pub fn section_body_height(&self, section: &CrossSectionData) -> f64 {
        let data = &section.survey_data;
        if data.len() < 2 { return 0.0; }
        let max_elev = data.iter()
            .map(|d| d.elevation.max(d.planned_height))
            .fold(f64::MIN, f64::max);
        max_elev - section.dl
    }

    /// 複数セクションの最大本体高さ（メートル単位）
    pub fn max_section_body_height(&self, sections: &[CrossSectionData]) -> f64 {
        sections.iter()
            .map(|s| self.section_body_height(s))
            .fold(0.0_f64, f64::max)
    }

    /// セクションの全描画高さをDXF単位で計算
    ///
    /// 構成:
    /// - セクション本体: (max_elev - dl) * scale * v_scale_ratio （スケール適用）
    /// - 上マージン（旗揚げ）: TOP_MARGIN_DXF （固定DXF単位）
    /// - 下マージン（切削厚ラベル）: BOTTOM_MARGIN_DXF （固定DXF単位）
    pub fn section_height_dxf(&self, body_height_m: f64) -> f64 {
        body_height_m * self.scale * self.v_scale_ratio + TOP_MARGIN_DXF + BOTTOM_MARGIN_DXF
    }

    /// セル高さをDXF単位で計算
    ///
    /// - セクション全高さ（本体 + ラベルマージン）
    /// - 行間隔: row_gap * scale（固定）
    pub fn cell_height_dxf(&self, max_body_height_m: f64) -> f64 {
        self.section_height_dxf(max_body_height_m) + self.row_gap * self.scale
    }

    /// 複数セクションからセル高さを直接計算
    pub fn cell_height_for_sections(&self, sections: &[CrossSectionData]) -> f64 {
        let max_height = self.max_section_body_height(sections);
        self.cell_height_dxf(max_height)
    }

    /// Y方向スケール（DXF単位変換用）
    pub fn y_scale(&self) -> f64 {
        self.scale * self.v_scale_ratio
    }

    /// 下マージン（DXF単位）- セクション配置のベースオフセット用
    pub fn bottom_margin_dxf(&self) -> f64 {
        BOTTOM_MARGIN_DXF
    }
}

/// グリッド配置のバウンディングボックス（DXF単位）
#[derive(Debug, Clone)]
pub struct GridBounds {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
    /// 列ごとの高さ（DXF単位）- 使用している列のみ有効
    pub column_heights: Vec<f64>,
}

impl GridBounds {
    pub fn width(&self) -> f64 { self.max_x - self.min_x }
    pub fn height(&self) -> f64 { self.max_y - self.min_y }

    /// 紙上サイズに換算（mm）
    pub fn to_paper_size(&self, plot_scale: f64) -> (f64, f64) {
        (self.width() / plot_scale, self.height() / plot_scale)
    }

    /// 図枠に収まるかチェック（列ごとの有効高さでチェック）
    pub fn fits_in_frame(&self, plot_scale: f64, _columns: usize) -> bool {
        let (w_mm, _) = self.to_paper_size(plot_scale);
        if w_mm > available_area::FRAME_USABLE_WIDTH_MM {
            return false;
        }
        // 列ごとに有効高さをチェック
        for (col, &col_height_dxf) in self.column_heights.iter().enumerate() {
            let col_height_mm = col_height_dxf / plot_scale;
            let max_height = available_area::height_for_column(col);
            if col_height_mm > max_height {
                return false;
            }
        }
        true
    }
}

/// グリッド配置時の全体バウンディングボックスを計算
/// 旗揚げやテキストを含む実際の描画範囲を返す
pub fn calc_grid_bounds(
    sections: &[CrossSectionData],
    columns: usize,
    column_gap: f64,
    row_gap: f64,
) -> Option<GridBounds> {
    calc_grid_bounds_with_v_scale(sections, columns, column_gap, row_gap, 2.0)
}

/// 縦スケール倍率を指定してグリッド配置のバウンディングボックスを計算
pub fn calc_grid_bounds_with_v_scale(
    sections: &[CrossSectionData],
    columns: usize,
    column_gap: f64,
    row_gap: f64,
    v_scale_ratio: f64,
) -> Option<GridBounds> {
    if sections.is_empty() { return None; }

    let scale = 1000.0;
    let y_scale = scale * v_scale_ratio;
    // 図枠計算時は4列固定を使用（引数は互換性のため残す）
    let columns = available_area::COLUMN_COUNT.min(columns.max(1).min(sections.len()));
    let rows_per_column = (sections.len() + columns - 1) / columns;

    // GridLayoutParamsで計算を共通化
    let params = GridLayoutParams::new(column_gap, row_gap).with_v_scale(v_scale_ratio);

    // 列ごとの最大幅を計算
    let mut col_max_left: Vec<f64> = vec![0.0; columns];
    let mut col_max_right: Vec<f64> = vec![0.0; columns];

    for (idx, section) in sections.iter().enumerate() {
        if section.survey_data.len() < 2 { continue; }
        let col = idx / rows_per_column;
        if col >= columns { break; }
        let data = &section.survey_data;
        let min_dist = data.first().unwrap().cumulative_distance;
        let max_dist = data.last().unwrap().cumulative_distance;
        col_max_left[col] = col_max_left[col].max(min_dist.abs());
        col_max_right[col] = col_max_right[col].max(max_dist);
    }

    // セル高さ（GridLayoutParamsで計算）
    let cell_height = params.cell_height_for_sections(sections);

    // 列ごとのCL位置を計算
    let mut col_x_offsets: Vec<f64> = Vec::with_capacity(columns);
    let mut cumulative_x = 0.0;

    for col in 0..columns {
        let cell_width = (col_max_left[col] + col_max_right[col] + column_gap) * scale;
        let cl_x = cumulative_x + (col_max_left[col] + column_gap / 2.0) * scale;
        col_x_offsets.push(cl_x);
        cumulative_x += cell_width;
    }

    // 各セクションのバウンディングボックスを計算して全体と列ごとを求める
    let mut global_min_x = f64::MAX;
    let mut global_min_y = f64::MAX;
    let mut global_max_x = f64::MIN;
    let mut global_max_y = f64::MIN;

    // 列ごとのmin/max Y
    let mut col_min_y: Vec<f64> = vec![f64::MAX; columns];
    let mut col_max_y: Vec<f64> = vec![f64::MIN; columns];

    for (idx, section) in sections.iter().enumerate() {
        if section.survey_data.len() < 2 { continue; }
        let col = idx / rows_per_column;
        if col >= columns { break; }
        let row_in_col = idx % rows_per_column;

        let data = &section.survey_data;
        let dl = round_dl(section.dl);

        // セクションの基準位置（下マージン分上にシフト）
        let offset_x = col_x_offsets[col];
        let offset_y = row_in_col as f64 * cell_height + BOTTOM_MARGIN_DXF;

        // X方向の範囲（左端〜右端）
        let l_dist = data.first().unwrap().cumulative_distance;
        let r_dist = data.last().unwrap().cumulative_distance;
        let local_min_x = offset_x + l_dist * scale;
        let local_max_x = offset_x + r_dist * scale;

        // Y方向の範囲
        // 最低位置：cutting_bottomまたは切削厚ラベルの下端のいずれか低い方
        let min_cutting = data.iter().map(|d| d.cutting_bottom).fold(f64::MAX, f64::min);
        let cutting_bottom_y = offset_y + (min_cutting - dl) * y_scale;
        // 切削厚ラベルの下端（DL線から300+300+100=700mm下、さらにテキスト高さ300mm）
        let cutting_label_bottom_y = offset_y - 1000.0;  // DL基準なのでoffset_yから直接引く
        let local_min_y = cutting_bottom_y.min(cutting_label_bottom_y);

        // 最高位置：旗揚げの最上部
        // flag_y = cl_ground_y + 1600.0、その上にテキスト（text_height * 1.5 + 1300.0）
        let cl_idx = section.cl_index.min(data.len() - 1);
        let cl_elev = data[cl_idx].elevation;
        let cl_ground_y = offset_y + (cl_elev - dl) * y_scale;
        let flag_y = cl_ground_y + 1600.0;
        let text_height = 300.0;
        let local_max_y = flag_y + 1300.0 + text_height * 1.5 + 200.0;  // 余裕

        global_min_x = global_min_x.min(local_min_x);
        global_min_y = global_min_y.min(local_min_y);
        global_max_x = global_max_x.max(local_max_x);
        global_max_y = global_max_y.max(local_max_y);

        // 列ごとのY範囲を更新
        col_min_y[col] = col_min_y[col].min(local_min_y);
        col_max_y[col] = col_max_y[col].max(local_max_y);
    }

    if global_min_x == f64::MAX {
        return None;
    }

    // 列ごとの高さを計算
    let column_heights: Vec<f64> = (0..columns)
        .map(|col| {
            if col_min_y[col] == f64::MAX {
                0.0
            } else {
                col_max_y[col] - col_min_y[col]
            }
        })
        .collect();

    Some(GridBounds {
        min_x: global_min_x,
        min_y: global_min_y,
        max_x: global_max_x,
        max_y: global_max_y,
        column_heights,
    })
}

/// A3図枠に収まる最大列数を計算
pub fn calc_max_columns_for_frame(
    sections: &[CrossSectionData],
    column_gap: f64,
    row_gap: f64,
    plot_scale: f64,
) -> usize {
    if sections.is_empty() { return 1; }

    // 1列から順にチェックして収まる最大列数を探す
    let max_possible = sections.len();
    let mut max_fitting = 1;

    for cols in 1..=max_possible {
        if let Some(bounds) = calc_grid_bounds(sections, cols, column_gap, row_gap) {
            if bounds.fits_in_frame(plot_scale, cols) {
                max_fitting = cols;
            } else {
                // 列数が増えると幅は減るが高さは増える
                // 収まらなくなったらその前の列数が最大
                break;
            }
        }
    }

    max_fitting
}

/// A3図枠(1:500)に収まる最適列数を計算（後方互換性のため維持）
pub fn calc_optimal_columns_for_frame(
    sections: &[CrossSectionData],
    column_gap: f64,
    row_gap: f64,
    plot_scale: f64,
) -> usize {
    calc_max_columns_for_frame(sections, column_gap, row_gap, plot_scale)
}

/// 1ページに収まる最大セクション数を計算
/// 指定された列数とスケールで、A3図枠に収まる最大のセクション数を返す
pub fn calc_sections_per_page(
    sections: &[CrossSectionData],
    _columns: usize,  // 無視（4列固定のため）
    column_gap: f64,
    row_gap: f64,
    plot_scale: f64,
) -> usize {
    if sections.is_empty() { return 0; }

    // 図枠付きは4列固定
    let columns = available_area::COLUMN_COUNT;

    // 1個から順にチェックして収まる最大数を探す
    let mut max_fitting = 1;

    for n in 1..=sections.len() {
        let test_sections = &sections[0..n];
        if let Some(bounds) = calc_grid_bounds(test_sections, columns, column_gap, row_gap) {
            if bounds.fits_in_frame(plot_scale, columns) {
                max_fitting = n;
            } else {
                break;
            }
        }
    }

    max_fitting
}

/// 複数横断図をグリッド配置したDrawingを生成
/// 道路工事の配置ルール: 左下起点、列ごとに下から上へ、左から右へ
pub fn generate_multi_drawing(sections: &[CrossSectionData], columns: usize, column_gap: f64, row_gap: f64) -> Drawing {
    generate_multi_drawing_internal(sections, columns, column_gap, row_gap, None, None, 1.0, false, 2.0)
}

/// スケール倍率指定で複数横断図をグリッド配置したDrawingを生成（コンボモード用）
pub fn generate_multi_drawing_scaled(sections: &[CrossSectionData], columns: usize, column_gap: f64, row_gap: f64, scale_multiplier: f64) -> Drawing {
    generate_multi_drawing_internal(sections, columns, column_gap, row_gap, None, None, scale_multiplier, false, 2.0)
}

/// スケール倍率指定で複数横断図をグリッド配置（補完点を勾配補間）
pub fn generate_multi_drawing_scaled_interpolated(sections: &[CrossSectionData], columns: usize, column_gap: f64, row_gap: f64, scale_multiplier: f64) -> Drawing {
    generate_multi_drawing_internal(sections, columns, column_gap, row_gap, None, None, scale_multiplier, true, 2.0)
}

/// 1:500図枠付きで複数横断図をグリッド配置したDrawingを生成
pub fn generate_multi_drawing_with_frame(
    sections: &[CrossSectionData],
    columns: usize,
    column_gap: f64,
    row_gap: f64,
    title_info: &TitleBlockInfo,
) -> Drawing {
    generate_multi_drawing_internal(sections, columns, column_gap, row_gap, Some(500.0), Some(title_info), 1.0, false, 2.0)
}

/// 任意スケールの図枠付きで複数横断図をグリッド配置したDrawingを生成
pub fn generate_multi_drawing_with_frame_at_scale(
    sections: &[CrossSectionData],
    columns: usize,
    column_gap: f64,
    row_gap: f64,
    title_info: &TitleBlockInfo,
    frame_scale: f64,
) -> Drawing {
    generate_multi_drawing_internal(sections, columns, column_gap, row_gap, Some(frame_scale), Some(title_info), 1.0, false, 2.0)
}

/// 複数横断図をグリッド配置したDrawingを生成（補完点を勾配補間）
/// 中間測点の補完点（V2, V4等）を採用点間の勾配で線形補間する
pub fn generate_multi_drawing_interpolated(sections: &[CrossSectionData], columns: usize, column_gap: f64, row_gap: f64, v_scale_ratio: f64) -> Drawing {
    generate_multi_drawing_internal(sections, columns, column_gap, row_gap, None, None, 1.0, true, v_scale_ratio)
}

/// 任意スケールの図枠付きで複数横断図をグリッド配置（補完点を勾配補間）
pub fn generate_multi_drawing_with_frame_at_scale_interpolated(
    sections: &[CrossSectionData],
    columns: usize,
    column_gap: f64,
    row_gap: f64,
    title_info: &TitleBlockInfo,
    frame_scale: f64,
    v_scale_ratio: f64,
) -> Drawing {
    generate_multi_drawing_internal(sections, columns, column_gap, row_gap, Some(frame_scale), Some(title_info), 1.0, true, v_scale_ratio)
}

/// 複数横断図をグリッド配置（内部実装）
/// frame_scale: None=図枠なし、Some(200.0)=1:200、Some(500.0)=1:500
/// scale_multiplier: 横断図全体のスケール倍率（コンボモード用）
/// interpolate_intermediate: 中間測点の補完点を勾配補間するか
/// v_scale_ratio: 縦方向スケール倍率（1.0〜3.0、デフォルト2.0）
fn generate_multi_drawing_internal(
    sections: &[CrossSectionData],
    columns: usize,
    column_gap: f64,
    row_gap: f64,
    frame_scale: Option<f64>,
    title_info: Option<&TitleBlockInfo>,
    scale_multiplier: f64,
    interpolate_intermediate: bool,
    v_scale_ratio: f64,
) -> Drawing {
    let scale = 1000.0 * scale_multiplier;

    let mut drawing = new_drawing();

    for (name, color_idx) in [
        ("GROUND", 7), ("PLAN", 1), ("TEXT", 7),
        ("DIMENSION", 8), ("CUTTING", 5), ("FRAME", 9)
    ] {
        drawing.add_layer(dxf::tables::Layer {
            name: name.to_string(),
            color: Color::from_index(color_idx),
            ..Default::default()
        });
    }

    if sections.is_empty() { return drawing; }

    // 図枠付きの場合は4列固定、図枠なしの場合は指定列数を使用
    let columns = if frame_scale.is_some() {
        available_area::COLUMN_COUNT  // 4列固定
    } else {
        columns
    };

    // GridLayoutParamsで計算を共通化
    let params = GridLayoutParams::new(column_gap, row_gap).with_v_scale(v_scale_ratio);

    // 道路工事の配置: 左下起点、列ごとに下から上
    let rows_per_column = (sections.len() + columns - 1) / columns;  // ceil division

    // 列ごとの最大幅を計算
    let mut col_max_left: Vec<f64> = vec![0.0; columns];
    let mut col_max_right: Vec<f64> = vec![0.0; columns];

    for (idx, section) in sections.iter().enumerate() {
        if section.survey_data.len() < 2 { continue; }
        let col = idx / rows_per_column;
        let data = &section.survey_data;
        let min_dist = data.first().unwrap().cumulative_distance;
        let max_dist = data.last().unwrap().cumulative_distance;
        col_max_left[col] = col_max_left[col].max(min_dist.abs());
        col_max_right[col] = col_max_right[col].max(max_dist);
    }

    // セル高さ（GridLayoutParamsで計算）
    let cell_height = params.cell_height_for_sections(sections);

    // 図枠付きの場合: 4列均等分割の中心線を使用
    // 図枠なしの場合: 従来の動的計算
    let (col_x_offsets, col_y_offsets, grid_bounds) = if let Some(fs) = frame_scale {
        // 列ごとのCL位置は固定（列の中心 * スケール）
        let col_centers: Vec<f64> = (0..columns)
            .map(|col| {
                if col < 4 {
                    available_area::COLUMN_CENTERS[col] * fs
                } else {
                    // 4列以上の場合は最後の列の中心を使用
                    available_area::COLUMN_CENTERS[3] * fs
                }
            })
            .collect();

        // グリッド全体の実際のバウンディングボックスを取得（Y配置とデバッグ用）
        let bounds = calc_grid_bounds(sections, columns, column_gap, row_gap);
        let content_height_mm = bounds.as_ref().map(|b| b.height() / fs).unwrap_or(0.0);

        // 列ごとのY方向オフセットを計算
        let mut col_offsets: Vec<f64> = Vec::with_capacity(columns);
        for col in 0..columns {
            // 列ごとの有効高さと下端位置
            let col_height = available_area::height_for_column(col);
            let col_bottom = available_area::bottom_for_column(col);

            // Y: 列ごとの利用可能領域でセンタリング
            let target_bottom = col_bottom + (col_height - content_height_mm) / 2.0;
            let offset_y = if let Some(ref b) = bounds {
                target_bottom * fs - b.min_y
            } else {
                0.0
            };
            col_offsets.push(offset_y);
        }

        (col_centers, col_offsets, bounds)
    } else {
        // 図枠なしの場合: 従来の動的計算（列ごとのセル幅と累積X位置を計算）
        let mut col_x_offsets: Vec<f64> = Vec::with_capacity(columns);
        let mut cumulative_x = 0.0;

        for col in 0..columns {
            let cell_width = (col_max_left[col] + col_max_right[col] + column_gap) * scale;
            // この列のCL位置: 累積X + 列内の左マージン
            let cl_x = cumulative_x + (col_max_left[col] + column_gap / 2.0) * scale;
            col_x_offsets.push(cl_x);
            cumulative_x += cell_width;
        }

        (col_x_offsets, vec![0.0; columns], None)
    };

    // セクションを描画
    for (idx, section) in sections.iter().enumerate() {
        if section.survey_data.len() < 2 { continue; }
        let col = idx / rows_per_column;
        let row_in_col = idx % rows_per_column;

        // X: 列の中心線を使用（図枠あり）または動的計算（図枠なし）
        let offset_x = if col < col_x_offsets.len() {
            col_x_offsets[col]
        } else {
            col_x_offsets.last().copied().unwrap_or(0.0)
        };

        // 列ごとのY方向オフセットを適用（下マージン分上にシフト）
        let col_y_offset = if col < col_y_offsets.len() {
            col_y_offsets[col]
        } else {
            col_y_offsets.last().copied().unwrap_or(0.0)
        };
        let offset_y = row_in_col as f64 * cell_height + col_y_offset + BOTTOM_MARGIN_DXF;

        // 中間測点（起終点以外）の場合のみ、補完点を勾配補間
        let is_start_or_end = section.is_route_start || section.is_route_end;
        if interpolate_intermediate && !is_start_or_end {
            let mut section_to_draw = section.clone();
            section_to_draw.interpolate_planned_heights();
            draw_section_at_offset(&mut drawing, &section_to_draw, offset_x, offset_y, scale, scale_multiplier, v_scale_ratio);
        } else {
            draw_section_at_offset(&mut drawing, section, offset_x, offset_y, scale, scale_multiplier, v_scale_ratio);
        }
    }

    // 図枠を描画
    if let Some(fs) = frame_scale {
        const TEXT_SIZE_MM: f64 = 3.0;

        let info = title_info.cloned().unwrap_or_else(|| {
            TitleBlockInfo::new()
                .with_top_title("横断図")
                .with_scale(&format!("1:{} (A3)", fs as u32))
        });

        draw_drawing_frame(&mut drawing, &info, 0.0, 0.0, fs, TEXT_SIZE_MM);

        // グリッド外形を描画（デバッグ用）
        // 各列の中心線から推測したグリッド範囲を描画
        if info.show_debug_markers {
            if let Some(bounds) = &grid_bounds {
                const COLOR_RED: i16 = 1;

                // 列の左右端から計算（最初の列の左端〜最後の列の右端）
                let first_col = 0;
                let last_col = (columns - 1).min(3);
                let grid_min_x = available_area::COLUMN_LEFTS[first_col] * fs;
                let grid_max_x = available_area::COLUMN_RIGHTS[last_col] * fs;

                // 最小のY方向オフセット（左列が最も低い位置）を使用
                let min_y_offset = col_y_offsets.iter().copied().fold(f64::MAX, f64::min);
                let max_y_offset = col_y_offsets.iter().copied().fold(f64::MIN, f64::max);

                let grid_min_y = bounds.min_y + min_y_offset;
                let grid_max_y = bounds.max_y + max_y_offset;

                add_line(&mut drawing, grid_min_x, grid_min_y, grid_max_x, grid_min_y, COLOR_RED, "DEBUG");
                add_line(&mut drawing, grid_max_x, grid_min_y, grid_max_x, grid_max_y, COLOR_RED, "DEBUG");
                add_line(&mut drawing, grid_max_x, grid_max_y, grid_min_x, grid_max_y, COLOR_RED, "DEBUG");
                add_line(&mut drawing, grid_min_x, grid_max_y, grid_min_x, grid_min_y, COLOR_RED, "DEBUG");
            }
        }
    }

    drawing
}

/// 全ページを1つのDXFに垂直配置して生成
/// 各ページはA3図枠付きで、下から上に向かって配置される
pub fn generate_all_pages_drawing(
    all_sections: &[CrossSectionData],
    columns: usize,
    column_gap: f64,
    row_gap: f64,
    frame_scale: f64,
    base_title_info: &TitleBlockInfo,
    drawing_number_base: &str,
    v_scale_ratio: f64,
) -> Drawing {
    generate_all_pages_drawing_internal(
        all_sections, columns, column_gap, row_gap, frame_scale, base_title_info, drawing_number_base, false, v_scale_ratio
    )
}

/// 全ページを1つのDXFに垂直配置して生成（補完点を勾配補間）
pub fn generate_all_pages_drawing_interpolated(
    all_sections: &[CrossSectionData],
    columns: usize,
    column_gap: f64,
    row_gap: f64,
    frame_scale: f64,
    base_title_info: &TitleBlockInfo,
    drawing_number_base: &str,
    v_scale_ratio: f64,
) -> Drawing {
    generate_all_pages_drawing_internal(
        all_sections, columns, column_gap, row_gap, frame_scale, base_title_info, drawing_number_base, true, v_scale_ratio
    )
}

/// 全ページを1つのDXFに垂直配置して生成（内部実装）
fn generate_all_pages_drawing_internal(
    all_sections: &[CrossSectionData],
    columns: usize,
    column_gap: f64,
    row_gap: f64,
    frame_scale: f64,
    base_title_info: &TitleBlockInfo,
    drawing_number_base: &str,
    interpolate_intermediate: bool,
    v_scale_ratio: f64,
) -> Drawing {
    let mut drawing = new_drawing();

    for (name, color_idx) in [
        ("GROUND", 7), ("PLAN", 1), ("TEXT", 7),
        ("DIMENSION", 8), ("CUTTING", 5), ("FRAME", 9), ("TITLEBLOCK", 7), ("DEBUG", 1)
    ] {
        drawing.add_layer(dxf::tables::Layer {
            name: name.to_string(),
            color: Color::from_index(color_idx),
            ..Default::default()
        });
    }

    if all_sections.is_empty() { return drawing; }

    // ページあたりのセクション数を計算
    let sections_per_page = calc_sections_per_page(all_sections, columns, column_gap, row_gap, frame_scale);
    if sections_per_page == 0 { return drawing; }

    let total_pages = (all_sections.len() + sections_per_page - 1) / sections_per_page;

    // A3の高さ（mm）をDXF単位に変換
    const A3_HEIGHT_MM: f64 = 297.0;
    let page_height_dxf = A3_HEIGHT_MM * frame_scale;
    // ページ間のギャップ（10mm）
    const PAGE_GAP_MM: f64 = 10.0;
    let page_gap_dxf = PAGE_GAP_MM * frame_scale;

    // 各ページを描画
    for page in 0..total_pages {
        let start_idx = page * sections_per_page;
        let end_idx = (start_idx + sections_per_page).min(all_sections.len());
        let page_sections = &all_sections[start_idx..end_idx];

        // ページのY方向オフセット（下から上に配置）
        let page_offset_y = page as f64 * (page_height_dxf + page_gap_dxf);

        // ページ番号付きの図番
        let page_num = if total_pages > 1 {
            format!("{}-{}", drawing_number_base, page + 1)
        } else {
            drawing_number_base.to_string()
        };

        let info = base_title_info.clone()
            .with_drawing_number(&page_num);

        // このページのセクションを描画
        draw_page_content(
            &mut drawing, page_sections, columns, column_gap, row_gap, frame_scale, &info, page_offset_y,
            interpolate_intermediate, v_scale_ratio
        );
    }

    drawing
}

/// 1ページ分のコンテンツを描画（オフセット付き）
/// interpolate_intermediate: 中間測点の補完点を勾配補間するか（起終点はフラグで判定）
fn draw_page_content(
    drawing: &mut Drawing,
    sections: &[CrossSectionData],
    _columns: usize,
    column_gap: f64,
    row_gap: f64,
    frame_scale: f64,
    title_info: &TitleBlockInfo,
    page_offset_y: f64,
    interpolate_intermediate: bool,
    v_scale_ratio: f64,
) {
    let scale = 1000.0;
    let columns = available_area::COLUMN_COUNT;  // 4列固定

    if sections.is_empty() { return; }

    // GridLayoutParamsで計算を共通化
    let params = GridLayoutParams::new(column_gap, row_gap).with_v_scale(v_scale_ratio);

    let rows_per_column = (sections.len() + columns - 1) / columns;

    // セル高さ（GridLayoutParamsで計算）
    let cell_height = params.cell_height_for_sections(sections);

    // 列ごとのCL位置（固定）
    let col_centers: Vec<f64> = (0..columns)
        .map(|col| available_area::COLUMN_CENTERS[col] * frame_scale)
        .collect();

    // グリッドバウンドを計算してY方向オフセットを決定
    let bounds = calc_grid_bounds(sections, columns, column_gap, row_gap);
    let content_height_mm = bounds.as_ref().map(|b| b.height() / frame_scale).unwrap_or(0.0);

    let mut col_y_offsets: Vec<f64> = Vec::with_capacity(columns);
    for col in 0..columns {
        let col_height = available_area::height_for_column(col);
        let col_bottom = available_area::bottom_for_column(col);
        let target_bottom = col_bottom + (col_height - content_height_mm) / 2.0;
        let offset_y = if let Some(ref b) = bounds {
            target_bottom * frame_scale - b.min_y
        } else {
            0.0
        };
        col_y_offsets.push(offset_y);
    }

    // セクションを描画
    for (idx, section) in sections.iter().enumerate() {
        if section.survey_data.len() < 2 { continue; }
        let col = idx / rows_per_column;
        let row_in_col = idx % rows_per_column;

        let offset_x = col_centers[col.min(columns - 1)];
        let col_y_offset = col_y_offsets[col.min(columns - 1)];
        // 下マージン分上にシフト
        let offset_y = row_in_col as f64 * cell_height + col_y_offset + page_offset_y + BOTTOM_MARGIN_DXF;

        // 中間測点（起終点以外）の場合のみ、補完点を勾配補間
        let is_start_or_end = section.is_route_start || section.is_route_end;
        if interpolate_intermediate && !is_start_or_end {
            let mut section_to_draw = section.clone();
            section_to_draw.interpolate_planned_heights();
            draw_section_at_offset(drawing, &section_to_draw, offset_x, offset_y, scale, 1.0, v_scale_ratio);
        } else {
            draw_section_at_offset(drawing, section, offset_x, offset_y, scale, 1.0, v_scale_ratio);
        }
    }

    // 図枠を描画
    const TEXT_SIZE_MM: f64 = 3.0;
    draw_drawing_frame(drawing, title_info, 0.0, page_offset_y, frame_scale, TEXT_SIZE_MM);
}

/// 全ページを1つのDXFに垂直配置してバイト配列で返す
pub fn generate_all_pages_dxf_bytes(
    all_sections: &[CrossSectionData],
    columns: usize,
    column_gap: f64,
    row_gap: f64,
    frame_scale: f64,
    base_title_info: &TitleBlockInfo,
    drawing_number_base: &str,
    v_scale_ratio: f64,
) -> Vec<u8> {
    let drawing = generate_all_pages_drawing_interpolated(
        all_sections, columns, column_gap, row_gap, frame_scale, base_title_info, drawing_number_base, v_scale_ratio
    );
    let mut output: Vec<u8> = Vec::new();
    drawing.save(&mut output).expect("Failed to save DXF");
    output
}

/// グリッド配置内の特定セクションのバウンディングボックスを計算
/// generate_multi_drawingと同じ配置ロジックを使用
pub fn calc_section_bounds_in_grid(
    sections: &[CrossSectionData], target_idx: usize, columns: usize, column_gap: f64, row_gap: f64
) -> Option<(f32, f32, f32, f32)> {
    calc_section_bounds_in_grid_with_v_scale(sections, target_idx, columns, column_gap, row_gap, 2.0)
}

/// 縦スケール倍率を指定してグリッド配置内の特定セクションのバウンディングボックスを計算
pub fn calc_section_bounds_in_grid_with_v_scale(
    sections: &[CrossSectionData], target_idx: usize, columns: usize, column_gap: f64, row_gap: f64, v_scale_ratio: f64
) -> Option<(f32, f32, f32, f32)> {
    if sections.is_empty() || target_idx >= sections.len() { return None; }

    let params = GridLayoutParams::new(column_gap, row_gap).with_v_scale(v_scale_ratio);
    let scale = params.scale;
    let rows_per_column = (sections.len() + columns - 1) / columns;

    // 列ごとの最大幅と全体の最大高さを計算（generate_multi_drawingと同じ）
    let mut col_max_left: Vec<f64> = vec![0.0; columns];
    let mut col_max_right: Vec<f64> = vec![0.0; columns];
    let mut max_height: f64 = 0.0;

    for (idx, section) in sections.iter().enumerate() {
        if section.survey_data.len() < 2 { continue; }
        let col = idx / rows_per_column;
        let data = &section.survey_data;
        let min_dist = data.first().unwrap().cumulative_distance;
        let max_dist = data.last().unwrap().cumulative_distance;
        col_max_left[col] = col_max_left[col].max(min_dist.abs());
        col_max_right[col] = col_max_right[col].max(max_dist);
        max_height = max_height.max(params.section_body_height(section));
    }

    // 列ごとのCL位置を計算
    let mut col_x_offsets: Vec<f64> = Vec::with_capacity(columns);
    let mut cumulative_x = 0.0;

    for col in 0..columns {
        let cell_width = (col_max_left[col] + col_max_right[col] + column_gap) * scale;
        let cl_x = cumulative_x + (col_max_left[col] + column_gap / 2.0) * scale;
        col_x_offsets.push(cl_x);
        cumulative_x += cell_width;
    }

    let cell_height = params.cell_height_dxf(max_height);

    // ターゲットセクションの位置を計算
    let target_section = &sections[target_idx];
    if target_section.survey_data.len() < 2 { return None; }

    let col = target_idx / rows_per_column;
    let row_in_col = target_idx % rows_per_column;
    let offset_x = col_x_offsets[col];
    // 下マージン分上にシフト
    let offset_y = row_in_col as f64 * cell_height + BOTTOM_MARGIN_DXF;

    // セクションのバウンディングボックスを計算
    let data = &target_section.survey_data;
    let dl = round_dl(target_section.dl);
    let y_scale = params.y_scale();

    let min_dist = data.first().unwrap().cumulative_distance;
    let max_dist = data.last().unwrap().cumulative_distance;
    let _min_elev = data.iter().map(|d| d.elevation.min(d.planned_height)).fold(f64::MAX, f64::min);
    let max_elev = data.iter().map(|d| d.elevation.max(d.planned_height)).fold(f64::MIN, f64::max);

    let min_x = (offset_x + min_dist * scale) as f32;
    let max_x = (offset_x + max_dist * scale) as f32;
    let min_y = (offset_y + (dl - dl) * y_scale) as f32;  // DL位置
    let max_y = (offset_y + (max_elev - dl + 2.0) * y_scale) as f32;  // 旗揚げ分のマージン

    Some((min_x, min_y, max_x, max_y))
}

/// 横断図の描画コンテンツを構築（中間表現）
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
pub fn draw_section_at_offset(drawing: &mut Drawing, section: &CrossSectionData,
                          offset_x: f64, offset_y: f64, scale: f64, scale_multiplier: f64, v_scale_ratio: f64) {
    let style = SectionStyle::default().scaled(scale_multiplier);
    let content = build_section_content(section, offset_x, offset_y, scale, &style, v_scale_ratio);
    content.add_to_dxf(drawing);
}

pub fn generate_multi_dxf_bytes(sections: &[CrossSectionData], columns: usize, column_gap: f64, row_gap: f64) -> Vec<u8> {
    let drawing = generate_multi_drawing(sections, columns, column_gap, row_gap);
    let mut output: Vec<u8> = Vec::new();
    drawing.save(&mut output).expect("Failed to save DXF");
    output
}

/// 1:500図枠付きDXFバイト列を生成
pub fn generate_multi_dxf_bytes_with_frame(
    sections: &[CrossSectionData],
    columns: usize,
    column_gap: f64,
    row_gap: f64,
    title_info: &TitleBlockInfo,
) -> Vec<u8> {
    let drawing = generate_multi_drawing_with_frame(sections, columns, column_gap, row_gap, title_info);
    let mut output: Vec<u8> = Vec::new();
    drawing.save(&mut output).expect("Failed to save DXF");
    output
}

/// 任意スケールの図枠付きDXFバイト列を生成
pub fn generate_multi_dxf_bytes_with_frame_at_scale(
    sections: &[CrossSectionData],
    columns: usize,
    column_gap: f64,
    row_gap: f64,
    title_info: &TitleBlockInfo,
    frame_scale: f64,
    v_scale_ratio: f64,
) -> Vec<u8> {
    let drawing = generate_multi_drawing_with_frame_at_scale_interpolated(sections, columns, column_gap, row_gap, title_info, frame_scale, v_scale_ratio);
    let mut output: Vec<u8> = Vec::new();
    drawing.save(&mut output).expect("Failed to save DXF");
    output
}
// add_dimension_as_lines_to_content is now provided by dxf_helpers.rs
