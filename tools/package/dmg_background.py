#!/usr/bin/env python3
"""Draws the background of the macOS disk image window (tools/package/dmg.py):
the title in the game's 7x9 console face, an arrow from BRINEWAKE to
Applications, and the tide along the bottom, in the game's palette.

Scripted, not drawn in Aseprite: everything is placed on a 300x190 texel
grid and scaled by whole numbers, 2x for the 600x380 point window and 4x for
Retina, then both sizes go into one TIFF.

    python3 tools/package/dmg_background.py OUT.tiff
"""

import os
import re
import subprocess
import sys
import tempfile

import numpy as np
from PIL import Image

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))

# The game's palette (tools/trailer/make_trailer.py).
INK = (19, 31, 39)
PANEL = (28, 45, 54)
EDGE = (71, 96, 102)
WHITE = (239, 228, 197)
MUTED = (151, 169, 167)
GOLD = (236, 177, 96)
WATER = (40, 78, 88)
FOAM = (88, 132, 138)
# Finder draws the icon names over the picture in black (light mode) or
# white (dark mode). This slate has the same contrast, about 4.5:1, with
# both, so the plates behind the names read in either.
PLATE = (96, 124, 128)

W, H = 300, 190
# Icon centres in texels; dmg.py places the icons at twice these, in points.
APP = (75, 92)
APPLICATIONS = (225, 92)


def load_glyphs():
    src = open(os.path.join(ROOT, "crates", "bw_desktop", "src", "font7.rs")).read()
    glyphs = {}
    for m in re.finditer(r"'(\\'|[^'])' => \[([0-9, ]+)\]", src):
        glyphs[m.group(1).replace("\\'", "'")] = [int(x) for x in m.group(2).split(",")]
    return glyphs


GLYPHS = load_glyphs()


def text(img, s, x, y, scale, color, shadow=INK):
    """Draws s with its top-left at (x, y), one font pixel per scale x scale
    texels, with a hard shadow one font pixel down-right."""
    for dx, dy, c in ((scale, scale, shadow), (0, 0, color)):
        if c is None:
            continue
        for i, ch in enumerate(s.upper()):
            for r, bits in enumerate(GLYPHS.get(ch, [0] * 9)):
                for col in range(7):
                    if bits & (1 << (6 - col)):
                        px = x + dx + (i * 8 + col) * scale
                        py = y + dy + r * scale
                        img[py : py + scale, px : px + scale] = c


def text_width(s, scale):
    return (len(s) * 8 - 1) * scale


def centred(img, s, y, scale, color, cx=W // 2, shadow=INK):
    text(img, s, cx - text_width(s, scale) // 2, y, scale, color, shadow)


def rect(img, x, y, w, h, color):
    img[y : y + h, x : x + w] = color


def plate(img, cx, cy, w, h):
    """A nameplate behind an icon's name: an edge-coloured frame, clipped
    corners."""
    x, y = cx - w // 2, cy - h // 2
    rect(img, x, y, w, h, EDGE)
    rect(img, x + 1, y + 1, w - 2, h - 2, PLATE)
    for px, py in ((x, y), (x + w - 1, y), (x, y + h - 1), (x + w - 1, y + h - 1)):
        img[py, px] = INK


def arrow(img, x0, x1, y):
    """A dashed shaft and a stepped head, three texels thick."""
    for x in range(x0, x1 - 10, 6):
        rect(img, x, y - 1, 4, 3, GOLD)
    for i in range(8):
        rect(img, x1 - 10 + i, y - 7 + i, 1, 15 - 2 * i, GOLD)


def tide(img):
    """Water along the bottom with a stepped crest and a few foam dashes."""
    top = 160
    for x in range(W):
        crest = top + int(round(2 * np.sin(x / 11.0) + 1.5 * np.sin(x / 4.7 + 1.0)))
        img[crest:, x] = WATER
        img[crest, x] = FOAM
    for x, y in ((24, 172), (70, 179), (131, 170), (188, 181), (240, 173), (279, 180)):
        rect(img, x, y, 7, 1, FOAM)


def draw():
    img = np.zeros((H, W, 3), dtype=np.uint8)
    img[:, :] = INK
    # A lighter band behind the icons, as in the game's menus.
    rect(img, 0, 50, W, 88, PANEL)
    rect(img, 0, 50, W, 1, EDGE)
    rect(img, 0, 137, W, 1, EDGE)
    centred(img, "BRINEWAKE", 12, 2, GOLD)
    centred(img, "THE TIDE DECIDES.", 36, 1, MUTED, shadow=None)
    # Names sit just below 48-point icons (dmg.py: icon_size 96 points).
    plate(img, APP[0], APP[1] + 33, 64, 11)
    plate(img, APPLICATIONS[0], APPLICATIONS[1] + 33, 64, 11)
    arrow(img, APP[0] + 34, APPLICATIONS[0] - 30, APP[1])
    centred(img, "DRAG BRINEWAKE INTO APPLICATIONS", 144, 1, WHITE, shadow=None)
    tide(img)
    return img


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else "dmg-background.tiff"
    img = draw()
    with tempfile.TemporaryDirectory() as tmp:
        sizes = []
        for factor, name in ((2, "background.png"), (4, "background@2x.png")):
            path = os.path.join(tmp, name)
            big = np.kron(img, np.ones((factor, factor, 1), dtype=np.uint8))
            Image.fromarray(big).save(path, dpi=(72 * factor // 2,) * 2)
            sizes.append(path)
        # One TIFF holding both sizes, the form Finder takes for Retina.
        subprocess.run(["tiffutil", "-cathidpicheck", *sizes, "-out", out], check=True)
    print(f"wrote {out}")


if __name__ == "__main__":
    main()
