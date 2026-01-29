//! eframe::App トレイト実装
//!
//! CrossSectionApp の UI 更新ロジック

use eframe::egui::{self, Color32, Pos2};
use std::collections::HashSet;

use crate::app::CrossSectionApp;
use crate::{
    CrossSectionData,
    ViewMode,
    TitleBlockInfo,
    DrawingContent,
    IrViewState,
    calc_dxf_bounds,
    download_file,
    generate_dxf_bytes,
    generate_multi_dxf_bytes,
    generate_all_pages_dxf_bytes,
    generate_longitudinal_dxf_bytes,
    generate_area_expansion_dxf_bytes,
    generate_all_dekigata_dxf_bytes,
    generate_combo_dxf_bytes,
    generate_all_routes_combo_dxf_bytes,
    generate_alignment_test_dxf_bytes,
    generate_title_block_dxf_for_download,
};

use crate::wasm_integration::{
    take_pending_csv, take_pending_json, take_pending_xlsx, take_pending_error,
    start_json_fetch, start_gemini_parse,
    is_ezdxf_ready, add_layouts_to_dxf_async,
};
#[cfg(target_arch = "wasm32")]
use crate::wasm_integration::trigger_csv_dialog;

impl eframe::App for CrossSectionApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 起動時にsections.jsonをfetch開始
        start_json_fetch();

        // JSONのfetch結果を処理
        if let Some(json_text) = take_pending_json() {
            self.loading_stage = "JSONをパース中".to_string();
            match CrossSectionData::from_json_with_title(&json_text) {
                Ok(data) => {
                    let sections = data.sections;
                    // タイトルブロック情報を適用
                    if let Some(tb) = data.title_block {
                        if !tb.project_name.is_empty() { self.project_name = tb.project_name; }
                        if !tb.drawing_type.is_empty() { self.drawing_type = tb.drawing_type; }
                        if !tb.route_name.is_empty() { self.route_name = tb.route_name; }
                        if !tb.author.is_empty() { self.author = tb.author; }
                        if !tb.date.is_empty() { self.date = tb.date; }
                        if !tb.drawing_number.is_empty() { self.drawing_number = tb.drawing_number; }
                    }

                    self.loading_stage = format!("{}測点のデータを処理中", sections.len());
                    let dump = if sections.is_empty() {
                        "JSONデータなし".to_string()
                    } else {
                        let first = &sections[0];
                        let last = &sections[sections.len() - 1];
                        format!(
                            "JSON: {}測点 {} ~ {} (DL={:.2}, {}点)",
                            sections.len(),
                            first.survey_point_name,
                            last.survey_point_name,
                            first.dl,
                            first.survey_data.len()
                        )
                    };
                    // ルート一覧を抽出
                    let mut route_set: HashSet<String> = HashSet::new();
                    for s in &sections {
                        route_set.insert(s.route_id.clone());
                    }
                    let mut routes: Vec<String> = route_set.into_iter().collect();
                    routes.sort();
                    self.routes = routes;
                    if !self.routes.is_empty() {
                        self.selected_route = self.routes[0].clone();
                    }

                    self.sections = sections;
                    self.selected_index = Some(0);
                    self.loading_stage = "DXFプレビューを生成中".to_string();
                    self.status_message = Some(dump);
                    self.recalc_pages();
                    self.update_dxf_preview();
                }
                Err(e) => {
                    self.status_message = Some(format!("JSONエラー: {e}"));
                }
            }
        }

        if let Some(csv_text) = take_pending_csv() {
            self.handle_csv_loaded(&csv_text);
        }
        if let Some(xlsx_bytes) = take_pending_xlsx() {
            if self.api_key.is_empty() {
                self.status_message = Some("APIキーが未設定です".to_string());
            } else {
                self.status_message = Some("Geminiで解析中...".to_string());
                self.loading_stage = "Gemini解析中".to_string();
                start_gemini_parse(self.api_key.clone(), xlsx_bytes);
            }
        }
        if let Some(err) = take_pending_error() {
            self.status_message = Some(format!("Geminiエラー: {err}"));
        }
        let screen_width = ctx.screen_rect().width();
        let is_mobile = screen_width < 600.0;

        // 初回フレームフラグをクリア
        if self.is_first_frame {
            self.is_first_frame = false;
        }

        if is_mobile {
            // モバイル: トップバー + フルスクリーン図面
            egui::TopBottomPanel::top("mobile_top").show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        // ルート選択（複数ルートがある場合のみ表示）
                        if self.routes.len() > 1 {
                            let response = egui::ComboBox::from_id_salt("route_select_mobile")
                                .selected_text(&self.selected_route)
                                .width(100.0)
                                .show_ui(ui, |ui| {
                                    let mut selected = None;
                                    for route in &self.routes {
                                        if ui.selectable_label(
                                            self.selected_route == *route,
                                            route
                                        ).clicked() {
                                            selected = Some(route.clone());
                                        }
                                    }
                                    selected
                                });
                            if let Some(route) = response.inner.flatten() {
                                self.selected_route = route;
                                self.selected_index = Some(0);
                                self.update_dxf_preview();
                            }
                        }

                        // 測点プルダウン（フィルター済みセクションから選択）
                        // 出来形モードでは整数測点のみ表示
                        let filtered: Vec<_> = if self.view_mode == ViewMode::Dekigata {
                            self.dekigata_filtered_sections()
                        } else {
                            self.filtered_sections()
                        };
                        let current_name = self.selected_index
                            .and_then(|i| filtered.get(i))
                            .map(|s| s.survey_point_name.as_str())
                            .unwrap_or("--");

                        let response = egui::ComboBox::from_id_salt("station_select")
                            .selected_text(current_name)
                            .width(150.0)
                            .show_ui(ui, |ui| {
                                ui.set_min_width(140.0);
                                let mut selected = None;
                                for (i, section) in filtered.iter().enumerate() {
                                    let response = ui.selectable_label(
                                        self.selected_index == Some(i),
                                        &section.survey_point_name
                                    );
                                    if response.clicked() {
                                        selected = Some(i);
                                    }
                                }
                                selected
                            });
                        if let Some(idx) = response.inner.flatten() {
                            self.selected_index = Some(idx);
                            // 出来形モード時はモード維持
                            if self.view_mode != ViewMode::Dekigata {
                                self.view_mode = ViewMode::Single;
                            }
                            self.update_dxf_preview();
                        }

                    });

                    ui.horizontal_wrapped(|ui| {
                        // 表示モード切替（タッチ操作しやすいよう幅を確保）
                        let mode_text = match self.view_mode {
                            ViewMode::Combo => "コンボ",
                            ViewMode::Single => "単一",
                            ViewMode::AllGrid => "全横断",
                            ViewMode::Longitudinal => "縦断",
                            ViewMode::AreaExpansion => "展開図",
                            ViewMode::Dekigata => "出来形",
                            ViewMode::AlignTest => "テスト",
                            ViewMode::TitleBlock => "図枠",
                        };
                        let response = egui::ComboBox::from_id_salt("view_mode_select")
                            .selected_text(mode_text)
                            .width(120.0)
                            .show_ui(ui, |ui| {
                                ui.set_min_width(160.0);
                                let mut selected = None;
                                if ui.selectable_label(self.view_mode == ViewMode::Single, "単一横断").clicked() {
                                    selected = Some(ViewMode::Single);
                                }
                                if ui.selectable_label(self.view_mode == ViewMode::Combo, "コンボ (縦断+全横断)").clicked() {
                                    selected = Some(ViewMode::Combo);
                                }
                                if ui.selectable_label(self.view_mode == ViewMode::AllGrid, "全横断").clicked() {
                                    selected = Some(ViewMode::AllGrid);
                                }
                                if ui.selectable_label(self.view_mode == ViewMode::Longitudinal, "縦断図").clicked() {
                                    selected = Some(ViewMode::Longitudinal);
                                }
                                if ui.selectable_label(self.view_mode == ViewMode::AreaExpansion, "面積展開図").clicked() {
                                    selected = Some(ViewMode::AreaExpansion);
                                }
                                if ui.selectable_label(self.view_mode == ViewMode::Dekigata, "出来形管理用紙").clicked() {
                                    selected = Some(ViewMode::Dekigata);
                                }
                                if ui.selectable_label(self.view_mode == ViewMode::AlignTest, "アライメントテスト").clicked() {
                                    selected = Some(ViewMode::AlignTest);
                                }
                                if ui.selectable_label(self.view_mode == ViewMode::TitleBlock, "タイトル枠").clicked() {
                                    selected = Some(ViewMode::TitleBlock);
                                }
                                selected
                            });
                        if let Some(mode) = response.inner.flatten() {
                            if self.view_mode != mode {
                                self.view_mode = mode;
                                self.update_dxf_preview();
                            }
                        }
                    });

                    if matches!(self.view_mode, ViewMode::AllGrid | ViewMode::Combo | ViewMode::Single | ViewMode::Dekigata) {
                        ui.horizontal_wrapped(|ui| {
                            // 図枠スケール選択
                            let scale_label = match self.plot_scale {
                                None => "手動".to_string(),
                                Some(s) => format!("1:{}", s),
                            };
                            egui::ComboBox::from_id_salt("scale_select_mobile")
                                .selected_text(scale_label)
                                .width(60.0)
                                .show_ui(ui, |ui| {
                                    if ui.selectable_label(self.plot_scale.is_none(), "手動").clicked() {
                                        self.plot_scale = None;
                                        self.recalc_pages();
                                        self.update_dxf_preview();
                                    }
                                    if ui.selectable_label(self.plot_scale == Some(100), "1:100").clicked() {
                                        self.plot_scale = Some(100);
                                        self.recalc_pages();
                                        self.update_dxf_preview();
                                    }
                                    if ui.selectable_label(self.plot_scale == Some(200), "1:200").clicked() {
                                        self.plot_scale = Some(200);
                                        self.recalc_pages();
                                        self.update_dxf_preview();
                                    }
                                    if ui.selectable_label(self.plot_scale == Some(500), "1:500").clicked() {
                                        self.plot_scale = Some(500);
                                        self.recalc_pages();
                                        self.update_dxf_preview();
                                    }
                                });

                            // 列数選択（セクション数まで）
                            ui.label(format!("{}列", self.grid_columns));
                            if ui.small_button("+").clicked() && self.grid_columns < self.max_columns {
                                self.grid_columns += 1;
                                self.recalc_pages();
                                self.update_dxf_preview();
                            }
                            if ui.small_button("-").clicked() && self.grid_columns > 1 {
                                self.grid_columns -= 1;
                                self.recalc_pages();
                                self.update_dxf_preview();
                            }

                        });
                        ui.horizontal_wrapped(|ui| {
                            // 列間隔
                            ui.label(format!("列{:.1}m", self.column_gap));
                            if ui.small_button("+").clicked() && self.column_gap < 5.0 {
                                self.column_gap += 0.5;
                                self.recalc_pages();
                                self.update_dxf_preview();
                            }
                            if ui.small_button("-").clicked() && self.column_gap > -5.0 {
                                self.column_gap -= 0.5;
                                self.recalc_pages();
                                self.update_dxf_preview();
                            }
                            ui.separator();
                            // 行間隔
                            ui.label(format!("行{:.1}m", self.row_gap));
                            if ui.small_button("+").clicked() && self.row_gap < 3.0 {
                                self.row_gap += 0.5;
                                self.recalc_pages();
                                self.update_dxf_preview();
                            }
                            if ui.small_button("-").clicked() && self.row_gap > 0.0 {
                                self.row_gap -= 0.5;
                                self.recalc_pages();
                                self.update_dxf_preview();
                            }
                            ui.separator();
                            // 縦スケール倍率
                            ui.label("縦倍率");
                            let v_scale_options = [1.0, 1.5, 2.0, 2.5, 3.0, 4.0, 5.0];
                            egui::ComboBox::from_id_salt("v_scale_mobile")
                                .selected_text(format!("{:.1}x", self.v_scale_ratio))
                                .show_ui(ui, |ui| {
                                    for &opt in &v_scale_options {
                                        if ui.selectable_value(&mut self.v_scale_ratio, opt, format!("{:.1}x", opt)).changed() {
                                            self.recalc_pages();
                                            self.update_dxf_preview();
                                        }
                                    }
                                });
                            // 出来形スケール（出来形モード時のみ）
                            if self.view_mode == ViewMode::Dekigata {
                                ui.separator();
                                ui.label("図枠");
                                let dekigata_scale_options = [30.0, 40.0, 50.0, 75.0, 100.0];
                                egui::ComboBox::from_id_salt("dekigata_scale_mobile")
                                    .selected_text(format!("1:{}", self.dekigata_scale as u32))
                                    .show_ui(ui, |ui| {
                                        for &opt in &dekigata_scale_options {
                                            if ui.selectable_value(&mut self.dekigata_scale, opt, format!("1:{}", opt as u32)).changed() {
                                                self.update_dxf_preview();
                                            }
                                        }
                                    });
                            }
                            // デバッグガイド表示切替
                            if self.plot_scale.is_some() {
                                if ui.checkbox(&mut self.show_debug_guides, "ガイド").changed() {
                                    self.update_dxf_preview();
                                }
                            }
                            // 工事名非表示
                            if self.plot_scale.is_some() || self.view_mode == ViewMode::Dekigata {
                                if ui.checkbox(&mut self.hide_project_name, "工事名非表示").changed() {
                                    self.update_dxf_preview();
                                }
                            }
                            // 奇数測点のみ（出来形モード時）
                            if self.view_mode == ViewMode::Dekigata {
                                if ui.checkbox(&mut self.odd_points_only, "奇数測点のみ").changed() {
                                    self.update_dxf_preview();
                                }
                            }
                        });
                        // ページナビゲーション（複数ページある場合のみ）
                        if self.total_pages > 1 && self.plot_scale.is_some() {
                            ui.horizontal_wrapped(|ui| {
                                if ui.small_button("<").clicked() && self.current_page > 0 {
                                    self.current_page -= 1;
                                    self.update_dxf_preview();
                                }
                                ui.label(format!("{}/{}", self.current_page + 1, self.total_pages));
                                if ui.small_button(">").clicked() && self.current_page < self.total_pages - 1 {
                                    self.current_page += 1;
                                    self.update_dxf_preview();
                                }
                            });
                        }
                    }

                    ui.horizontal_wrapped(|ui| {
                        // DXFダウンロード（フィルター済みセクションを使用）
                        let filtered_for_dxf: Vec<CrossSectionData> = self.filtered_sections()
                            .into_iter().cloned().collect();
                        match self.view_mode {
                            ViewMode::Combo if !filtered_for_dxf.is_empty() => {
                                if ui.button("DXF").clicked() {
                                    // 全ルートを図枠ごと縦に並べたDXFを生成
                                    let dxf_content = generate_all_routes_combo_dxf_bytes(&self.sections, self.grid_columns, self.column_gap);
                                    download_file("combo.dxf", &dxf_content);
                                }
                            }
                            ViewMode::AllGrid if !filtered_for_dxf.is_empty() => {
                                if ui.button("DXF").clicked() {
                                    let (dxf_content, filename) = if let Some(scale) = self.plot_scale {
                                        // 全ページを1つのDXFに垂直配置
                                        let display_project_name = if self.hide_project_name { "" } else { &self.project_name };
                                        let info = TitleBlockInfo::new()
                                            .with_project_name(display_project_name)
                                            .with_drawing_type(&self.drawing_type)
                                            .with_route_name(&self.route_name)
                                            .with_author(&self.author)
                                            .with_date(&self.date)
                                            .with_top_title(&self.drawing_type)
                                            .with_scale(&format!("1:{} (A3)", scale))
                                            .with_debug_markers(false);
                                        (generate_all_pages_dxf_bytes(&filtered_for_dxf, self.grid_columns, self.column_gap, self.row_gap, scale as f64, &info, &self.drawing_number, self.v_scale_ratio), "cross_sections.dxf".to_string())
                                    } else {
                                        (generate_multi_dxf_bytes(&filtered_for_dxf, self.grid_columns, self.column_gap, self.row_gap), "cross_sections_all.dxf".to_string())
                                    };
                                    download_file(&filename, &dxf_content);
                                }
                            }
                            ViewMode::Longitudinal if !filtered_for_dxf.is_empty() => {
                                if ui.button("DXF").clicked() {
                                    let dxf_content = generate_longitudinal_dxf_bytes(&filtered_for_dxf);
                                    download_file("longitudinal.dxf", &dxf_content);
                                }
                            }
                            ViewMode::AreaExpansion if !filtered_for_dxf.is_empty() => {
                                if ui.button("DXF").clicked() {
                                    let dxf_content = generate_area_expansion_dxf_bytes(&filtered_for_dxf);
                                    download_file("area_expansion.dxf", &dxf_content);
                                }
                            }
                            ViewMode::Dekigata => {
                                // 整数測点のみフィルタ（中間測点は除外）
                                let dekigata_sections: Vec<CrossSectionData> = self.dekigata_filtered_sections()
                                    .into_iter().cloned().collect();
                                if !dekigata_sections.is_empty() {
                                    // ezdxf準備状態でボタンテキスト変更
                                    let btn_text = if is_ezdxf_ready() { "DXF+Layout" } else { "DXF" };
                                    if ui.button(btn_text).clicked() {
                                        let display_project_name = if self.hide_project_name { "" } else { &self.project_name };
                                        let info = TitleBlockInfo::new()
                                            .with_project_name(display_project_name)
                                            .with_drawing_type("出来形管理用紙")
                                            .with_route_name(&self.route_name)
                                            .with_author(&self.author)
                                            .with_date(&self.date)
                                            .with_top_title("出来形管理用紙")
                                            .with_scale(&format!("1:{} (A3)", self.dekigata_scale as u32))
                                            .with_debug_markers(false);
                                        let dxf_content = generate_all_dekigata_dxf_bytes(&dekigata_sections, &info, &self.drawing_number, self.v_scale_ratio, self.dekigata_scale);

                                        if is_ezdxf_ready() {
                                            // レイアウト追加してダウンロード
                                            let page_count = dekigata_sections.len() as u32;
                                            add_layouts_to_dxf_async(dxf_content, self.dekigata_scale, page_count);
                                        } else {
                                            // 通常ダウンロード
                                            download_file("dekigata.dxf", &dxf_content);
                                        }
                                    }
                                }
                            }
                            ViewMode::Single => {
                                if self.selected_index.is_some() && ui.button("DXF").clicked() {
                                    if let Some(idx) = self.selected_index {
                                        if let Some(section) = filtered_for_dxf.get(idx) {
                                            // 起終点以外は補間を適用
                                            let is_start_or_end = section.is_route_start || section.is_route_end;
                                            let mut section = section.clone();
                                            if !is_start_or_end {
                                                section.interpolate_planned_heights();
                                            }
                                            let dxf_content = generate_dxf_bytes(&section);
                                            let filename = format!("{}.dxf", section.survey_point_name);
                                            download_file(&filename, &dxf_content);
                                        }
                                    }
                                }
                            }
                            ViewMode::AlignTest => {
                                if ui.button("DXF").clicked() {
                                    let dxf_content = generate_alignment_test_dxf_bytes();
                                    download_file("alignment_test.dxf", &dxf_content);
                                }
                            }
                            ViewMode::TitleBlock => {
                                if ui.button("DXF").clicked() {
                                    let dxf_content = generate_title_block_dxf_for_download();
                                    download_file("title_block.dxf", &dxf_content);
                                }
                            }
                            _ => {}
                        }
                    });

                    ui.collapsing("API", |ui| {
                        self.api_key_ui(ui);
                        self.gemini_excel_ui(ui);
                    });
                });
            });
        } else {
            // デスクトップ: サイドパネル
            egui::SidePanel::left("side_panel").min_width(180.0).show(ctx, |ui| {
                ui.heading("Cross Section");
                ui.separator();

                #[cfg(target_arch = "wasm32")]
                if ui.button("CSVを読み込む").clicked() {
                    trigger_csv_dialog();
                }
                if let Some(message) = &self.status_message {
                    ui.label(message);
                }
                ui.collapsing("API", |ui| {
                    self.api_key_ui(ui);
                    self.gemini_excel_ui(ui);
                });

                // ルート選択（複数ルートがある場合のみ表示）
                if self.routes.len() > 1 {
                    ui.horizontal(|ui| {
                        ui.label("路線:");
                        let response = egui::ComboBox::from_id_salt("route_select_desktop")
                            .selected_text(&self.selected_route)
                            .show_ui(ui, |ui| {
                                let mut selected = None;
                                for route in &self.routes {
                                    if ui.selectable_label(
                                        self.selected_route == *route,
                                        route
                                    ).clicked() {
                                        selected = Some(route.clone());
                                    }
                                }
                                selected
                            });
                        if let Some(route) = response.inner.flatten() {
                            self.selected_route = route;
                            self.selected_index = Some(0);
                            self.update_dxf_preview();
                        }
                    });
                }

                // 表示モード切替
                ui.horizontal(|ui| {
                    ui.label("表示:");
                    if ui.selectable_label(self.view_mode == ViewMode::Combo, "コンボ").clicked() {
                        self.view_mode = ViewMode::Combo;
                        self.update_dxf_preview();
                    }
                    if ui.selectable_label(self.view_mode == ViewMode::Single, "単一").clicked() {
                        self.view_mode = ViewMode::Single;
                        self.update_dxf_preview();
                    }
                    if ui.selectable_label(self.view_mode == ViewMode::AllGrid, "全横断").clicked() {
                        self.view_mode = ViewMode::AllGrid;
                        self.update_dxf_preview();
                    }
                    if ui.selectable_label(self.view_mode == ViewMode::Longitudinal, "縦断").clicked() {
                        self.view_mode = ViewMode::Longitudinal;
                        self.update_dxf_preview();
                    }
                    if ui.selectable_label(self.view_mode == ViewMode::AreaExpansion, "展開図").clicked() {
                        self.view_mode = ViewMode::AreaExpansion;
                        self.update_dxf_preview();
                    }
                    if ui.selectable_label(self.view_mode == ViewMode::Dekigata, "出来形").clicked() {
                        self.view_mode = ViewMode::Dekigata;
                        self.update_dxf_preview();
                    }
                    if ui.selectable_label(self.view_mode == ViewMode::TitleBlock, "図枠").clicked() {
                        self.view_mode = ViewMode::TitleBlock;
                        self.update_dxf_preview();
                    }
                });

                // AllGrid/Combo/Single/Dekigataモード時の列数・間隔調整
                if matches!(self.view_mode, ViewMode::AllGrid | ViewMode::Combo | ViewMode::Single | ViewMode::Dekigata) {
                    ui.horizontal(|ui| {
                        // 図枠スケール選択
                        let scale_label = match self.plot_scale {
                            None => "手動".to_string(),
                            Some(s) => format!("1:{}", s),
                        };
                        egui::ComboBox::from_id_salt("scale_select_desktop")
                            .selected_text(scale_label)
                            .width(70.0)
                            .show_ui(ui, |ui| {
                                if ui.selectable_label(self.plot_scale.is_none(), "手動").clicked() {
                                    self.plot_scale = None;
                                    self.recalc_pages();
                                    self.update_dxf_preview();
                                }
                                if ui.selectable_label(self.plot_scale == Some(100), "1:100").clicked() {
                                    self.plot_scale = Some(100);
                                    self.recalc_pages();
                                    self.update_dxf_preview();
                                }
                                if ui.selectable_label(self.plot_scale == Some(200), "1:200").clicked() {
                                    self.plot_scale = Some(200);
                                    self.recalc_pages();
                                    self.update_dxf_preview();
                                }
                                if ui.selectable_label(self.plot_scale == Some(500), "1:500").clicked() {
                                    self.plot_scale = Some(500);
                                    self.recalc_pages();
                                    self.update_dxf_preview();
                                }
                            });

                        // 列数選択（セクション数まで）
                        ui.label(format!("{}列", self.grid_columns));
                        if ui.small_button("+").clicked() && self.grid_columns < self.max_columns {
                            self.grid_columns += 1;
                            self.recalc_pages();
                            self.update_dxf_preview();
                        }
                        if ui.small_button("-").clicked() && self.grid_columns > 1 {
                            self.grid_columns -= 1;
                            self.recalc_pages();
                            self.update_dxf_preview();
                        }
                    });
                    ui.horizontal(|ui| {
                        // 列間隔
                        ui.label(format!("列{:.1}m", self.column_gap));
                        if ui.small_button("+").clicked() && self.column_gap < 5.0 {
                            self.column_gap += 0.5;
                            self.recalc_pages();
                            self.update_dxf_preview();
                        }
                        if ui.small_button("-").clicked() && self.column_gap > -5.0 {
                            self.column_gap -= 0.5;
                            self.recalc_pages();
                            self.update_dxf_preview();
                        }
                        ui.separator();
                        // 行間隔
                        ui.label(format!("行{:.1}m", self.row_gap));
                        if ui.small_button("+").clicked() && self.row_gap < 3.0 {
                            self.row_gap += 0.5;
                            self.recalc_pages();
                            self.update_dxf_preview();
                        }
                        if ui.small_button("-").clicked() && self.row_gap > 0.0 {
                            self.row_gap -= 0.5;
                            self.recalc_pages();
                            self.update_dxf_preview();
                        }
                        ui.separator();
                        // 縦スケール倍率
                        ui.label("縦倍率");
                        let v_scale_options = [1.0, 1.5, 2.0, 2.5, 3.0, 4.0, 5.0];
                        egui::ComboBox::from_id_salt("v_scale_desktop")
                            .selected_text(format!("{:.1}x", self.v_scale_ratio))
                            .show_ui(ui, |ui| {
                                for &opt in &v_scale_options {
                                    if ui.selectable_value(&mut self.v_scale_ratio, opt, format!("{:.1}x", opt)).changed() {
                                        self.recalc_pages();
                                        self.update_dxf_preview();
                                    }
                                }
                            });
                        // 出来形スケール（出来形モード時のみ）
                        if self.view_mode == ViewMode::Dekigata {
                            ui.separator();
                            ui.label("図枠");
                            let dekigata_scale_options = [30.0, 40.0, 50.0, 75.0, 100.0];
                            egui::ComboBox::from_id_salt("dekigata_scale_desktop")
                                .selected_text(format!("1:{}", self.dekigata_scale as u32))
                                .show_ui(ui, |ui| {
                                    for &opt in &dekigata_scale_options {
                                        if ui.selectable_value(&mut self.dekigata_scale, opt, format!("1:{}", opt as u32)).changed() {
                                            self.update_dxf_preview();
                                        }
                                    }
                                });
                        }
                        // デバッグガイド表示切替
                        if self.plot_scale.is_some() {
                            if ui.checkbox(&mut self.show_debug_guides, "ガイド").changed() {
                                self.update_dxf_preview();
                            }
                        }
                        // 工事名非表示
                        if self.plot_scale.is_some() || self.view_mode == ViewMode::Dekigata {
                            if ui.checkbox(&mut self.hide_project_name, "工事名非表示").changed() {
                                self.update_dxf_preview();
                            }
                        }
                        // 奇数測点のみ（出来形モード時）
                        if self.view_mode == ViewMode::Dekigata {
                            if ui.checkbox(&mut self.odd_points_only, "奇数測点のみ").changed() {
                                self.update_dxf_preview();
                            }
                        }
                        // ページナビゲーション（複数ページある場合のみ）
                        if self.total_pages > 1 && self.plot_scale.is_some() {
                            ui.separator();
                            if ui.small_button("<").clicked() && self.current_page > 0 {
                                self.current_page -= 1;
                                self.update_dxf_preview();
                            }
                            ui.label(format!("{}/{}", self.current_page + 1, self.total_pages));
                            if ui.small_button(">").clicked() && self.current_page < self.total_pages - 1 {
                                self.current_page += 1;
                                self.update_dxf_preview();
                            }
                        }
                    });
                }

                // DXFダウンロード（フィルター済みセクションを使用）
                let filtered_for_download: Vec<CrossSectionData> = self.filtered_sections()
                    .into_iter().cloned().collect();

                match self.view_mode {
                    ViewMode::AlignTest => {
                        if ui.button("Download Test DXF").clicked() {
                            let dxf_content = generate_alignment_test_dxf_bytes();
                            download_file("alignment_test.dxf", &dxf_content);
                        }
                    }
                    ViewMode::TitleBlock => {
                        if ui.button("Download Title Block DXF").clicked() {
                            let dxf_content = generate_title_block_dxf_for_download();
                            download_file("title_block.dxf", &dxf_content);
                        }
                    }
                    _ if filtered_for_download.is_empty() => {}
                    ViewMode::Combo => {
                        if ui.button("Download Combo DXF").clicked() {
                            let dxf_content = generate_combo_dxf_bytes(&filtered_for_download, self.grid_columns, self.column_gap);
                            download_file("combo.dxf", &dxf_content);
                        }
                    }
                    ViewMode::AllGrid => {
                        if ui.button("Download All DXF").clicked() {
                            let (dxf_content, filename) = if let Some(scale) = self.plot_scale {
                                // 全ページを1つのDXFに垂直配置
                                let display_project_name = if self.hide_project_name { "" } else { &self.project_name };
                                let info = TitleBlockInfo::new()
                                    .with_project_name(display_project_name)
                                    .with_drawing_type(&self.drawing_type)
                                    .with_route_name(&self.route_name)
                                    .with_author(&self.author)
                                    .with_date(&self.date)
                                    .with_top_title(&self.drawing_type)
                                    .with_scale(&format!("1:{} (A3)", scale))
                                    .with_debug_markers(false);
                                (generate_all_pages_dxf_bytes(&filtered_for_download, self.grid_columns, self.column_gap, self.row_gap, scale as f64, &info, &self.drawing_number, self.v_scale_ratio), "cross_sections.dxf".to_string())
                            } else {
                                (generate_multi_dxf_bytes(&filtered_for_download, self.grid_columns, self.column_gap, self.row_gap), "cross_sections_all.dxf".to_string())
                            };
                            download_file(&filename, &dxf_content);
                        }
                    }
                    ViewMode::Longitudinal => {
                        if ui.button("Download 縦断 DXF").clicked() {
                            let dxf_content = generate_longitudinal_dxf_bytes(&filtered_for_download);
                            download_file("longitudinal.dxf", &dxf_content);
                        }
                    }
                    ViewMode::AreaExpansion => {
                        if ui.button("Download 展開図 DXF").clicked() {
                            let dxf_content = generate_area_expansion_dxf_bytes(&filtered_for_download);
                            download_file("area_expansion.dxf", &dxf_content);
                        }
                    }
                    ViewMode::Dekigata => {
                        // 整数測点のみフィルタ（中間測点は除外）
                        let dekigata_sections: Vec<CrossSectionData> = self.dekigata_filtered_sections()
                            .into_iter().cloned().collect();
                        if !dekigata_sections.is_empty() {
                            let btn_text = if is_ezdxf_ready() { "Download 出来形 DXF+Layout" } else { "Download 出来形 DXF" };
                            if ui.button(btn_text).clicked() {
                                let display_project_name = if self.hide_project_name { "" } else { &self.project_name };
                                let info = TitleBlockInfo::new()
                                    .with_project_name(display_project_name)
                                    .with_drawing_type("出来形管理用紙")
                                    .with_route_name(&self.route_name)
                                    .with_author(&self.author)
                                    .with_date(&self.date)
                                    .with_top_title("出来形管理用紙")
                                    .with_scale(&format!("1:{} (A3)", self.dekigata_scale as u32))
                                    .with_debug_markers(false);
                                let dxf_content = generate_all_dekigata_dxf_bytes(&dekigata_sections, &info, &self.drawing_number, self.v_scale_ratio, self.dekigata_scale);

                                if is_ezdxf_ready() {
                                    let page_count = dekigata_sections.len() as u32;
                                    add_layouts_to_dxf_async(dxf_content, self.dekigata_scale, page_count);
                                } else {
                                    download_file("dekigata.dxf", &dxf_content);
                                }
                            }
                        }
                    }
                    ViewMode::Single => {
                        if let Some(idx) = self.selected_index {
                            if let Some(section) = filtered_for_download.get(idx) {
                                if ui.button("Download DXF").clicked() {
                                    // 起終点以外は補間を適用
                                    let is_start_or_end = section.is_route_start || section.is_route_end;
                                    let mut section = section.clone();
                                    if !is_start_or_end {
                                        section.interpolate_planned_heights();
                                    }
                                    let dxf_content = generate_dxf_bytes(&section);
                                    let filename = format!("{}.dxf", section.survey_point_name);
                                    download_file(&filename, &dxf_content);
                                }
                            }
                        }
                    }
                }
                ui.separator();

                // 測点リスト（全モードで表示、選択時は単一モードに切替）
                // 借用の問題を避けるため、表示に必要なデータを先にコピー
                // 出来形モードでは整数測点のみ表示
                let filtered_names: Vec<String> = if self.view_mode == ViewMode::Dekigata {
                    self.dekigata_filtered_sections()
                        .iter().map(|s| s.survey_point_name.clone()).collect()
                } else {
                    self.filtered_sections()
                        .iter().map(|s| s.survey_point_name.clone()).collect()
                };
                let filtered_count = filtered_names.len();

                // モード別の情報表示
                match self.view_mode {
                    ViewMode::Single => {
                        ui.label(format!("単一横断 ({}測点)", filtered_count));
                    }
                    ViewMode::AllGrid => {
                        ui.label(format!("全横断グリッド ({}測点)", filtered_count));
                    }
                    ViewMode::Combo => {
                        ui.label(format!("縦断+全横断 ({}測点)", filtered_count));
                    }
                    ViewMode::Longitudinal => {
                        ui.label(format!("縦断図 ({}測点)", filtered_count));
                    }
                    ViewMode::AreaExpansion => {
                        ui.label(format!("面積展開図 ({}測点)", filtered_count));
                    }
                    ViewMode::Dekigata => {
                        let dekigata_count = self.dekigata_filtered_sections().len();
                        ui.label(format!("出来形管理用紙 ({}測点/整数のみ)", dekigata_count));
                    }
                    ViewMode::AlignTest => {
                        ui.label("アライメントテスト");
                    }
                    ViewMode::TitleBlock => {
                        ui.label("タイトル枠テスト");
                    }
                }

                ui.label("Stations:");
                let mut new_selection = None;
                egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                    for (i, name) in filtered_names.iter().enumerate() {
                        let selected = self.selected_index == Some(i);
                        if ui.selectable_label(selected, name).clicked() {
                            new_selection = Some(i);
                        }
                    }
                });
                if let Some(idx) = new_selection {
                    self.selected_index = Some(idx);
                    // 出来形モード時はモード維持
                    if self.view_mode != ViewMode::Dekigata {
                        self.view_mode = ViewMode::Single;
                    }
                    self.update_dxf_preview();
                }

                // 単一モード時のみ詳細情報を表示
                if self.view_mode == ViewMode::Single {
                    ui.separator();
                    let filtered_list: Vec<_> = self.filtered_sections();
                    if let Some(idx) = self.selected_index {
                        if let Some(section) = filtered_list.get(idx) {
                            ui.label(format!("DL: {:.3}", section.dl));
                            ui.label(format!("L->CL: {:.2}m", section.l_to_cl_distance));
                        }
                    }
                }

                ui.separator();
                ui.label("Legend:");
                ui.horizontal(|ui| {
                    ui.colored_label(Color32::BLACK, "=");
                    ui.label("Ground(GH)");
                });
                ui.horizontal(|ui| {
                    ui.colored_label(Color32::from_rgb(255, 0, 0), "=");
                    ui.label("Planned(FH)");
                });
                ui.horizontal(|ui| {
                    ui.colored_label(Color32::from_rgb(0, 0, 255), "=");
                    ui.label("Cutting");
                });
                ui.horizontal(|ui| {
                    ui.colored_label(Color32::from_rgb(128, 128, 128), "=");
                    ui.label("Dimension");
                });
            });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let available = ui.available_size();
            let (response, painter) = ui.allocate_painter(available, egui::Sense::click_and_drag());

            // 白背景
            painter.rect_filled(response.rect, 0.0, Color32::from_rgb(250, 250, 250));
            self.dxf_view_state.canvas_rect = response.rect;

            // canvas_rect更新後にfit_to_dxfを実行
            if self.needs_fit {
                // Singleモードでfit_boundsがある場合は選択した横断図にフィット
                if let Some((min_x, min_y, max_x, max_y)) = self.fit_bounds {
                    self.dxf_view_state.fit_to_dxf(min_x, min_y, max_x, max_y);
                } else if let Some(ref drawing) = self.dxf_drawing {
                    // それ以外は全体にフィット
                    let (min_x, min_y, max_x, max_y) = calc_dxf_bounds(drawing);
                    self.dxf_view_state.fit_to_dxf(min_x, min_y, max_x, max_y);
                }
                self.needs_fit = false;
            }

            if response.dragged() {
                self.dxf_view_state.pan += response.drag_delta();
            }

            // キャンバス上にホバー時のみズーム処理
            if response.hovered() {
                // マウスホイールズーム（マウス位置中心）
                let scroll = ui.input(|i| i.raw_scroll_delta.y);
                if scroll != 0.0 {
                    if let Some(mouse_pos) = response.hover_pos() {
                        let zoom_factor = if scroll > 0.0 { 1.1 } else { 0.9 };
                        let local = mouse_pos - self.dxf_view_state.canvas_rect.min;

                        // pan調整: マウス位置のDXF座標がズーム後も同じスクリーン位置に留まる
                        self.dxf_view_state.pan = local * (1.0 - zoom_factor)
                                                + self.dxf_view_state.pan * zoom_factor;
                        self.dxf_view_state.zoom *= zoom_factor;
                        self.dxf_view_state.zoom = self.dxf_view_state.zoom.clamp(0.001, 50.0);
                    }
                }

                // ピンチズーム（画面中心）
                let zoom_delta = ui.input(|i| i.zoom_delta());
                if zoom_delta != 1.0 {
                    let center = self.dxf_view_state.canvas_rect.center();
                    let local = center - self.dxf_view_state.canvas_rect.min;

                    self.dxf_view_state.pan = local * (1.0 - zoom_delta)
                                            + self.dxf_view_state.pan * zoom_delta;
                    self.dxf_view_state.zoom *= zoom_delta;
                    self.dxf_view_state.zoom = self.dxf_view_state.zoom.clamp(0.001, 50.0);
                }
            }

            if let Some(ref drawing) = self.dxf_drawing {
                let content = DrawingContent::from_drawing(drawing);
                let view: IrViewState = (&self.dxf_view_state).into();
                content.render_egui(&painter, &view);

                // ズームセンター（マウス位置）にレティクル描画
                if let Some(mouse_pos) = response.hover_pos() {
                    let reticle_size = 15.0;
                    let reticle_color = Color32::from_rgba_unmultiplied(255, 0, 0, 180);
                    // 横線
                    painter.line_segment(
                        [Pos2::new(mouse_pos.x - reticle_size, mouse_pos.y),
                         Pos2::new(mouse_pos.x + reticle_size, mouse_pos.y)],
                        egui::Stroke::new(1.5, reticle_color)
                    );
                    // 縦線
                    painter.line_segment(
                        [Pos2::new(mouse_pos.x, mouse_pos.y - reticle_size),
                         Pos2::new(mouse_pos.x, mouse_pos.y + reticle_size)],
                        egui::Stroke::new(1.5, reticle_color)
                    );
                    // 中心円
                    painter.circle_stroke(mouse_pos, 5.0, egui::Stroke::new(1.5, reticle_color));
                }
            } else {
                // ローディングアニメーション
                self.loading_frame += 1;
                let dots = match (self.loading_frame / 15) % 4 {
                    0 => "",
                    1 => ".",
                    2 => "..",
                    _ => "...",
                };
                let spinner = match (self.loading_frame / 5) % 4 {
                    0 => "◐",
                    1 => "◓",
                    2 => "◑",
                    _ => "◒",
                };
                let loading_text = format!("{} {} {}", spinner, self.loading_stage, dots);
                painter.text(
                    response.rect.center(),
                    egui::Align2::CENTER_CENTER,
                    &loading_text,
                    egui::FontId::proportional(18.0),
                    Color32::from_rgb(100, 100, 100)
                );
                // 再描画をリクエスト（アニメーション継続）
                ctx.request_repaint();
            }
        });
    }
}
