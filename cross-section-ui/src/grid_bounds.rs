//! グリッド配置のバウンディングボックス計算
//!
//! GridBounds構造体とcalc_*関数群を提供

use crate::{
    grid_params::{GridLayoutParams, DEFAULT_BOTTOM_MARGIN_DXF},
    round_dl, title_block::available_area, CrossSectionData,
};

// ============================================================================
// GridBounds
// ============================================================================

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
    pub fn width(&self) -> f64 {
        self.max_x - self.min_x
    }
    pub fn height(&self) -> f64 {
        self.max_y - self.min_y
    }

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

// ============================================================================
// Helper functions (extracted from duplicated code)
// ============================================================================

/// 列ごとの最大左右幅を計算
///
/// Returns: (col_max_left, col_max_right) - 各列のCLから左右への最大距離
pub fn calc_column_widths(
    sections: &[CrossSectionData],
    columns: usize,
    rows_per_column: usize,
) -> (Vec<f64>, Vec<f64>) {
    let mut col_max_left: Vec<f64> = vec![0.0; columns];
    let mut col_max_right: Vec<f64> = vec![0.0; columns];

    for (idx, section) in sections.iter().enumerate() {
        if section.survey_data.len() < 2 {
            continue;
        }
        let col = idx / rows_per_column;
        if col >= columns {
            break;
        }
        let data = &section.survey_data;
        let min_dist = data.first().unwrap().cumulative_distance;
        let max_dist = data.last().unwrap().cumulative_distance;
        col_max_left[col] = col_max_left[col].max(min_dist.abs());
        col_max_right[col] = col_max_right[col].max(max_dist);
    }

    (col_max_left, col_max_right)
}

/// 列ごとのCL X位置を計算（図枠なしの場合）
///
/// Returns: 各列のCL X座標（DXF単位）
pub fn calc_column_x_offsets(
    col_max_left: &[f64],
    col_max_right: &[f64],
    column_gap: f64,
    scale: f64,
) -> Vec<f64> {
    let columns = col_max_left.len();
    let mut col_x_offsets: Vec<f64> = Vec::with_capacity(columns);
    let mut cumulative_x = 0.0;

    for col in 0..columns {
        let cell_width = (col_max_left[col] + col_max_right[col] + column_gap) * scale;
        let cl_x = cumulative_x + (col_max_left[col] + column_gap / 2.0) * scale;
        col_x_offsets.push(cl_x);
        cumulative_x += cell_width;
    }

    col_x_offsets
}

// ============================================================================
// calc_grid_bounds functions
// ============================================================================

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
    if sections.is_empty() {
        return None;
    }

    let scale = 1000.0;
    let y_scale = scale * v_scale_ratio;
    // 図枠計算時は4列固定を使用（引数は互換性のため残す）
    let columns = available_area::COLUMN_COUNT.min(columns.max(1).min(sections.len()));
    let rows_per_column = (sections.len() + columns - 1) / columns;

    // GridLayoutParamsで計算を共通化
    let params = GridLayoutParams::new(column_gap, row_gap).with_v_scale(v_scale_ratio);

    // 列ごとの最大幅を計算（共通ヘルパー使用）
    let (col_max_left, col_max_right) = calc_column_widths(sections, columns, rows_per_column);

    // セル高さ（GridLayoutParamsで計算）
    let cell_height = params.cell_height_for_sections(sections);

    // 列ごとのCL位置を計算（共通ヘルパー使用）
    let col_x_offsets = calc_column_x_offsets(&col_max_left, &col_max_right, column_gap, scale);

    // 各セクションのバウンディングボックスを計算して全体と列ごとを求める
    let mut global_min_x = f64::MAX;
    let mut global_min_y = f64::MAX;
    let mut global_max_x = f64::MIN;
    let mut global_max_y = f64::MIN;

    // 列ごとのmin/max Y
    let mut col_min_y: Vec<f64> = vec![f64::MAX; columns];
    let mut col_max_y: Vec<f64> = vec![f64::MIN; columns];

    for (idx, section) in sections.iter().enumerate() {
        if section.survey_data.len() < 2 {
            continue;
        }
        let col = idx / rows_per_column;
        if col >= columns {
            break;
        }
        let row_in_col = idx % rows_per_column;

        let data = &section.survey_data;
        let dl = round_dl(section.dl);

        // セクションの基準位置（下マージン分上にシフト）
        let offset_x = col_x_offsets[col];
        let offset_y = row_in_col as f64 * cell_height + DEFAULT_BOTTOM_MARGIN_DXF;

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
        let cutting_label_bottom_y = offset_y - 1000.0; // DL基準なのでoffset_yから直接引く
        let local_min_y = cutting_bottom_y.min(cutting_label_bottom_y);

        // 最高位置：旗揚げの最上部
        // flag_y = cl_ground_y + 1600.0、その上にテキスト（text_height * 1.5 + 1300.0）
        let cl_idx = section.cl_index.min(data.len() - 1);
        let cl_elev = data[cl_idx].elevation;
        let cl_ground_y = offset_y + (cl_elev - dl) * y_scale;
        let flag_y = cl_ground_y + 1600.0;
        let text_height = 300.0;
        let local_max_y = flag_y + 1300.0 + text_height * 1.5 + 200.0; // 余裕

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
    if sections.is_empty() {
        return 1;
    }

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
    _columns: usize, // 無視（4列固定のため）
    column_gap: f64,
    row_gap: f64,
    plot_scale: f64,
) -> usize {
    if sections.is_empty() {
        return 0;
    }

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

/// グリッド配置内の特定セクションのバウンディングボックスを計算
/// generate_multi_drawingと同じ配置ロジックを使用
pub fn calc_section_bounds_in_grid(
    sections: &[CrossSectionData],
    target_idx: usize,
    columns: usize,
    column_gap: f64,
    row_gap: f64,
) -> Option<(f32, f32, f32, f32)> {
    calc_section_bounds_in_grid_with_v_scale(sections, target_idx, columns, column_gap, row_gap, 2.0)
}

/// 縦スケール倍率を指定してグリッド配置内の特定セクションのバウンディングボックスを計算
pub fn calc_section_bounds_in_grid_with_v_scale(
    sections: &[CrossSectionData],
    target_idx: usize,
    columns: usize,
    column_gap: f64,
    row_gap: f64,
    v_scale_ratio: f64,
) -> Option<(f32, f32, f32, f32)> {
    if sections.is_empty() || target_idx >= sections.len() {
        return None;
    }

    let params = GridLayoutParams::new(column_gap, row_gap).with_v_scale(v_scale_ratio);
    let scale = params.scale;
    let rows_per_column = (sections.len() + columns - 1) / columns;

    // 列ごとの最大幅と全体の最大高さを計算（共通ヘルパー使用）
    let (col_max_left, col_max_right) = calc_column_widths(sections, columns, rows_per_column);

    let mut max_height: f64 = 0.0;
    for section in sections.iter() {
        if section.survey_data.len() >= 2 {
            max_height = max_height.max(params.section_body_height(section));
        }
    }

    // 列ごとのCL位置を計算（共通ヘルパー使用）
    let col_x_offsets = calc_column_x_offsets(&col_max_left, &col_max_right, column_gap, scale);

    let cell_height = params.cell_height_dxf(max_height);

    // ターゲットセクションの位置を計算
    let target_section = &sections[target_idx];
    if target_section.survey_data.len() < 2 {
        return None;
    }

    let col = target_idx / rows_per_column;
    let row_in_col = target_idx % rows_per_column;
    let offset_x = col_x_offsets[col];
    // 下マージン分上にシフト
    let offset_y = row_in_col as f64 * cell_height + DEFAULT_BOTTOM_MARGIN_DXF;

    // セクションのバウンディングボックスを計算
    let data = &target_section.survey_data;
    let dl = round_dl(target_section.dl);
    let y_scale = params.y_scale();

    let min_dist = data.first().unwrap().cumulative_distance;
    let max_dist = data.last().unwrap().cumulative_distance;
    let _min_elev = data
        .iter()
        .map(|d| d.elevation.min(d.planned_height))
        .fold(f64::MAX, f64::min);
    let max_elev = data
        .iter()
        .map(|d| d.elevation.max(d.planned_height))
        .fold(f64::MIN, f64::max);

    let min_x = (offset_x + min_dist * scale) as f32;
    let max_x = (offset_x + max_dist * scale) as f32;
    let min_y = (offset_y + (dl - dl) * y_scale) as f32; // DL位置
    let max_y = (offset_y + (max_elev - dl + 2.0) * y_scale) as f32; // 旗揚げ分のマージン

    Some((min_x, min_y, max_x, max_y))
}
