use crate::geometry::objects_to_bbox;
use crate::table::{table_rows_to_csv, TableSettings};
use crate::text::{chars_to_textmap, dedupe_chars, extract_text_lines, extract_words, DedupeOptions, SearchOptions, TextOptions};
use crate::types::{BBox, ObjectCounts, Page, PdfDocument, SearchMatch, TextLine, Word};
use crate::{Error, Result};
use std::collections::BTreeMap;

impl PdfDocument {
    pub fn object_counts(&self) -> ObjectCounts {
        self.pages
            .iter()
            .map(Page::object_counts)
            .fold(ObjectCounts::default(), |mut acc, counts| {
                acc += counts;
                acc
            })
    }

    pub fn extract_text(&self) -> String {
        self.extract_text_with_page_separator("\n\n")
    }

    pub fn extract_text_with_page_separator(&self, page_separator: &str) -> String {
        let options = TextOptions::default();
        self.extract_text_with_options(&options, page_separator)
    }

    pub fn extract_text_with_options(&self, options: &TextOptions, page_separator: &str) -> String {
        self.pages
            .iter()
            .map(|page| page.extract_text_with_options(&page_scoped_text_options(page, options)))
            .collect::<Vec<_>>()
            .join(page_separator)
    }

    pub fn extract_words(&self) -> Vec<Word> {
        let options = TextOptions::default();
        self.extract_words_with_options(&options, false)
    }

    pub fn extract_words_with_options(&self, options: &TextOptions, return_chars: bool) -> Vec<Word> {
        self.pages
            .iter()
            .flat_map(|page| extract_words(&page.chars, &page_scoped_text_options(page, options), return_chars))
            .collect()
    }

    pub fn extract_text_lines(&self, strip: bool, return_chars: bool) -> Vec<TextLine> {
        let options = TextOptions::default();
        self.extract_text_lines_with_options(&options, strip, return_chars)
    }

    pub fn extract_text_lines_with_options(&self, options: &TextOptions, strip: bool, return_chars: bool) -> Vec<TextLine> {
        self.pages
            .iter()
            .flat_map(|page| extract_text_lines(&page.chars, &page_scoped_text_options(page, options), strip, return_chars))
            .collect()
    }

    pub fn search(&self, pattern: &str) -> Result<Vec<SearchMatch>> {
        let search_options = SearchOptions::default();
        let text_options = TextOptions::default();
        self.search_with_options(pattern, &search_options, &text_options)
    }

    pub fn search_literal(&self, pattern: &str, case_sensitive: bool) -> Result<Vec<SearchMatch>> {
        let mut search_options = SearchOptions::literal();
        search_options.case_sensitive = case_sensitive;
        let text_options = TextOptions::default();
        self.search_with_options(pattern, &search_options, &text_options)
    }

    pub fn search_with_options(&self, pattern: &str, options: &SearchOptions, text_options: &TextOptions) -> Result<Vec<SearchMatch>> {
        let regex = if options.regex {
            regex::RegexBuilder::new(pattern)
                .case_insensitive(!options.case_sensitive)
                .build()?
        } else {
            regex::RegexBuilder::new(&regex::escape(pattern))
                .case_insensitive(!options.case_sensitive)
                .build()?
        };

        let mut matches = Vec::new();
        for page in &self.pages {
            let page_options = page_scoped_text_options(page, text_options);
            let textmap = chars_to_textmap(&page.chars, &page_options);
            matches.extend(textmap.search_compiled(&regex, options));
        }
        Ok(matches)
    }

    pub fn dedupe_chars(&self, options: &DedupeOptions) -> Self {
        let mut doc = self.clone();
        for page in &mut doc.pages {
            page.chars = dedupe_chars(&page.chars, options);
            page.is_original = false;
        }
        doc
    }

    pub fn filter_pages<F>(&self, mut predicate: F) -> Self
    where
        F: FnMut(&Page) -> bool,
    {
        let mut doc = self.clone();
        doc.pages = self.pages.iter().filter(|page| predicate(page)).cloned().collect();
        renumber_document_pages(&mut doc);
        doc
    }

    pub fn select_pages<I>(&self, page_numbers: I) -> Result<Self>
    where
        I: IntoIterator<Item = usize>,
    {
        let mut pages = Vec::new();
        for page_number in page_numbers {
            if page_number == 0 || page_number > self.pages.len() {
                return Err(Error::InvalidPage { page_number });
            }
            pages.push(self.pages[page_number - 1].clone());
        }
        Ok(self.with_pages(pages))
    }

    pub fn pages_range(&self, start_page: usize, end_page_inclusive: usize) -> Result<Self> {
        if start_page == 0 || end_page_inclusive == 0 || start_page > end_page_inclusive {
            return Err(Error::InvalidPage { page_number: start_page });
        }
        if end_page_inclusive > self.pages.len() {
            return Err(Error::InvalidPage { page_number: end_page_inclusive });
        }
        Ok(self.with_pages(self.pages[start_page - 1..end_page_inclusive].to_vec()))
    }

    pub fn reverse_pages(&self) -> Self {
        let mut pages = self.pages.clone();
        pages.reverse();
        self.with_pages(pages)
    }

    pub fn split_every(&self, chunk_size: usize) -> Result<Vec<Self>> {
        if chunk_size == 0 {
            return Err(Error::Message("chunk_size must be greater than zero".to_string()));
        }
        Ok(self
            .pages
            .chunks(chunk_size)
            .map(|chunk| self.with_pages(chunk.to_vec()))
            .collect())
    }

    pub fn append_document(&self, other: &PdfDocument) -> Self {
        let mut pages = self.pages.clone();
        pages.extend(other.pages.clone());
        self.with_pages(pages)
    }

    pub fn prepend_document(&self, other: &PdfDocument) -> Self {
        let mut pages = other.pages.clone();
        pages.extend(self.pages.clone());
        self.with_pages(pages)
    }

    pub fn with_pages(&self, pages: Vec<Page>) -> Self {
        let mut doc = self.clone();
        doc.pages = pages;
        renumber_document_pages(&mut doc);
        doc
    }
}

impl Page {
    pub fn content_bbox(&self) -> Option<BBox> {
        let mut bbox = objects_to_bbox(&self.chars);
        merge_object_bbox(&mut bbox, objects_to_bbox(&self.lines));
        merge_object_bbox(&mut bbox, objects_to_bbox(&self.rects));
        merge_object_bbox(&mut bbox, objects_to_bbox(&self.curves));
        merge_object_bbox(&mut bbox, objects_to_bbox(&self.images));
        merge_object_bbox(&mut bbox, objects_to_bbox(&self.annots));
        merge_object_bbox(&mut bbox, objects_to_bbox(&self.hyperlinks));
        bbox
    }

    pub fn text_bbox(&self) -> Option<BBox> {
        objects_to_bbox(&self.chars)
    }

    pub fn graphics_bbox(&self) -> Option<BBox> {
        let mut bbox = objects_to_bbox(&self.lines);
        merge_object_bbox(&mut bbox, objects_to_bbox(&self.rects));
        merge_object_bbox(&mut bbox, objects_to_bbox(&self.curves));
        merge_object_bbox(&mut bbox, objects_to_bbox(&self.images));
        bbox
    }

    pub fn crop_with_margin(&self, bbox: BBox, x_margin: f64, y_margin: f64, relative: bool, strict: bool) -> Result<Self> {
        self.crop(bbox.expand(x_margin, y_margin), relative, strict)
    }

    pub fn within_bbox_with_tolerance(&self, bbox: BBox, tolerance: f64, relative: bool, strict: bool) -> Result<Self> {
        self.within_bbox(bbox.pad(tolerance), relative, strict)
    }

    pub fn outside_bbox_with_tolerance(&self, bbox: BBox, tolerance: f64, relative: bool, strict: bool) -> Result<Self> {
        self.outside_bbox(bbox.pad(tolerance), relative, strict)
    }

    pub fn trim_to_content(&self) -> Result<Self> {
        match self.content_bbox() {
            Some(bbox) => self.crop(bbox, false, false),
            None => Ok(self.clone()),
        }
    }

    pub fn extract_text_in_bbox(&self, bbox: BBox) -> Result<String> {
        let options = self.default_text_options();
        self.extract_text_in_bbox_with_options(bbox, &options)
    }

    pub fn extract_text_in_bbox_with_options(&self, bbox: BBox, options: &TextOptions) -> Result<String> {
        let cropped = self.crop(bbox, false, false)?;
        let mut text_options = options.clone();
        if text_options.layout_bbox.is_none() {
            text_options.layout_bbox = Some(cropped.bbox);
        }
        if text_options.layout_width.is_none() {
            text_options.layout_width = Some(cropped.bbox.width());
        }
        if text_options.layout_height.is_none() {
            text_options.layout_height = Some(cropped.bbox.height());
        }
        Ok(cropped.extract_text_with_options(&text_options))
    }

    pub fn extract_words_in_bbox(&self, bbox: BBox, options: &TextOptions, return_chars: bool) -> Result<Vec<Word>> {
        let cropped = self.crop(bbox, false, false)?;
        let mut text_options = options.clone();
        if text_options.layout_bbox.is_none() {
            text_options.layout_bbox = Some(cropped.bbox);
        }
        Ok(extract_words(&cropped.chars, &text_options, return_chars))
    }

    pub fn extract_words_outside_bbox(&self, bbox: BBox, options: &TextOptions, return_chars: bool) -> Result<Vec<Word>> {
        let cropped = self.outside_bbox(bbox, false, false)?;
        Ok(extract_words(&cropped.chars, options, return_chars))
    }

    pub fn search_literal(&self, pattern: &str, case_sensitive: bool) -> Result<Vec<SearchMatch>> {
        let mut options = SearchOptions::literal();
        options.case_sensitive = case_sensitive;
        self.search_with_options(pattern, &options, &self.default_text_options())
    }

    pub fn search_case_insensitive(&self, pattern: &str) -> Result<Vec<SearchMatch>> {
        let options = SearchOptions::default().case_insensitive();
        self.search_with_options(pattern, &options, &self.default_text_options())
    }

    pub fn deduped_text(&self, dedupe_options: &DedupeOptions, text_options: &TextOptions) -> String {
        self.dedupe_chars(dedupe_options).extract_text_with_options(text_options)
    }

    pub fn extract_table_csv(&self, settings: TableSettings) -> Result<Option<String>> {
        self.extract_table(settings)?
            .map(|rows| table_rows_to_csv(&rows))
            .transpose()
    }

    pub fn extract_tables_csv(&self, settings: TableSettings) -> Result<Vec<String>> {
        self.extract_tables(settings)?
            .iter()
            .map(|rows| table_rows_to_csv(rows))
            .collect()
    }
}

fn page_scoped_text_options(page: &Page, options: &TextOptions) -> TextOptions {
    let mut scoped = options.clone();
    if scoped.layout_bbox.is_none() {
        scoped.layout_bbox = Some(page.bbox);
    }
    if scoped.layout_width.is_none() {
        scoped.layout_width = Some(page.width);
    }
    if scoped.layout_height.is_none() {
        scoped.layout_height = Some(page.height);
    }
    scoped
}

fn merge_object_bbox(current: &mut Option<BBox>, next: Option<BBox>) {
    if let Some(next) = next {
        *current = Some(current.map(|bbox| bbox.union(next)).unwrap_or(next));
    }
}

fn renumber_document_pages(doc: &mut PdfDocument) {
    let mut doctop_offset = 0.0;
    let mut page_number_map = BTreeMap::new();
    for (idx, page) in doc.pages.iter_mut().enumerate() {
        let page_number = idx + 1;
        let old_page_number = page.page_number;
        let old_doctop_offset = page.doctop_offset;
        let doctop_delta = doctop_offset - old_doctop_offset;
        page_number_map.insert(old_page_number, page_number);
        page.page_number = page_number;
        page.doctop_offset = doctop_offset;
        page.is_original = false;

        renumber_page_objects(&mut page.chars, page_number, doctop_delta);
        renumber_page_objects(&mut page.lines, page_number, doctop_delta);
        renumber_page_objects(&mut page.rects, page_number, doctop_delta);
        renumber_page_objects(&mut page.curves, page_number, doctop_delta);
        renumber_page_objects(&mut page.images, page_number, doctop_delta);
        renumber_page_objects(&mut page.annots, page_number, doctop_delta);
        renumber_page_objects(&mut page.hyperlinks, page_number, doctop_delta);

        if let Some(structure_tree) = &mut page.structure_tree {
            update_structure_page_numbers(structure_tree, page_number);
        }

        doctop_offset += page.height;
    }

    if let Some(structure_tree) = &mut doc.structure_tree {
        remap_structure_page_numbers(structure_tree, &page_number_map);
    }
}

fn renumber_page_objects<T>(objects: &mut [T], page_number: usize, doctop_delta: f64)
where
    T: PageNumbered,
{
    for object in objects {
        object.set_page_number(page_number);
        object.shift_doctop(doctop_delta);
    }
}

fn update_structure_page_numbers(node: &mut crate::types::StructureElement, page_number: usize) {
    if node.page_number.is_some() {
        node.page_number = Some(page_number);
    }
    for child in &mut node.children {
        update_structure_page_numbers(child, page_number);
    }
}

fn remap_structure_page_numbers(node: &mut crate::types::StructureElement, page_number_map: &BTreeMap<usize, usize>) {
    if let Some(page_number) = node.page_number {
        node.page_number = page_number_map.get(&page_number).copied();
    }
    for child in &mut node.children {
        remap_structure_page_numbers(child, page_number_map);
    }
}

trait PageNumbered {
    fn set_page_number(&mut self, page_number: usize);
    fn shift_doctop(&mut self, doctop_delta: f64);
}

macro_rules! impl_page_numbered {
    ($($ty:ty),* $(,)?) => {
        $(
            impl PageNumbered for $ty {
                fn set_page_number(&mut self, page_number: usize) {
                    self.page_number = page_number;
                }

                fn shift_doctop(&mut self, doctop_delta: f64) {
                    self.doctop += doctop_delta;
                }
            }
        )*
    };
}

impl_page_numbered!(
    crate::types::Char,
    crate::types::Line,
    crate::types::RectObject,
    crate::types::Curve,
    crate::types::ImageObject,
    crate::types::Annotation,
    crate::types::Hyperlink,
);
