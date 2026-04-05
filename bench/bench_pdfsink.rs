//! Comprehensive pdfsink-rs benchmark.
//! Outputs JSON with timings and extracted data for accuracy comparison.
//!
//! Build: cargo build --release --example bench_pdfsink
//! Run:   ./target/release/examples/bench_pdfsink

use pdfsink_rs::*;
use serde_json::{json, Value};
use std::fs;
use std::panic;
use std::path::{Path, PathBuf};
use std::time::Instant;

const TIMING_ITERS: u32 = 3;

fn timed<F, R>(f: F, iters: u32) -> (f64, R)
where
    F: Fn() -> R,
{
    let mut best = f64::INFINITY;
    let mut result: Option<R> = None;
    for _ in 0..iters {
        let t0 = Instant::now();
        let r = f();
        let elapsed = t0.elapsed().as_secs_f64();
        if elapsed < best {
            best = elapsed;
        }
        result = Some(r);
    }
    (best, result.unwrap())
}

fn benchmark_pdf(path: &Path) -> Value {
    let filename = path.file_name().unwrap().to_str().unwrap();
    let size_bytes = fs::metadata(path).map(|m| m.len()).unwrap_or(0);

    // 1. Open / parse timing
    let (open_time, doc) = timed(|| PdfDocument::open(path), TIMING_ITERS);
    let doc = match doc {
        Ok(d) => d,
        Err(e) => {
            return json!({
                "file": filename,
                "size_bytes": size_bytes,
                "error": format!("{}", e),
            });
        }
    };

    let num_pages = doc.len();

    // 2. Full-document text extraction
    let (text_time, full_text) = timed(
        || {
            doc.pages()
                .iter()
                .map(|p| p.extract_text())
                .collect::<Vec<_>>()
                .join("\n")
        },
        TIMING_ITERS,
    );
    let text_length = full_text.len();
    let text_preview: String = full_text.chars().take(500).collect();

    // 3. Per-page text for first 3 pages
    let mut page_texts = Vec::new();
    for i in 0..num_pages.min(3) {
        let page = &doc.pages()[i];
        let (t, txt) = timed(|| page.extract_text(), TIMING_ITERS);
        page_texts.push(json!({
            "page": i + 1,
            "time_s": t,
            "text": txt,
            "char_count": txt.len(),
        }));
    }

    // 4. Word extraction (first 3 pages)
    let mut page_words = Vec::new();
    for i in 0..num_pages.min(3) {
        let page = &doc.pages()[i];
        let (t, words) = timed(|| page.extract_words(), TIMING_ITERS);
        let words_preview: Vec<Value> = words
            .iter()
            .take(20)
            .map(|w| {
                json!({
                    "text": w.text,
                    "x0": round3(w.x0),
                    "top": round3(w.top),
                    "x1": round3(w.x1),
                    "bottom": round3(w.bottom),
                })
            })
            .collect();
        page_words.push(json!({
            "page": i + 1,
            "time_s": t,
            "word_count": words.len(),
            "words_preview": words_preview,
        }));
    }

    // 5. Character extraction (first page)
    let page1_chars = if num_pages > 0 {
        let page = &doc.pages()[0];
        let (t, _) = timed(|| page.chars.len(), TIMING_ITERS);
        let chars_preview: Vec<Value> = page
            .chars
            .iter()
            .take(30)
            .map(|c| {
                json!({
                    "text": c.text,
                    "x0": round3(c.x0),
                    "top": round3(c.top),
                    "fontname": c.fontname,
                    "size": round3(c.size),
                })
            })
            .collect();
        Some(json!({
            "time_s": t,
            "char_count": page.chars.len(),
            "chars_preview": chars_preview,
        }))
    } else {
        None
    };

    // 6. Table extraction (all pages)
    let mut table_results = Vec::new();
    let mut total_table_time = 0.0;
    for i in 0..num_pages {
        let page = &doc.pages()[i];
        let t0 = Instant::now();
        match page.extract_tables(TableSettings::default()) {
            Ok(tables) => {
                let t = t0.elapsed().as_secs_f64();
                total_table_time += t;
                for (ti, tbl) in tables.iter().enumerate() {
                    let rows = tbl.len();
                    let cols = tbl.first().map(|r| r.len()).unwrap_or(0);
                    let data_preview: Vec<Vec<Option<String>>> =
                        tbl.iter().take(5).cloned().collect();
                    table_results.push(json!({
                        "page": i + 1,
                        "table_index": ti,
                        "rows": rows,
                        "cols": cols,
                        "data_preview": data_preview,
                    }));
                }
            }
            Err(e) => {
                let t = t0.elapsed().as_secs_f64();
                total_table_time += t;
                table_results.push(json!({
                    "page": i + 1,
                    "error": format!("{}", e),
                }));
            }
        }
    }
    let tables_found = table_results
        .iter()
        .filter(|t| t.get("rows").is_some())
        .count();

    // 7. Object counts (first page)
    let page1_objects = if num_pages > 0 {
        let counts = doc.pages()[0].object_counts();
        Some(json!({
            "chars": counts.chars,
            "lines": counts.lines,
            "rects": counts.rects,
            "curves": counts.curves,
            "images": counts.images,
        }))
    } else {
        None
    };

    // 8. Page dimensions
    let page1_dimensions = if num_pages > 0 {
        let p = &doc.pages()[0];
        Some(json!({
            "width": round3(p.width),
            "height": round3(p.height),
        }))
    } else {
        None
    };

    // 9. Search (first page)
    let search_result = if num_pages > 0 {
        let search_word = full_text
            .split_whitespace()
            .find(|w| w.len() >= 4 && w.chars().all(|c| c.is_alphabetic()));
        if let Some(word) = search_word {
            let page = &doc.pages()[0];
            let pattern = word.to_string();
            let (t, matches) = timed(|| page.search(&pattern), TIMING_ITERS);
            Some(json!({
                "pattern": pattern,
                "time_s": t,
                "match_count": matches.map(|m| m.len()).unwrap_or(0),
            }))
        } else {
            None
        }
    } else {
        None
    };

    // 10. Layout analysis (first page)
    let layout_result = if num_pages > 0 {
        let page = &doc.pages()[0];
        let (tl_time, tl) = timed(|| page.textlinehorizontals(), TIMING_ITERS);
        let (tb_time, tb) = timed(|| page.textboxhorizontals(), TIMING_ITERS);
        let (edges_time, edges) = timed(|| page.edges(), TIMING_ITERS);
        Some(json!({
            "textlinehorizontals_time_s": tl_time,
            "textlinehorizontal_count": tl.len(),
            "textboxhorizontals_time_s": tb_time,
            "textboxhorizontal_count": tb.len(),
            "edges_time_s": edges_time,
            "edge_count": edges.len(),
            "horizontal_edges": page.horizontal_edges().len(),
            "vertical_edges": page.vertical_edges().len(),
            "rect_edges": page.rect_edges().len(),
            "curve_edges": page.curve_edges().len(),
        }))
    } else {
        None
    };

    // 11. JSON serialization (first page)
    let json_result = if num_pages > 0 {
        let page = &doc.pages()[0];
        let (t, result) = timed(
            || page.to_json::<Vec<u8>>(None, None, None, None, Some(3), Some(2)),
            TIMING_ITERS,
        );
        let len = result.ok().flatten().map(|s| s.len()).unwrap_or(0);
        Some(json!({
            "time_s": t,
            "output_bytes": len,
        }))
    } else {
        None
    };

    // 12. CSV serialization (first page)
    let csv_result = if num_pages > 0 {
        let page = &doc.pages()[0];
        let (t, result) = timed(
            || page.to_csv::<Vec<u8>>(None, None, Some(3), None, None),
            TIMING_ITERS,
        );
        let len = result.ok().flatten().map(|s| s.len()).unwrap_or(0);
        Some(json!({
            "time_s": t,
            "output_bytes": len,
        }))
    } else {
        None
    };

    // 13. Image rendering (first page)
    let render_result = if num_pages > 0 {
        let page = &doc.pages()[0];
        let (t, img) = timed(
            || page.to_image(Some(72.0), None, None, false, false),
            TIMING_ITERS,
        );
        match img {
            Ok(img) => Some(json!({
                "time_s": t,
                "width_px": img.width(),
                "height_px": img.height(),
            })),
            Err(e) => Some(json!({
                "time_s": t,
                "error": format!("{}", e),
            })),
        }
    } else {
        None
    };

    // 14. Document-level aggregates
    let (agg_time, _) = timed(
        || {
            let _chars = doc.chars().len();
            let _lines = doc.lines().len();
            let _rects = doc.rects().len();
            let _curves = doc.curves().len();
            let _images = doc.images().len();
            let _annots = doc.annots().len();
            let _hyperlinks = doc.hyperlinks().len();
        },
        TIMING_ITERS,
    );

    // 15. Metadata
    let metadata_keys: Vec<String> = doc.metadata.keys().cloned().collect();

    // 16. Page box info (first page)
    let page_boxes = if num_pages > 0 {
        let p = &doc.pages()[0];
        Some(json!({
            "mediabox": [round3(p.mediabox.x0), round3(p.mediabox.top), round3(p.mediabox.x1), round3(p.mediabox.bottom)],
            "cropbox": [round3(p.cropbox.x0), round3(p.cropbox.top), round3(p.cropbox.x1), round3(p.cropbox.bottom)],
            "has_trimbox": p.trimbox.is_some(),
            "has_bleedbox": p.bleedbox.is_some(),
            "has_artbox": p.artbox.is_some(),
        }))
    } else {
        None
    };

    let mut result = json!({
        "file": filename,
        "size_bytes": size_bytes,
        "open_time_s": open_time,
        "num_pages": num_pages,
        "extract_text_time_s": text_time,
        "text_length": text_length,
        "text_preview": text_preview,
        "page_texts": page_texts,
        "page_words": page_words,
        "table_extraction_time_s": total_table_time,
        "tables_found": tables_found,
        "table_results": &table_results[..table_results.len().min(20)],
        "aggregate_objects_time_s": agg_time,
        "metadata_keys": metadata_keys,
    });

    if let Some(c) = page1_chars {
        result["page1_chars"] = c;
    }
    if let Some(o) = page1_objects {
        result["page1_objects"] = o;
    }
    if let Some(d) = page1_dimensions {
        result["page1_dimensions"] = d;
    }
    if let Some(s) = search_result {
        result["search"] = s;
    }
    if let Some(l) = layout_result {
        result["layout"] = l;
    }
    if let Some(j) = json_result {
        result["json_serialization"] = j;
    }
    if let Some(c) = csv_result {
        result["csv_serialization"] = c;
    }
    if let Some(r) = render_result {
        result["render"] = r;
    }
    if let Some(b) = page_boxes {
        result["page_boxes"] = b;
    }

    result
}

fn round3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

fn main() {
    let bench_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bench");
    let pdf_dir = bench_dir.join("pdfs");
    let results_file = bench_dir.join("results_pdfsink.json");

    let mut pdfs: Vec<PathBuf> = fs::read_dir(&pdf_dir)
        .expect("cannot read pdf dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|e| e == "pdf").unwrap_or(false))
        .collect();
    pdfs.sort();

    eprintln!("Benchmarking {} PDFs with pdfsink-rs", pdfs.len());
    eprintln!("{}", "=".repeat(60));

    let mut all_pdfs = Vec::new();

    for path in &pdfs {
        let filename = path.file_name().unwrap().to_str().unwrap();
        eprint!("\n  {} ...", filename);
        let path_clone = path.clone();
        let result = panic::catch_unwind(|| benchmark_pdf(&path_clone));
        match result {
            Ok(val) => {
                if let Some(err) = val.get("error") {
                    eprintln!(" FAILED: {}", err);
                } else {
                    eprintln!(
                        " OK ({} pages, open={:.3}s, text={:.3}s, tables={:.3}s)",
                        val["num_pages"],
                        val["open_time_s"].as_f64().unwrap_or(0.0),
                        val["extract_text_time_s"].as_f64().unwrap_or(0.0),
                        val["table_extraction_time_s"].as_f64().unwrap_or(0.0),
                    );
                }
                all_pdfs.push(val);
            }
            Err(_) => {
                eprintln!(" PANIC (pdf-extract crash)");
                all_pdfs.push(json!({
                    "file": filename,
                    "size_bytes": fs::metadata(path).map(|m| m.len()).unwrap_or(0),
                    "error": "PANIC: pdf-extract crashed on this file",
                }));
            }
        }
    }

    let output = json!({
        "tool": "pdfsink-rs",
        "version": "0.2.0",
        "pdfs": all_pdfs,
    });

    fs::write(&results_file, serde_json::to_string_pretty(&output).unwrap())
        .expect("cannot write results");
    eprintln!("\nResults written to {}", results_file.display());
}
