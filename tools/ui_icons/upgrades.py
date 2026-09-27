"""Upgrade and doctrine icons."""

from draw import disc, ring, iso_box
from icons import icon, Icon, shade_by_light


def plate_shape(ic, x, y):
    ic.rows(x, y, [
        "gggggggggggg",
        "ghhhhhhhhhgf",
        "ghgggggggggf",
        "ghgggggggggf",
        "ghgggggggggf",
        "ghgggggggggf",
        "ghgggggggggf",
        ".fgggggggggf.",
        ".fggggggggf.",
        "..fggggggf..",
        "...ffgggf...",
        ".....ff.....",
        ])


@icon("ui_up_plate")
def plate():
    ic = Icon()
    # A riveted hull plate, shield-cut.
    ic.widths(3, 4, [14, 14, 14, 14, 14, 14, 14, 14, 12, 10, 8, 6, 4], 14, "g")
    shade_by_light(ic, "g", 9.5, 9, "h", "f", amount=6)
    ic.hline(3, 16, 4, "h")
    for x, y in [(5, 6), (14, 6), (5, 12), (14, 12), (9, 15), (10, 15)]:
        ic.px(x, y, "d")
        ic.px(x - 1, y - 1, "h") if ic.get(x - 1, y - 1) != "." else None
    ic.vline(9, 6, 13, "e")
    ic.vline(10, 6, 13, "h")
    ic.outline()
    return ic


@icon("ui_up_siege")
def siege():
    ic = Icon()
    # A heavy shell, nose up and to the right, fins behind.
    for i in range(9):
        x, y = 3 + i, 17 - i
        ic.rect(x, y, 4, 4, "f" if i < 6 else "O")
    shade_by_light(ic, "f", 9, 13, "g", "e", amount=1)
    shade_by_light(ic, "O", 13, 10, "p", "o", amount=1)
    ic.pts("P", [(13, 7), (14, 6), (15, 5)])
    ic.pts("o", [(2, 17), (3, 19), (1, 20), (5, 21), (6, 22)])
    ic.rows(1, 17, ["o..", "oo.", ".oo", "..o"])
    ic.hline(7, 8, 12, "d")
    ic.outline()
    return ic


@icon("ui_up_temper")
def temper():
    ic = Icon()
    # Steel drawn from the fire: a hot bar on the anvil, sparks off it.
    ic.rows(1, 12, [
        "ffffffffffffffff",
        ".eeeeeeeeeeeeee.",
        "....eeeeeeee....",
        ".....dddddd.....",
        "....dddddddd....",
        "...eeeeeeeeee...",
        ])
    ic.hline(1, 16, 12, "g")
    ic.rect(4, 9, 11, 3, "O")
    ic.hline(4, 14, 9, "P")
    ic.hline(5, 13, 10, "y")
    ic.hline(4, 14, 11, "o")
    ic.pts("y", [(6, 6), (9, 4), (12, 5), (15, 3)])
    ic.pts("W", [(9, 5), (12, 6)])
    ic.pts("A", [(4, 7), (16, 7)])
    ic.outline()
    return ic


@icon("ui_up_tracks")
def tracks():
    ic = Icon()
    # A loop of tread round two road wheels.
    ic.rows(1, 8, [
        "..dddddddddddddd..",
        ".deeeeeeeeeeeeeed.",
        "de..............ed",
        "de..............ed",
        "de..............ed",
        "de..............ed",
        ".deeeeeeeeeeeeeed.",
        "..dddddddddddddd..",
        ])
    for x in range(3, 17, 3):
        ic.px(x, 8, "f")
        ic.px(x, 15, "f")
    disc(ic, 3, 10, 4, "O")
    disc(ic, 13, 10, 4, "O")
    disc(ic, 8, 10, 4, "o")
    for x in (4, 9, 14):
        ic.px(x, 11, "p")
    ic.outline()
    return ic


@icon("ui_up_refit")
def refit():
    ic = Icon()
    # A spanner over a hull plate: more hull on every machine.
    ic.rect(2, 10, 13, 9, "f")
    ic.hline(2, 14, 10, "g")
    ic.vline(2, 10, 18, "g")
    ic.hline(3, 14, 18, "e")
    ic.vline(14, 11, 18, "e")
    ic.pts("d", [(4, 12), (12, 12), (4, 16), (12, 16)])
    # The spanner, jaws up-left.
    ic.rows(1, 0, [
        ".hh..hh......",
        "hhg..ghf.....",
        "hgg..ggf.....",
        "hgggggff.....",
        ".fggggff.....",
        "..ffhgf......",
        "....hggf.....",
        ".....hggf....",
        "......hggf...",
        ".......hggf..",
        "........hggf.",
        ".........fff.",
        ])
    ic.rows(15, 13, ["..J..", "..J..", "JJmJJ", "..J..", "..J.."], key={"J": "m", "m": "M"})
    ic.outline()
    return ic


@icon("ui_up_overhaul")
def overhaul():
    ic = Icon()
    # Three hull plates bolted one over another, thickening down: each
    # level of OVERHAUL adds one.  The top corners stay quiet for the level
    # mark (left) and the key (right), so the plates sit low.
    for i, (x, y) in enumerate([(3, 9), (5, 13), (7, 17)]):
        ic.rect(x, y, 14, 5, "f")
        ic.hline(x, x + 13, y, "h" if i == 2 else "g")
        ic.vline(x, y, y + 4, "g")
        ic.hline(x + 1, x + 13, y + 4, "e")
        ic.vline(x + 13, y + 1, y + 4, "e")
        ic.pts("d", [(x + 2, y + 2), (x + 11, y + 2)])
    # A brass band round the newest plate: salvage spent on it.
    ic.vline(14, 18, 20, "A")
    ic.vline(15, 18, 20, "a")
    ic.px(14, 17, "y")
    ic.outline()
    return ic


@icon("ui_up_cranes")
def cranes():
    ic = Icon()
    # A crane hook lifting a bale of scrap.
    ic.vline(9, 0, 5, "f")
    ic.rows(6, 5, [
        "..ggg..",
        ".gh.fg.",
        "......g",
        ".....g.",
        "g...gf.",
        ".ggg...",
        ])
    iso_box(ic, 3, 12, 12, 5, "p", "O", "o", edge="P")
    ic.pts("o", [(6, 14), (11, 14)])
    ic.outline()
    return ic


@icon("ui_up_salvage_sonar")
def salvage_sonar():
    ic = Icon()
    # A ping finding a buried crate.
    for y in range(24):
        for x in range(24):
            d2 = (x - 9.5) ** 2 + (y - 13.5) ** 2
            if y > 13:
                continue
            if 7.5 ** 2 <= d2 < 9.3 ** 2 or 4 ** 2 <= d2 < 5.6 ** 2:
                ic.px(x, y, "n" if x < 10 else "B")
    iso_box(ic, 6, 11, 8, 4, "p", "O", "o", edge="P")
    ic.hline(1, 18, 18, "u")
    ic.hline(2, 17, 19, "w")
    ic.outline()
    return ic


@icon("ui_up_scrap_recovery")
def scrap_recovery():
    ic = Icon()
    # Wrecked scrap coming back round as salvage.
    for y in range(24):
        for x in range(24):
            d2 = (x - 9.5) ** 2 + (y - 11.5) ** 2
            if 6.5 ** 2 <= d2 < 8.6 ** 2 and not (x > 11 and y < 9):
                ic.px(x, y, "m" if y < 11 else "J")
    ic.rows(12, 2, ["mmmm", ".mmm", "..mm", "...m"], key={"m": "M"})
    iso_box(ic, 5, 8, 8, 4, "p", "O", "o", edge="P")
    ic.outline()
    return ic


@icon("ui_up_overpressure")
def overpressure():
    ic = Icon()
    # The gauge pinned past its red line.
    ring(ic, 2, 3, 16, 12, "f")
    disc(ic, 4, 5, 12, "C")
    ic.pts("W", [(7, 6), (8, 6), (6, 7)])
    shade_by_light(ic, "f", 9.5, 10.5, "g", "e", amount=6)
    ic.pts("r", [(14, 7), (15, 8), (15, 9), (16, 10), (15, 7)])
    for x, y in [(6, 10), (7, 7), (10, 6)]:
        ic.px(x, y, "u")
    ic.pts("d", [(10, 11), (11, 10), (12, 9), (13, 8), (14, 7), (9, 11)])
    ic.rect(9, 11, 2, 2, "d")
    ic.pts("W", [(6, 6), (5, 7)])
    ic.rect(8, 19, 4, 3, "f")
    ic.hline(8, 11, 19, "g")
    ic.outline()
    return ic


@icon("ui_up_bleed_valves")
def bleed_valves():
    ic = Icon()
    # A handwheel on a pipe and a steady drip of spent pressure.
    ic.rect(0, 11, 17, 4, "f")
    ic.hline(0, 16, 11, "g")
    ic.hline(0, 16, 14, "e")
    ic.rect(6, 8, 4, 3, "f")
    ic.pts("g", [(6, 8)])
    ring(ic, 3, 1, 10, 6, "J")
    shade_by_light(ic, "J", 7.5, 5.5, "m", "j", amount=3)
    ic.rect(7, 5, 2, 2, "M")
    ic.rect(14, 15, 3, 2, "f")
    ic.pts("n", [(15, 18), (15, 19), (14, 20), (15, 21), (16, 20), (15, 20)])
    ic.px(15, 20, "B")
    ic.outline()
    return ic


@icon("ui_doc_hauling")
def doctrine_hauling():
    ic = Icon()
    # Hauling: the worker's load, two crates stacked and strapped.
    iso_box(ic, 3, 10, 12, 5, "p", "O", "o", edge="P")
    iso_box(ic, 5, 4, 8, 4, "P", "p", "O")
    ic.vline(9, 13, 20, "u")
    ic.rows(0, 18, ["....", "...."])
    ic.rows(15, 13, ["m.", "mm", "mmm", "mm", "m."], key={"m": "M"})
    ic.outline()
    return ic


@icon("ui_doc_fire_control")
def doctrine_fire_control():
    ic = Icon()
    # Fire control: the rangefinder's split sight on three marks.
    ring(ic, 3, 4, 14, 10, "f")
    shade_by_light(ic, "f", 9.5, 10.5, "g", "e", amount=5)
    ic.hline(4, 15, 11, "C")
    ic.vline(10, 5, 16, "C")
    ic.pts("r", [(7, 8), (13, 8), (12, 14)])
    ic.rect(9, 10, 3, 3, "r")
    ic.px(10, 11, "W")
    ic.outline()
    return ic
