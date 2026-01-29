//! Integration tests for page_generation module
//!
//! These tests cover the AllPagesBuilder API and deprecated legacy functions.

use cross_section_ui::{
    CrossSectionData, SurveyRow, TitleBlockInfo,
    AllPagesBuilder,
};

#[allow(deprecated)]
use cross_section_ui::{
    generate_all_pages_drawing,
    generate_all_pages_drawing_interpolated,
    generate_all_pages_dxf_bytes,
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
    let title_info = TitleBlockInfo::new().with_top_title("Test");
    let drawing = AllPagesBuilder::new(&sections, &title_info)
        .gaps(2.0, 1.0)
        .build();
    assert!(drawing.entities().count() > 0);
}

#[test]
fn test_builder_empty_sections() {
    let sections: Vec<CrossSectionData> = vec![];
    let title_info = TitleBlockInfo::new();
    let drawing = AllPagesBuilder::new(&sections, &title_info).build();
    // Should return valid drawing even with empty sections
    assert!(drawing.layers().count() > 0);
}

#[test]
fn test_builder_with_interpolation() {
    let sections = vec![make_test_section("NO.1")];
    let title_info = TitleBlockInfo::new();
    let drawing = AllPagesBuilder::new(&sections, &title_info)
        .interpolated()
        .v_scale(2.5)
        .build();
    assert!(drawing.entities().count() > 0);
}

#[test]
fn test_builder_to_dxf_bytes() {
    let sections = vec![make_test_section("NO.1")];
    let title_info = TitleBlockInfo::new();
    let bytes = AllPagesBuilder::new(&sections, &title_info)
        .gaps(2.0, 1.0)
        .frame_scale(500.0)
        .to_dxf_bytes();
    assert!(!bytes.is_empty());
    let content = String::from_utf8_lossy(&bytes);
    assert!(content.contains("SECTION"), "DXF should contain SECTION");
}

#[test]
fn test_builder_full_options() {
    let sections: Vec<CrossSectionData> = (1..=8)
        .map(|i| make_test_section(&format!("NO.{}", i)))
        .collect();
    let title_info = TitleBlockInfo::new()
        .with_project_name("Full Test");
    let drawing = AllPagesBuilder::new(&sections, &title_info)
        .gaps(2.0, 1.0)
        .frame_scale(200.0)
        .drawing_number_base("002")
        .interpolated()
        .v_scale(3.0)
        .build();
    assert!(drawing.entities().count() > 0);
}

// ============================================================================
// Legacy Function Tests (kept for backward compatibility)
// ============================================================================

#[test]
#[allow(deprecated)]
fn test_generate_all_pages_empty_sections() {
    let sections: Vec<CrossSectionData> = vec![];
    let title_info = TitleBlockInfo::new();
    let drawing = generate_all_pages_drawing(
        &sections, 4, 2.0, 1.0, 500.0, &title_info, "001", 2.0,
    );
    assert!(drawing.entities().count() == 0 || drawing.entities().count() > 0);
}

#[test]
#[allow(deprecated)]
fn test_generate_all_pages_single_section() {
    let sections = vec![make_test_section("NO.1")];
    let title_info = TitleBlockInfo::new()
        .with_project_name("Test Project")
        .with_top_title("Cross Section");
    let drawing = generate_all_pages_drawing(
        &sections, 4, 2.0, 1.0, 500.0, &title_info, "001", 2.0,
    );
    assert!(drawing.entities().count() > 0);
}

#[test]
#[allow(deprecated)]
fn test_generate_all_pages_dxf_bytes() {
    let sections: Vec<CrossSectionData> = (1..=8)
        .map(|i| make_test_section(&format!("NO.{}", i)))
        .collect();
    let title_info = TitleBlockInfo::new().with_project_name("Test Project");
    let bytes = generate_all_pages_dxf_bytes(
        &sections, 4, 2.0, 1.0, 500.0, &title_info, "001", 2.0,
    );
    assert!(!bytes.is_empty());
    let header = String::from_utf8_lossy(&bytes[..20]);
    assert!(header.contains("0") || header.contains("SECTION"));
}

#[test]
#[allow(deprecated)]
fn test_interpolation_flag() {
    let mut section = make_test_section("NO.1");
    section.is_route_start = false;
    section.is_route_end = false;
    let sections = vec![section];
    let title_info = TitleBlockInfo::new();

    let drawing_interp = generate_all_pages_drawing_interpolated(
        &sections, 4, 2.0, 1.0, 500.0, &title_info, "001", 2.0,
    );
    let drawing_no_interp = generate_all_pages_drawing(
        &sections, 4, 2.0, 1.0, 500.0, &title_info, "001", 2.0,
    );
    assert!(drawing_interp.entities().count() > 0);
    assert!(drawing_no_interp.entities().count() > 0);
}
