//! コンボ図面生成（縦断図・面積展開図・横断図の組み合わせ）

use dxf::Drawing;

use crate::{
    CrossSectionData,
    new_drawing,
    generate_area_expansion_drawing,
    generate_longitudinal_drawing,
    generate_multi_drawing_scaled_interpolated,
    calc_dxf_bounds,
    add_text,
    draw_drawing_frame,
    TitleBlockInfo,
    HAlign,
};

/// コンボ図面を生成（縦断図・面積展開図・横断図を配置）
pub fn generate_combo_drawing(sections: &[CrossSectionData], columns: usize, column_gap: f64) -> Drawing {
    if sections.is_empty() {
        return new_drawing();
    }

    // ルートIDを判定（ルート2かどうか）
    let is_route_2 = sections.first().map(|s| s.route_id == "route_2").unwrap_or(false);

    // 面積展開図を生成し、バウンディングボックスを取得（コンボでは測点名非表示）
    let area_expansion = generate_area_expansion_drawing(sections, false);
    let (_area_min_x, area_min_y, _area_max_x, _area_max_y) = calc_dxf_bounds(&area_expansion);

    // 縦断図を生成し、バウンディングボックスを取得
    let longitudinal = generate_longitudinal_drawing(sections);
    let (_long_min_x, _long_min_y, long_max_x, long_max_y) = calc_dxf_bounds(&longitudinal);

    // 横断図を生成（ルート2のみ、5倍拡大、補完点を勾配補間）
    let cross_sections = if is_route_2 {
        let row_gap = 1.0;
        Some(generate_multi_drawing_scaled_interpolated(sections, columns, column_gap, row_gap, 5.0))
    } else {
        None
    };

    // 新しいDrawingを作成
    let mut drawing = new_drawing();

    // 全てのレイヤーをマージ
    for layer in area_expansion.layers() {
        let mut new_layer = dxf::tables::Layer::default();
        new_layer.name = layer.name.clone();
        new_layer.color = layer.color.clone();
        drawing.add_layer(new_layer);
    }
    for layer in longitudinal.layers() {
        if drawing.layers().find(|l| l.name == layer.name).is_none() {
            let mut new_layer = dxf::tables::Layer::default();
            new_layer.name = layer.name.clone();
            new_layer.color = layer.color.clone();
            drawing.add_layer(new_layer);
        }
    }
    if let Some(ref cs) = cross_sections {
        for layer in cs.layers() {
            if drawing.layers().find(|l| l.name == layer.name).is_none() {
                let mut new_layer = dxf::tables::Layer::default();
                new_layer.name = layer.name.clone();
                new_layer.color = layer.color.clone();
                drawing.add_layer(new_layer);
            }
        }
    }

    // === レイアウト計算 ===
    let spacing = 2000.0; // 2m間隔（測点名非表示のため縮小）

    // 縦断図を原点に配置
    for entity in longitudinal.entities() {
        drawing.add_entity(entity.clone());
    }

    // 面積展開図: 縦断図の上に配置
    let area_x_offset = 0.0_f64;
    let area_y_offset = long_max_y as f64 + spacing - area_min_y as f64;
    for entity in area_expansion.entities() {
        let mut shifted_entity = entity.clone();
        shift_entity_xy(&mut shifted_entity, area_x_offset, area_y_offset);
        drawing.add_entity(shifted_entity);
    }

    // 横断図配置前に縦断図＋展開図のバウンディングボックスを取得
    let (_left_min_x, left_min_y, _left_max_x, left_max_y) = calc_dxf_bounds(&drawing);
    let left_center_y = (left_min_y as f64 + left_max_y as f64) / 2.0;

    // 横断図: ルート2のみ、縦断図・展開図の右側に配置（縦方向は中央揃え）
    if let Some(ref cs) = cross_sections {
        let (cs_min_x, cs_min_y, _cs_max_x, cs_max_y) = calc_dxf_bounds(cs);
        let cs_center_y = (cs_min_y as f64 + cs_max_y as f64) / 2.0;
        let cs_x_offset = long_max_x as f64 + spacing * 2.0 - cs_min_x as f64;
        let cs_y_offset = left_center_y - cs_center_y;  // 縦断図＋展開図の中央に合わせる
        for entity in cs.entities() {
            let mut shifted_entity = entity.clone();
            shift_entity_xy(&mut shifted_entity, cs_x_offset, cs_y_offset);
            drawing.add_entity(shifted_entity);
        }
    }

    // 配置後の全体バウンディングボックスを計算
    let (data_min_x, data_min_y, data_max_x, data_max_y) = calc_dxf_bounds(&drawing);
    let data_center_x = (data_min_x as f64 + data_max_x as f64) / 2.0;
    let data_center_y = (data_min_y as f64 + data_max_y as f64) / 2.0;

    // 図枠スケール（ルート2は1:500、それ以外は1:700）
    let frame_scale = if is_route_2 { 500.0 } else { 700.0 };
    let frame_text_size = 3.0;

    // 縦断図のスケール比率（V:H = 5:1）
    let longitudinal_v_ratio = 5.0;
    // 横断図のスケール設定（scale_multiplier=5.0, v_scale_ratio=2.0）
    let cross_scale_multiplier = 5.0;
    let cross_v_scale_ratio = 2.0;

    // 図枠の内枠中心（A3: 420x297mm, マージン10mm）
    let frame_center_x = 210.0;
    let frame_center_y = 148.5;

    // 図枠の原点（図枠中心とデータ中心を一致させる）
    let frame_origin_x = data_center_x - frame_center_x * frame_scale;
    let frame_origin_y = data_center_y - frame_center_y * frame_scale;

    // スケール文字列を動的に生成
    let scale_text = format!(
        "H=1:{} V=1:{}",
        frame_scale as u32,
        (frame_scale / longitudinal_v_ratio) as u32
    );

    // ルート2のときはタイトルに横断図も含める、路線名も変更
    let (drawing_type, top_title, route_name) = if is_route_2 {
        ("縦断図　横断図", "縦断図　横断図", "東側取付道路")
    } else {
        ("縦断図", "縦断図", "市道南千反畑第１号線　本線")
    };

    let frame_info = TitleBlockInfo::new()
        .with_project_name("市道 南千反畑町第１号線舗装補修工事")
        .with_drawing_type(drawing_type)
        .with_route_name(route_name)
        .with_date("2026年1月")
        .with_scale(&scale_text)
        .with_drawing_number("1/1")
        .with_author("有限会社　三雄建設")
        .with_top_title(top_title)
        .with_credit("")
        .with_debug_markers(false);

    draw_drawing_frame(&mut drawing, &frame_info, frame_origin_x, frame_origin_y, frame_scale, frame_text_size);

    // ルート2の場合、横断図範囲に縮尺を表示
    if let Some(ref cs) = cross_sections {
        let (_cs_min_x, cs_min_y, _cs_max_x, cs_max_y) = calc_dxf_bounds(cs);
        let cs_center_y = (cs_min_y as f64 + cs_max_y as f64) / 2.0;
        let cs_y_offset = left_center_y - cs_center_y;
        // 横断図の左下に縮尺表示（配置後の位置）
        let scale_label_x = long_max_x as f64 + spacing * 2.0;
        let scale_label_y = cs_min_y as f64 + cs_y_offset - frame_text_size * frame_scale;
        // 横断図のスケール: H = frame_scale / scale_multiplier, V = H / v_scale_ratio
        let cross_h_scale = (frame_scale / cross_scale_multiplier) as u32;
        let cross_v_scale = (cross_h_scale as f64 / cross_v_scale_ratio) as u32;
        let cross_scale_text = format!("横断図 Scale H=1:{} V=1:{}", cross_h_scale, cross_v_scale);
        add_text(&mut drawing, scale_label_x, scale_label_y, &cross_scale_text,
            frame_text_size * frame_scale, 7, "TEXT", HAlign::Left);
    }

    drawing
}

/// エンティティのXY座標をシフトする
fn shift_entity_xy(entity: &mut dxf::entities::Entity, offset_x: f64, offset_y: f64) {
    use dxf::entities::EntityType;
    match &mut entity.specific {
        EntityType::Line(line) => {
            line.p1.x += offset_x;
            line.p1.y += offset_y;
            line.p2.x += offset_x;
            line.p2.y += offset_y;
        }
        EntityType::Text(text) => {
            text.location.x += offset_x;
            text.location.y += offset_y;
            text.second_alignment_point.x += offset_x;
            text.second_alignment_point.y += offset_y;
        }
        EntityType::MText(mtext) => {
            mtext.insertion_point.x += offset_x;
            mtext.insertion_point.y += offset_y;
        }
        EntityType::Circle(circle) => {
            circle.center.x += offset_x;
            circle.center.y += offset_y;
        }
        EntityType::Arc(arc) => {
            arc.center.x += offset_x;
            arc.center.y += offset_y;
        }
        EntityType::Polyline(_polyline) => {
            // Polyline vertices are stored separately in DXF, skip for now
        }
        EntityType::LwPolyline(lwpoly) => {
            for vertex in &mut lwpoly.vertices {
                vertex.x += offset_x;
                vertex.y += offset_y;
            }
        }
        EntityType::RotatedDimension(dim) => {
            dim.dimension_base.definition_point_1.x += offset_x;
            dim.dimension_base.definition_point_1.y += offset_y;
            dim.dimension_base.text_mid_point.x += offset_x;
            dim.dimension_base.text_mid_point.y += offset_y;
            dim.insertion_point.x += offset_x;
            dim.insertion_point.y += offset_y;
            dim.definition_point_2.x += offset_x;
            dim.definition_point_2.y += offset_y;
            dim.definition_point_3.x += offset_x;
            dim.definition_point_3.y += offset_y;
        }
        _ => {} // その他のエンティティは無視
    }
}

/// エンティティのY座標をシフトする
#[allow(dead_code)]
fn shift_entity_y(entity: &mut dxf::entities::Entity, offset: f64) {
    use dxf::entities::EntityType;
    match &mut entity.specific {
        EntityType::Line(line) => {
            line.p1.y += offset;
            line.p2.y += offset;
        }
        EntityType::Text(text) => {
            text.location.y += offset;
        }
        EntityType::MText(mtext) => {
            mtext.insertion_point.y += offset;
        }
        EntityType::Circle(circle) => {
            circle.center.y += offset;
        }
        EntityType::Arc(arc) => {
            arc.center.y += offset;
        }
        EntityType::Polyline(_polyline) => {
            // Polyline vertices are stored separately in DXF, skip for now
        }
        EntityType::LwPolyline(lwpoly) => {
            for vertex in &mut lwpoly.vertices {
                vertex.y += offset;
            }
        }
        EntityType::RotatedDimension(dim) => {
            dim.dimension_base.definition_point_1.y += offset;
            dim.dimension_base.text_mid_point.y += offset;
            dim.insertion_point.y += offset;
            dim.definition_point_2.y += offset;
            dim.definition_point_3.y += offset;
        }
        _ => {} // その他のエンティティは無視
    }
}

/// コンボDXFバイト列を生成
pub fn generate_combo_dxf_bytes(sections: &[CrossSectionData], columns: usize, column_gap: f64) -> Vec<u8> {
    let drawing = generate_combo_drawing(sections, columns, column_gap);
    let mut output: Vec<u8> = Vec::new();
    drawing.save(&mut output).expect("Failed to save DXF");
    output
}

/// 全ルートを縦に並べたコンボDXFを生成（図枠ごと）
pub fn generate_all_routes_combo_dxf_bytes(sections: &[CrossSectionData], columns: usize, column_gap: f64) -> Vec<u8> {
    use std::collections::BTreeSet;

    // ルートIDを収集（ソート済み）
    let route_ids: BTreeSet<String> = sections.iter().map(|s| s.route_id.clone()).collect();

    if route_ids.is_empty() {
        return Vec::new();
    }

    let mut combined_drawing = new_drawing();
    let spacing = 10000.0; // ルート間の垂直スペース（10m）
    let mut current_y_offset = 0.0;

    for route_id in route_ids {
        // このルートのセクションをフィルター
        let route_sections: Vec<CrossSectionData> = sections.iter()
            .filter(|s| s.route_id == route_id)
            .cloned()
            .collect();

        if route_sections.is_empty() {
            continue;
        }

        // このルートのコンボ図面を生成
        let route_drawing = generate_combo_drawing(&route_sections, columns, column_gap);
        let (_, min_y, _, max_y) = calc_dxf_bounds(&route_drawing);
        let route_height = max_y as f64 - min_y as f64;

        // レイヤーをマージ（最初のルートのみ）
        if current_y_offset == 0.0 {
            for layer in route_drawing.layers() {
                let mut new_layer = dxf::tables::Layer::default();
                new_layer.name = layer.name.clone();
                new_layer.color = layer.color.clone();
                combined_drawing.add_layer(new_layer);
            }
        }

        // エンティティをY方向にシフトして追加
        let y_shift = current_y_offset - min_y as f64;
        for entity in route_drawing.entities() {
            let mut shifted_entity = entity.clone();
            shift_entity_xy(&mut shifted_entity, 0.0, y_shift);
            combined_drawing.add_entity(shifted_entity);
        }

        // 次のルートのY位置を更新
        current_y_offset += route_height + spacing;
    }

    let mut output: Vec<u8> = Vec::new();
    combined_drawing.save(&mut output).expect("Failed to save DXF");
    output
}
