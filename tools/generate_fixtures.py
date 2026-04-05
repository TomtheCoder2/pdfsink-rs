#!/usr/bin/env python3
from __future__ import annotations

import os
from pathlib import Path

from PIL import Image, ImageDraw
from reportlab.lib.pagesizes import letter
from reportlab.pdfgen import canvas

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "tests" / "fixtures"


def ensure_image(path: Path) -> None:
    img = Image.new("RGB", (40, 40), "white")
    draw = ImageDraw.Draw(img)
    draw.rectangle((5, 5, 35, 35), outline="black", fill="red")
    draw.line((5, 35, 35, 5), fill="blue", width=2)
    img.save(path)


def simple_text(path: Path) -> None:
    c = canvas.Canvas(str(path), pagesize=letter)
    c.setFont("Helvetica", 12)
    c.drawString(72, 720, "Hello world")
    c.drawString(72, 700, "A second line with 123")
    c.drawString(250, 680, "Right block")
    c.showPage()
    c.save()


def crop_regions(path: Path) -> None:
    c = canvas.Canvas(str(path), pagesize=letter)
    c.setFont("Helvetica", 12)
    c.drawString(72, 720, "LEFT TOP")
    c.drawString(72, 680, "LEFT BOTTOM")
    c.drawString(320, 720, "RIGHT TOP")
    c.drawString(320, 680, "RIGHT BOTTOM")
    c.line(300, 650, 300, 740)
    c.showPage()
    c.save()


def table_lines(path: Path) -> None:
    c = canvas.Canvas(str(path), pagesize=letter)
    c.setFont("Helvetica", 10)
    x0, y0 = 72, 500
    cell_w, cell_h = 120, 30
    rows, cols = 4, 3
    for i in range(rows + 1):
        y = y0 + i * cell_h
        c.line(x0, y, x0 + cols * cell_w, y)
    for j in range(cols + 1):
        x = x0 + j * cell_w
        c.line(x, y0, x, y0 + rows * cell_h)
    texts = [
        ["Name", "Age", "City"],
        ["Alice", "31", "Oakland"],
        ["Bob", "27", "Berkeley"],
        ["Cara", "29", "San Jose"],
    ]
    for r, row in enumerate(texts):
        for col, text in enumerate(row):
            c.drawString(x0 + 10 + col * cell_w, y0 + (rows - r - 1) * cell_h + 10, text)
    c.showPage()
    c.save()


def table_text_only(path: Path) -> None:
    c = canvas.Canvas(str(path), pagesize=letter)
    c.setFont("Helvetica", 10)
    start_y = 700
    xs = [72, 220, 320]
    headers = ["Item", "Qty", "Price"]
    rows = [["Apples", "10", "$5"], ["Bananas", "4", "$2"], ["Carrots", "7", "$3"]]
    for x, h in zip(xs, headers):
        c.drawString(x, start_y, h)
    for idx, row in enumerate(rows, 1):
        y = start_y - idx * 24
        for x, text in zip(xs, row):
            c.drawString(x, y, text)
    c.showPage()
    c.save()


def objects_showcase(path: Path, image_path: Path) -> None:
    c = canvas.Canvas(str(path), pagesize=letter)
    c.setFont("Helvetica", 12)
    c.drawString(72, 740, "Objects Showcase")
    c.line(72, 700, 200, 700)
    c.rect(72, 620, 100, 40, stroke=1, fill=0)
    curve = c.beginPath()
    curve.moveTo(250, 620)
    curve.curveTo(280, 680, 330, 560, 360, 620)
    c.drawPath(curve, stroke=1, fill=0)
    c.drawImage(str(image_path), 420, 620, width=40, height=40)
    c.drawString(72, 580, "OpenAI Link")
    c.linkURL("https://openai.com", (72, 575, 140, 590), relative=0)
    c.showPage()
    c.save()


def rotated_and_duplicates(path: Path) -> None:
    c = canvas.Canvas(str(path), pagesize=letter)
    c.setFont("Helvetica", 12)
    c.drawString(72, 720, "DUPLICATE")
    c.drawString(72, 720, "DUPLICATE")
    c.saveState()
    c.translate(300, 500)
    c.rotate(90)
    c.drawString(0, 0, "VERTICAL")
    c.restoreState()
    c.showPage()
    c.save()


def multipage(path: Path) -> None:
    c = canvas.Canvas(str(path), pagesize=letter)
    c.setFont("Helvetica", 12)
    c.drawString(72, 720, "Page One")
    c.showPage()
    c.drawString(72, 720, "Page Two")
    c.showPage()
    c.save()


def main() -> None:
    FIXTURES.mkdir(parents=True, exist_ok=True)
    image_path = FIXTURES / "tiny.png"
    ensure_image(image_path)
    simple_text(FIXTURES / "simple_text.pdf")
    crop_regions(FIXTURES / "crop_regions.pdf")
    table_lines(FIXTURES / "table_lines.pdf")
    table_text_only(FIXTURES / "table_text_only.pdf")
    objects_showcase(FIXTURES / "objects_showcase.pdf", image_path)
    rotated_and_duplicates(FIXTURES / "rotated_and_duplicates.pdf")
    multipage(FIXTURES / "multipage.pdf")
    print(f"Generated fixtures in {FIXTURES}")


if __name__ == "__main__":
    main()
