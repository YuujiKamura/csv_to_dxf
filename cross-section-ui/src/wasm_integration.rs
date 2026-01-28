//! WASM統合モジュール - ブラウザ向けファイルダイアログ、Fetch API、エントリポイント

#[cfg(target_arch = "wasm32")]
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
#[cfg(target_arch = "wasm32")]
use base64::Engine as _;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{prelude::*, JsCast};
#[cfg(target_arch = "wasm32")]
use web_sys::{Event, FileReader, HtmlCanvasElement, HtmlInputElement, Headers, Request, RequestInit, Response};

#[cfg(target_arch = "wasm32")]
use crate::CrossSectionApp;

#[cfg(target_arch = "wasm32")]
thread_local! {
    static PENDING_CSV: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
    static PENDING_JSON: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
    static PENDING_XLSX: std::cell::RefCell<Option<Vec<u8>>> = std::cell::RefCell::new(None);
    static PENDING_ERROR: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
    static JSON_FETCH_STARTED: std::cell::Cell<bool> = std::cell::Cell::new(false);
    static GEMINI_IN_FLIGHT: std::cell::Cell<bool> = std::cell::Cell::new(false);
}

#[cfg(target_arch = "wasm32")]
pub fn trigger_csv_dialog() {
    let Some(window) = web_sys::window() else { return; };
    let Some(document) = window.document() else { return; };

    let Ok(input) = document.create_element("input") else { return; };
    let Ok(input) = input.dyn_into::<HtmlInputElement>() else { return; };
    input.set_type("file");
    input.set_accept(".csv,text/csv");

    let on_change: std::rc::Rc<std::cell::RefCell<Option<Closure<dyn FnMut(Event)>>>> =
        std::rc::Rc::new(std::cell::RefCell::new(None));
    let on_load: std::rc::Rc<std::cell::RefCell<Option<Closure<dyn FnMut(Event)>>>> =
        std::rc::Rc::new(std::cell::RefCell::new(None));

    let on_change_handle = on_change.clone();
    let on_load_handle = on_load.clone();

    *on_change.borrow_mut() = Some(Closure::<dyn FnMut(Event)>::new(move |event: Event| {
        let Some(target) = event.target() else { return; };
        let Ok(input) = target.dyn_into::<HtmlInputElement>() else { return; };
        let Some(files) = input.files() else { return; };
        let Some(file) = files.get(0) else { return; };

        let Ok(reader) = FileReader::new() else { return; };
        let reader_clone = reader.clone();
        let on_load_handle_clone = on_load_handle.clone();
        let on_change_handle = on_change_handle.clone();

        *on_load_handle.borrow_mut() = Some(Closure::<dyn FnMut(Event)>::new(move |_event: Event| {
            let result = reader_clone.result();
            if let Ok(result) = result {
                if let Some(text) = result.as_string() {
                    PENDING_CSV.with(|cell| {
                        *cell.borrow_mut() = Some(text);
                    });
                }
            }
            reader_clone.set_onload(None);
            on_load_handle_clone.borrow_mut().take();
        }));

        if let Some(handler) = on_load_handle.borrow().as_ref() {
            reader.set_onload(Some(handler.as_ref().unchecked_ref()));
        }

        let _ = reader.read_as_text(&file);
        input.set_onchange(None);
        on_change_handle.borrow_mut().take();
    }));

    if let Some(handler) = on_change.borrow().as_ref() {
        input.set_onchange(Some(handler.as_ref().unchecked_ref()));
    }
    input.click();
}

#[cfg(target_arch = "wasm32")]
pub fn trigger_xlsx_dialog() {
    let Some(window) = web_sys::window() else { return; };
    let Some(document) = window.document() else { return; };

    let Ok(input) = document.create_element("input") else { return; };
    let Ok(input) = input.dyn_into::<HtmlInputElement>() else { return; };
    input.set_type("file");
    input.set_accept(".xlsx,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet");

    let on_change: std::rc::Rc<std::cell::RefCell<Option<Closure<dyn FnMut(Event)>>>> =
        std::rc::Rc::new(std::cell::RefCell::new(None));
    let on_load: std::rc::Rc<std::cell::RefCell<Option<Closure<dyn FnMut(Event)>>>> =
        std::rc::Rc::new(std::cell::RefCell::new(None));

    let on_change_handle = on_change.clone();
    let on_load_handle = on_load.clone();

    *on_change.borrow_mut() = Some(Closure::<dyn FnMut(Event)>::new(move |event: Event| {
        let Some(target) = event.target() else { return; };
        let Ok(input) = target.dyn_into::<HtmlInputElement>() else { return; };
        let Some(files) = input.files() else { return; };
        let Some(file) = files.get(0) else { return; };

        let Ok(reader) = FileReader::new() else { return; };
        let reader_clone = reader.clone();
        let on_load_handle_clone = on_load_handle.clone();
        let on_change_handle = on_change_handle.clone();

        *on_load_handle.borrow_mut() = Some(Closure::<dyn FnMut(Event)>::new(move |_event: Event| {
            let result = reader_clone.result();
            if let Ok(result) = result {
                if let Some(buf) = result.dyn_ref::<js_sys::ArrayBuffer>() {
                    let array = js_sys::Uint8Array::new(buf);
                    let mut bytes = vec![0u8; array.length() as usize];
                    array.copy_to(&mut bytes);
                    PENDING_XLSX.with(|cell| {
                        *cell.borrow_mut() = Some(bytes);
                    });
                }
            }
            reader_clone.set_onload(None);
            on_load_handle_clone.borrow_mut().take();
        }));

        if let Some(handler) = on_load_handle.borrow().as_ref() {
            reader.set_onload(Some(handler.as_ref().unchecked_ref()));
        }

        let _ = reader.read_as_array_buffer(&file);
        input.set_onchange(None);
        on_change_handle.borrow_mut().take();
    }));

    if let Some(handler) = on_change.borrow().as_ref() {
        input.set_onchange(Some(handler.as_ref().unchecked_ref()));
    }
    input.click();
}

#[cfg(target_arch = "wasm32")]
pub fn take_pending_csv() -> Option<String> {
    PENDING_CSV.with(|cell| cell.borrow_mut().take())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn take_pending_csv() -> Option<String> {
    None
}

#[cfg(target_arch = "wasm32")]
pub fn take_pending_json() -> Option<String> {
    PENDING_JSON.with(|cell| cell.borrow_mut().take())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn take_pending_json() -> Option<String> {
    None
}

#[cfg(target_arch = "wasm32")]
pub fn take_pending_xlsx() -> Option<Vec<u8>> {
    PENDING_XLSX.with(|cell| cell.borrow_mut().take())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn take_pending_xlsx() -> Option<Vec<u8>> {
    None
}

#[cfg(target_arch = "wasm32")]
pub fn take_pending_error() -> Option<String> {
    PENDING_ERROR.with(|cell| cell.borrow_mut().take())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn take_pending_error() -> Option<String> {
    None
}

#[cfg(target_arch = "wasm32")]
fn extract_json_from_text(text: &str) -> String {
    if !text.contains("```") {
        return text.trim().to_string();
    }
    let parts: Vec<&str> = text.split("```").collect();
    if parts.len() >= 2 {
        let mut candidate = parts[1];
        if let Some(rest) = candidate.strip_prefix("json") {
            candidate = rest;
        }
        return candidate.trim().to_string();
    }
    text.trim().to_string()
}

#[cfg(target_arch = "wasm32")]
pub fn start_gemini_parse(api_key: String, xlsx_bytes: Vec<u8>) {
    use wasm_bindgen_futures::JsFuture;

    let already_started = GEMINI_IN_FLIGHT.with(|cell| {
        if cell.get() { return true; }
        cell.set(true);
        false
    });
    if already_started { return; }

    wasm_bindgen_futures::spawn_local(async move {
        let result = async {
            let window = web_sys::window().ok_or("No window")?;
            let base64_data = BASE64_STANDARD.encode(&xlsx_bytes);
            let prompt = [
                "You are given an Excel file. Extract road cross-section data and output ONLY JSON array matching schema:",
                "[{",
                "  \"survey_point_name\": string,",
                "  \"dl\": number,",
                "  \"cl_index\": number,",
                "  \"l_to_cl_distance\": number,",
                "  \"survey_data\": [{",
                "    \"unit_distance\": number,",
                "    \"elevation\": number,",
                "    \"planned_height\": number,",
                "    \"cumulative_distance\": number,",
                "    \"cutting_bottom\": number",
                "  }],",
                "  \"route_distance\": number | null,",
                "  \"route_id\": string",
                "}]",
                "Rules:",
                "- Output JSON only (no markdown).",
                "- If multiple routes exist, set route_id per route; otherwise use \"route_1\".",
                "- cl_index is the index of the center line point within survey_data.",
                "- l_to_cl_distance is the distance from left edge to CL (meters).",
                "- cumulative_distance is distance from CL (left is negative, right positive).",
                "- cutting_bottom is ground elevation minus cutting depth if explicit data is not present.",
            ].join("\n");

            let payload = serde_json::json!({
                "contents": [{
                    "role": "user",
                    "parts": [
                        { "text": prompt },
                        { "inlineData": {
                            "mimeType": "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                            "data": base64_data
                        }}
                    ]
                }],
                "generationConfig": {
                    "temperature": 0.2,
                    "topP": 0.9,
                    "maxOutputTokens": 8192
                }
            });

            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}",
                api_key
            );
            let mut opts = RequestInit::new();
            opts.method("POST");
            let body = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
            opts.body(Some(&JsValue::from_str(&body)));

            let headers = Headers::new().map_err(|_| "Failed to create headers")?;
            headers
                .set("Content-Type", "application/json")
                .map_err(|_| "Failed to set headers")?;
            opts.headers(&headers);

            let request = Request::new_with_str_and_init(&url, &opts)
                .map_err(|_| "Failed to create request")?;

            let resp_value = JsFuture::from(window.fetch_with_request(&request))
                .await
                .map_err(|_| "Failed to fetch")?;

            let resp: Response = resp_value.dyn_into().map_err(|_| "Invalid response")?;
            let text = JsFuture::from(resp.text().map_err(|_| "Failed to read response")?)
                .await
                .map_err(|_| "Failed to read response text")?;
            let text = text.as_string().ok_or("Response is not text")?;

            let value: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
            let content = value
                .get("candidates")
                .and_then(|c| c.get(0))
                .and_then(|c| c.get("content"))
                .and_then(|c| c.get("parts"))
                .and_then(|p| p.get(0))
                .and_then(|p| p.get("text"))
                .and_then(|t| t.as_str())
                .ok_or("Gemini response missing content")?;

            Ok::<String, String>(extract_json_from_text(content))
        }.await;

        match result {
            Ok(json_text) => {
                PENDING_JSON.with(|cell| {
                    *cell.borrow_mut() = Some(json_text);
                });
            }
            Err(err) => {
                PENDING_ERROR.with(|cell| {
                    *cell.borrow_mut() = Some(err);
                });
            }
        }
        GEMINI_IN_FLIGHT.with(|cell| cell.set(false));
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn start_gemini_parse(_api_key: String, _xlsx_bytes: Vec<u8>) {
    // ネイティブでは何もしない
}

#[cfg(target_arch = "wasm32")]
pub fn start_json_fetch() {
    use wasm_bindgen_futures::JsFuture;

    let already_started = JSON_FETCH_STARTED.with(|cell| {
        if cell.get() { return true; }
        cell.set(true);
        false
    });
    if already_started { return; }

    wasm_bindgen_futures::spawn_local(async {
        let window = match web_sys::window() {
            Some(w) => w,
            None => return,
        };

        // 相対パスでsections.jsonを取得
        let url = "sections.json";
        let mut opts = RequestInit::new();
        opts.method("GET");

        let request = match Request::new_with_str_and_init(url, &opts) {
            Ok(r) => r,
            Err(_) => return,
        };

        let resp_value = match JsFuture::from(window.fetch_with_request(&request)).await {
            Ok(r) => r,
            Err(_) => return,
        };

        let resp: Response = match resp_value.dyn_into() {
            Ok(r) => r,
            Err(_) => return,
        };

        let text = match resp.text() {
            Ok(t) => t,
            Err(_) => return,
        };

        let text: String = match JsFuture::from(text).await {
            Ok(t) => match t.as_string() {
                Some(s) => s,
                None => return,
            },
            Err(_) => return,
        };

        PENDING_JSON.with(|cell| {
            *cell.borrow_mut() = Some(text);
        });
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn start_json_fetch() {
    // ネイティブでは何もしない
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).ok();

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;
    let document = window.document().ok_or_else(|| JsValue::from_str("No document"))?;
    let canvas = document
        .get_element_by_id("canvas")
        .ok_or_else(|| JsValue::from_str("No canvas element"))?
        .dyn_into::<HtmlCanvasElement>()
        .map_err(|_| JsValue::from_str("Not a canvas"))?;

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async move {
        eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(CrossSectionApp::new(cc)))),
            )
            .await
            .expect("Failed to start eframe");
    });

    Ok(())
}
