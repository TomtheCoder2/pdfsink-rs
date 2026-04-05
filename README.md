# pdfsink-rs

A native pure-Rust PDF extraction library inspired by [pdfplumber](https://github.com/jsvine/pdfplumber). Drop-in conceptual replacement — same capabilities, **~10-50x faster**.

## Benchmarks: pdfsink-rs vs pdfplumber

Tested against pdfplumber 0.11.8 on real-world government PDFs.

### Speed

| PDF | Pages | Size | pdfsink-rs | pdfplumber | Speedup |
|-----|------:|-----:|-----------:|-----------:|--------:|
| US Budget FY2025 | 188 | 2.4 MB | 775 ms | 11.1 s | **14x** |
| NIST SP 800-53 | 492 | 6.1 MB | 4.07 s | 33.9 s | **8x** |
| IRS W-9 | 6 | 138 KB | 31 ms | 509 ms | **17x** |
| UN Charter | 54 | 3.0 MB | 130 ms | 1.95 s | **15x** |
| Census Table | 1 | 58 KB | 4.0 ms | 40 ms | **10x** |
| **Total (12 PDFs)** | | | **5.0 s** | **47.5 s** | **9.5x** |

Text extraction alone is **~50x faster**. Table extraction is **~250x faster**.

### Accuracy

| Metric | Result |
|--------|--------|
| Text similarity vs pdfplumber | **99.7%** |
| Word count match | **21/21 pages** |
| Character count match | **exact on all PDFs** |
| Page dimensions | **exact on all PDFs** |
| Line/rect object counts | **exact on matching PDFs** |
| Table detection (simple PDFs) | **1:1 match** |

## Features

- Open a PDF and access pages
- Inspect page objects (`chars`, `lines`, `rects`, `curves`, `images`, `annots`, `hyperlinks`)
- Crop / within-bbox / outside-bbox filtering
- Text extraction, word extraction, line extraction, regex search
- Table finding and table extraction (lines, lines_strict, text, explicit strategies)
- CLI for inspection and debugging

## Example

```rust
use pdfsink_rs::PdfDocument;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pdf = PdfDocument::open("document.pdf")?;
    let page = pdf.page(1)?;

    // Extract text
    println!("{}", page.extract_text());

    // Extract words with positions
    for word in page.extract_words() {
        println!("{} @ ({}, {})", word.text, word.x0, word.top);
    }

    // Extract tables
    use pdfsink_rs::TableSettings;
    if let Some(table) = page.extract_table(TableSettings::default())? {
        for row in &table {
            println!("{:?}", row);
        }
    }

    Ok(())
}
```

## CLI

```text
pdfsink-rs info <file.pdf>
pdfsink-rs text <file.pdf> [--page N]
pdfsink-rs words <file.pdf> [--page N]
pdfsink-rs search <file.pdf> <pattern> [--page N]
pdfsink-rs objects <file.pdf> [--page N]
pdfsink-rs links <file.pdf> [--page N]
pdfsink-rs table <file.pdf> [--page N] [--strategy lines|lines_strict|text|explicit]
pdfsink-rs svg <file.pdf> [--page N] [output.svg]
```

## Architecture

Built on `lopdf` for PDF parsing and `pdf-extract` for content stream processing. No Python runtime dependency.

| File | Purpose |
|------|---------|
| `src/lib.rs` | Public API (PdfDocument, Page methods) |
| `src/parse.rs` | PDF parsing, page-object extraction |
| `src/text.rs` | Text/word extraction, search, layout |
| `src/table.rs` | Table detection and extraction |
| `src/geometry.rs` | Bbox operations, cropping, filtering |
| `src/clustering.rs` | Value clustering for layout analysis |

## Running Tests

```bash
cargo test
```

## Running Benchmarks

```bash
# Rust
cargo run --release --example bench_pdfsink

# pdfplumber (requires: pip install pdfplumber)
python bench/bench_pdfplumber.py

# Compare
python bench/compare.py
```

## License

MIT
