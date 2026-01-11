use yew::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, CanvasRenderingContext2d};

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrossSectionData {
    pub survey_point_name: String,
    pub dl: f64,
    pub cl_index: usize,
    pub l_to_cl_distance: f64,
    pub survey_data: Vec<SurveyRow>,
}

impl CrossSectionData {
    fn rotate<T: Clone>(arr: &[T], k: usize) -> Vec<T> {
        if arr.is_empty() { return vec![]; }
        let n = arr.len();
        let k = k % n;
        let mut result = arr[k..].to_vec();
        result.extend_from_slice(&arr[..k]);
        result
    }
    fn shift_unit_distances(distances: &[f64], cl_index: usize) -> Vec<f64> { Self::rotate(distances, cl_index) }
    fn calc_cumulative_distances(unit_distances: &[f64], cl_index: usize) -> Vec<f64> {
        let shifted = Self::shift_unit_distances(unit_distances, cl_index);
        let mut cumulative = vec![0.0];
        for d in shifted.iter().skip(1) { cumulative.push(cumulative.last().unwrap() + d); }
        let l_to_cl: f64 = unit_distances[..cl_index].iter().sum();
        cumulative.iter().map(|c| c - l_to_cl).collect()
    }
    fn compute_dl(elevations: &[f64]) -> f64 {
        elevations.iter().cloned().min_by(|a, b| a.partial_cmp(b).unwrap()).map(|min| min.floor() - 1.0).unwrap_or(0.0)
    }
    fn compute_l_to_cl(unit_distances: &[f64], cl_index: usize) -> f64 { unit_distances[..cl_index].iter().sum() }
    fn sample_no0() -> Self {
        let unit_distances = vec![0.0, 1.6, 1.6, 1.6, 1.6];
        let elevations = vec![10.45, 10.43, 10.42, 10.44, 10.48];
        let planned_heights = vec![10.50, 10.50, 10.50, 10.50, 10.50];
        let pavement_thickness = 0.10;
        let cl_index = 2;
        let cumulative = Self::calc_cumulative_distances(&unit_distances, cl_index);
        let dl = Self::compute_dl(&elevations);
        let l_to_cl = Self::compute_l_to_cl(&unit_distances, cl_index);
        let survey_data: Vec<SurveyRow> = (0..unit_distances.len()).map(|i| SurveyRow {
            unit_distance: unit_distances[i], elevation: elevations[i], planned_height: planned_heights[i],
            cumulative_distance: cumulative[i], cutting_bottom: planned_heights[i] - pavement_thickness,
        }).collect();
        CrossSectionData { survey_point_name: "No.0".to_string(), dl, cl_index, l_to_cl_distance: l_to_cl, survey_data }
    }
    fn sample_no1() -> Self {
        let unit_distances = vec![0.0, 1.5, 1.5, 1.5, 1.5];
        let elevations = vec![10.52, 10.51, 10.50, 10.52, 10.53];
        let planned_heights = vec![10.55, 10.55, 10.55, 10.55, 10.55];
        let pavement_thickness = 0.10;
        let cl_index = 2;
        let cumulative = Self::calc_cumulative_distances(&unit_distances, cl_index);
        let dl = Self::compute_dl(&elevations);
        let l_to_cl = Self::compute_l_to_cl(&unit_distances, cl_index);
        let survey_data: Vec<SurveyRow> = (0..unit_distances.len()).map(|i| SurveyRow {
            unit_distance: unit_distances[i], elevation: elevations[i], planned_height: planned_heights[i],
            cumulative_distance: cumulative[i], cutting_bottom: planned_heights[i] - pavement_thickness,
        }).collect();
        CrossSectionData { survey_point_name: "No.1".to_string(), dl, cl_index, l_to_cl_distance: l_to_cl, survey_data }
    }
}

enum Msg { SelectStation(usize), LoadSample }
struct App { sections: Vec<CrossSectionData>, selected_index: Option<usize> }

impl Component for App {
    type Message = Msg;
    type Properties = ();
    fn create(_ctx: &Context<Self>) -> Self { Self { sections: vec![], selected_index: None } }
    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::SelectStation(i) => { self.selected_index = Some(i); true }
            Msg::LoadSample => {
                self.sections = vec![CrossSectionData::sample_no0(), CrossSectionData::sample_no1()];
                self.selected_index = Some(0); true
            }
        }
    }
    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_load = ctx.link().callback(|_| Msg::LoadSample);
        html! {
            <div class="app-container">
                <header><h1>{"横断図・切削計算システム"}</h1></header>
                <div class="main-content">
                    <aside class="sidebar">
                        <button class="btn btn-primary" onclick={on_load}>{"サンプル読込"}</button>
                        <div class="station-list">
                        { for self.sections.iter().enumerate().map(|(i, s)| {
                            let selected = self.selected_index == Some(i);
                            let onclick = ctx.link().callback(move |_| Msg::SelectStation(i));
                            html! { <div class={classes!("station-item", selected.then_some("selected"))} onclick={onclick}>{ &s.survey_point_name }</div> }
                        })}
                        </div>
                        { self.view_info() }
                    </aside>
                    <main class="canvas-area">
                        { self.view_canvas() }
                        <div class="legend">
                            <div class="legend-item"><div class="legend-color surface"></div><span>{"表層天端"}</span></div>
                            <div class="legend-item"><div class="legend-color existing"></div><span>{"現地盤"}</span></div>
                            <div class="legend-item"><div class="legend-color cutting"></div><span>{"切削底面"}</span></div>
                        </div>
                        { self.view_table() }
                    </main>
                </div>
            </div>
        }
    }
}

impl App {
    fn view_info(&self) -> Html {
        match self.selected_index.and_then(|i| self.sections.get(i)) {
            Some(s) => html! {
                <div class="info-panel">
                    <p><strong>{"DL: "}</strong>{ format!("{:.2}", s.dl) }</p>
                    <p><strong>{"CLIndex: "}</strong>{ s.cl_index }</p>
                    <p><strong>{"L->CL: "}</strong>{ format!("{:.2}m", s.l_to_cl_distance) }</p>
                </div>
            },
            None => html! {},
        }
    }
    fn view_canvas(&self) -> Html {
        match self.selected_index.and_then(|i| self.sections.get(i)) {
            Some(s) => html! { <CrossSectionCanvas section={s.clone()} /> },
            None => html! { <p>{"測点を選択してください"}</p> },
        }
    }
    fn view_table(&self) -> Html {
        match self.selected_index.and_then(|i| self.sections.get(i)) {
            Some(s) => html! {
                <table class="calculation-table">
                    <thead><tr><th>{"累積距離"}</th><th>{"現地盤高"}</th><th>{"計画高"}</th><th>{"切削底面"}</th><th>{"切削厚(cm)"}</th></tr></thead>
                    <tbody>{ for s.survey_data.iter().map(|p| html! {
                        <tr>
                            <td>{ format!("{:.2}", p.cumulative_distance) }</td>
                            <td>{ format!("{:.3}", p.elevation) }</td>
                            <td>{ format!("{:.3}", p.planned_height) }</td>
                            <td>{ format!("{:.3}", p.cutting_bottom) }</td>
                            <td>{ format!("{:.1}", p.cutting_depth() * 100.0) }</td>
                        </tr>
                    })}</tbody>
                </table>
            },
            None => html! {},
        }
    }
}

#[derive(Properties, PartialEq)]
struct CanvasProps { section: CrossSectionData }

#[function_component(CrossSectionCanvas)]
fn cross_section_canvas(props: &CanvasProps) -> Html {
    let canvas_ref = use_node_ref();
    let section = props.section.clone();
    { let cr = canvas_ref.clone(); use_effect_with(section.clone(), move |s| { if let Some(c) = cr.cast::<HtmlCanvasElement>() { draw_cross_section(&c, s); } || () }); }
    html! { <canvas ref={canvas_ref} width="800" height="400" /> }
}

fn draw_cross_section(canvas: &HtmlCanvasElement, section: &CrossSectionData) {
    let ctx: CanvasRenderingContext2d = canvas.get_context("2d").unwrap().unwrap().dyn_into().unwrap();
    let (w, h) = (canvas.width() as f64, canvas.height() as f64);
    ctx.set_fill_style_str("#fff"); ctx.fill_rect(0.0, 0.0, w, h);
    if section.survey_data.is_empty() { return; }
    let xs: Vec<f64> = section.survey_data.iter().map(|p| p.cumulative_distance).collect();
    let ys: Vec<f64> = section.survey_data.iter().flat_map(|p| [p.elevation, p.planned_height, p.cutting_bottom]).collect();
    let (min_x, max_x) = (*xs.iter().min_by(|a,b| a.partial_cmp(b).unwrap()).unwrap(), *xs.iter().max_by(|a,b| a.partial_cmp(b).unwrap()).unwrap());
    let (min_y, max_y) = (*ys.iter().min_by(|a,b| a.partial_cmp(b).unwrap()).unwrap(), *ys.iter().max_by(|a,b| a.partial_cmp(b).unwrap()).unwrap());
    let pad = 60.0;
    let sx = (w - pad*2.0) / (max_x - min_x).max(0.1);
    let sy = (h - pad*2.0) / (max_y - min_y).max(0.01);
    let tx = |x: f64| pad + (x - min_x) * sx;
    let ty = |y: f64| h - pad - (y - min_y) * sy;
    ctx.set_stroke_style_str("#888"); ctx.set_line_width(1.0);
    ctx.set_line_dash(&js_sys::Array::from_iter([5.0, 5.0].iter().map(|&v| wasm_bindgen::JsValue::from_f64(v)))).unwrap();
    ctx.begin_path(); let cl_x = tx(0.0); ctx.move_to(cl_x, pad); ctx.line_to(cl_x, h - pad); ctx.stroke();
    ctx.set_line_dash(&js_sys::Array::new()).unwrap();
    let dl = |pts: &[(f64,f64)], c: &str| { if pts.len()<2 {return;} ctx.set_stroke_style_str(c); ctx.set_line_width(2.5); ctx.begin_path(); ctx.move_to(tx(pts[0].0), ty(pts[0].1)); for (x,y) in pts.iter().skip(1) { ctx.line_to(tx(*x), ty(*y)); } ctx.stroke(); };
    let surf: Vec<_> = section.survey_data.iter().map(|p| (p.cumulative_distance, p.planned_height)).collect();
    let exis: Vec<_> = section.survey_data.iter().map(|p| (p.cumulative_distance, p.elevation)).collect();
    let cutt: Vec<_> = section.survey_data.iter().map(|p| (p.cumulative_distance, p.cutting_bottom)).collect();
    dl(&surf, "#e74c3c"); dl(&exis, "#3498db"); dl(&cutt, "#2ecc71");
    ctx.set_fill_style_str("#333"); ctx.set_font("14px sans-serif");
    let _ = ctx.fill_text(&section.survey_point_name, pad, 25.0);
    let _ = ctx.fill_text(&format!("DL={:.2}", section.dl), pad + 100.0, 25.0);
}

fn main() { yew::Renderer::<App>::new().render(); }
