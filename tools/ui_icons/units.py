"""Unit icons: each machine's silhouette and the one part that names it
(the crane, the lamp, the shield), in its faction's colour."""

from draw import disc, ring
from icons import icon, Icon

UNION = {"lit": "p", "mid": "O", "dark": "o"}
ASSEMBLY = {"lit": "m", "mid": "J", "dark": "j"}


def head(ic, x, y, w=7, h=6):
    """The cube head every machine wears: cream, a glass visor."""
    ic.rect(x, y, w, h, "C")
    ic.hline(x, x + w - 1, y, "W")
    ic.vline(x + w - 1, y + 1, y + h - 1, "c")
    ic.hline(x, x + w - 1, y + h - 1, "c")
    ic.rect(x + 1, y + 2, w - 3, 2, "B")
    ic.px(x + 1, y + 2, "n")


def body(ic, x, y, w, h, pal):
    ic.rect(x, y, w, h, pal["mid"])
    ic.hline(x, x + w - 1, y, pal["lit"])
    ic.vline(x, y, y + h - 1, pal["lit"])
    ic.hline(x + 1, x + w - 1, y + h - 1, pal["dark"])
    ic.vline(x + w - 1, y + 1, y + h - 1, pal["dark"])


def legs(ic, x, y, gap, pal, h=4):
    for lx in (x, x + gap):
        ic.rect(lx, y, 2, h, pal["dark"])
        ic.px(lx, y, pal["mid"])
        ic.rect(lx - 1, y + h, 4, 1, pal["mid"])
        ic.px(lx - 1, y + h, pal["lit"])


def tracks(ic, x, y, w):
    ic.rect(x, y, w, 3, "d")
    ic.hline(x + 1, x + w - 2, y, "e")
    for tx in range(x + 1, x + w - 1, 3):
        ic.px(tx, y + 1, "f")


def wheel(ic, x, y, d=8):
    ring(ic, x, y, d, d - 4, "o")
    disc(ic, x + d // 2 - 2, y + d // 2 - 2, 4, "f")
    ic.px(x + d // 2 - 1, y + d // 2 - 1, "g")
    ic.pts("O", [(x + 1, y + 2), (x + 2, y + 1)])


@icon("ui_unit_wick")
def wick():
    ic = Icon()
    # Assembly worker: a walker with a grabbing claw held out.
    body(ic, 3, 11, 9, 5, ASSEMBLY)
    head(ic, 4, 5)
    ic.rect(1, 7, 3, 5, "A")  # the pressure tank on its back
    ic.px(1, 7, "y")
    ic.vline(3, 8, 11, "a")
    # The claw arm.
    ic.hline(12, 17, 13, "J")
    ic.hline(12, 17, 12, "m")
    ic.rows(17, 10, ["..P", ".P.", "P..", "P..", ".P.", "..P"], key={"P": "y"})
    ic.pts("a", [(18, 11), (18, 14)])
    legs(ic, 4, 16, 5, ASSEMBLY, h=4)
    ic.outline()
    return ic


@icon("ui_unit_hook")
def hook():
    ic = Icon()
    # Union worker: a tracked crane with its hook down.
    body(ic, 2, 12, 12, 5, UNION)
    head(ic, 7, 7)
    ic.rect(3, 9, 4, 3, "o")
    # The boom, rising right.
    ic.rows(3, 1, [
        "........CC",
        "......CCWc",
        "....CCWc..",
        "..CCWc....",
        "CCWc......",
        "Cc........",
        "C.........",
        ], key={"W": "W"})
    ic.vline(13, 2, 8, "e")
    ic.rows(12, 9, [".f.", "..f", "ff."], key={"f": "O"})
    tracks(ic, 1, 17, 14)
    ic.outline()
    return ic


@icon("ui_unit_tidewatch")
def tidewatch():
    ic = Icon()
    # Union scout: one wheel, a mast with the watch lamp.
    ic.vline(7, 3, 12, "g")
    ic.vline(8, 3, 12, "f")
    disc(ic, 5, 0, 6, "d")
    ic.rect(6, 1, 4, 3, "n")
    ic.pts("M", [(6, 1), (7, 1)])
    body(ic, 4, 12, 10, 4, UNION)
    ic.rect(11, 9, 5, 4, "C")
    ic.hline(11, 15, 9, "W")
    wheel(ic, 1, 13, 10)
    ic.outline()
    return ic


@icon("ui_unit_lampwright")
def lampwright():
    ic = Icon()
    # Assembly mender: a walker carrying a lamp on a crooked stalk.
    body(ic, 4, 11, 8, 5, ASSEMBLY)
    ic.rect(5, 9, 6, 2, "a")
    ic.hline(5, 10, 9, "A")
    ic.rows(6, 2, ["....ff", "...f..", "..f...", ".f....", "f....."], key={"f": "u"})
    ic.rect(11, 0, 6, 6, "d")
    ic.rect(12, 1, 4, 4, "y")
    ic.pts("W", [(12, 1), (13, 1), (12, 2)])
    ic.pts("A", [(15, 4), (14, 4), (15, 3)])
    legs(ic, 5, 16, 4, ASSEMBLY, h=4)
    ic.outline()
    return ic


@icon("ui_unit_riveter")
def riveter():
    ic = Icon()
    # Union line gun: a wheeled carriage and a stubby rivet gun.
    body(ic, 2, 8, 11, 7, UNION)
    head(ic, 9, 5)
    ic.rect(15, 9, 6, 4, "f")
    ic.hline(15, 20, 9, "g")
    ic.hline(15, 20, 12, "e")
    ic.rect(20, 9, 2, 4, "d")
    ic.rect(3, 5, 4, 3, "e")
    ic.pts("f", [(3, 5), (4, 5)])
    wheel(ic, 2, 13, 10)
    ic.outline()
    return ic


@icon("ui_unit_reedguard")
def reedguard():
    ic = Icon()
    # Assembly line: a walker behind a tall bound-reed shield.
    body(ic, 4, 7, 9, 9, ASSEMBLY)
    head(ic, 6, 3)
    ic.rect(1, 6, 3, 7, "A")
    ic.hline(1, 3, 6, "y")
    # The reed shield, bound in bands.
    ic.rect(9, 8, 8, 10, "v")
    for x in (9, 11, 13, 15):
        ic.vline(x, 8, 17, "u")
    ic.hline(9, 16, 10, "a")
    ic.hline(9, 16, 15, "a")
    ic.hline(9, 16, 8, "c")
    legs(ic, 4, 16, 4, ASSEMBLY, h=4)
    ic.outline()
    return ic


@icon("ui_unit_loom")
def loom():
    ic = Icon()
    # Assembly siege: the great arched frame over a glowing core.
    ic.rows(1, 0, [
        ".....mmmmmm.....",
        "...mmMMMMMMmm...",
        "..mMMmmmmmmMMm..",
        ".mMm........mJj.",
        ".mm..........Jj.",
        "mMm..........JJj",
        "mm............Jj",
        "mm............Jj",
        "mm............Jj",
        "mm............Jj",
        "Jm............jj",
        "JJ............jj",
        "JJ............jj",
        "Jj............jj",
        "jj............jj",
        "jj............jj",
        ])
    ic.rect(5, 6, 8, 8, "A")
    ic.rect(6, 7, 6, 6, "y")
    ic.rect(7, 8, 3, 3, "W")
    ic.hline(5, 12, 13, "a")
    ic.vline(12, 7, 13, "a")
    ic.rows(6, 14, ["gggggg", ".ffff.", "..ee.."])
    ic.rect(0, 16, 3, 2, "J")
    ic.rect(15, 16, 3, 2, "j")
    ic.outline()
    return ic


@icon("ui_unit_sounder")
def sounder():
    ic = Icon()
    # Union spotter: long legs and the listening dish on top.
    ic.rows(4, 0, [
        "..cCC..",
        ".cCWWC.",
        "cCWWWCc",
        "cCCWCCc",
        ".ccCcc.",
        "...f...",
        ])
    ic.pts("P", [(10, 1), (11, 0)])
    ic.vline(7, 5, 8, "f")
    body(ic, 4, 9, 9, 5, UNION)
    head(ic, 9, 7)
    ic.rows(3, 14, ["CC....CC", ".C....C.", ".C.....C", "C......C", "CC....CC"], key={"C": "c"})
    ic.pts("C", [(3, 14), (9, 14), (3, 18), (10, 18)])
    ic.outline()
    return ic


@icon("ui_unit_skipper")
def skipper():
    ic = Icon()
    # Assembly raider: a low skimming hull, head tucked into it.
    ic.rows(0, 9, [
        "....mmmmmmmmmm....",
        "..mmMMMMMMMMMMmm..",
        ".mMMmmmmmmmmmmMJj.",
        "mMmmmmmmmmmmmmmmJj",
        "JmmmmmmmmmmmmmmmJj",
        ".JJJJJJJJJJJJJJJj.",
        "..jjjjjjjjjjjjjj..",
        ])
    head(ic, 6, 5)
    ic.rect(2, 7, 3, 3, "A")
    ic.px(2, 7, "y")
    ic.rows(3, 16, ["mm......mm", "Jj......Jj", "JJJ....JJJ"])
    ic.outline()
    return ic


@icon("ui_unit_bulwark")
def bulwark():
    ic = Icon()
    # Union heavy: a tracked hull behind a riveted plate column.
    body(ic, 1, 9, 11, 7, UNION)
    head(ic, 3, 3)
    ic.rect(3, 9, 7, 2, "f")
    ic.hline(3, 9, 9, "g")
    # The plate column, studded.
    ic.rect(12, 2, 5, 15, "C")
    ic.vline(12, 2, 16, "W")
    ic.vline(16, 3, 16, "c")
    for y in (4, 9, 14):
        ic.px(14, y, "d")
    tracks(ic, 0, 16, 16)
    ic.outline()
    return ic


@icon("ui_unit_caisson")
def caisson():
    ic = Icon()
    # Union siege cart: a tall grey-roofed box on two big wheels.
    ic.rect(3, 1, 13, 4, "g")
    ic.hline(3, 15, 1, "h")
    for x in (6, 9, 12):
        ic.vline(x, 2, 4, "f")
    ic.hline(3, 15, 4, "f")
    ic.rect(4, 5, 11, 9, "C")
    ic.vline(4, 5, 13, "W")
    ic.vline(14, 5, 13, "c")
    ic.hline(4, 14, 9, "o")
    ic.hline(4, 14, 10, "O")
    ic.rect(2, 12, 16, 2, "O")
    wheel(ic, 1, 13, 8)
    wheel(ic, 10, 13, 8)
    ic.outline()
    return ic


@icon("ui_unit_caulker")
def caulker():
    ic = Icon()
    # Union mender: tracks, and a jointed welding arm with its spark.
    body(ic, 2, 11, 11, 5, UNION)
    head(ic, 5, 6)
    ic.rect(2, 8, 3, 3, "o")
    ic.rows(6, 2, [
        "......fg",
        "....ff..",
        "..Of....",
        ".OO.....",
        "OO......",
        ], key={})
    ic.pts("W", [(15, 0), (14, 1), (16, 1), (15, 2)])
    ic.px(15, 1, "y")
    tracks(ic, 1, 16, 13)
    ic.outline()
    return ic


@icon("ui_unit_tender")
def tender():
    ic = Icon()
    # Assembly mender: a walker whose arm holds a mending torch.
    body(ic, 3, 9, 9, 7, ASSEMBLY)
    ic.rect(2, 6, 5, 4, "v")
    ic.hline(2, 6, 6, "c")
    ic.vline(4, 6, 9, "u")
    head(ic, 7, 4)
    ic.rows(11, 1, ["....yW", "...J..", "..Jm..", ".Jm...", "Jm...."])
    legs(ic, 4, 16, 4, ASSEMBLY, h=4)
    ic.outline()
    return ic


@icon("ui_unit_dredger")
def dredger():
    ic = Icon()
    # Assembly siege: a squat walker and its long raised launch rail.
    body(ic, 1, 11, 16, 5, ASSEMBLY)
    ic.rect(2, 8, 4, 3, "A")
    ic.hline(2, 5, 8, "y")
    head(ic, 6, 6)
    ic.rows(12, 5, [
        "..gg....",
        "..fggg..",
        "...ffggg",
        "......ff",
        ], key={})
    ic.rows(12, 8, ["ffe"])
    legs(ic, 3, 16, 9, ASSEMBLY, h=3)
    ic.outline()
    return ic


@icon("ui_unit_barge")
def barge():
    ic = Icon()
    # Assembly transport: a long hull with its hold boards and a pilot.
    ic.rows(0, 12, [
        "mMMMMMMMMMMMMMMMMMm.",
        "JmmmmmmmmmmmmmmmmmJj",
        ".JJJJJJJJJJJJJJJJJj.",
        "..jjjjjjjjjjjjjjjj..",
        ])
    ic.rect(6, 9, 11, 3, "v")
    ic.hline(6, 16, 9, "c")
    for x in (9, 12, 15):
        ic.vline(x, 10, 11, "u")
    head(ic, 1, 6, 5, 6)
    ic.outline()
    return ic


@icon("ui_unit_lifter")
def lifter():
    ic = Icon()
    # Union transport: the gas envelope and the gondola under it.
    ic.rows(0, 1, [
        "....CCCCCCCC....",
        "..CCWWWWWWCCcc..",
        ".CWWWWCCCCCCccc.",
        "CCWWCCCCCCCCcccc",
        "CCCCCCCCCCCCcccu",
        ".cccccccccccccu.",
        "..ccccccccccuu..",
        "....uuuuuuuu....",
        ])
    ic.pts("f", [(4, 9), (4, 10), (11, 9), (11, 10)])
    body(ic, 2, 11, 12, 5, UNION)
    ic.rect(4, 12, 4, 2, "B")
    ic.px(4, 12, "n")
    ic.rect(9, 12, 3, 2, "o")
    ic.outline()
    return ic
