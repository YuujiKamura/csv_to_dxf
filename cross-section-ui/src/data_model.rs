//! データモデル定義
//!
//! 横断図の測量データ構造体とCSV/JSONパーサー

use serde::{Deserialize, Serialize};

use crate::drawing_ir::{SectionData, SectionPoint};

// ============================================================================
// Data Structures
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SurveyRow {
    pub unit_distance: f64,
    pub elevation: f64,
    pub planned_height: f64,
    pub cumulative_distance: f64,
    pub cutting_bottom: f64,
}

impl SurveyRow {
    pub fn cutting_depth(&self) -> f64 { self.elevation - self.cutting_bottom }
    pub fn pavement_thickness(&self) -> f64 { self.planned_height - self.cutting_bottom }
}

// Implement SectionPoint trait for SurveyRow
impl SectionPoint for SurveyRow {
    fn elevation(&self) -> f64 {
        self.elevation
    }

    fn planned_height(&self) -> f64 {
        self.planned_height
    }

    fn cumulative_distance(&self) -> f64 {
        self.cumulative_distance
    }

    fn cutting_bottom(&self) -> f64 {
        self.cutting_bottom
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CsvSection {
    pub name: String,
    pub unit_distances: Vec<f64>,
    pub elevations: Vec<f64>,
    pub planned_heights: Vec<f64>,
    pub cutting_depth: f64,
    pub cl_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrossSectionData {
    pub survey_point_name: String,
    pub dl: f64,
    pub cl_index: usize,
    pub l_to_cl_distance: f64,
    pub survey_data: Vec<SurveyRow>,
    /// 路線距離（m）- 測点の絶対位置。指定されていればparse_station_distanceより優先
    pub route_distance: Option<f64>,
    /// ルートID（route_1, route_2など）
    #[serde(default = "default_route_id")]
    pub route_id: String,
    /// 路線の起点かどうか（補間しない）
    #[serde(default)]
    pub is_route_start: bool,
    /// 路線の終点かどうか（補間しない）
    #[serde(default)]
    pub is_route_end: bool,
}

pub fn default_route_id() -> String {
    "route_1".to_string()
}

impl CrossSectionData {
    pub fn calc_cumulative_distances(unit_distances: &[f64], cl_index: usize) -> Vec<f64> {
        let mut cumulative = Vec::with_capacity(unit_distances.len());
        let mut sum = 0.0;
        for (i, &d) in unit_distances.iter().enumerate() {
            if i == 0 { cumulative.push(0.0); }
            else { sum += d; cumulative.push(sum); }
        }
        let cl_offset = cumulative[cl_index];
        cumulative.iter().map(|&c| c - cl_offset).collect()
    }

    #[allow(dead_code)]
    pub fn from_3point(
        name: &str, w_l: f64, w_r: f64,
        gh_l: f64, gh_cl: f64, gh_r: f64,
        fh_l: f64, fh_cl: f64, fh_r: f64,
        dl: f64, cutting_depth: f64,
    ) -> Self {
        let unit_distances = vec![0.0, w_l, w_r];
        let cl_index = 1;
        let elevations = vec![gh_l, gh_cl, gh_r];
        let planned_heights = vec![fh_l, fh_cl, fh_r];
        let cumulative = Self::calc_cumulative_distances(&unit_distances, cl_index);
        let cutting_bottoms: Vec<f64> = planned_heights.iter().map(|&fh| fh - cutting_depth).collect();
        let l_to_cl = cumulative[0].abs();

        let survey_data: Vec<SurveyRow> = (0..unit_distances.len()).map(|i| SurveyRow {
            unit_distance: unit_distances[i],
            elevation: elevations[i],
            planned_height: planned_heights[i],
            cumulative_distance: cumulative[i],
            cutting_bottom: cutting_bottoms[i],
        }).collect();

        CrossSectionData {
            survey_point_name: name.to_string(),
            dl, cl_index, l_to_cl_distance: l_to_cl, survey_data,
            route_distance: None, // 測点名からパースされる
            route_id: default_route_id(),
            is_route_start: false,
            is_route_end: false,
        }
    }


    pub fn from_json(json: &str) -> Result<Vec<Self>, String> {
        let mut sections: Vec<Self> = serde_json::from_str(json)
            .map_err(|e| format!("JSON parse error: {e}"))?;
        Self::set_route_start_end_flags(&mut sections);
        Ok(sections)
    }

    /// 新形式JSON（タイトルブロック情報付き）をパース
    pub fn from_json_with_title(json: &str) -> Result<SectionsData, String> {
        // まず新形式（オブジェクト）でパースを試みる
        if let Ok(mut data) = serde_json::from_str::<SectionsData>(json) {
            Self::set_route_start_end_flags(&mut data.sections);
            return Ok(data);
        }
        // 旧形式（配列）でパース
        let mut sections: Vec<Self> = serde_json::from_str(json)
            .map_err(|e| format!("JSON parse error: {e}"))?;
        Self::set_route_start_end_flags(&mut sections);
        Ok(SectionsData {
            title_block: None,
            sections,
        })
    }

    /// 各ルートごとに起終点フラグを設定
    fn set_route_start_end_flags(sections: &mut [Self]) {
        use std::collections::HashMap;

        // 各ルートの最初と最後のインデックスを収集
        let mut route_indices: HashMap<String, (usize, usize)> = HashMap::new();
        for (i, section) in sections.iter().enumerate() {
            let route_id = &section.route_id;
            route_indices
                .entry(route_id.clone())
                .and_modify(|(_, last)| *last = i)
                .or_insert((i, i));
        }

        // フラグを設定
        for (first, last) in route_indices.values() {
            sections[*first].is_route_start = true;
            sections[*last].is_route_end = true;
        }
    }

    /// 補完点の計画高を採用点間の勾配から線形補間する
    /// - 採用点: V1（左端）、CL（センター）、Vn（右端）は固定
    /// - 補完点: V2, V4等を勾配で補間
    pub fn interpolate_planned_heights(&mut self) {
        let n = self.survey_data.len();
        if n < 3 { return; }

        let cl = self.cl_index.min(n - 1);

        // 固定点の値を取得
        let fh_left = self.survey_data[0].planned_height;
        let fh_center = self.survey_data[cl].planned_height;
        let fh_right = self.survey_data[n - 1].planned_height;

        let dist_left = self.survey_data[0].cumulative_distance;
        let dist_center = self.survey_data[cl].cumulative_distance;
        let dist_right = self.survey_data[n - 1].cumulative_distance;

        // 左側（V1〜CL間）の補間
        if (dist_center - dist_left).abs() > 1e-9 {
            for i in 1..cl {
                // 舗装厚を保持（planned_height更新前に取得）
                let pavement_thickness = self.survey_data[i].pavement_thickness();

                let d = self.survey_data[i].cumulative_distance;
                let t = (d - dist_left) / (dist_center - dist_left);
                self.survey_data[i].planned_height = fh_left + t * (fh_center - fh_left);
                // cutting_bottomも再計算（舗装厚を維持）
                self.survey_data[i].cutting_bottom = self.survey_data[i].planned_height - pavement_thickness;
            }
        }

        // 右側（CL〜Vn間）の補間
        if (dist_right - dist_center).abs() > 1e-9 {
            for i in (cl + 1)..(n - 1) {
                // 舗装厚を保持（planned_height更新前に取得）
                let pavement_thickness = self.survey_data[i].pavement_thickness();

                let d = self.survey_data[i].cumulative_distance;
                let t = (d - dist_center) / (dist_right - dist_center);
                self.survey_data[i].planned_height = fh_center + t * (fh_right - fh_center);
                // cutting_bottomも再計算（舗装厚を維持）
                self.survey_data[i].cutting_bottom = self.survey_data[i].planned_height - pavement_thickness;
            }
        }
    }

    pub fn from_csv_section(section: &CsvSection) -> Result<Self, String> {
        let count = section.unit_distances.len();
        if count == 0 {
            return Err(format!("{}: no rows", section.name));
        }
        if section.elevations.len() != count || section.planned_heights.len() != count {
            return Err(format!("{}: column length mismatch", section.name));
        }
        let cl_index = section.cl_index.unwrap_or(0).min(count.saturating_sub(1));
        let cumulative = Self::calc_cumulative_distances(&section.unit_distances, cl_index);
        let cutting_bottoms: Vec<f64> = section
            .planned_heights
            .iter()
            .map(|&fh| fh - section.cutting_depth)
            .collect();
        let l_to_cl = cumulative[0].abs();
        let dl = section
            .elevations
            .iter()
            .cloned()
            .fold(f64::INFINITY, f64::min)
            .floor()
            - 1.0;

        let survey_data: Vec<SurveyRow> = (0..count)
            .map(|i| SurveyRow {
                unit_distance: section.unit_distances[i],
                elevation: section.elevations[i],
                planned_height: section.planned_heights[i],
                cumulative_distance: cumulative[i],
                cutting_bottom: cutting_bottoms[i],
            })
            .collect();

        Ok(CrossSectionData {
            survey_point_name: section.name.clone(),
            dl,
            cl_index,
            l_to_cl_distance: l_to_cl,
            survey_data,
            route_distance: None,
            route_id: default_route_id(),
            is_route_start: false,
            is_route_end: false,
        })
    }
}

/// タイトルブロック情報（JSON用）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TitleBlockJson {
    #[serde(default)]
    pub project_name: String,
    #[serde(default)]
    pub drawing_type: String,
    #[serde(default)]
    pub route_name: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub drawing_number: String,
}

/// セクションデータ（タイトルブロック情報付き）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionsData {
    #[serde(default)]
    pub title_block: Option<TitleBlockJson>,
    pub sections: Vec<CrossSectionData>,
}

// ============================================================================
// SectionData Trait Implementation
// ============================================================================

impl SectionData for CrossSectionData {
    type Point = SurveyRow;

    fn survey_points(&self) -> &[Self::Point] {
        &self.survey_data
    }

    fn datum_level(&self) -> f64 {
        self.dl
    }

    fn cl_index(&self) -> usize {
        self.cl_index
    }

    fn name(&self) -> &str {
        &self.survey_point_name
    }
}

// ============================================================================
// CSV Loaders (wasm only)
// ============================================================================

#[cfg(target_arch = "wasm32")]
pub fn parse_csv_sections(content: &str) -> Result<Vec<CsvSection>, String> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(content.as_bytes());
    let headers = reader
        .headers()
        .map_err(|e| format!("CSV header error: {e}"))?
        .clone();

    let get_idx = |name: &str| -> Option<usize> { headers.iter().position(|h| h.trim() == name) };

    let name_idx = get_idx("name")
        .or_else(|| get_idx("測点名"))
        .ok_or_else(|| "missing column: name/測点名".to_string())?;
    let unit_idx = get_idx("unit_distance")
        .or_else(|| get_idx("単延長"))
        .ok_or_else(|| "missing column: unit_distance/単延長".to_string())?;
    let elev_idx = get_idx("elevation")
        .or_else(|| get_idx("現地盤"))
        .ok_or_else(|| "missing column: elevation/現地盤".to_string())?;
    let plan_idx = get_idx("planned_height")
        .or_else(|| get_idx("計画高"))
        .ok_or_else(|| "missing column: planned_height/計画高".to_string())?;
    let cut_idx = get_idx("cutting_depth")
        .or_else(|| get_idx("切削厚"))
        .ok_or_else(|| "missing column: cutting_depth/切削厚".to_string())?;

    let section_idx = get_idx("section").or_else(|| get_idx("区間"));
    let cl_flag_idx = get_idx("cl")
        .or_else(|| get_idx("CL"))
        .or_else(|| get_idx("center"))
        .or_else(|| get_idx("中央"));
    let cl_index_idx = get_idx("cl_index").or_else(|| get_idx("CL_index"));
    let mut sections: Vec<CsvSection> = Vec::new();
    let mut current_name = String::new();
    let mut unit_distances = Vec::new();
    let mut elevations = Vec::new();
    let mut planned_heights = Vec::new();
    let mut cutting_depth: Option<f64> = None;
    let mut cl_index: Option<usize> = None;

    let mut flush = |name: &str,
                     unit_distances: &mut Vec<f64>,
                     elevations: &mut Vec<f64>,
                     planned_heights: &mut Vec<f64>,
                     cutting_depth: &mut Option<f64>,
                     cl_index: &mut Option<usize>,
                     out: &mut Vec<CsvSection>| {
        if unit_distances.is_empty() {
            return;
        }
        let cut = cutting_depth.unwrap_or(0.0);
        out.push(CsvSection {
            name: name.to_string(),
            unit_distances: std::mem::take(unit_distances),
            elevations: std::mem::take(elevations),
            planned_heights: std::mem::take(planned_heights),
            cutting_depth: cut,
            cl_index: *cl_index,
        });
        *cutting_depth = None;
        *cl_index = None;
    };

    for record in reader.records() {
        let record = record.map_err(|e| format!("CSV row error: {e}"))?;
        if record.iter().all(|v| v.trim().is_empty()) {
            continue;
        }

        let section_name = section_idx
            .and_then(|idx| record.get(idx))
            .map(|v| v.trim())
            .filter(|v| !v.is_empty());
        if let Some(section_name) = section_name {
            if current_name.is_empty() {
                current_name = section_name.to_string();
            } else if section_name != current_name {
                flush(
                    &current_name,
                    &mut unit_distances,
                    &mut elevations,
                    &mut planned_heights,
                    &mut cutting_depth,
                    &mut cl_index,
                    &mut sections,
                );
                current_name = section_name.to_string();
            }
        }

        let name = record.get(name_idx).unwrap_or("未設定").trim();
        if current_name.is_empty() {
            current_name = name.to_string();
        }

        unit_distances.push(parse_cell(record.get(unit_idx))?);
        elevations.push(parse_cell(record.get(elev_idx))?);
        planned_heights.push(parse_cell(record.get(plan_idx))?);
        let cut_value = parse_cell(record.get(cut_idx))?;
        cutting_depth = Some(cutting_depth.map_or(cut_value, |v| v.max(cut_value)));
        if cl_index.is_none() {
            if let Some(idx) = cl_index_idx.and_then(|idx| record.get(idx)) {
                if !idx.trim().is_empty() {
                    cl_index = idx.trim().parse::<usize>().ok();
                }
            } else if let Some(flag) = cl_flag_idx.and_then(|idx| record.get(idx)) {
                let flag = flag.trim();
                if matches!(flag, "1" | "true" | "TRUE" | "yes" | "YES" | "y" | "Y" | "中央") {
                    cl_index = Some(unit_distances.len().saturating_sub(1));
                }
            }
        }
    }

    if !current_name.is_empty() {
        flush(
            &current_name,
            &mut unit_distances,
            &mut elevations,
            &mut planned_heights,
            &mut cutting_depth,
            &mut cl_index,
            &mut sections,
        );
    }

    if sections.is_empty() {
        return Err("no valid sections in CSV".to_string());
    }
    Ok(sections)
}

#[cfg(target_arch = "wasm32")]
pub fn parse_cell(value: Option<&str>) -> Result<f64, String> {
    let raw = value.unwrap_or("").trim();
    if raw.is_empty() {
        return Err("empty numeric cell".to_string());
    }
    raw.parse::<f64>()
        .map_err(|_| format!("invalid number: {raw}"))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn parse_csv_sections(_content: &str) -> Result<Vec<CsvSection>, String> {
    Err("CSV loader is only available in wasm builds".to_string())
}
