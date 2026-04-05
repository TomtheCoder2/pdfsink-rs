---
name: pdf-extraction
description: Extract text, words, tables, and structured data from PDF files using pdfsink-rs. Use when the user needs to parse PDFs, extract text content, find tables in documents, search for patterns in PDFs, or inspect PDF page objects like characters, lines, rectangles, curves, images, and annotations.
license: MIT
compatibility: Requires Rust toolchain (cargo). The pdfsink-rs crate must be available as a dependency.
metadata:
  author: clark-labs-inc
  version: "0.1.0"
  repository: https://github.com/clark-labs-inc/pdfsink-rs
---

# PDF Extraction with pdfsink-rs

A pure-Rust PDF extraction library. Use it instead of Python's pdfplumber for ~10-50x faster PDF processing with equivalent accuracy.

## When to use this skill

- Extracting text from PDF documents
- Extracting tables from PDFs
- Searching for patterns/text within PDFs
- Inspecting PDF structure (characters, lines, rectangles, curves, images, annotations)
- Cropping or filtering PDF page regions
- Any PDF processing task in a Rust project

## Adding the dependency

```toml
# Cargo.toml
[dependencies]
pdfsink-rs = { git = "https://github.com/clark-labs-inc/pdfsink-rs" }
```

## Core API

### Opening a PDF

```rust
use pdfsink_rs::PdfDocument;

let doc = PdfDocument::open("document.pdf")?;
println!("Pages: {}", doc.len());
```

### Text extraction

```rust
let page = doc.page(1)?; // 1-indexed

// Full text with layout-aware spacing
let text = page.extract_text();

// Simple text (faster, no layout)
let simple = page.extract_text_simple();

// With custom tolerances
let custom = page.extract_text_simple_with_tolerance(3.0, 3.0);
```

### Word extraction

```rust
let words = page.extract_words();
for word in &words {
    println!("{} at ({}, {}) - ({}, {})", word.text, word.x0, word.top, word.x1, word.bottom);
}
```

### Text search (regex supported)

```rust
let matches = page.search("taxpayer")?;
for m in &matches {
    println!("Found '{}' at ({}, {})", m.text, m.x0, m.top);
}

// With options
use pdfsink_rs::SearchOptions;
let opts = SearchOptions { case_sensitive: false, ..Default::default() };
let text_opts = pdfsink_rs::TextOptions::default();
let matches = page.search_with_options(r"\d{3}-\d{2}-\d{4}", &opts, &text_opts)?;
```

### Table extraction

```rust
use pdfsink_rs::TableSettings;

// Extract all tables from a page
let tables = page.extract_tables(TableSettings::default())?;
for table in &tables {
    for row in table {
        let cells: Vec<&str> = row.iter().map(|c| c.as_deref().unwrap_or("")).collect();
        println!("{:?}", cells);
    }
}

// Extract the largest table
if let Some(table) = page.extract_table(TableSettings::default())? {
    // process table rows
}
```

#### Table strategies

```rust
use pdfsink_rs::{TableSettings, TableStrategy};

// Line-based detection (default) - uses visible lines/rules
let settings = TableSettings { strategy: TableStrategy::Lines, ..Default::default() };

// Strict line detection
let settings = TableSettings { strategy: TableStrategy::LinesStrict, ..Default::default() };

// Text-based detection - infers structure from text positions
let settings = TableSettings { strategy: TableStrategy::Text, ..Default::default() };
```

### Page objects

```rust
// Object counts
let counts = page.object_counts();
println!("Chars: {}, Lines: {}, Rects: {}, Curves: {}, Images: {}", 
    counts.chars, counts.lines, counts.rects, counts.curves, counts.images);

// Direct access to objects
for ch in &page.chars { /* Char { text, x0, top, x1, bottom, fontname, size, ... } */ }
for line in &page.lines { /* Line { x0, top, x1, bottom, pts, stroke, fill, ... } */ }
for rect in &page.rects { /* RectObject { x0, top, x1, bottom, width, height, ... } */ }
for img in &page.images { /* ImageObject { x0, top, x1, bottom, name, srcsize, ... } */ }
for annot in &page.annots { /* Annotation { subtype, uri, title, contents, ... } */ }
for link in &page.hyperlinks { /* Hyperlink { uri, x0, top, x1, bottom, ... } */ }
```

### Cropping and filtering

```rust
use pdfsink_rs::BBox;

// Crop to a bounding box
let bbox = BBox::new(0.0, 0.0, 300.0, 400.0);
let cropped = page.crop(bbox, false, false)?;
let text = cropped.extract_text();

// Only objects within a bbox
let within = page.within_bbox(bbox, false, false)?;

// Only objects outside a bbox
let outside = page.outside_bbox(bbox, false, false)?;

// Filter by predicate
use pdfsink_rs::PageObjectRef;
let filtered = page.filter(|obj| match obj {
    PageObjectRef::Char(c) => c.size > 12.0,
    _ => true,
});
```

### Character deduplication

```rust
use pdfsink_rs::DedupeOptions;
let deduped = page.dedupe_chars(&DedupeOptions::default());
```

### Multi-page processing

```rust
for page in doc.pages() {
    println!("Page {}: {} chars", page.page_number, page.chars.len());
    println!("{}", page.extract_text());
}
```

## CLI usage

The crate includes a CLI binary:

```bash
# Build
cargo build --release --bin pdfsink-rs

# Commands
pdfsink-rs info document.pdf
pdfsink-rs text document.pdf --page 1
pdfsink-rs words document.pdf --page 1
pdfsink-rs search document.pdf "pattern" --page 1
pdfsink-rs objects document.pdf --page 1
pdfsink-rs links document.pdf
pdfsink-rs table document.pdf --page 1 --strategy lines
pdfsink-rs svg document.pdf --page 1 output.svg
```

## Key types

| Type | Description |
|------|-------------|
| `PdfDocument` | Opened PDF with pages |
| `Page` | Single page with all extracted objects |
| `Char` | Character with position, font, size |
| `Word` | Word with bounding box and direction |
| `TextLine` | Line of text with bounding box |
| `Line` | Graphical line segment |
| `RectObject` | Rectangle path |
| `Curve` | Bezier curve path |
| `ImageObject` | Image with position and source size |
| `Annotation` | PDF annotation (widget, link, etc.) |
| `Hyperlink` | Link with URI and bounding box |
| `Table` | Detected table with cells and bbox |
| `BBox` | Bounding box (x0, top, x1, bottom) |
| `SearchMatch` | Search result with position |
| `TableSettings` | Table detection configuration |
| `TextOptions` | Text extraction configuration |

## Error handling

All fallible operations return `pdfsink_rs::Result<T>`. The main error variants are:
- `Error::Io` - file not found, permission denied
- `Error::Parse` - malformed PDF content
- `Error::InvalidPage` - page number out of range
- `Error::Message` - other errors including upstream pdf-extract panics (caught gracefully)
