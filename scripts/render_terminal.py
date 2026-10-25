#!/usr/bin/env python3
"""Render a text file as a terminal-window PNG.

Usage:
    python3 scripts/render_terminal.py INPUT.txt OUTPUT.png [--title TEXT]

Paste real command output into a .txt file (lines starting with "$ " are
treated as typed commands) and re-run this script to refresh a screenshot.
Requires Pillow and a monospace TrueType font (DejaVu Sans Mono by default).
"""
import argparse
import sys
from PIL import Image, ImageDraw, ImageFont

FONT_CANDIDATES = [
    ("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
     "/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf"),
    ("/System/Library/Fonts/Menlo.ttc", "/System/Library/Fonts/Menlo.ttc"),
    ("C:/Windows/Fonts/consola.ttf", "C:/Windows/Fonts/consolab.ttf"),
]

BG, BAR, FG = "#1e1e2e", "#2a2b3d", "#cdd6f4"
GREEN, YELLOW, CYAN, DIM = "#a6e3a1", "#f9e2af", "#89dceb", "#7f849c"
DOTS = ("#f38ba8", "#f9e2af", "#a6e3a1")


def load_fonts(size):
    for regular, bold in FONT_CANDIDATES:
        try:
            return ImageFont.truetype(regular, size), ImageFont.truetype(bold, size)
        except OSError:
            continue
    sys.exit("no usable monospace font found; edit FONT_CANDIDATES")


def classify(line):
    """Return a list of (text, colour, is_bold) segments for one line."""
    if line.startswith("$ "):
        return [("$ ", GREEN, True), (line[2:], FG, True)]
    if line.startswith("--") or line.startswith("──"):
        return [(line, CYAN, True)]
    if line.startswith("Block #"):
        return [(line, YELLOW, True)]
    if "[coinbase]" in line or "[transfer]" in line:
        return [(line, CYAN, False)]
    if line.startswith("[") and "] " in line:
        head, tail = line.split("] ", 1)
        return [(head + "] ", DIM, False), (tail, FG, False)]
    return [(line, FG, False)]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("source")
    parser.add_argument("target")
    parser.add_argument("--title", default="minichain - zsh")
    parser.add_argument("--size", type=int, default=16)
    args = parser.parse_args()

    with open(args.source, encoding="utf-8") as handle:
        lines = handle.read().rstrip("\n").split("\n")

    font, bold = load_fonts(args.size)
    cell_w = font.getlength("M")
    line_h = int(args.size * 1.45)
    pad, bar_h = 22, 40
    width = int(max(len(l) for l in lines) * cell_w + pad * 2)
    width = max(width, 520)
    height = bar_h + pad * 2 + line_h * len(lines)

    image = Image.new("RGBA", (width, height), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    draw.rounded_rectangle((0, 0, width - 1, height - 1), radius=12, fill=BG)
    draw.rounded_rectangle((0, 0, width - 1, bar_h + 12), radius=12, fill=BAR)
    draw.rectangle((0, bar_h, width - 1, bar_h + 12), fill=BG)
    for i, colour in enumerate(DOTS):
        cx = 22 + i * 22
        draw.ellipse((cx - 6, bar_h // 2 - 6, cx + 6, bar_h // 2 + 6), fill=colour)
    title_w = font.getlength(args.title)
    draw.text(((width - title_w) / 2, bar_h // 2 - args.size // 2 - 1),
              args.title, font=font, fill=DIM)

    y = bar_h + pad
    for line in lines:
        x = pad
        for text, colour, strong in classify(line):
            face = bold if strong else font
            draw.text((x, y), text, font=face, fill=colour)
            x += face.getlength(text)
        y += line_h

    image.save(args.target)
    print(f"wrote {args.target} ({width}x{height})")


if __name__ == "__main__":
    main()
