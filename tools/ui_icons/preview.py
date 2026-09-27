"""Review sheet: every icon at 1x, 2x and 3x on the dock panel colour, and
as it sits in a 26x26 command-card button.  Nearest-neighbour only."""

import sys
from PIL import Image

from draw import PALETTE, N
from icons import ICONS

PANEL = (28, 45, 54)
BUTTON = (35, 55, 64)
EDGE = (73, 98, 106)


def to_image(ic):
    img = Image.new("RGBA", (N, N), (0, 0, 0, 0))
    for y, row in enumerate(ic.text()):
        for x, ch in enumerate(row):
            if PALETTE.get(ch):
                img.putpixel((x, y), PALETTE[ch] + (255,))
    return img


def main(out, keys=None):
    keys = keys or list(ICONS)
    cell = 3 * N + 26 + 2 * 26 + 24
    cols = 4
    rows = (len(keys) + cols - 1) // cols
    sheet = Image.new("RGB", (cols * (cell + N * 3 + 8), rows * (N * 3 + 8) + 8), PANEL)
    for i, k in enumerate(keys):
        img = to_image(ICONS[k]())
        ox = (i % cols) * (cell + N * 3 + 8) + 4
        oy = (i // cols) * (N * 3 + 8) + 4
        x = ox
        for s in (1, 2, 3):
            sheet.paste(img.resize((N * s, N * s), Image.NEAREST), (x, oy), img.resize((N * s, N * s), Image.NEAREST))
            x += N * s + 4
        # A button at 2x: 26x26 frame with the icon inset 1px.
        btn = Image.new("RGB", (26, 26), EDGE)
        btn.paste(Image.new("RGB", (24, 24), BUTTON), (1, 1))
        btn.paste(img, (1, 1), img)
        sheet.paste(btn.resize((52, 52), Image.NEAREST), (x, oy))
    sheet.save(out)


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2:] or None)
