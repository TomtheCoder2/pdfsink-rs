#!/usr/bin/env python3
from __future__ import annotations

import json
from pathlib import Path

import pdfplumber

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "tests" / "fixtures"
GOLDENS = ROOT / "tests" / "goldens" / "goldens.json"


def r(value: float) -> float:
    return round(value, 3)


def bbox_dict(obj: dict) -> dict:
    return {
        "x0": r(obj["x0"]),
        "top": r(obj["top"]),
        "x1": r(obj["x1"]),
        "bottom": r(obj["bottom"]),
        "width": r(obj["width"]),
        "height": r(obj["height"]),
    }


def main() -> None:
    payload = {}
    for pdf_path in sorted(FIXTURES.glob("*.pdf")):
        with pdfplumber.open(pdf_path) as pdf:
            pages = []
            for page in pdf.pages:
                page_data = {
                    "page_number": page.page_number,
                    "rotation": page.rotation,
                    "width": r(page.width),
                    "height": r(page.height),
                    "text": page.extract_text(),
                    "word_texts": [word["text"] for word in page.extract_words()],
                    "object_counts": {
                        key: len(getattr(page, key))
                        for key in ["chars", "lines", "rects", "curves", "images", "annots", "hyperlinks"]
                    },
                }

                if pdf_path.name == "simple_text.pdf":
                    matches = page.search("second", regex=False)
                    page_data["search_second"] = [
                        {
                            "text": m["text"],
                            "bbox": {
                                "x0": r(m["x0"]),
                                "top": r(m["top"]),
                                "x1": r(m["x1"]),
                                "bottom": r(m["bottom"]),
                            },
                            "char_count": len(m["chars"]),
                        }
                        for m in matches
                    ]

                if pdf_path.name == "crop_regions.pdf":
                    left = page.crop((0, 0, page.width / 2, page.height))
                    outside = page.outside_bbox((0, 0, page.width / 2, page.height))
                    page_data["crop_cases"] = {
                        "left_half_crop_text": left.extract_text(),
                        "left_half_outside_text": outside.extract_text(),
                    }

                if pdf_path.name == "table_lines.pdf":
                    table = page.find_table()
                    page_data["default_table"] = page.extract_table()
                    page_data["default_table_bbox"] = {
                        "x0": r(table.bbox[0]),
                        "top": r(table.bbox[1]),
                        "x1": r(table.bbox[2]),
                        "bottom": r(table.bbox[3]),
                    } if table else None
                    page_data["table_count"] = len(page.find_tables())

                if pdf_path.name == "table_text_only.pdf":
                    settings = {"vertical_strategy": "text", "horizontal_strategy": "text"}
                    page_data["default_table"] = page.extract_table()
                    page_data["text_table"] = page.extract_table(settings)
                    page_data["text_table_count"] = len(page.find_tables(settings))

                if pdf_path.name == "objects_showcase.pdf":
                    page_data["line0"] = bbox_dict(page.lines[0]) if page.lines else None
                    page_data["rect0"] = bbox_dict(page.rects[0]) if page.rects else None
                    page_data["curve0"] = bbox_dict(page.curves[0]) if page.curves else None
                    page_data["image0"] = {
                        **bbox_dict(page.images[0]),
                        "srcsize": list(page.images[0].get("srcsize", ())),
                        "name": str(page.images[0].get("name")),
                    } if page.images else None
                    page_data["hyperlinks"] = [
                        {
                            **bbox_dict(link),
                            "uri": link["uri"],
                        }
                        for link in page.hyperlinks
                    ]

                if pdf_path.name == "rotated_and_duplicates.pdf":
                    page_data["deduped_char_count"] = len(page.dedupe_chars().chars)

                pages.append(page_data)

            payload[pdf_path.stem] = {
                "file": pdf_path.name,
                "page_count": len(pdf.pages),
                "pages": pages,
            }

    GOLDENS.parent.mkdir(parents=True, exist_ok=True)
    GOLDENS.write_text(json.dumps(payload, indent=2, sort_keys=True), encoding="utf-8")
    print(f"wrote {GOLDENS}")


if __name__ == "__main__":
    main()
