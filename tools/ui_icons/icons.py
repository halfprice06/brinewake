"""The command-card icons, 24x24, one function each.

Light comes from the upper left.  Every icon keeps its top-right corner
(columns 17-23, rows 0-8) quiet, because the button's key sits there, and
reads at 1x on the dock panel.  See README.md for the review.
"""

from draw import Icon, disc, ring, iso_box

ICONS = {}


def icon(key):
    def wrap(fn):
        ICONS[key] = fn
        return fn

    return wrap


def shade_by_light(ic, colours, cx, cy, lit, dark, region=None, amount=3):
    """Recolour a cluster by its side of the light: pixels up-left of the
    centre take `lit`, down-right take `dark`."""
    for y in range(24):
        for x in range(24):
            if ic.get(x, y) not in colours:
                continue
            if region and not region(x, y):
                continue
            t = (x - cx) + (y - cy)
            if t <= -amount:
                ic.px(x, y, lit)
            elif t >= amount:
                ic.px(x, y, dark)


# --------------------------------------------------------------- orders


@icon("ui_cmd_attack")
def attack():
    ic = Icon()
    ring(ic, 2, 5, 16, 12, "O")
    shade_by_light(ic, "O", 9.5, 12.5, "p", "o", amount=5)
    # Four sighting ticks crossing the ring.
    ic.rect(9, 3, 2, 5, "C")
    ic.rect(9, 18, 2, 5, "C")
    ic.rect(0, 12, 5, 2, "C")
    ic.rect(15, 12, 5, 2, "C")
    ic.rect(9, 12, 2, 2, "W")
    ic.outline()
    return ic


@icon("ui_cmd_stop")
def stop():
    ic = Icon()
    ic.widths(2, 4, [10, 12, 14] + [16] * 10 + [14, 12, 10], 16, "r")
    # Bevel: the upper-left rim lit, the lower-right rim dark.
    for y in range(24):
        for x in range(24):
            if ic.get(x, y) != "r":
                continue
            edge = any(ic.get(x + dx, y + dy) == "." for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1)))
            if edge and x + y <= 21:
                ic.px(x, y, "p")
            elif edge:
                ic.px(x, y, "o")
    ic.rect(5, 11, 10, 3, "W")
    ic.hline(5, 14, 14, "c")
    ic.outline()
    return ic


@icon("ui_cmd_hold")
def hold():
    ic = Icon()
    ic.rows(
        1,
        2,
        [
            "......####......",
            ".....#....#.....",
            ".....#....#.....",
            "......####......",
            ".......##.......",
            "...##########...",
            "...##########...",
            ".......##.......",
            ".......##.......",
            ".......##.......",
            ".#.....##.....#.",
            "###....##....###",
            ".##....##....##.",
            "..##...##...##..",
            "...###.##.###...",
            ".....######.....",
            ".......##.......",
        ],
        key={"#": "g"},
    )
    # Right-hand halves and undersides in shadow, the upper-left lit.
    for y in range(24):
        for x in range(24):
            if ic.get(x, y) != "g":
                continue
            below = ic.get(x, y + 1) == "."
            if x >= 9 and (x >= 10 or below):
                ic.px(x, y, "f")
            elif ic.get(x - 1, y) == "." and ic.get(x, y - 1) == ".":
                ic.px(x, y, "h")
    ic.pts("h", [(6, 3), (7, 3), (4, 7), (5, 7), (6, 7), (7, 7), (8, 7), (9, 8)])
    ic.outline()
    return ic


@icon("ui_cmd_capture")
def capture():
    ic = Icon()
    # The pole, planted in a mound.
    ic.vline(5, 3, 19, "v")
    ic.vline(6, 3, 19, "u")
    ic.px(5, 2, "C")
    ic.px(6, 2, "c")
    # A swallow-tailed flag in the holder's gold.
    ic.rows(
        7,
        3,
        [
            "yyyyyyyyy.",
            "PPPPPPPPPP",
            "PPPPPPPPP.",
            "PPPPPPPP..",
            "PPPPPPPPP.",
            "AAAAAAAAAA",
            "aAAAAAAAA.",
        ],
    )
    ic.rows(7, 5, ["..y..y", ".yyyyyy"])
    ic.widths(1, 20, [6, 10], 10, "c")
    ic.hline(2, 4, 20, "C")
    ic.outline()
    return ic


@icon("ui_cmd_gather")
def gather():
    ic = Icon()
    # A wreck's salvage crate, grabbed by the worker's hook.
    iso_box(ic, 3, 11, 12, 6, "p", "O", "o", edge="P")
    ic.hline(6, 11, 12, "P")
    ic.pts("o", [(7, 13), (10, 13)])
    ic.vline(15, 0, 6, "f")
    ic.rows(
        12,
        6,
        [
            ".ggg.",
            "g...g",
            "g....",
            ".g...",
        ],
    )
    ic.pts("h", [(13, 6), (12, 7)])
    ic.outline()
    return ic


def machine_block(ic, x, y, body="c", lit="C", dark="u", visor="B"):
    """A small machine body seen from the front: the cube head of every
    machine in the roster."""
    ic.rect(x, y, 8, 6, body)
    ic.hline(x, x + 7, y, lit)
    ic.vline(x, y, y + 5, lit)
    ic.vline(x + 7, y + 1, y + 5, dark)
    ic.hline(x + 1, x + 7, y + 5, dark)
    ic.rect(x + 2, y + 2, 4, 2, visor)


@icon("ui_cmd_deploy")
def deploy():
    ic = Icon()
    machine_block(ic, 5, 4)
    # Outriggers splayed to the ground, feet planted.
    ic.pts("f", [(4, 10), (3, 11), (2, 12), (1, 13), (13, 10), (14, 11), (15, 12), (16, 13)])
    ic.pts("g", [(5, 10), (4, 11), (3, 12), (2, 13), (12, 10), (13, 11), (14, 12), (15, 13)])
    ic.rect(0, 14, 4, 2, "e")
    ic.rect(14, 14, 4, 2, "e")
    # The down arrow: set down here.
    ic.rows(6, 12, ["..mm..", "..mm..", "mmmmmm", ".mmmm.", "..mm.."], key={"m": "m"})
    ic.pts("J", [(8, 16), (9, 16), (10, 15), (11, 14)])
    ic.hline(0, 17, 20, "u")
    ic.hline(1, 16, 21, "w")
    ic.outline()
    return ic


@icon("ui_cmd_pack")
def pack():
    ic = Icon()
    machine_block(ic, 5, 10)
    # Outriggers folded against the body.
    ic.vline(4, 11, 16, "f")
    ic.vline(13, 11, 16, "f")
    # The up arrow: pack and move.
    ic.rows(6, 2, ["..mm..", ".mmmm.", "mmmmmm", "..mm..", "..mm.."])
    ic.pts("J", [(10, 4), (11, 4), (9, 5), (9, 6)])
    ic.hline(1, 16, 18, "u")
    ic.hline(2, 15, 19, "w")
    ic.outline()
    return ic


@icon("ui_cmd_keep")
def keep():
    ic = Icon()
    machine_block(ic, 3, 3)
    # Outriggers splayed and planted, as on DEPLOY.
    ic.pts("f", [(2, 9), (1, 10), (0, 11), (11, 9), (12, 10), (13, 11)])
    ic.pts("g", [(3, 9), (2, 10), (1, 11), (10, 9), (11, 10), (12, 11)])
    ic.hline(0, 14, 12, "u")
    ic.hline(0, 14, 13, "w")
    # The padlock: stays set down whatever the order.
    ic.rows(
        13,
        11,
        [
            ".aaaa.",
            "a....a",
            "a....a",
            "yyyyyy",
            "yAAAAA",
            "yAkkAa",
            "yAAkAa",
            "yAAAAa",
            "aaaaaa",
        ],
    )
    ic.outline()
    return ic


@icon("ui_cmd_surge")
def surge():
    ic = Icon()
    # Two forward chevrons of pressure and a puff behind.
    chevron = [
        "##....",
        "###...",
        ".###..",
        "..###.",
        "...###",
        "..###.",
        ".###..",
        "###...",
        "##....",
    ]
    ic.rows(5, 7, chevron, key={"#": "m"})
    ic.rows(11, 7, chevron, key={"#": "M"})
    ic.pts("J", [(5, 15), (6, 14), (7, 13), (8, 12), (11, 15), (12, 14), (13, 13), (14, 12)])
    ic.rows(0, 9, [".cc", "cCC", "cCc", ".c."])
    ic.outline()
    return ic


@icon("ui_cmd_unload")
def unload():
    ic = Icon()
    # A hull, its hatch open, a crate set down beside it.
    ic.rows(
        0,
        9,
        [
            "..............",
            "OOOOOOOOOOOOOO",
            ".oooooooooooo.",
            "..oooooooooo..",
        ],
    )
    ic.hline(1, 12, 9, "p")
    iso_box(ic, 10, 13, 8, 4, "P", "p", "O")
    ic.rows(2, 2, ["..mm..", "..mm..", "mmmmmm", ".mmmm.", "..mm.."])
    ic.pts("J", [(4, 6), (5, 6), (6, 5), (7, 4)])
    ic.outline()
    return ic


@icon("ui_cmd_board")
def board():
    ic = Icon()
    ic.rows(
        0,
        14,
        [
            "OOOOOOOOOOOOOOOO",
            ".oooooooooooooo.",
            "..oooooooooooo..",
        ],
    )
    ic.hline(1, 14, 13, "p")
    ic.hline(0, 15, 13, "p")
    # Into the hold: an arrow down onto the deck.
    ic.rows(5, 3, ["..mm..", "..mm..", "..mm..", "mmmmmm", ".mmmm.", "..mm.."])
    ic.pts("J", [(7, 8), (8, 8), (9, 7), (10, 6)])
    ic.rect(6, 12, 4, 1, "o")
    ic.outline()
    return ic


@icon("ui_cmd_sound")
def sound():
    ic = Icon()
    # A sounding ping: two thick arcs spreading right from the emitter.
    cx, cy = 3.5, 12.5
    for y in range(24):
        for x in range(24):
            d2 = (x - cx) ** 2 + (y - cy) ** 2
            if x - cx < abs(y - cy) * 0.7:
                continue
            if 5.5 ** 2 <= d2 < 7.6 ** 2:
                ic.px(x, y, "n" if y < cy else "B")
            elif 10.5 ** 2 <= d2 < 12.6 ** 2:
                ic.px(x, y, "n" if y < cy else "B")
    ic.rect(1, 10, 5, 5, "g")
    ic.pts("h", [(1, 10), (2, 10), (3, 10), (1, 11), (1, 12)])
    ic.pts("f", [(5, 13), (5, 14), (4, 14), (3, 14)])
    ic.rect(2, 11, 2, 2, "W")
    ic.outline()
    return ic


@icon("ui_cmd_formation_compact")
def formation_compact():
    ic = Icon()
    for gx in range(3):
        for gy in range(3):
            x, y = 3 + gx * 5, 4 + gy * 5
            ic.rect(x, y, 3, 3, "m")
            ic.px(x, y, "M")
            ic.pts("J", [(x + 2, y + 2), (x + 1, y + 2), (x + 2, y + 1)])
    ic.outline()
    return ic


@icon("ui_cmd_formation_line")
def formation_line():
    ic = Icon()
    for gx in range(4):
        x, y = 1 + gx * 5, 10
        ic.rect(x, y, 3, 3, "m")
        ic.px(x, y, "M")
        ic.pts("J", [(x + 2, y + 2), (x + 1, y + 2), (x + 2, y + 1)])
    ic.outline()
    return ic


@icon("ui_cmd_formation_loose")
def formation_loose():
    ic = Icon()
    for x, y in [(2, 3), (11, 5), (6, 11), (15, 13), (3, 18), (11, 19)]:
        ic.rect(x, y, 3, 3, "m")
        ic.px(x, y, "M")
        ic.pts("J", [(x + 2, y + 2), (x + 1, y + 2), (x + 2, y + 1)])
    ic.outline()
    return ic


@icon("ui_cmd_face")
def face():
    ic = Icon()
    # A turning arrow round the machine.
    ring(ic, 2, 4, 16, 12, "C")
    # Open the ring at the upper right for the arrowhead.
    for x, y in [(14, 5), (15, 5), (15, 6), (16, 6), (16, 7), (17, 7), (17, 8), (16, 8)]:
        ic.px(x, y, ".")
    ic.rows(12, 3, ["PPPPP", ".PPPP", "..PPP", "...PP", "....P"])
    shade_by_light(ic, "C", 9.5, 11.5, "W", "c", amount=6)
    machine_block(ic, 6, 9)
    ic.outline()
    return ic


@icon("ui_cmd_cancel")
def cancel():
    ic = Icon()
    x_rows = [
        "##........##",
        "###......###",
        ".###....###.",
        "..###..###..",
        "...######...",
        "....####....",
        "....####....",
        "...######...",
        "..###..###..",
        ".###....###.",
        "###......###",
        "##........##",
    ]
    ic.rows(3, 6, x_rows, key={"#": "r"})
    shade_by_light(ic, "r", 8.5, 11.5, "p", "o", amount=4)
    ic.outline()
    return ic


@icon("ui_cmd_vent")
def vent():
    ic = Icon()
    # A handwheel valve on a pipe, venting a plume.
    ic.rect(0, 16, 18, 4, "f")
    ic.hline(0, 17, 16, "g")
    ic.hline(0, 17, 19, "e")
    ic.rect(7, 12, 4, 4, "f")
    ic.pts("g", [(7, 12), (8, 12)])
    ring(ic, 4, 5, 10, 6, "O")
    shade_by_light(ic, "O", 8.5, 9.5, "p", "o", amount=3)
    ic.rect(8, 9, 2, 2, "P")
    ic.rows(12, 0, ["..cc", ".cCC.", "cCWC", ".cC."])
    ic.rows(15, 3, [".c", "cC", "c."])
    ic.outline()
    return ic


@icon("ui_cmd_reclaim")
def reclaim():
    ic = Icon()
    # Pressure (the jade gauge) melted into salvage (the crate).
    disc(ic, 1, 2, 10, "J")
    ring(ic, 1, 2, 10, 6, "m")
    ic.pts("C", [(5, 7), (6, 6), (7, 5)])
    ic.rows(9, 9, ["pp..", ".pp.", "..pp", ".pp.", "pp.."])
    iso_box(ic, 8, 13, 12, 5, "p", "O", "o", edge="P")
    ic.outline()
    return ic


@icon("ui_cmd_army")
def army():
    ic = Icon()
    # Three rank chevrons: every combat machine.
    chevron = [
        "##............##",
        "####........####",
        ".######..######.",
        "...##########...",
        ".....######.....",
    ]
    for i in range(3):
        ic.rows(1, 4 + i * 6, chevron, key={"#": "P" if i == 0 else "A"})
    shade_by_light(ic, "A", 8.5, 12, "P", "a", amount=8)
    ic.outline()
    return ic


import units  # noqa: E402,F401  (registers the unit icons)
import buildings  # noqa: E402,F401
import upgrades  # noqa: E402,F401
import stats  # noqa: E402,F401
import compact  # noqa: E402,F401
