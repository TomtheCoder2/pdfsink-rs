#!/usr/bin/env python3
"""
Compare pdfsink-rs vs pdfplumber benchmark results.
Reads results_pdfsink.json and results_pdfplumber.json, prints a report.
"""

import json
import os
import difflib

BENCH_DIR = os.path.dirname(__file__)


def load(name):
    with open(os.path.join(BENCH_DIR, name)) as f:
        return json.load(f)


def by_file(results):
    return {r["file"]: r for r in results["pdfs"]}


def fmt_time(s):
    if s is None:
        return "N/A"
    if s < 0.001:
        return f"{s*1_000_000:.0f}µs"
    if s < 1.0:
        return f"{s*1000:.1f}ms"
    return f"{s:.2f}s"


def speedup(rust_s, py_s):
    if rust_s is None or py_s is None or rust_s == 0:
        return "N/A"
    ratio = py_s / rust_s
    return f"{ratio:.1f}x"


def text_similarity(a, b):
    """Return similarity ratio between two strings."""
    if not a and not b:
        return 1.0
    if not a or not b:
        return 0.0
    return difflib.SequenceMatcher(None, a, b).ratio()


def print_separator(char="=", width=100):
    print(char * width)


def main():
    rust = load("results_pdfsink.json")
    py = load("results_pdfplumber.json")

    rust_by_file = by_file(rust)
    py_by_file = by_file(py)

    all_files = sorted(set(list(rust_by_file.keys()) + list(py_by_file.keys())))

    print()
    print_separator("=")
    print(f"  PDFSINK-RS vs PDFPLUMBER BENCHMARK COMPARISON")
    print(f"  pdfsink-rs {rust.get('version', '?')}  vs  pdfplumber {py.get('version', '?')}")
    print_separator("=")

    # ─── SECTION 1: Speed Comparison ───
    print()
    print("1. SPEED COMPARISON")
    print_separator("-")
    header = f"{'PDF':<30} {'Pages':>5} {'Size':>8} | {'Open (Rust)':>11} {'Open (Py)':>10} {'Speedup':>8} | {'Text (Rust)':>11} {'Text (Py)':>10} {'Speedup':>8} | {'Tables (Rust)':>13} {'Tables (Py)':>12} {'Speedup':>8}"
    print(header)
    print_separator("-")

    total_rust_open = 0
    total_py_open = 0
    total_rust_text = 0
    total_py_text = 0
    total_rust_table = 0
    total_py_table = 0

    for f in all_files:
        r = rust_by_file.get(f, {})
        p = py_by_file.get(f, {})

        if "error" in r or "error" in p:
            err = r.get("error", "") or p.get("error", "")
            print(f"  {f:<28} ERROR: {err[:60]}")
            continue

        pages = r.get("num_pages", p.get("num_pages", "?"))
        size = r.get("size_bytes", 0)
        size_str = f"{size/1024:.0f}KB" if size < 1_000_000 else f"{size/1_000_000:.1f}MB"

        ro = r.get("open_time_s")
        po = p.get("open_time_s")
        rt = r.get("extract_text_time_s")
        pt = p.get("extract_text_time_s")
        rtb = r.get("table_extraction_time_s")
        ptb = p.get("table_extraction_time_s")

        if ro: total_rust_open += ro
        if po: total_py_open += po
        if rt: total_rust_text += rt
        if pt: total_py_text += pt
        if rtb: total_rust_table += rtb
        if ptb: total_py_table += ptb

        print(
            f"  {f:<28} {pages:>5} {size_str:>8} | "
            f"{fmt_time(ro):>11} {fmt_time(po):>10} {speedup(ro, po):>8} | "
            f"{fmt_time(rt):>11} {fmt_time(pt):>10} {speedup(rt, pt):>8} | "
            f"{fmt_time(rtb):>13} {fmt_time(ptb):>12} {speedup(rtb, ptb):>8}"
        )

    print_separator("-")
    print(
        f"  {'TOTAL':<28} {'':>5} {'':>8} | "
        f"{fmt_time(total_rust_open):>11} {fmt_time(total_py_open):>10} {speedup(total_rust_open, total_py_open):>8} | "
        f"{fmt_time(total_rust_text):>11} {fmt_time(total_py_text):>10} {speedup(total_rust_text, total_py_text):>8} | "
        f"{fmt_time(total_rust_table):>13} {fmt_time(total_py_table):>12} {speedup(total_rust_table, total_py_table):>8}"
    )

    # ─── SECTION 2: Accuracy - Text Extraction ───
    print()
    print()
    print("2. TEXT EXTRACTION ACCURACY (per-page comparison)")
    print_separator("-")
    print(f"  {'PDF':<30} {'Page':>4} | {'Rust len':>9} {'Py len':>8} {'Similarity':>11} {'Match?':>7}")
    print_separator("-")

    total_sim = 0.0
    total_pages_compared = 0

    for f in all_files:
        r = rust_by_file.get(f, {})
        p = py_by_file.get(f, {})

        if "error" in r or "error" in p:
            continue

        r_pages = {pt["page"]: pt for pt in r.get("page_texts", [])}
        p_pages = {pt["page"]: pt for pt in p.get("page_texts", [])}

        for pg in sorted(set(list(r_pages.keys()) + list(p_pages.keys()))):
            rt = r_pages.get(pg, {}).get("text", "")
            pt_text = p_pages.get(pg, {}).get("text", "")
            sim = text_similarity(rt, pt_text)
            match = "YES" if sim > 0.95 else ("CLOSE" if sim > 0.80 else "NO")
            total_sim += sim
            total_pages_compared += 1
            print(
                f"  {f:<30} {pg:>4} | "
                f"{len(rt):>9} {len(pt_text):>8} {sim:>10.1%} {match:>7}"
            )
            if sim < 0.90 and (rt or pt_text):
                # Show first difference
                r_lines = rt.splitlines()[:5]
                p_lines = pt_text.splitlines()[:5]
                print(f"    Rust first lines: {r_lines}")
                print(f"    Py   first lines: {p_lines}")

    if total_pages_compared > 0:
        print_separator("-")
        print(f"  Average similarity: {total_sim/total_pages_compared:.1%} across {total_pages_compared} pages")

    # ─── SECTION 3: Word Extraction Accuracy ───
    print()
    print()
    print("3. WORD EXTRACTION ACCURACY")
    print_separator("-")
    print(f"  {'PDF':<30} {'Page':>4} | {'Rust words':>11} {'Py words':>10} {'Delta':>7} | {'First 5 match?':>15}")
    print_separator("-")

    for f in all_files:
        r = rust_by_file.get(f, {})
        p = py_by_file.get(f, {})

        if "error" in r or "error" in p:
            continue

        r_words = {pw["page"]: pw for pw in r.get("page_words", [])}
        p_words = {pw["page"]: pw for pw in p.get("page_words", [])}

        for pg in sorted(set(list(r_words.keys()) + list(p_words.keys()))):
            rw = r_words.get(pg, {})
            pw = p_words.get(pg, {})
            rwc = rw.get("word_count", 0)
            pwc = pw.get("word_count", 0)
            delta = rwc - pwc

            # Compare first 5 word texts
            r_preview = [w["text"] for w in rw.get("words_preview", [])[:5]]
            p_preview = [w["text"] for w in pw.get("words_preview", [])[:5]]
            first5_match = r_preview == p_preview

            print(
                f"  {f:<30} {pg:>4} | "
                f"{rwc:>11} {pwc:>10} {delta:>+7} | "
                f"{'YES' if first5_match else 'NO':>15}"
            )
            if not first5_match:
                print(f"    Rust: {r_preview}")
                print(f"    Py:   {p_preview}")

    # ─── SECTION 4: Object Counts ───
    print()
    print()
    print("4. OBJECT COUNTS (page 1)")
    print_separator("-")
    print(f"  {'PDF':<30} | {'Chars R/P':>12} {'Lines R/P':>12} {'Rects R/P':>12} {'Curves R/P':>12} {'Images R/P':>12}")
    print_separator("-")

    for f in all_files:
        r = rust_by_file.get(f, {})
        p = py_by_file.get(f, {})

        if "error" in r or "error" in p:
            continue

        ro = r.get("page1_objects", {})
        po = p.get("page1_objects", {})

        def fmt_pair(key):
            rv = ro.get(key, "?")
            pv = po.get(key, "?")
            marker = "" if rv == pv else " *"
            return f"{rv}/{pv}{marker}"

        print(
            f"  {f:<30} | "
            f"{fmt_pair('chars'):>12} "
            f"{fmt_pair('lines'):>12} "
            f"{fmt_pair('rects'):>12} "
            f"{fmt_pair('curves'):>12} "
            f"{fmt_pair('images'):>12}"
        )

    print("  (* = mismatch)")

    # ─── SECTION 5: Table Extraction ───
    print()
    print()
    print("5. TABLE EXTRACTION")
    print_separator("-")
    print(f"  {'PDF':<30} | {'Rust tables':>12} {'Py tables':>10} {'Match?':>7}")
    print_separator("-")

    for f in all_files:
        r = rust_by_file.get(f, {})
        p = py_by_file.get(f, {})

        if "error" in r or "error" in p:
            continue

        rt = r.get("tables_found", 0)
        pt = p.get("tables_found", 0)
        match = "YES" if rt == pt else "NO"
        print(f"  {f:<30} | {rt:>12} {pt:>10} {match:>7}")

        # Compare table data if both found tables
        r_tables = [t for t in r.get("table_results", []) if "rows" in t]
        p_tables = [t for t in p.get("table_results", []) if "rows" in t]

        for i in range(min(len(r_tables), len(p_tables), 3)):
            rtbl = r_tables[i]
            ptbl = p_tables[i]
            dim_match = rtbl.get("rows") == ptbl.get("rows") and rtbl.get("cols") == ptbl.get("cols")
            print(
                f"    Table {i}: Rust {rtbl.get('rows')}x{rtbl.get('cols')}, "
                f"Py {ptbl.get('rows')}x{ptbl.get('cols')} "
                f"{'dims match' if dim_match else 'DIMS DIFFER'}"
            )
            # Compare first row of data
            r_first = rtbl.get("data_preview", [[]])[0] if rtbl.get("data_preview") else []
            p_first = ptbl.get("data_preview", [[]])[0] if ptbl.get("data_preview") else []
            if r_first and p_first:
                # Normalize None
                r_norm = [str(c) if c is not None else "" for c in r_first]
                p_norm = [str(c) if c is not None else "" for c in p_first]
                data_match = r_norm == p_norm
                if not data_match:
                    print(f"      Row 0 Rust: {r_norm[:6]}")
                    print(f"      Row 0 Py:   {p_norm[:6]}")

    # ─── SECTION 6: Page Dimensions ───
    print()
    print()
    print("6. PAGE DIMENSIONS (page 1)")
    print_separator("-")

    for f in all_files:
        r = rust_by_file.get(f, {})
        p = py_by_file.get(f, {})

        if "error" in r or "error" in p:
            continue

        rd = r.get("page1_dimensions", {})
        pd = p.get("page1_dimensions", {})
        w_match = abs(rd.get("width", 0) - pd.get("width", 0)) < 0.1
        h_match = abs(rd.get("height", 0) - pd.get("height", 0)) < 0.1
        print(
            f"  {f:<30} Rust: {rd.get('width', '?')}x{rd.get('height', '?')}  "
            f"Py: {pd.get('width', '?')}x{pd.get('height', '?')}  "
            f"{'MATCH' if w_match and h_match else 'MISMATCH'}"
        )

    # ─── SECTION 7: Summary ───
    print()
    print()
    print_separator("=")
    print("  SUMMARY")
    print_separator("=")

    if total_py_open > 0:
        print(f"  Overall open speedup:          {speedup(total_rust_open, total_py_open)}")
    if total_py_text > 0:
        print(f"  Overall text extraction speedup: {speedup(total_rust_text, total_py_text)}")
    if total_py_table > 0:
        print(f"  Overall table extraction speedup: {speedup(total_rust_table, total_py_table)}")
    if total_pages_compared > 0:
        avg_sim = total_sim / total_pages_compared
        print(f"  Average text similarity:        {avg_sim:.1%}")
    print()

    # Count issues
    errors = sum(1 for f in all_files if "error" in rust_by_file.get(f, {}))
    print(f"  PDFs with errors (Rust): {errors}/{len(all_files)}")
    errors_py = sum(1 for f in all_files if "error" in py_by_file.get(f, {}))
    print(f"  PDFs with errors (Py):   {errors_py}/{len(all_files)}")
    print()


if __name__ == "__main__":
    main()
