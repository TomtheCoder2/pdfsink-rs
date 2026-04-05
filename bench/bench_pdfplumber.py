#!/usr/bin/env python3
"""
Comprehensive pdfplumber benchmark.
Outputs JSON with timings and extracted data for accuracy comparison.
"""

import json
import os
import sys
import time
import traceback

import pdfplumber

PDF_DIR = os.path.join(os.path.dirname(__file__), "pdfs")
RESULTS_FILE = os.path.join(os.path.dirname(__file__), "results_pdfplumber.json")

# How many timing iterations per operation
TIMING_ITERS = 3


def timed(fn, iters=TIMING_ITERS):
    """Run fn() iters times, return (best_seconds, result_of_last_call)."""
    best = float("inf")
    result = None
    for _ in range(iters):
        t0 = time.perf_counter()
        result = fn()
        elapsed = time.perf_counter() - t0
        best = min(best, elapsed)
    return best, result


def benchmark_pdf(path):
    """Run all benchmarks on a single PDF, return dict of results."""
    filename = os.path.basename(path)
    results = {"file": filename, "size_bytes": os.path.getsize(path)}

    # 1. Open / parse timing (fresh open each iteration)
    open_time, pdf = timed(lambda: pdfplumber.open(path))
    results["open_time_s"] = open_time
    results["num_pages"] = len(pdf.pages)

    # 2. Full-document text extraction (fresh open + extract each iteration to avoid caching)
    def open_and_extract_all_text():
        p = pdfplumber.open(path)
        texts = []
        for page in p.pages:
            texts.append(page.extract_text() or "")
        p.close()
        return "\n".join(texts)

    text_time, full_text = timed(open_and_extract_all_text)
    # Subtract open time to get pure text extraction time
    results["extract_text_time_s"] = max(0, text_time - open_time)
    results["open_plus_text_time_s"] = text_time
    results["text_length"] = len(full_text)
    results["text_preview"] = full_text[:500]

    # 3. Per-page text for first 3 pages (for accuracy comparison)
    # Re-open fresh to avoid caching
    pdf = pdfplumber.open(path)
    page_texts = []
    for i, page in enumerate(pdf.pages[:3]):
        t0 = time.perf_counter()
        txt = page.extract_text() or ""
        t = time.perf_counter() - t0
        page_texts.append({
            "page": i + 1,
            "time_s": t,
            "text": txt,
            "char_count": len(txt),
        })
    results["page_texts"] = page_texts
    pdf.close()

    # 4. Word extraction (first 3 pages)
    pdf = pdfplumber.open(path)
    page_words = []
    for i, page in enumerate(pdf.pages[:3]):
        t, words = timed(lambda p=page: p.extract_words(), iters=1)
        page_words.append({
            "page": i + 1,
            "time_s": t,
            "word_count": len(words),
            "words_preview": [
                {
                    "text": w["text"],
                    "x0": round(w["x0"], 3),
                    "top": round(w["top"], 3),
                    "x1": round(w["x1"], 3),
                    "bottom": round(w["bottom"], 3),
                }
                for w in words[:20]
            ],
        })
    results["page_words"] = page_words

    # 5. Character extraction (first page only, for detailed comparison)
    if pdf.pages:
        page0 = pdf.pages[0]
        t, chars = timed(lambda: page0.chars)
        results["page1_chars"] = {
            "time_s": t,
            "char_count": len(chars),
            "chars_preview": [
                {
                    "text": c.get("text", ""),
                    "x0": round(c.get("x0", 0), 3),
                    "top": round(c.get("top", 0), 3),
                    "fontname": c.get("fontname", ""),
                    "size": round(c.get("size", 0), 3),
                }
                for c in chars[:30]
            ],
        }

    # 6. Table extraction (all pages, collect results)
    table_results = []
    total_table_time = 0.0
    for i, page in enumerate(pdf.pages):
        try:
            t, tables = timed(lambda p=page: p.extract_tables(), iters=1)
            total_table_time += t
            if tables:
                for ti, tbl in enumerate(tables):
                    table_results.append({
                        "page": i + 1,
                        "table_index": ti,
                        "rows": len(tbl),
                        "cols": len(tbl[0]) if tbl else 0,
                        "data_preview": tbl[:5],
                    })
        except Exception as e:
            table_results.append({
                "page": i + 1,
                "error": str(e),
            })
    results["table_extraction_time_s"] = total_table_time
    results["tables_found"] = len([t for t in table_results if "rows" in t])
    results["table_results"] = table_results[:20]  # cap output

    # 7. Object counts (first page)
    if pdf.pages:
        page0 = pdf.pages[0]
        results["page1_objects"] = {
            "chars": len(page0.chars),
            "lines": len(page0.lines),
            "rects": len(page0.rects),
            "curves": len(page0.curves),
            "images": len(page0.images),
        }

    # 8. Page dimensions
    if pdf.pages:
        page0 = pdf.pages[0]
        results["page1_dimensions"] = {
            "width": round(page0.width, 3),
            "height": round(page0.height, 3),
        }

    # 9. Search (first page, simple pattern)
    if pdf.pages and full_text:
        # Find a word to search for
        words_in_text = full_text.split()
        search_word = None
        for w in words_in_text:
            if len(w) >= 4 and w.isalpha():
                search_word = w
                break
        if search_word:
            page0 = pdf.pages[0]
            t, search_result = timed(lambda: page0.search(search_word))
            results["search"] = {
                "pattern": search_word,
                "time_s": t,
                "match_count": len(search_result),
            }

    pdf.close()
    return results


def main():
    pdfs = sorted(
        [os.path.join(PDF_DIR, f) for f in os.listdir(PDF_DIR) if f.endswith(".pdf")]
    )

    print(f"Benchmarking {len(pdfs)} PDFs with pdfplumber {pdfplumber.__version__}")
    print("=" * 60)

    all_results = {
        "tool": "pdfplumber",
        "version": pdfplumber.__version__,
        "pdfs": [],
    }

    for path in pdfs:
        filename = os.path.basename(path)
        print(f"\n  {filename} ...", end=" ", flush=True)
        try:
            result = benchmark_pdf(path)
            all_results["pdfs"].append(result)
            print(
                f"OK ({result['num_pages']} pages, "
                f"open={result['open_time_s']:.3f}s, "
                f"text={result['extract_text_time_s']:.3f}s, "
                f"tables={result['table_extraction_time_s']:.3f}s)"
            )
        except Exception as e:
            print(f"FAILED: {e}")
            traceback.print_exc()
            all_results["pdfs"].append({
                "file": filename,
                "error": str(e),
            })

    with open(RESULTS_FILE, "w") as f:
        json.dump(all_results, f, indent=2, default=str)

    print(f"\nResults written to {RESULTS_FILE}")


if __name__ == "__main__":
    main()
