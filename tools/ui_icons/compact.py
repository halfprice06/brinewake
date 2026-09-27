"""The Saltglass Compact's icons: its eight machines, its buildings, and the
orders only it or the Confluence has (GLINT, LAY, and DRY for each arm).

A Compact machine has no cube head: it wears a violet glaze pod with a
salt-white cap and a glass lens, and stands on thin stilts with salt shoes.
Each icon shows that pod and the one part that names the machine (the rake,
the lance, the dish, the mirror mast, the kiln pot, the hopper, the pan)."""

from draw import disc, iso_box
from icons import icon, Icon

COMPACT = {"lit": "l", "mid": "I", "dark": "i"}


def pod(ic, x, y, w, h, lens=True):
    """The glazed pod: a violet body lit on its upper-left, a crust cap."""
    ic.rect(x, y, w, h, "I")
    ic.vline(x, y, y + h - 1, "l")
    ic.hline(x + 1, x + w - 1, y + h - 1, "i")
    ic.vline(x + w - 1, y + 1, y + h - 1, "i")
    ic.hline(x, x + w - 1, y, "t")
    ic.hline(x + 1, x + w - 2, y - 1, "S")
    if lens:
        ic.px(x + w - 2, y + 2, "B")
        ic.px(x + w - 3, y + 2, "n")


def stilt(ic, x, y, h, back=1):
    """A stilt from its hip at (x, y) down `h` rows: a backward knee half
    way, a salt shoe."""
    knee = y + h // 2
    ic.vline(x, y, knee, "i")
    ic.vline(x - back, knee, y + h - 1, "i")
    ic.px(x - back + 1, knee, "O")
    ic.hline(x - back - 1, x - back + 1, y + h, "S")
    ic.px(x - back - 1, y + h, "t")


def dish(ic, x, y, d, rim="O"):
    """A round mirror: rim, glass, a sheen across it and the glint."""
    disc(ic, x, y, d, rim)
    disc(ic, x + 1, y + 1, d - 2, "B")
    for i in range(d - 3):
        ic.px(x + 1 + i, y + d - 3 - i, "n")
    ic.px(x + 2, y + 1, "T")
    ic.px(x + 1, y + 2, "n")


# --------------------------------------------------------------- machines


@icon("ui_unit_raker")
def raker():
    ic = Icon()
    # Worker: a shallow basket on four stilts, the rake held up and back.
    pod(ic, 6, 11, 12, 4)
    ic.hline(7, 16, 11, "s")  # salt inside the basket
    ic.hline(8, 14, 10, "S")
    for sx in (7, 10, 14, 17):
        stilt(ic, sx, 15, 6, back=1)
    # The rake: a copper pole from the front corner up and back, the head.
    for i in range(9):
        ic.px(16 - i, 10 - i, "O")
        ic.px(17 - i, 10 - i, "o")
    ic.hline(4, 10, 1, "i")
    ic.pts("S", [(4, 2), (6, 2), (8, 2), (10, 2)])
    ic.outline()
    return ic


@icon("ui_unit_brander")
def brander():
    ic = Icon()
    # Line machine: a lean pod on a long stride, the fire-lance held level.
    ic.rect(2, 7, 4, 7, "l")  # the brine tank on its back
    ic.vline(2, 7, 13, "L")
    ic.vline(5, 8, 13, "i")
    pod(ic, 5, 8, 9, 6, lens=False)
    ic.rect(9, 6, 5, 3, "t")
    ic.hline(9, 13, 6, "T")
    ic.hline(10, 13, 8, "B")
    # The lance: copper, a brass band, the burner lens at its tip.
    ic.hline(6, 20, 11, "O")
    ic.hline(6, 20, 12, "o")
    ic.pts("P", [(12, 11), (13, 11)])
    ic.rect(20, 10, 3, 3, "B")
    ic.px(20, 10, "T")
    stilt(ic, 7, 14, 7, back=-2)
    stilt(ic, 11, 14, 7, back=2)
    ic.outline()
    return ic


@icon("ui_unit_heliostat")
def heliostat():
    ic = Icon()
    # Heavy: the great mirror dish on a copper yoke over a braced body.
    dish(ic, 1, 1, 14)
    ic.pts("o", [(15, 7), (16, 8), (16, 9)])
    pod(ic, 4, 14, 12, 4, lens=False)
    ic.vline(8, 12, 13, "O")
    ic.vline(11, 12, 13, "o")
    for sx in (5, 9, 13, 17):
        stilt(ic, sx, 18, 4, back=1 if sx < 11 else -1)
    ic.outline()
    return ic


@icon("ui_unit_glinter")
def glinter():
    ic = Icon()
    # Harasser: a small pod under a tall mast, the heliograph mirror on top
    # throwing its glint.
    dish(ic, 3, 0, 10)
    ic.vline(8, 10, 12, "O")
    pod(ic, 5, 13, 8, 4)
    stilt(ic, 7, 17, 5, back=1)
    stilt(ic, 11, 17, 5, back=1)
    # The glint: a four-point star beside the mirror.
    ic.pts("T", [(15, 3), (14, 3), (16, 3), (15, 2), (15, 4), (13, 3), (17, 3)])
    ic.pts("n", [(15, 1), (15, 5)])
    ic.outline()
    return ic


@icon("ui_unit_stilt")
def stilt_unit():
    ic = Icon()
    # Scout: two very long legs, a small pod, a neck held forward to the
    # spyglass lens, a salt vane behind.
    pod(ic, 6, 7, 7, 4, lens=False)
    for i in range(4):
        ic.px(12 + i, 6 - i, "l")
    ic.rect(13, 1, 4, 3, "I")
    ic.hline(13, 16, 1, "t")
    ic.rect(15, 2, 2, 2, "B")
    ic.px(15, 2, "T")
    ic.pts("S", [(4, 6), (3, 5), (2, 4), (2, 3), (3, 4)])
    stilt(ic, 8, 11, 11, back=2)
    stilt(ic, 11, 11, 11, back=2)
    ic.outline()
    return ic


@icon("ui_unit_glazier")
def glazier():
    ic = Icon()
    # Mender: a round kiln pot glowing at its mouth, the blowpipe and its
    # gather of hot glass held out.
    ic.rect(4, 7, 9, 9, "I")
    ic.vline(4, 7, 15, "l")
    ic.vline(5, 7, 15, "l")
    ic.vline(12, 8, 15, "i")
    ic.hline(4, 12, 11, "S")
    ic.hline(4, 12, 6, "O")
    ic.hline(5, 11, 5, "A")
    ic.hline(6, 10, 5, "y")
    ic.rect(2, 3, 2, 4, "I")
    ic.px(2, 3, "l")
    ic.pts("O", [(13, 10), (14, 11), (15, 11), (16, 12), (17, 12), (18, 13)])
    ic.rect(19, 12, 3, 3, "A")
    ic.px(19, 12, "y")
    ic.px(20, 12, "T")
    for sx in (5, 8, 11, 14):
        stilt(ic, sx, 16, 5, back=1)
    ic.outline()
    return ic


@icon("ui_unit_salter")
def salter():
    ic = Icon()
    # Causeway layer: a long hopper heaped with salt on four stilts, the
    # chute sloping to the ground and salt pouring from it.
    ic.rect(1, 9, 15, 6, "I")
    ic.vline(1, 9, 14, "l")
    ic.hline(2, 15, 14, "i")
    ic.hline(0, 16, 8, "i")
    ic.rows(2, 4, [
        "....tttttt...",
        "..ttTttttttS.",
        ".tttttttttSSS",
        "SSSSSSSSSSSSS",
    ])
    ic.rows(16, 10, ["ii..", ".ii.", "..ii", "...i"])
    ic.pts("t", [(20, 14), (19, 16), (20, 18), (19, 20)])
    ic.hline(17, 22, 21, "S")
    ic.hline(18, 21, 20, "t")
    for sx in (3, 6, 11, 14):
        stilt(ic, sx, 15, 5, back=1)
    ic.outline()
    return ic


@icon("ui_unit_pan")
def pan():
    ic = Icon()
    # Mobile still: a wide brine pan ringed with mirror petals, on stilts.
    ic.rows(1, 10, [
        "...iiiiiiiiiiiii....",
        ".iISSSSSSSSSSSSSIi..",
        "iISBBBBnnnnBBBBBSIi.",
        "iISBBnnnnBBBBBBBSIi.",
        ".iISSSSSSSSSSSSSIi..",
        "...iiiiiiiiiiiii....",
    ])
    ic.px(8, 12, "T")
    for x, y in ((1, 3), (8, 1), (14, 3)):
        disc(ic, x, y, 6, "O")
        disc(ic, x + 1, y + 1, 4, "B")
        ic.px(x + 1, y + 2, "n")
        ic.px(x + 2, y + 1, "T")
    ic.pts("o", [(4, 9), (11, 7), (17, 9)])
    for sx in (5, 9, 13, 17):
        stilt(ic, sx, 16, 5, back=1)
    ic.outline()
    return ic


# -------------------------------------------------------------- buildings


@icon("ui_build_compact_hq")
def compact_hq():
    ic = Icon()
    # The Kiln: a banded bottle kiln on its salt pad, the mouth aglow.
    iso_box(ic, 1, 15, 20, 2, "S", "s", "i")
    ic.rows(3, 2, [
        "......tt......",
        ".....iIIl.....",
        ".....IIli.....",
        "....tttttt....",
        "...lIIIIIIi...",
        "..tttttttttt..",
        ".lllIIIIIIIIi.",
        ".llIIIIIIIIIi.",
        "lllIOOOOOOIIii",
        "llIIIIIIIIIIii",
        "llIIIkAAkIIIii",
        "llIIkAyyAkIIii",
        "llIIkAyyAkIIii",
        ".lIIkAyyAkIIi.",
        "..ttttttttti..",
    ])
    ic.outline()
    return ic


@icon("ui_build_compact_works")
def compact_works():
    ic = Icon()
    # The Glassworks: a glass-roofed hall on stilts and its tall chimney.
    ic.rect(16, 0, 3, 11, "I")
    ic.vline(16, 0, 10, "l")
    ic.hline(16, 18, 0, "t")
    ic.px(17, 5, "O")
    b = iso_box(ic, 1, 7, 20, 7, "B", "I", "i", edge="l")
    for x in range(3, 20, 4):
        for y in range(24):
            if ic.get(x, y) == "B" and y <= b.get(x, 99):
                ic.px(x, y, "O")
    ic.pts("n", [(6, 9), (7, 9), (12, 8), (13, 8)])
    ic.px(5, 10, "T")
    ic.rect(4, 15, 3, 3, "A")
    ic.px(4, 15, "y")
    ic.rect(14, 15, 4, 2, "B")
    for sx in (3, 11, 19):
        ic.vline(sx, 20, 21, "i")
    ic.hline(1, 21, 22, "S")
    ic.outline()
    return ic


@icon("ui_build_compact_drydock")
def compact_drydock():
    ic = Icon()
    # The Drydock: the stilted gantry over the slip, a new pod in the cradle.
    iso_box(ic, 1, 14, 20, 3, "B", "s", "i")
    ic.vline(2, 3, 18, "I")
    ic.vline(3, 3, 18, "l")
    ic.vline(16, 3, 13, "I")
    ic.hline(2, 17, 3, "O")
    ic.hline(2, 17, 2, "p")
    ic.vline(9, 4, 8, "k")
    ic.rect(8, 8, 3, 2, "O")
    pod(ic, 6, 12, 8, 4, lens=False)
    ic.pts("i", [(7, 16), (12, 16)])
    ic.outline()
    return ic


@icon("ui_build_compact_palisade")
def compact_palisade():
    ic = Icon()
    # The Palisade: salt blocks coursed between two glazed posts.
    ic.rows(1, 6, [
        "ll..................",
        "lI................ll",
        "lIttSttS.........lI",
        "lISSsSSsSttS.....lI",
        "lIttSttSSSSsSttS.lI",
        "lISSsSSsSttSSSSsSlI",
        "lIttSttSSSSsSttSSlI",
        "lISSsSSsSttSSSSsSlI",
        "lIiiiiiiSSSsSttSSlI",
        "lI......iiiiiiSSslI",
        "lI............iiilI",
        "ii...............ii",
    ])
    ic.pts("t", [(1, 5), (18, 6), (19, 6)])
    ic.outline()
    return ic


@icon("ui_build_compact_dropoff")
def compact_dropoff():
    ic = Icon()
    # The Rake shed: a canopy on stilts over a heap of salvage.
    ic.rows(1, 2, [
        ".........lL.........",
        ".......lLLlI........",
        ".....lLLlIIIIi......",
        "...lLLlIIIIIIIii....",
        ".lLLlIIIIIIIIIIiii..",
        "tttttttttttttttttttt",
    ])
    for sx in (2, 20):
        ic.vline(sx, 8, 19, "i")
    ic.vline(11, 8, 20, "i")
    ic.rows(3, 14, [
        "....OOpO......",
        "..OOoOOnOO....",
        ".OoOOBOOoOOO..",
        "iiiiiiiiiiiiii",
    ])
    ic.hline(0, 21, 20, "S")
    ic.outline()
    return ic


@icon("ui_build_compact_tower")
def compact_tower():
    ic = Icon()
    # The mirror nest: dishes round a lens, high on a stilted tripod.
    dish(ic, 5, 0, 10)
    disc(ic, 1, 4, 6, "O")
    disc(ic, 2, 5, 4, "B")
    ic.px(2, 5, "T")
    disc(ic, 14, 4, 6, "O")
    disc(ic, 15, 5, 4, "B")
    ic.px(15, 5, "T")
    ic.hline(4, 17, 11, "t")
    ic.hline(5, 16, 12, "I")
    ic.hline(6, 15, 13, "i")
    for x0, x1 in ((6, 3), (10, 10), (15, 18)):
        for y in range(14, 22):
            t = (y - 14) / 7
            ic.px(round(x0 + (x1 - x0) * t), y, "I")
    ic.hline(2, 19, 22, "S")
    ic.outline()
    return ic


# ------------------------------------------------------------------ orders


@icon("ui_cmd_glint")
def glint():
    ic = Icon()
    # GLINT: a heliograph mirror throwing a long, narrow ray.
    dish(ic, 1, 7, 10)
    ic.vline(6, 17, 21, "O")
    ic.hline(3, 9, 21, "o")
    for x in range(11, 24):
        ic.px(x, 11, "T")
        ic.px(x, 12, "t" if x % 3 else "T")
        if x > 13:
            ic.px(x, 10, "n")
            ic.px(x, 13, "n")
    ic.pts("T", [(10, 9), (10, 14)])
    ic.outline()
    return ic


@icon("ui_cmd_lay")
def lay():
    ic = Icon()
    # LAY: a salt causeway three cells wide crossing the water.
    for y in range(9, 24):
        for x in range(24):
            if (x + 2 * y) % 5 == 0:
                ic.px(x, y, "Q")
            else:
                ic.px(x, y, "q")
    # The strip, running up-right across the water, laid in rows.
    for y in range(9, 24):
        x0 = 20 - (y - 9)
        for x in range(x0 - 5, x0 + 5):
            if 0 <= x < 24:
                ic.px(x, y, "t" if (y - 9) % 4 else "S")
        ic.px(x0 - 6, y, "s")
        ic.px(x0 + 5, y, "s")
    # The head of the strip: new salt falling.
    ic.pts("T", [(16, 6), (18, 4), (15, 3)])
    ic.pts("S", [(17, 7), (14, 5)])
    ic.outline()
    return ic


def arms(ic, dry):
    """The Confluence's three arms meeting at the station: E, W and S. The
    dry arm is sand, the others deep water; `dry=None` floods them all."""
    water = {"E": [(13, 8, 11, 5)], "W": [(0, 8, 11, 5)], "S": [(9, 12, 6, 12)]}
    for arm, rects in water.items():
        for x, y, w, h in rects:
            wet = arm != dry
            ic.rect(x, y, w, h, "q" if wet else "v")
            for yy in range(y, y + h):
                for xx in range(x, x + w):
                    if wet and (xx + 2 * yy) % 5 == 0:
                        ic.px(xx, yy, "Q")
                    if not wet and (xx * 3 + yy) % 7 == 0:
                        ic.px(xx, yy, "u")
    # The station: a round gate where the arms meet.
    disc(ic, 8, 6, 8, "g")
    disc(ic, 9, 7, 6, "f")
    ic.pts("h", [(10, 7), (9, 8), (11, 7)])


def dry_icon(arm):
    def draw():
        ic = Icon()
        arms(ic, arm)
        ic.outline()
        return ic

    return draw


for _arm in ("E", "W", "S"):
    icon("ui_cmd_dry_" + _arm.lower())(dry_icon(_arm))
icon("ui_cmd_flood")(dry_icon(None))
