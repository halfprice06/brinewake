"""Building icons: each building as a small dimetric block, its roof and
the one fixture that names it (the tank, the gantry, the gun)."""

from draw import disc, iso_box
from icons import icon, Icon


def pad(ic, y):
    """The concrete footing, a flat diamond row under the block."""
    ic.hline(2, 19, y, "c")
    ic.hline(4, 17, y + 1, "a")


def windows(ic, bottom, x0, x1, dy, c="B"):
    for x in range(x0, x1 + 1):
        ic.px(x, bottom[x] + dy, c)


def tank(ic, x, y, w, h, body="g", lit="h", dark="f", cap="e"):
    ic.rect(x, y, w, h, body)
    ic.vline(x, y, y + h - 1, lit)
    ic.vline(x + w - 1, y, y + h - 1, dark)
    ic.hline(x, x + w - 1, y, cap)
    ic.hline(x + 1, x + w - 2, y, lit)


@icon("ui_build_union_works")
def union_works():
    ic = Icon()
    b = iso_box(ic, 1, 8, 20, 7, "C", "O", "o", edge="p")
    windows(ic, b, 3, 9, 3)
    windows(ic, b, 12, 18, 3, "b")
    ic.pts("W", [(9, 8), (10, 8), (11, 9), (12, 9)])
    tank(ic, 5, 4, 4, 6, "f", "g", "e", "d")
    ic.rows(12, 7, ["dd", "ee"], key={})
    ic.outline()
    return ic


@icon("ui_build_assembly_works")
def assembly_works():
    ic = Icon()
    # A jade canvas hall pitched over drums.
    ic.rows(0, 4, [
        ".........mm.........",
        ".......mmMMJJ.......",
        ".....mmMMMMJJJJ.....",
        "...mmMMMMMMJJJJjj...",
        ".mmMMMMMMMMJJJJjjjj.",
        "mMMMMMMMMMMJJJJjjjjj",
        "JmmmmmmmmmmJJJjjjjjj",
        ".JJJJJJJJJJJjjjjjjj.",
        ])
    ic.vline(10, 5, 10, "m")
    tank(ic, 2, 12, 5, 4, "a", "y", "u", "A")
    tank(ic, 14, 12, 5, 4, "a", "y", "u", "A")
    ic.rect(8, 12, 5, 4, "d")
    ic.rect(9, 13, 3, 3, "y")
    ic.hline(1, 19, 16, "c")
    ic.hline(3, 17, 17, "a")
    ic.outline()
    return ic


@icon("ui_build_condenser")
def condenser():
    ic = Icon()
    # Twin grey tanks, a gauge, an orange line between.
    tank(ic, 2, 5, 7, 12)
    disc(ic, 2, 3, 6, "g")
    ic.hline(3, 6, 3, "h")
    ic.rect(4, 0, 2, 3, "e")
    tank(ic, 11, 7, 7, 10)
    disc(ic, 11, 5, 6, "g")
    ic.hline(12, 15, 5, "h")
    ic.rect(12, 9, 3, 5, "b")
    ic.px(12, 9, "n")
    ic.hline(9, 10, 10, "O")
    ic.hline(9, 10, 11, "o")
    disc(ic, 3, 9, 4, "C")
    ic.px(5, 10, "k")
    ic.pts("O", [(18, 9), (19, 10), (19, 11), (19, 12), (18, 13)])
    ic.hline(1, 19, 17, "c")
    ic.hline(3, 17, 18, "a")
    ic.outline()
    return ic


@icon("ui_build_dropoff")
def dropoff():
    ic = Icon()
    # The salvage yard: crates stacked under a gantry.
    ic.vline(1, 4, 17, "e")
    ic.vline(19, 4, 17, "e")
    ic.rect(1, 3, 19, 2, "O")
    ic.hline(1, 19, 3, "p")
    ic.hline(9, 12, 4, "C")
    ic.vline(10, 5, 7, "f")
    iso_box(ic, 3, 9, 8, 5, "p", "O", "o", edge="P")
    iso_box(ic, 10, 8, 8, 6, "p", "O", "o", edge="P")
    iso_box(ic, 6, 12, 8, 3, "P", "O", "o")
    ic.hline(0, 20, 18, "c")
    ic.hline(2, 18, 19, "a")
    ic.outline()
    return ic


@icon("ui_build_tower")
def tower():
    ic = Icon()
    # The defence nest: a grey plinth, the cabin, the barrel.
    iso_box(ic, 2, 11, 16, 4, "g", "f", "e", edge="h")
    ic.rect(6, 4, 7, 8, "O")
    ic.vline(6, 4, 11, "p")
    ic.vline(12, 5, 11, "o")
    ic.hline(6, 12, 4, "C")
    ic.rect(8, 6, 3, 2, "B")
    ic.px(8, 6, "n")
    ic.vline(8, 0, 3, "f")
    ic.px(8, 0, "P")
    ic.rows(12, 6, ["gghh", "ffgg"], key={})
    ic.rows(16, 5, ["ee", "dd"], key={})
    ic.outline()
    return ic


@icon("ui_build_union_drydock")
def union_drydock():
    ic = Icon()
    b = iso_box(ic, 3, 8, 16, 7, "C", "O", "o", edge="p")
    windows(ic, b, 12, 16, 3)
    # The launch gantry in front, dark iron.
    ic.vline(1, 5, 19, "e")
    ic.vline(9, 7, 20, "e")
    ic.hline(1, 9, 5, "f")
    ic.pts("f", [(2, 6), (3, 6)])
    ic.rect(2, 16, 7, 3, "d")
    ic.rect(3, 16, 5, 2, "v")
    tank(ic, 13, 3, 3, 6, "f", "g", "e", "d")
    ic.outline()
    return ic


@icon("ui_build_assembly_drydock")
def assembly_drydock():
    ic = Icon()
    ic.rows(0, 3, [
        "..........mm........",
        "........mmMMJJ......",
        "......mmMMMMJJjj....",
        "....mmMMMMMMJJjjj...",
        "..mmMMMMMMMMJJjjjj..",
        ".mMMMMMMMMMMJJjjjjj.",
        "mMMMMMMMMMMMJJjjjjjj",
        "mMMMMMMMMMMMJJjjjjjj",
        ])
    ic.rect(2, 9, 9, 8, "d")
    ic.vline(2, 9, 17, "a")
    ic.vline(10, 9, 17, "a")
    ic.hline(2, 10, 9, "y")
    ic.rect(4, 14, 5, 2, "v")
    ic.hline(4, 8, 14, "c")
    tank(ic, 13, 11, 5, 6, "O", "p", "o", "A")
    ic.rect(0, 11, 2, 6, "J")
    ic.rect(11, 11, 2, 6, "j")
    ic.rect(18, 11, 2, 6, "j")
    ic.hline(0, 19, 17, "c")
    ic.hline(2, 17, 18, "a")
    ic.outline()
    return ic


@icon("ui_build_union_palisade")
def union_palisade():
    ic = Icon()
    # A braced steel wall section, rising to the right.
    ic.rows(1, 5, [
        "............gh",
        "..........gghf",
        "........ggffff",
        "......ggffffff",
        "....ggffffffff",
        "..ggffffffffff",
        "ghffffffffffff",
        "hfffffffffffff",
        "fffffffffffffe",
        "fffffffffffee.",
        "ffffffffffe...",
        "fffffffee.....",
        "ffffee........",
        "fee...........",
        ])
    for x in (2, 7, 12):
        for y in range(24):
            if ic.get(x, y) == "f":
                ic.px(x, y, "e")
    ic.pts("O", [(4, 12), (5, 12), (6, 11), (9, 10), (10, 9), (11, 9)])
    ic.pts("o", [(4, 13), (5, 13), (9, 11), (10, 10)])
    ic.outline()
    return ic


@icon("ui_build_assembly_palisade")
def assembly_palisade():
    ic = Icon()
    # Bound reed panels on posts.
    for i, x in enumerate(range(2, 16, 3)):
        top = 7 - i
        ic.vline(x, top, 18 - i, "v")
        ic.vline(x + 1, top, 18 - i, "u")
        ic.px(x, top, "c")
    ic.vline(1, 5, 19, "J")
    ic.vline(17, 1, 15, "j")
    ic.pts("m", [(1, 5)])
    for y0 in (10, 15):
        for x in range(1, 18):
            ic.px(x, y0 - (x - 1) // 3, "a")
    ic.outline()
    return ic


@icon("ui_build_union_hq")
def union_hq():
    ic = Icon()
    # The Union headquarters: a two-storey block, mast and pennant.
    b = iso_box(ic, 1, 9, 20, 7, "C", "O", "o", edge="p")
    windows(ic, b, 3, 9, 3)
    windows(ic, b, 12, 18, 3, "b")
    iso_box(ic, 5, 6, 8, 3, "C", "O", "o", edge="p")
    ic.vline(9, 0, 6, "f")
    ic.rect(10, 0, 3, 2, "p")
    ic.px(10, 0, "P")
    ic.outline()
    return ic


@icon("ui_build_assembly_hq")
def assembly_hq():
    ic = Icon()
    # The Assembly headquarters: the great canvas hall and its lamp.
    ic.rows(0, 6, [
        ".........mm.........",
        ".......mmMMJJ.......",
        ".....mmMMMMJJJJ.....",
        "...mmMMMMMMJJJJjj...",
        ".mmMMMMMMMMJJJJjjjj.",
        "mMMMMMMMMMMJJJJjjjjj",
        "mMMMMMMMMMMJJJJjjjjj",
        "JmmmmmmmmmmJJJjjjjjj",
        ".JJJJJJJJJJJjjjjjjj.",
        ])
    ic.vline(10, 2, 6, "u")
    ic.rect(8, 0, 5, 3, "d")
    ic.rect(9, 1, 3, 1, "y")
    ic.rect(7, 12, 7, 4, "d")
    ic.rect(8, 13, 5, 3, "y")
    ic.hline(8, 12, 13, "W")
    ic.hline(1, 19, 16, "c")
    ic.hline(3, 17, 17, "a")
    ic.outline()
    return ic
