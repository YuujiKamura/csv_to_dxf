//! グリッドレイアウトのパラメータと定数
//!
//! GridLayoutParams構造体とマージン定数を提供

use crate::{CrossSectionData, SectionData};

// ============================================================================
// Constants
// ============================================================================

// SectionStyleのデフォルト値から計算された固定マージン（DXF単位）
// 上方向: flag_base_offset + label_title_offset + text_height * title_scale
/// Default top margin for cross-section layout (flag raising area)
pub const DEFAULT_TOP_MARGIN_DXF: f64 = 1600.0 + 1300.0 + 300.0 * 1.5; // = 3350.0

// 下方向: cutting_label_offset + text_height + cutting_label_gap
/// Default bottom margin for cross-section layout (cutting depth labels)
pub const DEFAULT_BOTTOM_MARGIN_DXF: f64 = 300.0 + 300.0 + 100.0; // = 700.0

// ============================================================================
// GridLayoutParams
// ============================================================================

/// グリッドレイアウトのパラメータと計算ロジック
///
/// セル高さの計算を一元管理し、v_scale_ratioと行間隔の関係を正しく処理する。
/// - セクション本体高さ: v_scale_ratio適用（縦に伸縮）
/// - ラベルマージン（旗揚げ・切削厚）: 設定可能（デフォルトは固定DXF単位）
/// - 行間隔: 固定（v_scale_ratioに依存しない）
#[derive(Debug, Clone, Copy)]
pub struct GridLayoutParams {
    /// 基本スケール（通常1000.0 = 1m = 1000 DXF units）
    pub scale: f64,
    /// 縦方向スケール倍率（1.0〜5.0）
    pub v_scale_ratio: f64,
    /// 列間隔（メートル）
    pub column_gap: f64,
    /// 行間隔（メートル）
    pub row_gap: f64,
    /// 上マージン（DXF単位）- 旗揚げラベル領域
    pub top_margin: f64,
    /// 下マージン（DXF単位）- 切削厚ラベル領域
    pub bottom_margin: f64,
}

impl GridLayoutParams {
    /// デフォルトパラメータ（scale=1000, v_scale=2.0, default margins）
    pub fn new(column_gap: f64, row_gap: f64) -> Self {
        Self {
            scale: 1000.0,
            v_scale_ratio: 2.0,
            column_gap,
            row_gap,
            top_margin: DEFAULT_TOP_MARGIN_DXF,
            bottom_margin: DEFAULT_BOTTOM_MARGIN_DXF,
        }
    }

    /// v_scale_ratioを指定
    pub fn with_v_scale(mut self, v_scale_ratio: f64) -> Self {
        self.v_scale_ratio = v_scale_ratio;
        self
    }

    /// Set top margin (DXF units)
    pub fn with_top_margin(mut self, margin: f64) -> Self {
        self.top_margin = margin;
        self
    }

    /// Set bottom margin (DXF units)
    pub fn with_bottom_margin(mut self, margin: f64) -> Self {
        self.bottom_margin = margin;
        self
    }

    /// Set both margins (DXF units)
    pub fn with_margins(mut self, top: f64, bottom: f64) -> Self {
        self.top_margin = top;
        self.bottom_margin = bottom;
        self
    }

    // ========================================================================
    // Generic methods using SectionData trait
    // ========================================================================

    /// Calculate section body height (meters) using SectionData trait
    pub fn body_height<S: SectionData>(&self, section: &S) -> f64 {
        section.body_height()
    }

    /// Calculate maximum body height across sections using SectionData trait
    pub fn max_body_height<S: SectionData>(&self, sections: &[S]) -> f64 {
        sections
            .iter()
            .map(|s| self.body_height(s))
            .fold(0.0_f64, f64::max)
    }

    /// Calculate cell height for sections using SectionData trait
    pub fn cell_height<S: SectionData>(&self, sections: &[S]) -> f64 {
        let max_height = self.max_body_height(sections);
        self.cell_height_dxf(max_height)
    }

    // ========================================================================
    // CrossSectionData-specific methods (backward compatibility)
    // ========================================================================

    /// セクションの本体高さを計算（メートル単位、max_elev - dl）
    pub fn section_body_height(&self, section: &CrossSectionData) -> f64 {
        self.body_height(section)
    }

    /// 複数セクションの最大本体高さ（メートル単位）
    pub fn max_section_body_height(&self, sections: &[CrossSectionData]) -> f64 {
        self.max_body_height(sections)
    }

    /// セクションの全描画高さをDXF単位で計算
    ///
    /// 構成:
    /// - セクション本体: (max_elev - dl) * scale * v_scale_ratio （スケール適用）
    /// - 上マージン（旗揚げ）: self.top_margin
    /// - 下マージン（切削厚ラベル）: self.bottom_margin
    pub fn section_height_dxf(&self, body_height_m: f64) -> f64 {
        body_height_m * self.scale * self.v_scale_ratio + self.top_margin + self.bottom_margin
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
        self.cell_height(sections)
    }

    /// Y方向スケール（DXF単位変換用）
    pub fn y_scale(&self) -> f64 {
        self.scale * self.v_scale_ratio
    }

    /// 下マージン（DXF単位）- セクション配置のベースオフセット用
    pub fn bottom_margin_dxf(&self) -> f64 {
        self.bottom_margin
    }

    /// 上マージン（DXF単位）
    pub fn top_margin_dxf(&self) -> f64 {
        self.top_margin
    }
}
