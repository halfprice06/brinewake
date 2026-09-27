"""Art v23: the Saltglass Compact's missing pieces, as hand-placed pixels.

Every drawing here is a list of literal pixel rows, one character per
pixel, or a short list of explicit pixel coordinates.  Nothing is
generated from a curve, resampled or imported.  `build.lua` turns the
rows into layered Aseprite documents under art/source/v23/ and exports
them into the game atlas.  Run `python3 pieces.py OUT.json` to write the
JSON that `build.lua` reads.

The characters name colour roles.  Most are the v21 painter's roles
(tools/art_v21/painter.lua), so every pixel is a colour the Compact
already uses.  The trace's wash and residue colours are the ones the
Union's and the Assembly's traces use (art/source/v10), so a washed glaze
mark sits in the same water as theirs.
"""

import json
import sys

# One character per colour role.  '.' is transparent.
ROLES = {
    "u": "ui1",
    "0": "glaze0",
    "1": "glaze1",
    "2": "glaze2",
    "3": "glaze3",
    "4": "glaze4",
    "c": "crust0",
    "C": "crust1",
    "W": "crust2",
    "*": "glint",
    "r": "rust1",
    "R": "rust2",
    "g": "glass1",
    "G": "glass2",
    "h": "glass3",
    "a": "amber2",
    "A": "amber3",
    "d": "silt",
    "w": "wash0",
    "e": "wash1",
    "m": "residue",
}

HEX = {
    "ui1": "1e303f",
    "glaze0": "221733",
    "glaze1": "33295c",
    "glaze2": "4b3f8f",
    "glaze3": "6d69bd",
    "glaze4": "a4ade0",
    "crust0": "8d88a3",
    "crust1": "c7c5cf",
    "crust2": "efeee4",
    "glint": "fffbe6",
    "rust1": "874636",
    "rust2": "bf673a",
    "glass1": "24586d",
    "glass2": "438e9c",
    "glass3": "91ccd0",
    "amber2": "d78e42",
    "amber3": "f2c365",
    # The v10 traces' colours: the dark silt edge, the two wash tones and
    # the dried residue.
    "silt": "60584e",
    "wash0": "416b73",
    "wash1": "4d7c82",
    "residue": "667a68",
}


class Layer:
    """A transparent w x h cel that pixels are stamped into."""

    def __init__(self, w, h):
        self.w, self.h = w, h
        self.g = [["."] * w for _ in range(h)]

    def px(self, x, y, c):
        assert 0 <= x < self.w and 0 <= y < self.h, (x, y)
        assert c in ROLES, c
        self.g[y][x] = c

    def stamp(self, x0, y0, rows, only=None):
        """Copy literal rows with their top-left at (x0, y0).  '.' is skipped,
        and so is any character not in `only` when it is given."""
        for j, row in enumerate(rows):
            for i, c in enumerate(row):
                if c != "." and (only is None or c in only):
                    self.px(x0 + i, y0 + j, c)

    def rows(self):
        return ["".join(r) for r in self.g]


def split(w, h, rows, groups):
    """One literal drawing split into layers by colour: `groups` maps a
    layer name to the characters it holds."""
    out = {}
    for name, chars in groups.items():
        layer = Layer(w, h)
        layer.stamp(0, 0, rows, only=chars)
        out[name] = layer.rows()
    return out


# ---------------------------------------------------------------- plates
# The doctrine plates (32x24, top-left anchor) share the Union's and the
# Assembly's frame: a two-pixel border in the faction colour on the
# console's ui1 panel, a rivet at two corners.  The Compact's border is
# glaze2 and its rivets are salt.  Each emblem keeps the other two sides'
# silhouette, so HAUL and FIRE read the same across the three factions,
# and is built from the Compact's materials.


def plate_frame():
    f = Layer(32, 24)
    for y in range(24):
        for x in range(32):
            edge = x in (0, 31) or y in (0, 23)
            band = x in (1, 2, 29, 30) or y in (1, 2, 21, 22)
            f.px(x, y, "u" if edge or not band else "2")
    f.px(2, 2, "C")
    f.px(29, 21, "C")
    return f


# HAULING: the Union's gantry and crate.  Here the posts are glazed stilts
# with salt shoes, the beam is a copper yoke, and the crate is a violet
# glaze bin heaped with salt, the emblem's one white.  The staves stand
# where the others' crate slats do (x 11, 16, 21).
HAULING_EMBLEM = [
    # 0         1         2         3
    # 01234567890123456789012345678901
    "................................",  # 0
    "................................",  # 1
    "................................",  # 2
    "................................",  # 3
    "................................",  # 4
    "................................",  # 5
    "..........RRRRRRRRRRRRR.........",  # 6  the copper yoke
    "..........rrrrrrrrrrrrr.........",  # 7
    ".......4................4.......",  # 8  stilt tops
    ".......4.....WWWWCC.....4.......",  # 9  salt heaped over the bin
    ".......4..WWWWWWCCCCCc..4.......",  # 10
    ".......444444444444444444.......",  # 11 the bin's pale rim
    ".......433313333133331324.......",  # 12
    ".......433313333133331324.......",  # 13
    ".......4RRRRRRRRRRRRRRRr4.......",  # 14 a copper hoop
    ".......433313333133331324.......",  # 15
    ".......433313333133331324.......",  # 16
    ".......422212222122221214.......",  # 17
    ".......4................4.......",  # 18
    "......CCC..............CCC......",  # 19 salt shoes
    "................................",  # 20
    "................................",  # 21
    "................................",  # 22
    "................................",  # 23
]

# FIRE CONTROL: the others' balance with a lamp at each end.  Here a salt
# mast carries a copper beam; the left end holds a round mirror (glass, a
# sheen on its upper left and the glint), the right end the sun it
# answers (a glint core, amber rings, four short spikes).
FIRE_EMBLEM = [
    # 0         1         2         3
    # 01234567890123456789012345678901
    "................................",  # 0
    "................................",  # 1
    "................................",  # 2
    "................................",  # 3
    "................................",  # 4
    "................................",  # 5
    "................WC..............",  # 6  the mast
    "................WC..............",  # 7
    "................RR..............",  # 8  the pivot
    "..............rrRRRR............",  # 9  the beam
    ".......ggg..rrrrrRRRRR..a.......",  # 10
    "......g*hGgrrrr.WC.RRRAA*Aa.....",  # 11
    ".....ghhGGGrr...WC...RA***A.....",  # 12
    ".....ghGGGGg....WC...aA***Aa....",  # 13
    ".....gGGGGGg....WC....A***A.....",  # 14
    "......gGGGg.....WC....aA*Aa.....",  # 15
    ".......ggg......WC......a.......",  # 16
    "................WC..............",  # 17
    "................WC..............",  # 18
    "................WC..............",  # 19
    "...............CWCc.............",  # 20 its foot
    "................................",  # 21
    "................................",  # 22
    "................................",  # 23
]


def plate(emblem_rows):
    emblem = Layer(32, 24)
    emblem.stamp(0, 0, emblem_rows)
    return {"frame": plate_frame().rows(), "emblem": emblem.rows()}


# ------------------------------------------------------ headquarters annexes
# A completed doctrine adds a small annex to the Kiln (compact_hq), drawn
# over the headquarters at its anchor like the Union's and the Assembly's
# workshops.  The Kiln leaves room for it on the salt pad between the left
# heliostat's pole (x 31-36 with its foot plate) and the kiln wall (x 52):
# the annex stands on the pad there, x 37-50.  It meets the pad directly on
# its shoes or its own footing course; nothing is drawn beneath it
# (CONTRIBUTING.md: no building shadows).
#
# Both annexes are built on the same 2:1 box, 14 pixels wide: a top diamond
# (rows of 2, 6, 10, 14, 10, 6, 2 pixels) with a left wall under its lower
# left edge and a right wall under its lower right edge.  Light is the
# game's, from the upper left: left walls glaze3, right walls glaze2, the
# last wall row a step darker as the footing.

# HAULING annex: a salt crib.  A slatted glaze bin (staves in the wall's
# darker step) on three stilts, heaped with salt above its crusted rim and
# bound with a copper hoop that runs along the walls at the box's 2:1
# slope.  A crystal on the heap's shaded side catches the light on one
# frame of four.
CRIB = [
    # 0123456789abcd
    "......WW......",  # 0  the heap
    ".....WWWC.....",  # 1
    "...WWWWWCCc...",  # 2
    "..WWWWWWCCCc..",  # 3
    "..WWWWWCCCCc..",  # 4
    ".WWWWWCCCCCcc.",  # 5
    "CCWWWWCCCCCccc",  # 6  the rim's side corners
    "33CCWWCCcccc22",  # 7
    "3333CCCccc2222",  # 8
    "RR2323Cc2121rr",  # 9  the hoop begins at the side corners
    "33RR233221rr22",  # 10
    "2223RR32rr2111",  # 11
    ".22223Rr21111.",  # 12 the hoop at the front corner
    ".2..223211..1.",  # 13
    ".2....21....1.",  # 14 stilts
    ".2....2.....1.",  # 15
    "121...2....121",  # 16 shoes on the pad
    "......2.......",  # 17
    "......2.......",  # 18
    ".....121......",  # 19
]
CRIB_AT = (37, 92)
CRIB_GLINT = (47, 94)


def annex_crib(frame):
    base = Layer(160, 144)
    base.stamp(*CRIB_AT, CRIB)
    finish = Layer(160, 144)
    if frame == 1:
        finish.px(*CRIB_GLINT, "*")
    return {"structure": base.rows(), "finish": finish.rows()}


# FIRE CONTROL annex: a glazed plinth with a salt slab on top, carrying a
# copper mast and a sighting beam with a small mirror hung at each end.
# The mirrors catch the sun in turn: the left on frame 1, the right on
# frame 3.  The plinth's right edge is a glaze1 line so it stays apart from
# the kiln wall behind it.
PLINTH = [
    # 0123456789abcd
    "......CC......",  # 0  the mast cap
    "......Rr......",  # 1
    ".RRRRRRrRRRRR.",  # 2  the beam
    ".g....Rr....g.",  # 3  the mirrors
    "ghG...Rr...Ghg",  # 4
    "gGg...Rr...gGg",  # 5
    ".g....Rr....g.",  # 6
    "......Rr......",  # 7
    "....WWRrCC....",  # 8  the salt slab
    "..WWCCRrCCCC..",  # 9
    "WWCCCCRrCCCCcc",  # 10
    "33WWCCCCCCcc21",  # 11
    "3333WWCCcc2221",  # 12
    "333333Wc222221",  # 13
    "33333332222221",  # 14
    "33333332222221",  # 15
    "22333332222211",  # 16 the footing course
    "..2233322211..",  # 17
    "....223211....",  # 18
    "......21......",  # 19
]
PLINTH_AT = (37, 91)
# A mirror catching the sun: its face (plinth-local pixels) turns glint and
# salt white.
PLINTH_CATCH = (
    (((1, 4), "*"), ((2, 4), "W"), ((1, 5), "W")),
    (((12, 4), "*"), ((11, 4), "W"), ((12, 5), "W")),
)


def annex_plinth(frame):
    base = Layer(160, 144)
    base.stamp(*PLINTH_AT, PLINTH)
    finish = Layer(160, 144)
    if frame in (1, 3):
        for (x, y), c in PLINTH_CATCH[frame // 2]:
            finish.px(PLINTH_AT[0] + x, PLINTH_AT[1] + y, c)
    return {"structure": base.rows(), "finish": finish.rows()}


# ------------------------------------------------------------ glaze trace
# trace_glaze_{0..5} (32x16, anchor 16,8), the Compact's member of the
# trace family (trace_rivet, trace_binding, trace_jack).  Its fire-lances
# and beams fuse the lane's silt into glass where they land, and a Glazier's
# patch on a machine is the same glass.  Frame 0 is the fresh mark: a fused
# bead with its sheen on the upper left (no glint: a trace stays quiet
# beside the machines that matter), a dark silt edge where
# it melted into the lane, two spattered drops.  Frames 1-4 are the wash,
# in the other traces' water colours: the bead goes under (a violet glimmer
# stays for two frames), the ripples pass down across it and wear it away.
# Frame 5 is what a drained lane keeps: a fleck of glass in a residue line.
TRACE = [
    [  # 0 fresh
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "..............2332..............",
        "............134433321...........",
        "...........d122333221d..........",
        ".......32..dd1122211dd..........",
        ".......11d....dddd.......32.....",
        "........d................11d....",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
    ],
    [  # 1 the water arrives
        "................................",
        "................................",
        "................................",
        "......wwwee.....................",
        "..wwww.....eeew.......wwwww.....",
        "..............wwww..............",
        "............wwe2wwwww...........",
        "............wwwwwwwww...........",
        ".......ew....wwwwwww............",
        ".......ww................ew.....",
        ".........................ww.....",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
    ],
    [  # 2 a ripple crosses the bead
        "................................",
        "................................",
        "................................",
        "................................",
        "..............ww................",
        "......wwwee...ww................",
        "..wwww.....eeewwwe.....wwwww....",
        "............ww2wwwwww...........",
        "........w....wwwwww.............",
        "........w................ww.....",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
    ],
    [  # 3 it breaks up
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "..............ww................",
        "..............w.................",
        "................................",
        ".......wwwee..ww......ww........",
        "...wwww...eew2ew.....wwwwwww....",
        "................wwwwwwwww.......",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
    ],
    [  # 4 the last ripples pass
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "...............ww...............",
        "..............www...............",
        "................................",
        "................................",
        "................................",
        "................................",
        ".......wwwee....................",
        "...wwww.....eeew.......wwwww....",
        "................wwwwwww.........",
        "................................",
        "................................",
    ],
    [  # 5 the residue on a drained lane
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "..............m1................",
        "...............mmm..............",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
    ],
]
TRACE_LAYERS = {"glass": "01234*d", "wash": "we", "residue": "m"}


# --------------------------------------------------------------- GLINT
# The Glinter's heliograph flash and the light it throws down a lane.
#
# fx_glint_flash_{0..3} (32x32, anchor 16,16) is drawn at the mirror, on
# the mast, for the first moments.  It is a heliograph's flash, not the
# Heliostat's eight-point burn (fx_beam_flare): a long level streak with a
# short upright one, glint at the core, salt white, then the glaze's pale
# and mid violet toward the tips.  It is biggest on the first frame, the
# moment of the order, and shrinks over the next three.
FLASH = [
    [  # 0
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................3...............",
        "................3...............",
        "................4...............",
        "................4...............",
        "................W...............",
        "................*...............",
        ".............4W***W4............",
        "...3334444WWWW*****WWWW4444333..",
        ".............4W***W4............",
        "................*...............",
        "................W...............",
        "................4...............",
        "................4...............",
        "................3...............",
        "................3...............",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
    ],
    [  # 1
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................3...............",
        "................4...............",
        "................4...............",
        "................W...............",
        "..............W***W.............",
        "......33444WWW*****WWW44433.....",
        "..............W***W.............",
        "................W...............",
        "................4...............",
        "................4...............",
        "................3...............",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
    ],
    [  # 2
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................3...............",
        "................4...............",
        "................W...............",
        "..........3344WW*WW4433.........",
        "................W...............",
        "................4...............",
        "................3...............",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
    ],
    [  # 3
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................3...............",
        ".............334*433............",
        "................3...............",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
    ],
]
FLASH_LAYERS = {"core": "*W", "streaks": "34"}

# fx_glint_spot_{0..3} (32x16, anchor 16,8) marks where the ray's reach
# ends: a small lit point with a ring of light spreading from it along the
# ground's 2:1 diamond, bright and small, then wider, broken and dimmer.
# It loops while the ray is lit.
SPOT = [
    [  # 0
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "...............44...............",
        ".............44..44.............",
        "...........44..**..44...........",
        ".............33..33.............",
        "...............33...............",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
    ],
    [  # 1
        "................................",
        "................................",
        "................................",
        "................................",
        "...............44...............",
        ".............44..44.............",
        "...........44......44...........",
        ".........44..........44.........",
        ".......44......**......33.......",
        ".........44..........33.........",
        "...........33......33...........",
        ".............33..33.............",
        "...............33...............",
        "................................",
        "................................",
        "................................",
    ],
    [  # 2
        "................................",
        "................................",
        "...............33...............",
        "................................",
        "...........33......33...........",
        "................................",
        ".......33..............33.......",
        "................................",
        "...33..........44..........33...",
        "................................",
        ".......33..............33.......",
        "................................",
        "...........33......33...........",
        "................................",
        "...............33...............",
        "................................",
    ],
    [  # 3
        "................................",
        "...............22...............",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        ".22............33............22.",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "................................",
        "...............22...............",
    ],
]
SPOT_LAYERS = {"centre": "*", "ring": "234"}

# fx_glint_mote_{0..2} (16x16, anchor 8,8): a spark of the light running
# out along the ray from the mirror, twinkling as it goes: a cross, a
# smaller cross, a turned cross.
MOTE = [
    [
        "................",
        "................",
        "................",
        "................",
        "................",
        "................",
        "........3.......",
        "........4.......",
        "......34*43.....",
        "........4.......",
        "........3.......",
        "................",
        "................",
        "................",
        "................",
        "................",
    ],
    [
        "................",
        "................",
        "................",
        "................",
        "................",
        "................",
        "................",
        "........4.......",
        ".......4*4......",
        "........4.......",
        "................",
        "................",
        "................",
        "................",
        "................",
        "................",
    ],
    [
        "................",
        "................",
        "................",
        "................",
        "................",
        "................",
        "................",
        ".......3.3......",
        "........*.......",
        ".......3.3......",
        "................",
        "................",
        "................",
        "................",
        "................",
        "................",
    ],
]
MOTE_LAYERS = {"mote": "*34"}


def sequence(name, w, h, anchor, frames, groups, ms, tag):
    return {
        "name": name,
        "w": w,
        "h": h,
        "anchor": list(anchor),
        "layers": list(groups),
        "frames": [
            {"key": f"{name}_{k}", "ms": ms, "cels": split(w, h, rows, groups)}
            for k, rows in enumerate(frames)
        ],
        "tag": tag,
    }


def documents():
    docs = []
    for name, emblem in (("hauling", HAULING_EMBLEM), ("fire_control", FIRE_EMBLEM)):
        docs.append(
            {
                "name": f"doctrine_compact_{name}",
                "w": 32,
                "h": 24,
                "anchor": [0, 0],
                "layers": ["frame", "emblem"],
                "frames": [{"key": f"doctrine_compact_{name}", "ms": 200, "cels": plate(emblem)}],
                "tag": "plate",
            }
        )
    for name, fn in (("hauling", annex_crib), ("fire_control", annex_plinth)):
        docs.append(
            {
                "name": f"doctrine_compact_{name}_hq",
                "w": 160,
                "h": 144,
                "anchor": [80, 128],
                "layers": ["structure", "finish"],
                "frames": [
                    {"key": f"doctrine_compact_{name}_hq_{k}", "ms": 200, "cels": fn(k)}
                    for k in range(4)
                ],
                "tag": "annex",
            }
        )
    docs.append(sequence("trace_glaze", 32, 16, (16, 8), TRACE, TRACE_LAYERS, 200, "trace"))
    docs.append(sequence("fx_glint_flash", 32, 32, (16, 16), FLASH, FLASH_LAYERS, 67, "flash"))
    docs.append(sequence("fx_glint_spot", 32, 16, (16, 8), SPOT, SPOT_LAYERS, 133, "spot"))
    docs.append(sequence("fx_glint_mote", 16, 16, (8, 8), MOTE, MOTE_LAYERS, 100, "mote"))
    return docs


def check(docs):
    for d in docs:
        for f in d["frames"]:
            for layer in d["layers"]:
                rows = f["cels"][layer]
                assert len(rows) == d["h"], (d["name"], layer)
                assert all(len(r) == d["w"] for r in rows), (d["name"], layer)


def literal_widths():
    """Every literal drawing keeps its declared width (a short or long row
    would shift its pixels)."""
    for rows, w in (
        (HAULING_EMBLEM, 32),
        (FIRE_EMBLEM, 32),
        (CRIB, 14),
        (PLINTH, 14),
        *((f, 32) for f in TRACE),
        *((f, 32) for f in FLASH),
        *((f, 32) for f in SPOT),
        *((f, 16) for f in MOTE),
    ):
        for i, r in enumerate(rows):
            assert len(r) == w, (i, len(r), r)


def main():
    literal_widths()
    docs = documents()
    check(docs)
    json.dump({"roles": ROLES, "hex": HEX, "documents": docs}, open(sys.argv[1], "w"))
    print(f"{len(docs)} documents, {sum(len(d['frames']) for d in docs)} frames")


if __name__ == "__main__":
    main()
