//! フォントメトリクスに基づくDXFテキスト高さ補正
//!
//! Webビューア（egui）とCAD（CADWELL土木等）でテキストサイズの解釈が異なる:
//! - egui: font_size = em-square で描画
//! - CAD: text_height = キャップハイト（大文字の高さ）で解釈
//!
//! 同じ数値でも見た目のサイズが異なるため、DXF出力時に補正が必要。

/// Noto Sans JP のフォントメトリクス定数
pub mod constants {
    /// em-squareのユニット数
    pub const UNITS_PER_EM: f64 = 1000.0;
    /// キャップハイト（大文字Hの高さ）: 73.3% of em
    pub const CAP_HEIGHT: f64 = 733.0;
    /// タイポグラフィックアセンダー
    pub const TYPO_ASCENDER: f64 = 880.0;
    /// タイポグラフィックディセンダー
    pub const TYPO_DESCENDER: f64 = -120.0;
}

/// DXF text_height補正係数
///
/// eguiでのfont_size（em-square基準）をCADのtext_height（キャップハイト基準）に
/// 変換するためのスケール係数。
///
/// 計算: 1000 / 733 ≈ 1.364
pub const SCALE_FOR_CAP_HEIGHT: f64 = constants::UNITS_PER_EM / constants::CAP_HEIGHT;

/// 論理的な大文字高さをDXF text_heightに変換
///
/// # Arguments
/// * `cap_height` - egui描画で使用するフォントサイズ（em-square基準）
///
/// # Returns
/// CADで同じ見た目になるtext_height値
///
/// # Example
/// ```ignore
/// let egui_size = 100.0;
/// let dxf_height = cap_height_to_text_height(egui_size);
/// // dxf_height ≈ 136.4
/// ```
#[inline]
pub fn cap_height_to_text_height(cap_height: f64) -> f64 {
    cap_height * SCALE_FOR_CAP_HEIGHT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scale_factor() {
        // 補正係数が約1.364であることを確認
        assert!((SCALE_FOR_CAP_HEIGHT - 1.364).abs() < 0.001);
    }

    #[test]
    fn test_cap_height_conversion() {
        // 100pxの入力に対して約136.4pxの出力
        let result = cap_height_to_text_height(100.0);
        assert!((result - 136.4).abs() < 0.1);
    }

    #[test]
    fn test_zero_height() {
        assert_eq!(cap_height_to_text_height(0.0), 0.0);
    }
}
