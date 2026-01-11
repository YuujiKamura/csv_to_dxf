use yew::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, CanvasRenderingContext2d};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrossSectionPoint {
    pub distance: f64,
    pub existing_elevation: f64,
    pub surface_plan: f64,
    pub cutting_bottom: f64,
}

impl CrossSectionPoint {
    pub fn cutting_depth(&self) -> f64 { self.existing_elevation - self.cutting_bottom }
    pub fn pavement_thickness(&self) -> f64 { self.surface_plan - self.cutting_bottom }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrossSection {
    pub station_name: String,
    pub points: Vec<CrossSectionPoint>,
}

impl CrossSection {
    fn sample() -> Self {
        CrossSection {
            station_name: "No.0".to_string(),
            points: vec![
                CrossSectionPoint { distance: -3.2, existing_elevation: 10.45, surface_plan: 10.50, cutting_bottom: 10.40 },
                CrossSectionPoint { distance: -1.6, existing_elevation: 10.43, surface_plan: 10.50, cutting_bottom: 10.40 },
                CrossSectionPoint { distance: 0.0, existing_elevation: 10.42, surface_plan: 10.50, cutting_bottom: 10.40 },
                CrossSectionPoint { distance: 1.6, existing_elevation: 10.44, surface_plan: 10.50, cutting_bottom: 10.40 },
                CrossSectionPoint { distance: 3.2, existing_elevation: 10.48, surface_plan: 10.50, cutting_bottom: 10.40 },
            ],
        }
    }
    fn sample2() -> Self {
        CrossSection {
            station_name: "No.1".to_string(),
            points: vec![
                CrossSectionPoint { distance: -3.0, existing_elevation: 10.52, surface_plan: 10.55, cutting_bottom: 10.45 },
                CrossSectionPoint { distance: 0.0, existing_elevation: 10.50, surface_plan: 10.55, cutting_bottom: 10.45 },
                CrossSectionPoint { distance: 3.0, existing_elevation: 10.53, surface_plan: 10.55, cutting_bottom: 10.45 },
            ],
        }
    }
}

enum Msg { SelectStation(usize), LoadSample }

struct App { sections: Vec<CrossSection>, selected_index: Option<usize> }

impl Component for App {
    type Message = Msg;
    type Properties = ();
    fn create(_ctx: &Context<Self>) -> Self { Self { sections: vec![], selected_index: None } }
    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::SelectStation(i) => { self.selected_index = Some(i); true }
            Msg::LoadSample => { self.sections = vec![CrossSection::sample(), CrossSection::sample2()]; self.selected_index = Some(0); true }
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
                            let sel = self.selected_index == Some(i);
                            let oc = ctx.link().callback(move |_| Msg::SelectStation(i));
                            html! { <div class={classes!("station-item", sel.then_some("selected"))} onclick={oc}>{ &s.station_name }</div> }
                        })}
                        </div>
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
    fn view_canvas(&self) -> Html {
        match self.selected_index.and_then(|i| self.sections.get(i)) {
            Some(s) => html! { <CrossSectionCanvas section={s.clone()} /> },
            None => html! { <p>{"測点を選択"}</p> },
        }
    }
    fn view_table(&self) -> Html {
        match self.selected_index.and_then(|i| self.sections.get(i)) {
            Some(s) => html! {
                <table class="calculation-table">
                    <thead><tr><th>{"距離"}</th><th>{"現地盤高"}</th><th>{"切削厚(cm)"}</th></tr></thead>
                    <tbody>{ for s.points.iter().map(|p| html! {
                        <tr><td>{ format!("{:.2}", p.distance) }</td><td>{ format!("{:.3}", p.existing_elevation) }</td><td>{ format!("{:.1}", p.cutting_depth()*100.0) }</td></tr>
                    })}</tbody>
                </table>
            },
            None => html! {},
        }
    }
}

#[derive(Properties, PartialEq)]
struct CanvasProps { section: CrossSection }

#[function_component(CrossSectionCanvas)]
fn cross_section_canvas(props: &CanvasProps) -> Html {
    let canvas_ref = use_node_ref();
    let section = props.section.clone();
    { let cr = canvas_ref.clone(); use_effect_with(section.clone(), move |s| { if let Some(c) = cr.cast::<HtmlCanvasElement>() { draw(&c, s); } || () }); }
    html! { <canvas ref={canvas_ref} width="800" height="400" /> }
}

fn draw(canvas: &HtmlCanvasElement, section: &CrossSection) {
    let ctx: CanvasRenderingContext2d = canvas.get_context("2d").unwrap().unwrap().dyn_into().unwrap();
    let (w, h) = (canvas.width() as f64, canvas.height() as f64);
    ctx.set_fill_style_str("#fff"); ctx.fill_rect(0.0, 0.0, w, h);
    if section.points.is_empty() { return; }
    let xs: Vec<f64> = section.points.iter().map(|p| p.distance).collect();
    let ys: Vec<f64> = section.points.iter().flat_map(|p| [p.existing_elevation, p.surface_plan, p.cutting_bottom]).collect();
    let (min_x, max_x) = (*xs.iter().min_by(|a,b| a.partial_cmp(b).unwrap()).unwrap(), *xs.iter().max_by(|a,b| a.partial_cmp(b).unwrap()).unwrap());
    let (min_y, max_y) = (*ys.iter().min_by(|a,b| a.partial_cmp(b).unwrap()).unwrap(), *ys.iter().max_by(|a,b| a.partial_cmp(b).unwrap()).unwrap());
    let pad = 60.0;
    let sx = (w - pad*2.0) / (max_x - min_x).max(0.1);
    let sy = (h - pad*2.0) / (max_y - min_y).max(0.1);
    let tx = |x: f64| pad + (x - min_x) * sx;
    let ty = |y: f64| h - pad - (y - min_y) * sy;
    let dl = |pts: &[(f64,f64)], c: &str| { if pts.len()<2 {return;} ctx.set_stroke_style_str(c); ctx.set_line_width(2.5); ctx.begin_path(); ctx.move_to(tx(pts[0].0), ty(pts[0].1)); for (x,y) in pts.iter().skip(1) { ctx.line_to(tx(*x), ty(*y)); } ctx.stroke(); };
    let surf: Vec<_> = section.points.iter().map(|p| (p.distance, p.surface_plan)).collect();
    let exis: Vec<_> = section.points.iter().map(|p| (p.distance, p.existing_elevation)).collect();
    let cutt: Vec<_> = section.points.iter().map(|p| (p.distance, p.cutting_bottom)).collect();
    dl(&surf, "#e74c3c"); dl(&exis, "#3498db"); dl(&cutt, "#2ecc71");
    ctx.set_fill_style_str("#333"); ctx.set_font("14px sans-serif"); let _ = ctx.fill_text(&section.station_name, pad, 25.0);
}

fn main() { yew::Renderer::<App>::new().render(); }
