//! Integration tests for multi_drawing module
//!
//! These tests cover the MultiDrawingBuilder API and deprecated legacy functions.

use cross_section_ui::{
    CrossSectionData, SurveyRow, TitleBlockInfo,
    MultiDrawingBuilder,
    calc_column_widths,
};

#[allow(deprecated)]
use cross_section_ui::{
    generate_multi_drawing,
    generate_multi_drawing_with_frame,
    generate_multi_dxf_bytes,
};

// ============================================================================
// Test Helpers
// ============================================================================

fn make_test_section(name: &str) -> CrossSectionData {
    CrossSectionData {
        survey_point_name: name.to_string(),
        dl: 10.0,
        cl_index: 1,
        l_to_cl_distance: 3.0,
        survey_data: vec![
            SurveyRow {
                unit_distance: 0.0,
                elevation: 11.0,
                planned_height: 11.0,
                cumulative_distance: -3.0,
                cutting_bottom: 10.95,
            },
            SurveyRow {
                unit_distance: 3.0,
                elevation: 11.0,
                planned_height: 11.0,
                cumulative_distance: 0.0,
                cutting_bottom: 10.95,
            },
            SurveyRow {
                unit_distance: 3.0,
                elevation: 11.0,
                planned_height: 11.0,
                cumulative_distance: 3.0,
                cutting_bottom: 10.95,
            },
        ],
        route_distance: None,
        route_id: "route_1".to_string(),
        is_route_start: false,
        is_route_end: false,
    }
}

// ============================================================================
// Builder Tests
// ============================================================================

#[test]
fn test_builder_basic() {
    let sections = vec![make_test_section("NO.1")];
    let drawing = MultiDrawingBuilder::new(&sections)
        .columns(4)
        .gaps(2.0, 1.0)
        .build();
    assert!(drawing.entities().count() > 0);
    assert!(drawing.layers().count() >= 6);
}

#[test]
fn test_builder_empty_sections() {
    let sections: Vec<CrossSectionData> = vec![];
    let drawing = MultiDrawingBuilder::new(&sections).build();
    assert!(drawing.layers().count() >= 6);
}

#[test]
fn test_builder_with_frame() {
    let sections = vec![make_test_section("NO.1")];
    let title_info = TitleBlockInfo::new().with_top_title("Test");
    let drawing = MultiDrawingBuilder::new(&sections)
        .gaps(2.0, 1.0)
        .with_frame(500.0, &title_info)
        .build();
    assert!(drawing.entities().count() > 0);
    assert!(drawing.layers().any(|l| l.name == "TITLEBLOCK"));
}

#[test]
fn test_builder_interpolated() {
    let sections = vec![make_test_section("NO.1")];
    let drawing = MultiDrawingBuilder::new(&sections)
        .interpolated()
        .v_scale(2.5)
        .build();
    assert!(drawing.entities().count() > 0);
}

#[test]
fn test_builder_to_dxf_bytes() {
    let sections = vec![make_test_section("NO.1")];
    let bytes = MultiDrawingBuilder::new(&sections)
        .columns(4)
        .gaps(2.0, 1.0)
        .to_dxf_bytes();
    assert!(!bytes.is_empty());
    let content = String::from_utf8_lossy(&bytes);
    assert!(content.contains("SECTION"), "DXF should contain SECTION");
}

#[test]
fn test_builder_full_options() {
    let sections = vec![make_test_section("NO.1"), make_test_section("NO.2")];
    let title_info = TitleBlockInfo::new().with_top_title("Full Test");
    let drawing = MultiDrawingBuilder::new(&sections)
        .columns(4)
        .gaps(2.0, 1.0)
        .with_frame(200.0, &title_info)
        .interpolated()
        .v_scale(3.0)
        .scale_multiplier(1.5)
        .build();
    assert!(drawing.entities().count() > 0);
}

// ============================================================================
// Legacy Function Tests (kept for backward compatibility)
// ============================================================================

#[test]
#[allow(deprecated)]
fn test_generate_multi_drawing_empty() {
    let sections: Vec<CrossSectionData> = vec![];
    let drawing = generate_multi_drawing(&sections, 4, 2.0, 1.0);
    assert!(drawing.layers().count() >= 6);
}

#[test]
#[allow(deprecated)]
fn test_generate_multi_drawing_single() {
    let sections = vec![make_test_section("NO.1")];
    let drawing = generate_multi_drawing(&sections, 4, 2.0, 1.0);
    assert!(drawing.entities().count() > 0);
}

#[test]
#[allow(deprecated)]
fn test_generate_multi_drawing_with_frame() {
    let sections = vec![make_test_section("NO.1")];
    let title_info = TitleBlockInfo::new().with_top_title("Test");
    let drawing = generate_multi_drawing_with_frame(
        &sections, 4, 2.0, 1.0, &title_info
    );
    assert!(drawing.entities().count() > 0);
    assert!(drawing.layers().any(|l| l.name == "TITLEBLOCK"));
}

#[test]
#[allow(deprecated)]
fn test_generate_multi_dxf_bytes() {
    let sections = vec![make_test_section("NO.1")];
    let bytes = generate_multi_dxf_bytes(&sections, 4, 2.0, 1.0);
    assert!(!bytes.is_empty());
    let content = String::from_utf8_lossy(&bytes);
    assert!(content.contains("SECTION"), "DXF should contain SECTION");
}

#[test]
fn test_calc_column_widths() {
    let sections = vec![
        make_test_section("NO.1"),
        make_test_section("NO.2"),
    ];
    let rows_per_column = 1;
    let (left, right) = calc_column_widths(&sections, 2, rows_per_column);
    assert_eq!(left.len(), 2);
    assert_eq!(right.len(), 2);
    assert!((left[0] - 3.0).abs() < 0.001);
    assert!((right[0] - 3.0).abs() < 0.001);
}
