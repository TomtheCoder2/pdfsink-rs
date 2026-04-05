#!/usr/bin/env python3
"""
Full 1:1 CLI benchmark: pdfsink-rs vs pdfplumber.
Tests every command/feature for functional parity and speed.
"""

import json
import os
import subprocess
import sys
import tempfile
import time
import difflib

import pdfplumber

BENCH_DIR = os.path.dirname(__file__)
PDF_DIR = os.path.join(BENCH_DIR, "pdfs")
RUST_BIN = os.path.join(os.path.dirname(BENCH_DIR), "target", "release", "pdfsink-rs")

# Use a representative subset: small (simple_text), medium (irs_w9), large (nist_report), table (table_lines)
TEST_PDFS = ["simple_text.pdf", "table_lines.pdf", "irs_w9.pdf", "un_charter.pdf", "census_table.pdf"]

PASS = "PASS"
FAIL = "FAIL"
SKIP = "SKIP"


def timed_run(cmd, timeout=60):
    """Run a command, return (elapsed_seconds, stdout, stderr, returncode)."""
    t0 = time.perf_counter()
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)
        elapsed = time.perf_counter() - t0
        return elapsed, result.stdout, result.stderr, result.returncode
    except subprocess.TimeoutExpired:
        return timeout, "", "TIMEOUT", -1


def rust_cmd(command, pdf, *extra):
    return [RUST_BIN, command, os.path.join(PDF_DIR, pdf)] + list(extra)


def similarity(a, b):
    if not a and not b:
        return 1.0
    if not a or not b:
        return 0.0
    return difflib.SequenceMatcher(None, a, b).ratio()


def strip_ws(s):
    return "".join(s.split())


def fmt_time(s):
    if s < 0.001:
        return f"{s*1e6:.0f}us"
    if s < 1:
        return f"{s*1000:.1f}ms"
    return f"{s:.2f}s"


def speedup_str(rust_s, py_s):
    if rust_s == 0 or py_s == 0:
        return "N/A"
    return f"{py_s/rust_s:.1f}x"


results = []


def record(pdf, command, status, rust_time, py_time, detail=""):
    results.append({
        "pdf": pdf,
        "command": command,
        "status": status,
        "rust_time": rust_time,
        "py_time": py_time,
        "detail": detail,
    })


# ─── 1. INFO command ───
def test_info(pdf):
    rt, r_out, _, rc = timed_run(rust_cmd("info", pdf))
    if rc != 0:
        record(pdf, "info", FAIL, rt, 0, f"rust exit code {rc}")
        return

    t0 = time.perf_counter()
    p = pdfplumber.open(os.path.join(PDF_DIR, pdf))
    py_time = time.perf_counter() - t0

    r_data = json.loads(r_out)

    # Compare page count
    if r_data["page_count"] != len(p.pages):
        record(pdf, "info/page_count", FAIL, rt, py_time,
               f"rust={r_data['page_count']} py={len(p.pages)}")
    else:
        record(pdf, "info/page_count", PASS, rt, py_time,
               f"{r_data['page_count']} pages")

    # Compare page dimensions
    for i, rp in enumerate(r_data["pages"]):
        pp = p.pages[i]
        w_ok = abs(rp["width"] - float(pp.width)) < 0.1
        h_ok = abs(rp["height"] - float(pp.height)) < 0.1
        rot_ok = rp["rotation"] == (pp.rotation or 0)
        if w_ok and h_ok and rot_ok:
            record(pdf, f"info/page{i+1}_dims", PASS, 0, 0,
                   f"{rp['width']}x{rp['height']} rot={rp['rotation']}")
        else:
            record(pdf, f"info/page{i+1}_dims", FAIL, 0, 0,
                   f"rust={rp['width']}x{rp['height']}r{rp['rotation']} py={pp.width}x{pp.height}r{pp.rotation}")
        if i >= 2:  # only first 3 pages
            break

    p.close()


# ─── 2. TEXT command ───
def test_text(pdf):
    rt, r_out, _, rc = timed_run(rust_cmd("text", pdf, "1"))
    if rc != 0:
        record(pdf, "text", FAIL, rt, 0, f"rust exit code {rc}")
        return

    t0 = time.perf_counter()
    p = pdfplumber.open(os.path.join(PDF_DIR, pdf))
    py_text = p.pages[0].extract_text() or ""
    py_time = time.perf_counter() - t0

    r_text = r_out.rstrip("\n")
    sim = similarity(r_text, py_text)
    content_sim = similarity(strip_ws(r_text), strip_ws(py_text))

    status = PASS if sim > 0.95 else FAIL
    record(pdf, "text", status, rt, py_time,
           f"sim={sim:.1%} content_sim={content_sim:.1%} rust_len={len(r_text)} py_len={len(py_text)}")
    p.close()


# ─── 3. WORDS command ───
def test_words(pdf):
    rt, r_out, _, rc = timed_run(rust_cmd("words", pdf, "1"))
    if rc != 0:
        record(pdf, "words", FAIL, rt, 0, f"rust exit code {rc}")
        return

    t0 = time.perf_counter()
    p = pdfplumber.open(os.path.join(PDF_DIR, pdf))
    py_words = p.pages[0].extract_words()
    py_time = time.perf_counter() - t0

    r_words = json.loads(r_out)

    count_match = len(r_words) == len(py_words)

    # Compare first 10 word texts
    r_texts = [w["text"] for w in r_words[:10]]
    p_texts = [w["text"] for w in py_words[:10]]

    # Compare word positions (first 5)
    pos_ok = True
    pos_detail = []
    for i in range(min(5, len(r_words), len(py_words))):
        rw, pw = r_words[i], py_words[i]
        if rw["text"] == pw["text"]:
            x0_ok = abs(rw["x0"] - pw["x0"]) < 1.0
            top_ok = abs(rw["top"] - pw["top"]) < 1.0
            if not (x0_ok and top_ok):
                pos_ok = False
                pos_detail.append(f"word '{rw['text']}': rust=({rw['x0']:.1f},{rw['top']:.1f}) py=({pw['x0']:.1f},{pw['top']:.1f})")

    status = PASS if count_match else FAIL
    detail = f"rust={len(r_words)} py={len(py_words)} texts_match={r_texts==p_texts} pos_ok={pos_ok}"
    if pos_detail:
        detail += " " + "; ".join(pos_detail[:3])
    record(pdf, "words", status, rt, py_time, detail)
    p.close()


# ─── 4. SEARCH command ───
def test_search(pdf):
    # Find a word to search for
    p = pdfplumber.open(os.path.join(PDF_DIR, pdf))
    text = p.pages[0].extract_text() or ""
    search_word = None
    for w in text.split():
        if len(w) >= 4 and w.isalpha():
            search_word = w
            break
    if not search_word:
        record(pdf, "search", SKIP, 0, 0, "no suitable search term")
        p.close()
        return

    rt, r_out, _, rc = timed_run(rust_cmd("search", pdf, "1", search_word))
    if rc != 0:
        record(pdf, "search", FAIL, rt, 0, f"rust exit code {rc}")
        p.close()
        return

    t0 = time.perf_counter()
    py_matches = p.pages[0].search(search_word)
    py_time = time.perf_counter() - t0

    r_matches = json.loads(r_out)

    count_match = len(r_matches) == len(py_matches)
    status = PASS if count_match else FAIL
    detail = f"pattern='{search_word}' rust={len(r_matches)} py={len(py_matches)}"

    # Compare match positions
    if r_matches and py_matches:
        r0, p0 = r_matches[0], py_matches[0]
        x0_ok = abs(r0["x0"] - p0["x0"]) < 2.0
        top_ok = abs(r0["top"] - p0["top"]) < 2.0
        detail += f" first_pos_ok={x0_ok and top_ok}"

    record(pdf, "search", status, rt, py_time, detail)
    p.close()


# ─── 5. OBJECTS command ───
def test_objects(pdf):
    rt, r_out, _, rc = timed_run(rust_cmd("objects", pdf, "1"))
    if rc != 0:
        record(pdf, "objects", FAIL, rt, 0, f"rust exit code {rc}")
        return

    t0 = time.perf_counter()
    p = pdfplumber.open(os.path.join(PDF_DIR, pdf))
    page = p.pages[0]
    py_counts = {
        "chars": len(page.chars),
        "lines": len(page.lines),
        "rects": len(page.rects),
        "curves": len(page.curves),
        "images": len(page.images),
    }
    py_time = time.perf_counter() - t0

    r_data = json.loads(r_out)
    r_counts = {
        "chars": len(r_data.get("chars", [])),
        "lines": len(r_data.get("lines", [])),
        "rects": len(r_data.get("rects", [])),
        "curves": len(r_data.get("curves", [])),
        "images": len(r_data.get("images", [])),
    }

    mismatches = []
    for key in r_counts:
        if r_counts[key] != py_counts[key]:
            mismatches.append(f"{key}:{r_counts[key]}vs{py_counts[key]}")

    # Compare char positions (first 5)
    char_pos_ok = True
    if r_data.get("chars") and page.chars:
        for i in range(min(5, len(r_data["chars"]), len(page.chars))):
            rc_char = r_data["chars"][i]
            pc_char = page.chars[i]
            if rc_char["text"] != pc_char["text"]:
                char_pos_ok = False
                break
            if abs(rc_char["x0"] - pc_char["x0"]) > 0.5:
                char_pos_ok = False
                break

    # Compare annotations/hyperlinks
    r_annots = len(r_data.get("annots", []))
    r_links = len(r_data.get("hyperlinks", []))
    py_annots = len(page.annots) if hasattr(page, 'annots') else 0
    py_links = len(page.hyperlinks) if hasattr(page, 'hyperlinks') else 0

    status = PASS if not mismatches else FAIL
    detail = f"counts={'match' if not mismatches else ','.join(mismatches)} chars_pos={char_pos_ok} annots:{r_annots}vs{py_annots} links:{r_links}vs{py_links}"
    record(pdf, "objects", status, rt, py_time, detail)
    p.close()


# ─── 6. LINKS command ───
def test_links(pdf):
    rt, r_out, _, rc = timed_run(rust_cmd("links", pdf, "1"))
    if rc != 0:
        record(pdf, "links", FAIL, rt, 0, f"rust exit code {rc}")
        return

    t0 = time.perf_counter()
    p = pdfplumber.open(os.path.join(PDF_DIR, pdf))
    py_links = p.pages[0].hyperlinks if hasattr(p.pages[0], 'hyperlinks') else []
    py_time = time.perf_counter() - t0

    r_links = json.loads(r_out)

    count_match = len(r_links) == len(py_links)

    # Compare URIs
    r_uris = sorted([l.get("uri", "") for l in r_links])
    p_uris = sorted([l.get("uri", "") for l in py_links])
    uri_match = r_uris == p_uris

    status = PASS if count_match else FAIL
    detail = f"rust={len(r_links)} py={len(py_links)} uris_match={uri_match}"
    record(pdf, "links", status, rt, py_time, detail)
    p.close()


# ─── 7. TABLE command ───
def test_table(pdf):
    rt, r_out, _, rc = timed_run(rust_cmd("table", pdf, "1", "lines"))
    if rc != 0:
        record(pdf, "table/lines", FAIL, rt, 0, f"rust exit code {rc}")
        return

    t0 = time.perf_counter()
    p = pdfplumber.open(os.path.join(PDF_DIR, pdf))
    py_table = p.pages[0].extract_table()
    py_time = time.perf_counter() - t0

    r_table = json.loads(r_out)

    if r_table is None and py_table is None:
        record(pdf, "table/lines", PASS, rt, py_time, "both null")
    elif r_table is None or py_table is None:
        record(pdf, "table/lines", FAIL, rt, py_time,
               f"rust={'null' if r_table is None else f'{len(r_table)}rows'} py={'null' if py_table is None else f'{len(py_table)}rows'}")
    else:
        rows_match = len(r_table) == len(py_table)
        cols_match = (len(r_table[0]) if r_table else 0) == (len(py_table[0]) if py_table else 0)

        # Compare cell contents
        data_sim = 1.0
        if r_table and py_table:
            r_flat = " ".join(str(c) for row in r_table for c in row)
            p_flat = " ".join(str(c) for row in py_table for c in row)
            data_sim = similarity(strip_ws(r_flat), strip_ws(p_flat))

        status = PASS if rows_match and cols_match else FAIL
        r_dims = f"{len(r_table)}x{len(r_table[0]) if r_table else 0}"
        p_dims = f"{len(py_table)}x{len(py_table[0]) if py_table else 0}"
        record(pdf, "table/lines", status, rt, py_time,
               f"rust={r_dims} py={p_dims} content_sim={data_sim:.1%}")
    p.close()

    # Also test text strategy
    rt2, r_out2, _, rc2 = timed_run(rust_cmd("table", pdf, "1", "text"))
    t0 = time.perf_counter()
    p2 = pdfplumber.open(os.path.join(PDF_DIR, pdf))
    py_table2 = p2.pages[0].extract_table({"vertical_strategy": "text", "horizontal_strategy": "text"})
    py_time2 = time.perf_counter() - t0

    r_table2 = json.loads(r_out2) if rc2 == 0 else None

    if r_table2 is None and py_table2 is None:
        record(pdf, "table/text", PASS, rt2, py_time2, "both null")
    elif r_table2 is None or py_table2 is None:
        record(pdf, "table/text", FAIL, rt2, py_time2,
               f"rust={'null' if r_table2 is None else 'found'} py={'null' if py_table2 is None else 'found'}")
    else:
        rows_match = len(r_table2) == len(py_table2)
        status = PASS if rows_match else FAIL
        record(pdf, "table/text", status, rt2, py_time2,
               f"rust={len(r_table2)}rows py={len(py_table2)}rows")
    p2.close()


# ─── 8. SVG command ───
def test_svg(pdf):
    with tempfile.NamedTemporaryFile(suffix=".svg", delete=False) as tmp:
        svg_path = tmp.name

    rt, _, r_err, rc = timed_run(rust_cmd("svg", pdf, "1", svg_path))
    if rc != 0:
        record(pdf, "svg", FAIL, rt, 0, f"rust exit code {rc}: {r_err}")
        return

    try:
        svg_content = open(svg_path).read()
        has_svg_tag = "<svg" in svg_content
        has_viewbox = "viewBox" in svg_content
        size = len(svg_content)

        status = PASS if has_svg_tag and has_viewbox else FAIL
        record(pdf, "svg", status, rt, 0,
               f"size={size}bytes has_svg={has_svg_tag} has_viewbox={has_viewbox}")
    finally:
        os.unlink(svg_path)


# ─── 9. MULTIPAGE text ───
def test_multipage(pdf):
    """Test text extraction across multiple pages."""
    p = pdfplumber.open(os.path.join(PDF_DIR, pdf))
    if len(p.pages) < 2:
        p.close()
        return

    for pg in [1, min(2, len(p.pages))]:
        rt, r_out, _, rc = timed_run(rust_cmd("text", pdf, str(pg)))
        if rc != 0:
            record(pdf, f"text/page{pg}", FAIL, rt, 0, f"exit code {rc}")
            continue

        t0 = time.perf_counter()
        py_text = p.pages[pg - 1].extract_text() or ""
        py_time = time.perf_counter() - t0

        sim = similarity(r_out.rstrip("\n"), py_text)
        status = PASS if sim > 0.95 else FAIL
        record(pdf, f"text/page{pg}", status, rt, py_time, f"sim={sim:.1%}")

    p.close()


# ─── 10. CROP (library-level, via text comparison) ───
def test_crop(pdf):
    """Test crop by extracting text from a bbox region."""
    p = pdfplumber.open(os.path.join(PDF_DIR, pdf))
    page = p.pages[0]
    # Crop to top-left quadrant
    bbox = (0, 0, float(page.width) / 2, float(page.height) / 2)

    t0 = time.perf_counter()
    cropped = page.crop(bbox)
    py_text = cropped.extract_text() or ""
    py_time = time.perf_counter() - t0

    # No direct CLI for crop, but we verify via the library benchmark results
    record(pdf, "crop/top-left", PASS if py_text else SKIP, 0, py_time,
           f"py_text_len={len(py_text)} (library-only, no CLI equivalent)")
    p.close()


def main():
    # Check binary exists
    if not os.path.exists(RUST_BIN):
        print(f"ERROR: {RUST_BIN} not found. Run: cargo build --release")
        sys.exit(1)

    available_pdfs = [f for f in TEST_PDFS if os.path.exists(os.path.join(PDF_DIR, f))]
    print(f"CLI Benchmark: pdfsink-rs vs pdfplumber {pdfplumber.__version__}")
    print(f"Testing {len(available_pdfs)} PDFs x 10 command categories")
    print("=" * 90)

    for pdf in available_pdfs:
        print(f"\n  {pdf}:")
        for test_fn in [test_info, test_text, test_words, test_search,
                        test_objects, test_links, test_table, test_svg, test_multipage]:
            try:
                test_fn(pdf)
            except Exception as e:
                record(pdf, test_fn.__name__.replace("test_", ""), FAIL, 0, 0, str(e))

    # ─── Print results ───
    print("\n")
    print("=" * 90)
    print(f"  {'PDF':<22} {'Command':<20} {'Status':>6} {'Rust':>10} {'Python':>10} {'Speedup':>8}  Detail")
    print("-" * 90)

    pass_count = fail_count = skip_count = 0
    total_rust = total_py = 0

    for r in results:
        s = r["status"]
        if s == PASS:
            pass_count += 1
        elif s == FAIL:
            fail_count += 1
        else:
            skip_count += 1

        if r["rust_time"] > 0:
            total_rust += r["rust_time"]
        if r["py_time"] > 0:
            total_py += r["py_time"]

        rt_str = fmt_time(r["rust_time"]) if r["rust_time"] > 0 else "-"
        pt_str = fmt_time(r["py_time"]) if r["py_time"] > 0 else "-"
        sp_str = speedup_str(r["rust_time"], r["py_time"]) if r["rust_time"] > 0 and r["py_time"] > 0 else "-"
        detail = r["detail"][:50] if r["detail"] else ""

        print(f"  {r['pdf']:<22} {r['command']:<20} {s:>6} {rt_str:>10} {pt_str:>10} {sp_str:>8}  {detail}")

    print("-" * 90)
    print(f"  TOTAL: {pass_count} PASS, {fail_count} FAIL, {skip_count} SKIP")
    print(f"  Aggregate time: Rust={fmt_time(total_rust)}, Python={fmt_time(total_py)}, Speedup={speedup_str(total_rust, total_py)}")
    print()

    # Write JSON results
    out_path = os.path.join(BENCH_DIR, "results_cli.json")
    with open(out_path, "w") as f:
        json.dump(results, f, indent=2)
    print(f"  Results written to {out_path}")


if __name__ == "__main__":
    main()
