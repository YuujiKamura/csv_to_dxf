//! CrossSectionApp構造体と状態管理
//!
//! アプリケーションのメイン構造体、デフォルト値、ヘルパーメソッドを定義

use eframe::egui;
use dxf::Drawing;

use crate::{
    CrossSectionData,
    DxfViewState,
    ViewMode,
    PlotScale,
    TitleBlockInfo,
    parse_csv_sections,
    calc_sections_per_page,
    calc_grid_bounds,
    calc_section_bounds_in_grid,
    generate_alignment_test_drawing,
    generate_title_block_test_drawing,
    generate_combo_drawing,
    generate_multi_drawing_interpolated,
    generate_multi_drawing_with_frame_at_scale_interpolated,
    generate_longitudinal_drawing,
    generate_area_expansion_drawing,
    generate_dekigata_drawing,
};

#[cfg(target_arch = "wasm32")]
use crate::trigger_xlsx_dialog;

pub struct CrossSectionApp {
    pub(crate) sections: Vec<CrossSectionData>,
    pub(crate) selected_index: Option<usize>,
    pub(crate) dxf_drawing: Option<Drawing>,
    pub(crate) dxf_view_state: DxfViewState,
    pub(crate) view_mode: ViewMode,
    pub(crate) grid_columns: usize,         // グリッドの列数
    pub(crate) column_gap: f64,             // 列間隔（メートル単位）
    pub(crate) row_gap: f64,                // 行間隔（メートル単位）
    pub(crate) plot_scale: PlotScale,       // 図枠スケール（None=手動、Some(200)=1:200等）
    pub(crate) max_columns: usize,          // 現在のスケールで収まる最大列数
    pub(crate) fits_in_frame: bool,         // 現在の設定で図枠に収まるか
    pub(crate) status_message: Option<String>,
    pub(crate) needs_fit: bool,             // canvas_rect更新後にfit_to_dxfを呼ぶフラグ
    pub(crate) fit_bounds: Option<(f32, f32, f32, f32)>,  // フィット先のバウンディングボックス（Singleモード用）
    pub(crate) is_first_frame: bool,        // 初回フレームフラグ（モバイル判定用）
    pub(crate) loading_frame: usize,        // ローディングアニメーション用フレームカウンタ
    pub(crate) loading_stage: String,       // ローディング進捗メッセージ
    pub(crate) routes: Vec<String>,         // 利用可能なルートのリスト
    pub(crate) selected_route: String,      // 選択中のルート
    pub(crate) api_key: String,             // セッションのみのAPIキー
    pub(crate) api_key_set: bool,           // UI表示用フラグ
    // タイトルブロック情報
    pub(crate) project_name: String,        // 工事名
    pub(crate) drawing_type: String,        // 図面名
    pub(crate) route_name: String,          // 路線名
    pub(crate) author: String,              // 施工者
    pub(crate) date: String,                // 作成日
    pub(crate) drawing_number: String,      // 図面番号
    pub(crate) show_debug_guides: bool,     // デバッグガイド表示
    pub(crate) hide_project_name: bool,     // 工事名を非表示
    pub(crate) v_scale_ratio: f64,          // 縦方向スケール倍率（1.0〜5.0）
    pub(crate) dekigata_scale: f64,         // 出来形管理図のスケール（30〜100）
    pub(crate) odd_points_only: bool,       // 奇数測点のみ表示
    // ページ分割
    pub(crate) current_page: usize,         // 現在のページ（0始まり）
    pub(crate) total_pages: usize,          // 総ページ数
    pub(crate) sections_per_page: usize,    // 1ページあたりのセクション数
}

impl Default for CrossSectionApp {
    fn default() -> Self {
        Self {
            sections: Vec::new(),
            selected_index: None,
            dxf_drawing: None,
            dxf_view_state: DxfViewState::default(),
            view_mode: ViewMode::Dekigata, // デフォルトで出来形管理図
            grid_columns: 3,
            column_gap: 2.0,  // 列間隔2メートル（切削厚ラベル分）
            row_gap: 1.0,     // 行間隔1メートル
            plot_scale: None,  // デフォルトは手動列数指定
            max_columns: 99,   // 初期値（スケール未選択時）
            fits_in_frame: true,
            status_message: None,
            needs_fit: false,
            fit_bounds: None,
            is_first_frame: true,
            loading_frame: 0,
            loading_stage: "JSONデータを取得中".to_string(),
            routes: vec!["route_1".to_string()],
            selected_route: "route_1".to_string(),
            api_key: String::new(),
            api_key_set: false,
            // タイトルブロック情報（デフォルト）
            project_name: String::new(),
            drawing_type: "横断図".to_string(),
            route_name: String::new(),
            author: String::new(),
            date: String::new(),
            drawing_number: String::new(),
            show_debug_guides: true,  // デフォルトでON
            hide_project_name: false, // デフォルトで表示
            v_scale_ratio: 1.0,       // デフォルト縦スケール1倍
            dekigata_scale: 40.0,     // デフォルト出来形スケール1:40
            odd_points_only: true,    // デフォルトで奇数測点のみ
            // ページ分割
            current_page: 0,
            total_pages: 1,
            sections_per_page: 6,  // デフォルト6個/ページ
        }
    }
}

impl CrossSectionApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // 日本語フォントを設定
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "NotoSansJP".to_owned(),
            egui::FontData::from_static(
                include_bytes!("../static/NotoSansJP-Subset.ttf")
            )
        );
        fonts.families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "NotoSansJP".to_owned());
        fonts.families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .insert(0, "NotoSansJP".to_owned());
        cc.egui_ctx.set_fonts(fonts);

        let mut app = Self::default();
        app.status_message = Some("読み込み中...".to_string());
        app
    }

    pub(crate) fn api_key_ui(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.label("Gemini API Key");
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.api_key)
                    .password(true)
                    .desired_width(160.0),
            );
            if ui.button("Set").clicked() {
                self.api_key_set = !self.api_key.is_empty();
            }
            if ui.button("Clear").clicked() {
                self.api_key.clear();
                self.api_key_set = false;
            }
        });
        ui.label(if self.api_key_set {
            "APIキー設定済み（セッションのみ）"
        } else {
            "APIキー未設定"
        });
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn gemini_excel_ui(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        if ui.button("Excelを読み込む (Gemini)").clicked() {
            trigger_xlsx_dialog();
        }
        ui.label("Excelはローカルに保存されず、APIに送信されます。");
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn gemini_excel_ui(&mut self, _ui: &mut egui::Ui) {}

    pub(crate) fn handle_csv_loaded(&mut self, csv_text: &str) {
        match parse_csv_sections(csv_text) {
            Ok(raw_sections) => {
                let mut sections = Vec::with_capacity(raw_sections.len());
                for section in raw_sections.iter() {
                    match CrossSectionData::from_csv_section(section) {
                        Ok(data) => sections.push(data),
                        Err(err) => {
                            self.status_message = Some(format!("CSV変換エラー: {err}"));
                            return;
                        }
                    }
                }
                self.sections = sections;
                self.selected_index = self.sections.first().map(|_| 0);
                self.status_message = Some(format!("CSVを読み込みました ({} 区間)", self.sections.len()));
                self.update_dxf_preview();
            }
            Err(err) => {
                self.status_message = Some(format!("CSV読み込みエラー: {err}"));
            }
        }
    }

    /// 選択中のルートでフィルターされたセクションを返す
    pub(crate) fn filtered_sections(&self) -> Vec<&CrossSectionData> {
        self.sections.iter()
            .filter(|s| s.route_id == self.selected_route)
            .collect()
    }

    /// 出来形管理用紙用: 整数測点のみをフィルタ（「+」を含む中間測点は除外）
    /// odd_points_only がtrueの場合、奇数測点のみを返す
    pub(crate) fn dekigata_filtered_sections(&self) -> Vec<&CrossSectionData> {
        self.sections.iter()
            .filter(|s| s.route_id == self.selected_route)
            .filter(|s| !s.survey_point_name.contains('+'))
            .filter(|s| {
                if !self.odd_points_only {
                    return true;
                }
                // 測点名から数字を抽出して奇数かどうか判定
                // 例: "NO.1" -> 1, "NO.12" -> 12
                let num_str: String = s.survey_point_name
                    .chars()
                    .filter(|c| c.is_ascii_digit())
                    .collect();
                if let Ok(num) = num_str.parse::<i32>() {
                    num % 2 == 1  // 奇数のみ
                } else {
                    true  // 数字が取れない場合は含める
                }
            })
            .collect()
    }

    /// 現在のページに表示するセクションを返す
    pub(crate) fn current_page_sections(&self) -> Vec<&CrossSectionData> {
        let filtered = self.filtered_sections();
        if self.total_pages <= 1 || self.sections_per_page == 0 {
            return filtered;
        }
        let start = self.current_page * self.sections_per_page;
        let end = (start + self.sections_per_page).min(filtered.len());
        filtered[start..end].to_vec()
    }

    /// ページ数を再計算
    pub(crate) fn recalc_pages(&mut self) {
        let filtered: Vec<CrossSectionData> = self.filtered_sections()
            .into_iter().cloned().collect();

        if filtered.is_empty() {
            self.sections_per_page = 0;
            self.total_pages = 1;
            self.current_page = 0;
            return;
        }

        // スケールが指定されている場合のみページ分割
        if let Some(scale) = self.plot_scale {
            self.sections_per_page = calc_sections_per_page(
                &filtered,
                self.grid_columns,
                self.column_gap,
                self.row_gap,
                scale as f64,
            );

            if self.sections_per_page > 0 {
                self.total_pages = (filtered.len() + self.sections_per_page - 1) / self.sections_per_page;
            } else {
                self.total_pages = 1;
            }

            // 現在のページが範囲外なら調整
            if self.current_page >= self.total_pages {
                self.current_page = self.total_pages.saturating_sub(1);
            }
        } else {
            // 手動モードはページ分割なし
            self.sections_per_page = filtered.len();
            self.total_pages = 1;
            self.current_page = 0;
        }
    }

    pub(crate) fn update_dxf_preview(&mut self) {
        let filtered: Vec<CrossSectionData> = self.filtered_sections()
            .into_iter().cloned().collect();

        // fit_boundsをリセット（Singleモード以外はNone）
        self.fit_bounds = None;

        // 最大列数はセクション数（それ以上は意味がない）
        if !filtered.is_empty() {
            self.max_columns = filtered.len();
            // 列数がセクション数を超えていたら調整
            if self.grid_columns > self.max_columns {
                self.grid_columns = self.max_columns;
            }
        }

        // 図枠スケール選択時は収まるかチェック
        if let Some(scale) = self.plot_scale {
            if !filtered.is_empty() {
                if let Some(bounds) = calc_grid_bounds(&filtered, self.grid_columns, self.column_gap, self.row_gap) {
                    self.fits_in_frame = bounds.fits_in_frame(scale as f64, self.grid_columns);
                } else {
                    self.fits_in_frame = true;
                }
            }
        } else {
            self.fits_in_frame = true;
        }

        let columns = self.grid_columns;

        let drawing = match self.view_mode {
            ViewMode::AlignTest => {
                generate_alignment_test_drawing()
            }
            ViewMode::TitleBlock => {
                generate_title_block_test_drawing()
            }
            ViewMode::Combo if !filtered.is_empty() => {
                generate_combo_drawing(&filtered, columns, self.column_gap)
            }
            ViewMode::AllGrid if !filtered.is_empty() => {
                if let Some(scale) = self.plot_scale {
                    // 現在のページのセクションのみ使用
                    let page_sections: Vec<CrossSectionData> = self.current_page_sections()
                        .into_iter().cloned().collect();
                    // 図枠付きで描画
                    let page_num = if self.total_pages > 1 {
                        format!("{}-{}", self.drawing_number, self.current_page + 1)
                    } else {
                        self.drawing_number.clone()
                    };
                    let display_project_name = if self.hide_project_name { "" } else { &self.project_name };
                    let info = TitleBlockInfo::new()
                        .with_project_name(display_project_name)
                        .with_drawing_type(&self.drawing_type)
                        .with_route_name(&self.route_name)
                        .with_author(&self.author)
                        .with_date(&self.date)
                        .with_drawing_number(&page_num)
                        .with_top_title(&self.drawing_type)
                        .with_scale(&format!("1:{} (A3)", scale))
                        .with_debug_markers(self.show_debug_guides);
                    generate_multi_drawing_with_frame_at_scale_interpolated(&page_sections, columns, self.column_gap, self.row_gap, &info, scale as f64, self.v_scale_ratio)
                } else {
                    generate_multi_drawing_interpolated(&filtered, columns, self.column_gap, self.row_gap, self.v_scale_ratio)
                }
            }
            ViewMode::Longitudinal if !filtered.is_empty() => {
                generate_longitudinal_drawing(&filtered)
            }
            ViewMode::AreaExpansion if !filtered.is_empty() => {
                generate_area_expansion_drawing(&filtered, true)
            }
            ViewMode::Dekigata => {
                // 整数測点のみフィルタ（中間測点は除外）
                let dekigata_sections: Vec<CrossSectionData> = self.dekigata_filtered_sections()
                    .into_iter().cloned().collect();
                if !dekigata_sections.is_empty() {
                    // 出来形モードでは selected_index は dekigata_sections 内のインデックス
                    let display_idx = self.selected_index
                        .map(|idx| idx.min(dekigata_sections.len() - 1))
                        .unwrap_or(0);
                    let display_project_name = if self.hide_project_name { "" } else { &self.project_name };
                    let info = TitleBlockInfo::new()
                        .with_project_name(display_project_name)
                        .with_drawing_type("出来形管理用紙")
                        .with_route_name(&self.route_name)
                        .with_author(&self.author)
                        .with_date(&self.date)
                        .with_top_title("出来形管理用紙")
                        .with_scale(&format!("1:{} (A3)", self.dekigata_scale as u32))
                        .with_debug_markers(self.show_debug_guides);
                    // プレビュー用: 選択された測点を表示
                    generate_dekigata_drawing(&dekigata_sections[display_idx], &info, self.v_scale_ratio, self.dekigata_scale)
                } else {
                    return;
                }
            }
            ViewMode::Single if !filtered.is_empty() => {
                // 全横断図を生成し、選択した横断図の位置にフィット
                if let Some(idx) = self.selected_index {
                    self.fit_bounds = calc_section_bounds_in_grid(
                        &filtered, idx, columns, self.column_gap, self.row_gap
                    );
                }
                // 補間を適用（起終点以外の補完点を勾配補間）
                generate_multi_drawing_interpolated(&filtered, columns, self.column_gap, self.row_gap, self.v_scale_ratio)
            }
            _ => { return; }
        };

        self.dxf_drawing = Some(drawing);
        self.needs_fit = true; // canvas_rect更新後にfit_to_dxfを呼ぶ
    }
}
