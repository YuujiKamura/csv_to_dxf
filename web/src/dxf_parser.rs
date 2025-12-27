//! DXF Parser - Ported from trianglelist's DxfParser.kt
//!
//! Parses DXF text format and extracts geometric entities.

use serde::{Deserialize, Serialize};

/// DXF parsing result containing all entities
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DxfParseResult {
    pub lines: Vec<DxfLine>,
    pub circles: Vec<DxfCircle>,
    pub arcs: Vec<DxfArc>,
    pub polylines: Vec<DxfPolyline>,
    pub texts: Vec<DxfText>,
    pub header: Option<DxfHeader>,
}

/// LINE entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DxfLine {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub color: i32,
    pub layer: String,
}

/// CIRCLE entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DxfCircle {
    pub center_x: f64,
    pub center_y: f64,
    pub radius: f64,
    pub color: i32,
    pub layer: String,
}

/// ARC entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DxfArc {
    pub center_x: f64,
    pub center_y: f64,
    pub radius: f64,
    pub start_angle: f64,
    pub end_angle: f64,
    pub color: i32,
    pub layer: String,
}

/// LWPOLYLINE entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DxfPolyline {
    pub vertices: Vec<(f64, f64)>,
    pub is_closed: bool,
    pub color: i32,
    pub layer: String,
}

/// TEXT entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DxfText {
    pub x: f64,
    pub y: f64,
    pub text: String,
    pub height: f64,
    pub rotation: f64,
    pub color: i32,
    pub align_h: i32,
    pub align_v: i32,
    pub layer: String,
}

/// DXF Header information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DxfHeader {
    pub acad_ver: String,
    pub ins_units: i32,
    pub ext_min: (f64, f64, f64),
    pub ext_max: (f64, f64, f64),
    pub dim_scale: f64,
}

/// Skip these entity types (contain large binary data)
const SKIP_ENTITY_TYPES: &[&str] = &["OLE2FRAME", "IMAGE", "WIPEOUT"];

/// DXF Parser
pub struct DxfParser;

impl DxfParser {
    pub fn new() -> Self {
        DxfParser
    }

    /// Parse DXF text content
    pub fn parse(&self, dxf_text: &str) -> DxfParseResult {
        let filtered = self.remove_heavy_entities(dxf_text);
        let lines: Vec<&str> = filtered.lines().collect();
        let mut iter = PeekableLines::new(&lines);

        self.parse_content(&mut iter)
    }

    /// Remove heavy entities like OLE2FRAME before parsing
    fn remove_heavy_entities(&self, dxf_text: &str) -> String {
        let lines: Vec<&str> = dxf_text.lines().collect();
        let mut result = String::new();
        let mut skip_mode = false;
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i].trim();

            if line == "0" && i + 1 < lines.len() {
                let next_line = lines[i + 1].trim();

                if SKIP_ENTITY_TYPES.contains(&next_line) {
                    skip_mode = true;
                    i += 2;
                    continue;
                } else if skip_mode && Self::is_entity_type(next_line) {
                    skip_mode = false;
                }
            }

            if !skip_mode {
                result.push_str(lines[i]);
                result.push('\n');
            }
            i += 1;
        }

        result
    }

    fn is_entity_type(s: &str) -> bool {
        !s.is_empty()
            && s.chars().next().map(|c| c.is_ascii_uppercase()).unwrap_or(false)
            && s.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
    }

    fn parse_content(&self, iter: &mut PeekableLines) -> DxfParseResult {
        let mut result = DxfParseResult::default();
        let mut current_section: Option<String> = None;

        while let Some((code, value)) = self.read_group_code_pair(iter) {
            match code {
                0 => match value.as_str() {
                    "SECTION" => {
                        current_section = self.read_section_name(iter);
                    }
                    "ENDSEC" => {
                        current_section = None;
                    }
                    "LINE" => {
                        if current_section.as_deref() == Some("ENTITIES") {
                            if let Some(line) = self.parse_line_entity(iter) {
                                result.lines.push(line);
                            }
                        }
                    }
                    "CIRCLE" => {
                        if current_section.as_deref() == Some("ENTITIES") {
                            if let Some(circle) = self.parse_circle_entity(iter) {
                                result.circles.push(circle);
                            }
                        }
                    }
                    "ARC" => {
                        if current_section.as_deref() == Some("ENTITIES") {
                            if let Some(arc) = self.parse_arc_entity(iter) {
                                result.arcs.push(arc);
                            }
                        }
                    }
                    "LWPOLYLINE" => {
                        if current_section.as_deref() == Some("ENTITIES") {
                            if let Some(poly) = self.parse_polyline_entity(iter) {
                                result.polylines.push(poly);
                            }
                        }
                    }
                    "TEXT" => {
                        if current_section.as_deref() == Some("ENTITIES") {
                            if let Some(text) = self.parse_text_entity(iter) {
                                result.texts.push(text);
                            }
                        }
                    }
                    _ => {
                        if current_section.as_deref() == Some("ENTITIES") {
                            self.skip_entity(iter);
                        }
                    }
                },
                9 => {
                    if current_section.as_deref() == Some("HEADER") && value.starts_with('$') {
                        if result.header.is_none() {
                            result.header = Some(DxfHeader::default());
                        }
                        self.parse_header_variable(iter, result.header.as_mut().unwrap(), &value);
                    }
                }
                _ => {}
            }
        }

        result
    }

    fn read_section_name(&self, iter: &mut PeekableLines) -> Option<String> {
        let (code, value) = self.read_group_code_pair(iter)?;
        if code == 2 {
            Some(value)
        } else {
            None
        }
    }

    fn read_group_code_pair(&self, iter: &mut PeekableLines) -> Option<(i32, String)> {
        let code_str = iter.next()?.trim();
        let value = iter.next()?.trim().to_string();
        let code = code_str.parse().ok()?;
        Some((code, value))
    }

    fn parse_line_entity(&self, iter: &mut PeekableLines) -> Option<DxfLine> {
        let mut x1 = None;
        let mut y1 = None;
        let mut x2 = None;
        let mut y2 = None;
        let mut color = 7;
        let mut layer = "0".to_string();

        while let Some(next) = iter.peek() {
            if next.trim() == "0" {
                break;
            }

            if let Some((code, value)) = self.read_group_code_pair(iter) {
                match code {
                    8 => layer = value,
                    10 => x1 = value.parse().ok(),
                    20 => y1 = value.parse().ok(),
                    11 => x2 = value.parse().ok(),
                    21 => y2 = value.parse().ok(),
                    62 => color = value.parse().unwrap_or(7),
                    _ => {}
                }
            }
        }

        Some(DxfLine {
            x1: x1?,
            y1: y1?,
            x2: x2?,
            y2: y2?,
            color,
            layer,
        })
    }

    fn parse_circle_entity(&self, iter: &mut PeekableLines) -> Option<DxfCircle> {
        let mut center_x = None;
        let mut center_y = None;
        let mut radius = None;
        let mut color = 7;
        let mut layer = "0".to_string();

        while let Some(next) = iter.peek() {
            if next.trim() == "0" {
                break;
            }

            if let Some((code, value)) = self.read_group_code_pair(iter) {
                match code {
                    8 => layer = value,
                    10 => center_x = value.parse().ok(),
                    20 => center_y = value.parse().ok(),
                    40 => radius = value.parse().ok(),
                    62 => color = value.parse().unwrap_or(7),
                    _ => {}
                }
            }
        }

        Some(DxfCircle {
            center_x: center_x?,
            center_y: center_y?,
            radius: radius?,
            color,
            layer,
        })
    }

    fn parse_arc_entity(&self, iter: &mut PeekableLines) -> Option<DxfArc> {
        let mut center_x = None;
        let mut center_y = None;
        let mut radius = None;
        let mut start_angle = 0.0;
        let mut end_angle = 360.0;
        let mut color = 7;
        let mut layer = "0".to_string();

        while let Some(next) = iter.peek() {
            if next.trim() == "0" {
                break;
            }

            if let Some((code, value)) = self.read_group_code_pair(iter) {
                match code {
                    8 => layer = value,
                    10 => center_x = value.parse().ok(),
                    20 => center_y = value.parse().ok(),
                    40 => radius = value.parse().ok(),
                    50 => start_angle = value.parse().unwrap_or(0.0),
                    51 => end_angle = value.parse().unwrap_or(360.0),
                    62 => color = value.parse().unwrap_or(7),
                    _ => {}
                }
            }
        }

        Some(DxfArc {
            center_x: center_x?,
            center_y: center_y?,
            radius: radius?,
            start_angle,
            end_angle,
            color,
            layer,
        })
    }

    fn parse_polyline_entity(&self, iter: &mut PeekableLines) -> Option<DxfPolyline> {
        let mut vertices = Vec::new();
        let mut is_closed = false;
        let mut color = 7;
        let mut layer = "0".to_string();
        let mut current_x: Option<f64> = None;

        while let Some(next) = iter.peek() {
            if next.trim() == "0" {
                break;
            }

            if let Some((code, value)) = self.read_group_code_pair(iter) {
                match code {
                    8 => layer = value,
                    70 => is_closed = (value.parse::<i32>().unwrap_or(0) & 1) != 0,
                    10 => current_x = value.parse().ok(),
                    20 => {
                        if let (Some(x), Ok(y)) = (current_x, value.parse()) {
                            vertices.push((x, y));
                            current_x = None;
                        }
                    }
                    62 => color = value.parse().unwrap_or(7),
                    _ => {}
                }
            }
        }

        if vertices.is_empty() {
            None
        } else {
            Some(DxfPolyline {
                vertices,
                is_closed,
                color,
                layer,
            })
        }
    }

    fn parse_text_entity(&self, iter: &mut PeekableLines) -> Option<DxfText> {
        let mut x = None;
        let mut y = None;
        let mut x2 = None;
        let mut y2 = None;
        let mut text = None;
        let mut height = 1.0;
        let mut rotation = 0.0;
        let mut color = 7;
        let mut align_h = 0;
        let mut align_v = 0;
        let mut layer = "0".to_string();

        while let Some(next) = iter.peek() {
            if next.trim() == "0" {
                break;
            }

            if let Some((code, value)) = self.read_group_code_pair(iter) {
                match code {
                    8 => layer = value,
                    10 => x = value.parse().ok(),
                    20 => y = value.parse().ok(),
                    11 => x2 = value.parse().ok(),
                    21 => y2 = value.parse().ok(),
                    1 => text = Some(value),
                    40 => height = value.parse().unwrap_or(1.0),
                    50 => rotation = value.parse().unwrap_or(0.0),
                    62 => color = value.parse().unwrap_or(7),
                    72 => align_h = value.parse().unwrap_or(0),
                    73 => align_v = value.parse().unwrap_or(0),
                    _ => {}
                }
            }
        }

        // Use second insertion point if alignment is set
        let use_second = (align_h != 0 || align_v != 0) && x2.is_some() && y2.is_some();
        let final_x = if use_second { x2 } else { x };
        let final_y = if use_second { y2 } else { y };

        Some(DxfText {
            x: final_x?,
            y: final_y?,
            text: text?,
            height,
            rotation,
            color,
            align_h,
            align_v,
            layer,
        })
    }

    fn parse_header_variable(&self, iter: &mut PeekableLines, header: &mut DxfHeader, var_name: &str) {
        match var_name {
            "$ACADVER" => {
                if let Some((_, value)) = self.read_group_code_pair(iter) {
                    header.acad_ver = value;
                }
            }
            "$INSUNITS" => {
                if let Some((_, value)) = self.read_group_code_pair(iter) {
                    header.ins_units = value.parse().unwrap_or(6);
                }
            }
            "$DIMSCALE" => {
                if let Some((_, value)) = self.read_group_code_pair(iter) {
                    header.dim_scale = value.parse().unwrap_or(1.0);
                }
            }
            "$EXTMIN" => {
                let x = self.read_group_code_pair(iter).and_then(|(_, v)| v.parse().ok()).unwrap_or(0.0);
                let y = self.read_group_code_pair(iter).and_then(|(_, v)| v.parse().ok()).unwrap_or(0.0);
                let z = self.read_group_code_pair(iter).and_then(|(_, v)| v.parse().ok()).unwrap_or(0.0);
                header.ext_min = (x, y, z);
            }
            "$EXTMAX" => {
                let x = self.read_group_code_pair(iter).and_then(|(_, v)| v.parse().ok()).unwrap_or(0.0);
                let y = self.read_group_code_pair(iter).and_then(|(_, v)| v.parse().ok()).unwrap_or(0.0);
                let z = self.read_group_code_pair(iter).and_then(|(_, v)| v.parse().ok()).unwrap_or(0.0);
                header.ext_max = (x, y, z);
            }
            _ => {
                // Skip unknown header variables
                let _ = self.read_group_code_pair(iter);
            }
        }
    }

    fn skip_entity(&self, iter: &mut PeekableLines) {
        while let Some(next) = iter.peek() {
            if next.trim() == "0" {
                break;
            }
            iter.next();
            iter.next();
        }
    }
}

/// Peekable iterator over lines
struct PeekableLines<'a> {
    lines: &'a [&'a str],
    pos: usize,
}

impl<'a> PeekableLines<'a> {
    fn new(lines: &'a [&'a str]) -> Self {
        Self { lines, pos: 0 }
    }

    fn peek(&self) -> Option<&'a str> {
        self.lines.get(self.pos).copied()
    }

    fn next(&mut self) -> Option<&'a str> {
        let item = self.lines.get(self.pos).copied();
        if item.is_some() {
            self.pos += 1;
        }
        item
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_line() {
        let dxf = r#"0
SECTION
2
ENTITIES
0
LINE
8
Layer1
10
0.0
20
0.0
11
100.0
21
100.0
0
ENDSEC
0
EOF
"#;
        let parser = DxfParser::new();
        let result = parser.parse(dxf);

        assert_eq!(result.lines.len(), 1);
        assert_eq!(result.lines[0].x1, 0.0);
        assert_eq!(result.lines[0].y1, 0.0);
        assert_eq!(result.lines[0].x2, 100.0);
        assert_eq!(result.lines[0].y2, 100.0);
        assert_eq!(result.lines[0].layer, "Layer1");
    }
}
