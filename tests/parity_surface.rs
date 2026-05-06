use pdfsink_rs::{PdfDocument, TableSettings};
use std::path::{Path, PathBuf};

fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn fixture_path(name: &str) -> PathBuf {
    project_root().join("tests/fixtures").join(name)
}

#[test]
fn aggregates_and_serializers_have_output() {
    let pdf = PdfDocument::open(fixture_path("objects_showcase.pdf")).expect("open fixture");
    let page = pdf.page(1).expect("page 1");

    assert!(!pdf.chars().is_empty());
    assert!(pdf.lines().len() >= page.lines.len());
    assert!(page.objects().contains_key("char"));
    assert!(page.objects().contains_key("annot"));
    assert!(page.to_dict(None).is_object());

    let json = page
        .to_json::<Vec<u8>>(None, None, None, None, Some(2), Some(2))
        .expect("page json")
        .expect("page json string");
    assert!(json.contains("\"page_number\""));
    assert!(json.contains("\"chars\""));

    let csv = page
        .to_csv::<Vec<u8>>(None, None, Some(2), None, None)
        .expect("page csv")
        .expect("page csv string");
    assert!(csv.contains("object_type"));

    let pdf_json = pdf
        .to_json::<Vec<u8>>(None, None, None, None, Some(2), Some(2))
        .expect("pdf json")
        .expect("pdf json string");
    assert!(pdf_json.contains("\"pages\""));
}

#[test]
fn layout_edges_and_rendering_are_available() {
    let pdf = PdfDocument::open(fixture_path("table_lines.pdf")).expect("open fixture");
    let page = pdf.page(1).expect("page 1");

    assert!(!page.edges().is_empty());
    assert!(!page.horizontal_edges().is_empty());
    assert!(!page.vertical_edges().is_empty());
    assert!(!page.textlinehorizontals().is_empty());
    assert!(!page.textboxhorizontals().is_empty());

    let mut image = page
        .to_image(Some(96.0), None, None, false, false)
        .expect("page image");
    assert!(image.width() > 0);
    assert!(image.height() > 0);
    image
        .debug_tablefinder(Some(TableSettings::default()))
        .expect("debug tablefinder overlay");
}

#[test]
fn layout_edges_and_rendering_are_available_mem() {
    // open fixture_path("table_lines.pdf") and save to a memory buffer
    let path = fixture_path("table_lines.pdf");
    let buffer = std::fs::read(path).expect("read fixture");
    let pdf = PdfDocument::open_from_mem("path".to_string(), &*buffer).expect("open fixture");
    let page = pdf.page(1).expect("page 1");

    assert!(!page.edges().is_empty());
    assert!(!page.horizontal_edges().is_empty());
    assert!(!page.vertical_edges().is_empty());
    assert!(!page.textlinehorizontals().is_empty());
    assert!(!page.textboxhorizontals().is_empty());

    let mut image = page
        .to_image(Some(96.0), None, None, false, false)
        .expect("page image");
    assert!(image.width() > 0);
    assert!(image.height() > 0);
    image
        .debug_tablefinder(Some(TableSettings::default()))
        .expect("debug tablefinder overlay");
}
