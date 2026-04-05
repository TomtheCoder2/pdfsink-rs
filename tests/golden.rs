use pdfsink_rs::{BBox, PdfDocument, TableSettings, TableStrategy};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};


#[derive(Debug, Deserialize)]
struct FixtureGolden {
    file: String,
    page_count: usize,
    pages: Vec<PageGolden>,
}

#[derive(Debug, Deserialize)]
struct PageGolden {
    page_number: usize,
    rotation: i32,
    width: f64,
    height: f64,
    text: String,
    word_texts: Vec<String>,
    object_counts: ObjectCountsGolden,
    #[serde(default)]
    search_second: Vec<SearchGolden>,
    #[serde(default)]
    crop_cases: Option<CropCasesGolden>,
    #[serde(default)]
    default_table: Option<Vec<Vec<Option<String>>>>,
    #[serde(default)]
    default_table_bbox: Option<BBoxGolden>,
    #[serde(default)]
    table_count: Option<usize>,
    #[serde(default)]
    text_table: Option<Vec<Vec<Option<String>>>>,
    #[serde(default)]
    text_table_count: Option<usize>,
    #[serde(default)]
    line0: Option<ObjectGolden>,
    #[serde(default)]
    rect0: Option<ObjectGolden>,
    #[serde(default)]
    curve0: Option<ObjectGolden>,
    #[serde(default)]
    image0: Option<ImageGolden>,
    #[serde(default)]
    hyperlinks: Vec<HyperlinkGolden>,
    #[serde(default)]
    deduped_char_count: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct ObjectCountsGolden {
    chars: usize,
    lines: usize,
    rects: usize,
    curves: usize,
    images: usize,
    annots: usize,
    hyperlinks: usize,
}

#[derive(Debug, Deserialize)]
struct SearchGolden {
    text: String,
    bbox: BBoxGolden,
    char_count: usize,
}

#[derive(Debug, Deserialize)]
struct CropCasesGolden {
    left_half_crop_text: String,
    left_half_outside_text: String,
}

#[derive(Debug, Deserialize)]
struct BBoxGolden {
    x0: f64,
    top: f64,
    x1: f64,
    bottom: f64,
}

#[derive(Debug, Deserialize)]
struct ObjectGolden {
    x0: f64,
    top: f64,
    x1: f64,
    bottom: f64,
    width: f64,
    height: f64,
}

#[derive(Debug, Deserialize)]
struct ImageGolden {
    x0: f64,
    top: f64,
    x1: f64,
    bottom: f64,
    width: f64,
    height: f64,
    srcsize: Vec<u32>,
    name: String,
}

#[derive(Debug, Deserialize)]
struct HyperlinkGolden {
    x0: f64,
    top: f64,
    x1: f64,
    bottom: f64,
    width: f64,
    height: f64,
    uri: String,
}

fn goldens() -> HashMap<String, FixtureGolden> {
    let path = project_root().join("tests/goldens/goldens.json");
    let raw = fs::read_to_string(path).expect("read goldens.json");
    serde_json::from_str::<HashMap<String, FixtureGolden>>(&raw).expect("parse goldens.json")
}

fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn fixture_path(name: &str) -> PathBuf {
    project_root().join("tests/fixtures").join(name)
}

fn open_fixture(file: &str) -> PdfDocument {
    PdfDocument::open(fixture_path(file)).expect("open fixture")
}

fn approx_eq(a: f64, b: f64, tol: f64) {
    let delta = (a - b).abs();
    assert!(
        delta <= tol,
        "expected {a} ~= {b} within {tol}, delta={delta}"
    );
}

fn assert_bbox(actual: pdfsink_rs::BBox, expected: &BBoxGolden, tol: f64) {
    approx_eq(actual.x0, expected.x0, tol);
    approx_eq(actual.top, expected.top, tol);
    approx_eq(actual.x1, expected.x1, tol);
    approx_eq(actual.bottom, expected.bottom, tol);
}

fn assert_counts(actual: pdfsink_rs::ObjectCounts, expected: &ObjectCountsGolden) {
    assert_eq!(actual.chars, expected.chars);
    assert_eq!(actual.lines, expected.lines);
    assert_eq!(actual.rects, expected.rects);
    assert_eq!(actual.curves, expected.curves);
    assert_eq!(actual.images, expected.images);
    assert_eq!(actual.annots, expected.annots);
    assert_eq!(actual.hyperlinks, expected.hyperlinks);
}

#[test]
fn simple_text_matches_golden() {
    let goldens = goldens();
    let fixture = goldens.get("simple_text").unwrap();
    let pdf = open_fixture(&fixture.file);
    assert_eq!(pdf.len(), fixture.page_count);

    let page = pdf.page(1).unwrap();
    let expected = &fixture.pages[0];
    assert_eq!(page.page_number, expected.page_number);
    assert_eq!(page.rotation, expected.rotation);
    approx_eq(page.width, expected.width, 0.01);
    approx_eq(page.height, expected.height, 0.01);
    assert_eq!(page.extract_text(), expected.text.as_str());
    assert_eq!(
        page.extract_words()
            .into_iter()
            .map(|w| w.text)
            .collect::<Vec<_>>(),
        expected.word_texts.clone()
    );
    assert_counts(page.object_counts(), &expected.object_counts);

    let matches = page.search("second").unwrap();
    assert_eq!(matches.len(), expected.search_second.len());
    let actual = &matches[0];
    let wanted = &expected.search_second[0];
    assert_eq!(actual.text, wanted.text.as_str());
    assert_bbox(BBox::new(actual.x0, actual.top, actual.x1, actual.bottom), &wanted.bbox, 2.0);
    assert_eq!(actual.chars.as_ref().map(|chars| chars.len()), Some(wanted.char_count));
}

#[test]
fn crop_regions_behaviour_matches_golden() {
    let goldens = goldens();
    let fixture = goldens.get("crop_regions").unwrap();
    let pdf = open_fixture(&fixture.file);
    let page = pdf.page(1).unwrap();
    let expected = &fixture.pages[0];

    assert_eq!(page.extract_text(), expected.text.as_str());
    assert_counts(page.object_counts(), &expected.object_counts);

    let crop_cases = expected.crop_cases.as_ref().expect("crop cases present");
    let left = page
        .crop(BBox::new(0.0, 0.0, page.width / 2.0, page.height), false, true)
        .unwrap();
    let outside = page
        .outside_bbox(BBox::new(0.0, 0.0, page.width / 2.0, page.height), false, true)
        .unwrap();

    assert_eq!(left.extract_text(), crop_cases.left_half_crop_text.as_str());
    assert_eq!(outside.extract_text(), crop_cases.left_half_outside_text.as_str());
}

#[test]
fn table_lines_matches_golden() {
    let goldens = goldens();
    let fixture = goldens.get("table_lines").unwrap();
    let pdf = open_fixture(&fixture.file);
    let page = pdf.page(1).unwrap();
    let expected = &fixture.pages[0];

    assert_eq!(page.extract_text(), expected.text.as_str());
    assert_counts(page.object_counts(), &expected.object_counts);
    assert_eq!(page.extract_table(TableSettings::default()).unwrap(), expected.default_table.clone());

    let table = page.find_table(TableSettings::default()).unwrap().expect("table");
    let bbox = BBox::new(table.bbox.x0, table.bbox.top, table.bbox.x1, table.bbox.bottom);
    assert_bbox(bbox, expected.default_table_bbox.as_ref().unwrap(), 2.0);
    assert_eq!(page.find_tables(TableSettings::default()).unwrap().len(), expected.table_count.unwrap());
}

#[test]
fn table_text_strategy_matches_golden() {
    let goldens = goldens();
    let fixture = goldens.get("table_text_only").unwrap();
    let pdf = open_fixture(&fixture.file);
    let page = pdf.page(1).unwrap();
    let expected = &fixture.pages[0];

    assert_eq!(page.extract_text(), expected.text.as_str());
    assert_counts(page.object_counts(), &expected.object_counts);
    assert_eq!(page.extract_table(TableSettings::default()).unwrap(), expected.default_table.clone());

    let mut settings = TableSettings::default();
    settings.vertical_strategy = TableStrategy::Text;
    settings.horizontal_strategy = TableStrategy::Text;

    assert_eq!(page.extract_table(settings.clone()).unwrap(), expected.text_table.clone());
    assert_eq!(page.find_tables(settings).unwrap().len(), expected.text_table_count.unwrap());
}

#[test]
fn objects_showcase_matches_golden() {
    let goldens = goldens();
    let fixture = goldens.get("objects_showcase").unwrap();
    let pdf = open_fixture(&fixture.file);
    let page = pdf.page(1).unwrap();
    let expected = &fixture.pages[0];

    assert_eq!(page.extract_text(), expected.text.as_str());
    assert_counts(page.object_counts(), &expected.object_counts);

    let line = page.lines.first().expect("line0");
    let rect = page.rects.first().expect("rect0");
    let curve = page.curves.first().expect("curve0");
    let image = page.images.first().expect("image0");
    let hyperlink = page.hyperlinks.first().expect("hyperlink0");

    let line_golden = expected.line0.as_ref().unwrap();
    let rect_golden = expected.rect0.as_ref().unwrap();
    let curve_golden = expected.curve0.as_ref().unwrap();
    let image_golden = expected.image0.as_ref().unwrap();
    let hyperlink_golden = &expected.hyperlinks[0];

    assert_bbox(BBox::new(line.x0, line.top, line.x1, line.bottom), &BBoxGolden { x0: line_golden.x0, top: line_golden.top, x1: line_golden.x1, bottom: line_golden.bottom }, 1.0);
    assert_bbox(BBox::new(rect.x0, rect.top, rect.x1, rect.bottom), &BBoxGolden { x0: rect_golden.x0, top: rect_golden.top, x1: rect_golden.x1, bottom: rect_golden.bottom }, 1.0);
    assert_bbox(BBox::new(curve.x0, curve.top, curve.x1, curve.bottom), &BBoxGolden { x0: curve_golden.x0, top: curve_golden.top, x1: curve_golden.x1, bottom: curve_golden.bottom }, 1.0);
    assert_bbox(BBox::new(image.x0, image.top, image.x1, image.bottom), &BBoxGolden { x0: image_golden.x0, top: image_golden.top, x1: image_golden.x1, bottom: image_golden.bottom }, 1.0);
    assert_bbox(BBox::new(hyperlink.x0, hyperlink.top, hyperlink.x1, hyperlink.bottom), &BBoxGolden { x0: hyperlink_golden.x0, top: hyperlink_golden.top, x1: hyperlink_golden.x1, bottom: hyperlink_golden.bottom }, 1.0);

    approx_eq(line.width, line_golden.width, 1.0);
    approx_eq(rect.width, rect_golden.width, 1.0);
    approx_eq(rect.height, rect_golden.height, 1.0);
    approx_eq(curve.width, curve_golden.width, 1.0);
    approx_eq(image.width, image_golden.width, 1.0);
    approx_eq(image.height, image_golden.height, 1.0);
    assert_eq!(image.srcsize.0, image_golden.srcsize[0]);
    assert_eq!(image.srcsize.1, image_golden.srcsize[1]);
    assert!(!image.name.is_empty());
    assert!(!image_golden.name.is_empty());
    assert_eq!(hyperlink.uri, hyperlink_golden.uri.as_str());
}

#[test]
fn rotated_and_duplicates_matches_golden() {
    let goldens = goldens();
    let fixture = goldens.get("rotated_and_duplicates").unwrap();
    let pdf = open_fixture(&fixture.file);
    let page = pdf.page(1).unwrap();
    let expected = &fixture.pages[0];

    assert_eq!(page.extract_text(), expected.text.as_str());
    assert_counts(page.object_counts(), &expected.object_counts);
    let deduped = page.dedupe_chars(&pdfsink_rs::DedupeOptions::default());
    assert_eq!(deduped.chars.len(), expected.deduped_char_count.unwrap());
}

#[test]
fn multipage_matches_golden() {
    let goldens = goldens();
    let fixture = goldens.get("multipage").unwrap();
    let pdf = open_fixture(&fixture.file);
    assert_eq!(pdf.len(), fixture.page_count);

    for expected in &fixture.pages {
        let page = pdf.page(expected.page_number).unwrap();
        assert_eq!(page.page_number, expected.page_number);
        assert_eq!(page.extract_text(), expected.text.as_str());
        assert_counts(page.object_counts(), &expected.object_counts);
    }
}
