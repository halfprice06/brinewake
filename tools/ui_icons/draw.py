"""Small drawing helpers for the 24x24 command-card icons.

The icons are scripted pixel art: every icon in icons.py places its
clusters with these helpers and explicit pixel lists, and the result is
reviewed at 1x, 2x and 3x on the dock's own panel colour.  Nothing here
resamples an image.
"""

N = 24

# The palette: shared with the game atlas, plus the interface red.
PALETTE = {
    ".": None,
    "k": (16, 28, 41),  # ink outline
    "d": (38, 56, 68),  # deep
    "e": (64, 84, 101),  # steel 0
    "f": (101, 127, 137),  # steel 1
    "g": (158, 180, 181),  # steel 2
    "h": (216, 224, 207),  # steel 3
    "c": (190, 183, 155),  # cream 0
    "C": (231, 217, 181),  # cream 1
    "W": (255, 240, 206),  # cream 2
    "o": (135, 70, 54),  # orange 0
    "O": (191, 103, 58),  # orange 1
    "p": (233, 153, 78),  # orange 2
    "P": (242, 195, 101),  # orange 3
    "j": (50, 97, 95),  # jade 0
    "J": (86, 142, 124),  # jade 1
    "m": (147, 196, 160),  # jade 2
    "M": (219, 235, 188),  # jade 3
    "b": (48, 91, 102),  # glass 0
    "B": (67, 142, 156),  # glass 1
    "n": (145, 204, 208),  # glass 2
    "a": (175, 142, 91),  # amber 0
    "A": (215, 142, 66),  # amber 1
    "y": (255, 208, 131),  # amber 2
    "w": (85, 53, 58),  # plum
    "u": (125, 99, 78),  # wood 0
    "v": (157, 143, 118),  # wood 1
    "r": (226, 108, 84),  # interface red
    "q": (36, 88, 109),  # water 0
    "Q": (77, 124, 130),  # water 1
    # The Saltglass Compact: cobalt-violet glaze, salt crust, the lens glint.
    "i": (51, 41, 92),  # glaze 1
    "I": (75, 63, 143),  # glaze 2
    "l": (109, 105, 189),  # glaze 3
    "L": (164, 173, 224),  # glaze 4
    "s": (141, 136, 163),  # crust 0
    "S": (199, 197, 207),  # crust 1
    "t": (239, 238, 228),  # crust 2
    "T": (255, 251, 230),  # glint
}


class Icon:
    def __init__(self):
        self.g = [["." for _ in range(N)] for _ in range(N)]

    def px(self, x, y, c):
        if 0 <= x < N and 0 <= y < N:
            self.g[y][x] = c

    def get(self, x, y):
        if 0 <= x < N and 0 <= y < N:
            return self.g[y][x]
        return "."

    def rect(self, x, y, w, h, c):
        for yy in range(y, y + h):
            for xx in range(x, x + w):
                self.px(xx, yy, c)

    def hline(self, x0, x1, y, c):
        for x in range(min(x0, x1), max(x0, x1) + 1):
            self.px(x, y, c)

    def vline(self, x, y0, y1, c):
        for y in range(min(y0, y1), max(y0, y1) + 1):
            self.px(x, y, c)

    def pts(self, c, pts):
        for x, y in pts:
            self.px(x, y, c)

    def rows(self, x, y, rows, key=None):
        """Paste hand-written rows; '.' leaves the grid alone.  `key` maps
        letters to palette letters, so one drawing can be recoloured."""
        for dy, row in enumerate(rows):
            for dx, ch in enumerate(row):
                if ch == " " or ch == ".":
                    continue
                self.px(x + dx, y + dy, key.get(ch, ch) if key else ch)

    def widths(self, x, y, widths, total, c):
        """A symmetric shape from row widths, centred in `total` columns."""
        for dy, w in enumerate(widths):
            off = (total - w) // 2
            self.hline(x + off, x + off + w - 1, y + dy, c)

    def mask(self):
        return {(x, y) for y in range(N) for x in range(N) if self.g[y][x] != "."}

    def outline(self, c="k", diagonal=False):
        """Ink every empty pixel touching the drawing (a full dark outline,
        the icon's separation from any button state)."""
        m = self.mask()
        add = []
        for y in range(N):
            for x in range(N):
                if (x, y) in m:
                    continue
                near = [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)]
                if diagonal:
                    near += [(x + 1, y + 1), (x - 1, y - 1), (x + 1, y - 1), (x - 1, y + 1)]
                if any(p in m for p in near):
                    add.append((x, y))
        for x, y in add:
            self.px(x, y, c)

    def replace(self, a, b, region=None):
        for y in range(N):
            for x in range(N):
                if region and not region(x, y):
                    continue
                if self.g[y][x] == a:
                    self.g[y][x] = b

    def text(self):
        return ["".join(r) for r in self.g]


# Circles by hand-chosen row widths (quarter runs 4,2,1 | 1,2,4 and so on),
# rather than a formula's jogs.
CIRCLE = {
    4: [2, 4, 4, 2],
    6: [2, 4, 6, 6, 4, 2],
    8: [4, 6, 8, 8, 8, 8, 6, 4],
    10: [4, 8, 8, 10, 10, 10, 10, 8, 8, 4],
    12: [4, 8, 10, 10, 12, 12, 12, 12, 10, 10, 8, 4],
    14: [6, 10, 12, 12, 14, 14, 14, 14, 14, 14, 12, 12, 10, 6],
    16: [8, 12, 14, 14, 16, 16, 16, 16, 16, 16, 16, 16, 14, 14, 12, 8],
}


def disc(icon, x, y, d, c):
    icon.widths(x, y, CIRCLE[d], d, c)


def ring(icon, x, y, outer, inner, c):
    """A ring: the outer disc less a centred inner disc."""
    disc(icon, x, y, outer, c)
    off = (outer - inner) // 2
    widths = CIRCLE[inner]
    for dy, w in enumerate(widths):
        o = (inner - w) // 2
        for dx in range(w):
            icon.px(x + off + o + dx, y + off + dy, ".")


def iso_box(icon, x, y, w, h, top, left, right, edge=None):
    """A 2:1 dimetric box: a top diamond `w` wide (a multiple of 4) and
    sides `h` rows tall.  Returns the column-wise bottom of the top face."""
    half = w // 2
    cx = x + half
    rows = []
    for t in range(w // 4):
        rows.append(4 * t + 4)
    for t in range(w // 4 - 1):
        rows.append(w - 4 * (t + 1))
    bottom = {}
    for dy, width in enumerate(rows):
        x0 = cx - width // 2
        for xx in range(x0, x0 + width):
            icon.px(xx, y + dy, top)
            bottom[xx] = y + dy
    for xx, yb in bottom.items():
        for yy in range(yb + 1, yb + 1 + h):
            icon.px(xx, yy, left if xx < cx else right)
    if edge:
        # The near vertical edge, lit.
        for yy in range(bottom[cx] + 1, bottom[cx] + 1 + h):
            icon.px(cx, yy, edge)
    return bottom
