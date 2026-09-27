"""Build the layers and pieces of the three-arm sluice station (art v22).

The Confluence's station was the Split Basin's two-shaft lock, with N and S
painted over its shafts: on a map whose arms are E, W and S those letters
named nothing.  This pass gives the Confluence its own station, built on
the v13 lock (the same hull, walls and shafts, so the two maps share one
landmark), and drawn so it needs no letters:

- three gauge shafts, one per arm, where the arm leaves the station on
  screen: the left shaft is the arm running up-left (W), the right shaft
  the arm running up-right (E), and a shorter shaft standing on the deck's
  front lip is the arm running down toward the viewer (S);
- each shaft's float shows its arm's water: low when the arm is dry, high
  when it is deep, half way on the neutral tide;
- the semaphore arm and the handwheel's pale pointer both point along the
  dry arm, and stand upright when no arm is dry (neutral or flood).

The station is drawn in pieces so the engine can compose any tide and ease
between them: a base, a float per shaft level, the semaphore at ten
angles, the handwheel at eight turns and the warning pointers.

This is scripted pixel art.  The base reuses the v13 layers pixel for
pixel except where named below; the new front shaft, pulley and pieces are
placed by this script from the v13 shafts' own clusters and palette, and
the new semaphore angles are laid along straight and 45-degree lines.  No
image generation or external bitmap contributed a pixel.

Usage: python3 station3.py <v13 layer dir> <out dir>
  <v13 layer dir> holds the v13 `gate_north_dry` / `gate_south_dry` rest
  frames' layers, exported by station3.lua as
  {north,south}_{rear,hulls,cabins,working}.png.
Writes <out dir>/layers/*.png (the base document's layers),
<out dir>/pieces/*.png (one full 160x144 frame per piece) and
<out dir>/pieces.json (key, family, frame file).
"""

import json
import math
import os
import sys

from PIL import Image

W, H = 160, 144
CLEAR = (0, 0, 0, 0)

# The v13 palette roles this pass uses (all already in the atlas).
INK = (38, 56, 68, 255)
FRAME = (64, 84, 101, 255)
GLASS = (21, 46, 66, 255)
GLINT = (67, 142, 156, 255)
CAP = (190, 183, 155, 255)
RIM = (158, 180, 181, 255)
MAST = (101, 127, 137, 255)
ROPE = (125, 99, 78, 255)
PULLEY = (191, 103, 58, 255)
FLOAT = (157, 87, 56, 255)
FLOAT_TOP = (242, 195, 101, 255)
WATER = (77, 124, 130, 255)
FLAG_EDGE = (85, 53, 58, 255)
FLAG_STRIPE = (231, 217, 181, 255)
SPOKE = (233, 153, 78, 255)
POINTER = (190, 183, 155, 255)
HUB = (158, 180, 181, 255)
AMBER = (215, 142, 66, 255)
AMBER_LIGHT = (242, 195, 101, 255)

# v13 geometry, 160x144 frame, anchor (80, 128).
SIDE_FLOAT_X = {"left": 51, "right": 97}  # float's left column; rope at +6
SIDE_HIGH, SIDE_LOW = 63, 83  # float top row when deep / dry
SIDE_WATER_BOTTOM = 93
ROPE_TOP = 43
LETTER_BOXES = [(50, 23, 64, 34), (96, 23, 110, 34)]
FLAG_BOX = (56, 12, 106, 24)
PIVOT = (80.5, 21.5)
WHEEL_CENTRE = (81.0, 110.0)
WHEEL_BOX = (66, 98, 96, 123)
WHEEL_R = (10.5, 8.5)
POINTER_R = (12.5, 9.5)

# The new front shaft (the arm that runs toward the viewer).  It is the side
# shaft's construction at 15 columns: ink, frame, 7 of glass, frame with
# the tick marks, ink.  Its floor row is the side shafts' dry row, so a dry
# arm's float sits on the same line in every shaft.
FRONT_X0, FRONT_X1 = 73, 87  # inclusive outer columns
FRONT_GLASS = (77, 83)
FRONT_FLOAT_X = 76  # float columns 76..84, rope at 80
FRONT_HIGH, FRONT_LOW = 67, 83
FRONT_WATER_BOTTOM = 89
FRONT_TOP = 57  # cap row
FRONT_BOTTOM = 93

SIDE_TRAVEL = SIDE_LOW - SIDE_HIGH  # 20
FRONT_TRAVEL = FRONT_LOW - FRONT_HIGH  # 16


def load(path):
    return Image.open(path).convert("RGBA")


def blank():
    return Image.new("RGBA", (W, H), CLEAR)


def put(im, x, y, c):
    if 0 <= x < W and 0 <= y < H:
        im.putpixel((x, y), c)


# ---------------------------------------------------------------- base

def strip_working_parts(wp):
    """The v13 working parts without what this pass redraws as pieces: the
    letters, the semaphore arm, the hanging ropes, floats and water, and
    the handwheel's spokes.  Kept: mast and pivot knob, pulleys, the rope
    bar, and the handwheel's rim."""
    im = wp.copy()
    for x0, y0, x1, y1 in LETTER_BOXES:
        for y in range(y0, y1):
            for x in range(x0, x1):
                if im.getpixel((x, y)) == FLAG_STRIPE:
                    im.putpixel((x, y), CLEAR)
    x0, y0, x1, y1 = FLAG_BOX
    for y in range(y0, y1):
        for x in range(x0, x1):
            if im.getpixel((x, y)) in (FLAG_EDGE, FLAG_STRIPE):
                im.putpixel((x, y), CLEAR)
    for side, fx in SIDE_FLOAT_X.items():
        for y in range(ROPE_TOP, SIDE_WATER_BOTTOM + 1):
            for x in range(fx, fx + 13):
                im.putpixel((x, y), CLEAR)
    return blank_wheel(im)


def blank_wheel(src):
    """The handwheel with spokes, pointer and hub painted out: each hidden
    pixel takes the rim colour found at the same radius (v20's method)."""
    im = src.copy()
    cx, cy = WHEEL_CENTRE
    x0, y0, x1, y1 = WHEEL_BOX

    def rho(x, y):
        return math.hypot((x + 0.5 - cx) / 13.0, (y + 0.5 - cy) / 11.0)

    samples = []
    for y in range(y0, y1):
        for x in range(x0, x1):
            p = src.getpixel((x, y))
            if p[3] and p not in (SPOKE, POINTER, HUB):
                dx, dy = abs(x + 0.5 - cx), abs(y + 0.5 - cy)
                if dx > 2 and dy > 2:
                    samples.append((rho(x, y), p))
    for y in range(y0, y1):
        for x in range(x0, x1):
            p = src.getpixel((x, y))
            if p in (SPOKE, POINTER, HUB):
                # The pointer reaches past the rim onto the housing: clear it.
                if rho(x, y) > 1.0:
                    im.putpixel((x, y), CLEAR)
                else:
                    r = rho(x, y)
                    im.putpixel((x, y), min(samples, key=lambda s: abs(s[0] - r))[1])
    return im


def front_shaft():
    """The S arm's shaft on the deck's front lip, and its pulley on the bar."""
    im = blank()
    x0, x1 = FRONT_X0, FRONT_X1
    top, bottom = FRONT_TOP, FRONT_BOTTOM
    g0, g1 = FRONT_GLASS
    # Cap: a light slab one column narrower than the ink at each side, then
    # two rows between ink, then the pale rim, like the side shafts.
    for x in range(x0 + 1, x1):
        put(im, x, top, CAP)
    for y in (top + 1, top + 2):
        put(im, x0, y, INK)
        put(im, x1, y, INK)
        for x in range(x0 + 1, x1):
            put(im, x, y, CAP)
    y = top + 3
    put(im, x0, y, INK)
    put(im, x0 + 1, y, INK)
    put(im, x1, y, INK)
    put(im, x1 - 1, y, INK)
    for x in range(x0 + 2, x1 - 2):
        put(im, x, y, RIM)
    put(im, x1 - 2, y, FRAME)
    # Frame rows under the rim, then the glass window, then the foot.
    for y in range(top + 4, bottom - 1):
        put(im, x0, y, INK)
        put(im, x0 + 1, y, INK)
        put(im, x1, y, INK)
        put(im, x1 - 1, y, INK)
        for x in range(x0 + 2, x1 - 1):
            put(im, x, y, FRAME)
    for y in range(top + 6, FRONT_WATER_BOTTOM + 1):
        for x in range(g0, g1 + 1):
            put(im, x, y, GLASS)
        if top + 7 <= y <= FRONT_WATER_BOTTOM - 1:
            put(im, g0 + 1, y, GLINT)
    # Tick marks every sixth row on the right frame, as on the side shafts
    # (their ticks sit at 62, 68 ... 86; these share the rows).
    for y in (68, 74, 80, 86):
        put(im, x1 - 3, y, CAP)
        put(im, x1 - 2, y, CAP)
    for y in (bottom - 1, bottom):
        for x in range(x0, x1 + 1):
            put(im, x, y, INK)
    return im


def front_pulley(im):
    """A small block under the rope bar at the mast, carrying the S rope."""
    for x, y, c in [
        (79, 41, INK), (80, 41, PULLEY), (81, 41, INK),
        (79, 42, INK), (80, 42, PULLEY), (81, 42, INK),
    ]:
        put(im, x, y, c)


# ---------------------------------------------------------------- pieces

def side_float(level):
    """Rope, float and water of a side shaft (left position), `level`
    pixels above the dry row."""
    im = blank()
    x = SIDE_FLOAT_X["left"]
    top = SIDE_LOW - level
    for y in range(ROPE_TOP, top):
        put(im, x + 6, y, ROPE)
    for dx in range(13):
        edge = dx in (0, 12)
        put(im, x + dx, top, FLOAT if edge else FLOAT_TOP)
        put(im, x + dx, top + 1, FLOAT if edge else FLOAT_TOP)
        put(im, x + dx, top + 2, FLOAT)
        put(im, x + dx, top + 3, FLOAT)
    for y in range(top + 4, SIDE_WATER_BOTTOM + 1):
        for dx in range(4, 10):
            put(im, x + dx, y, WATER)
    return im


def front_float(level):
    im = blank()
    x = FRONT_FLOAT_X
    top = FRONT_LOW - level
    for y in range(ROPE_TOP, top):
        # Where it crosses the deck the rope needs no ink: brown on sand
        # already reads, and an outline would make it a second mast.
        put(im, x + 4, y, ROPE)
    for dx in range(9):
        edge = dx in (0, 8)
        put(im, x + dx, top, FLOAT if edge else FLOAT_TOP)
        put(im, x + dx, top + 1, FLOAT if edge else FLOAT_TOP)
        put(im, x + dx, top + 2, FLOAT)
        put(im, x + dx, top + 3, FLOAT)
    for y in range(top + 4, FRONT_WATER_BOTTOM + 1):
        for dx in range(3, 6):
            put(im, x + dx, y, WATER)
    return im


def flag_cluster(wp):
    x0, y0, x1, y1 = FLAG_BOX
    out = {}
    for y in range(y0, y1):
        for x in range(x0, x1):
            p = wp.getpixel((x, y))
            if p in (FLAG_EDGE, FLAG_STRIPE):
                out[(x, y)] = p
    return out


def mirror_cluster(cluster):
    return {(W - x, y): p for (x, y), p in cluster.items()}


# The semaphore's angles, counter-clockwise from screen east.  158 and 22
# are the v13 drawings (W and E); 90 stands upright (no arm dry); 270
# hangs down the mast (the S arm, toward the viewer); the rest are the
# in-betweens a swing passes through.
FLAG_ANGLES = [0, 22, 45, 90, 135, 158, 180, 225, 270, 315]

# The new angles are not rotations of the v13 blade (a rotated 23-pixel
# blade breaks into ragged steps): each is laid on the pixel grid along a
# straight or 45-degree line, which has a perfect run rhythm, with the v13
# blade's cross-section: a maroon outline on the lit side, the cream stripe,
# a maroon body and a maroon edge, a swallowtail at the tail and a two-pixel
# neck at the knob.  The lit side faces the upper-left key light.
#   (direction, lit-side normal, first pixel beside the knob, length)
AXIS_BLADES = {
    180: ((-1, 0), (0, -1), (77, 21), 20),
    # Upright it is shorter, so the tail clears the top of the frame.
    90: ((0, -1), (-1, 0), (80, 18), 16),
    270: ((0, 1), (-1, 0), (80, 26), 20),
}
# Diagonal blades are bands of 45-degree lines, shifted a column apart:
#   (direction, column shift toward the lit side, first pixel, steps)
DIAGONAL_BLADES = {
    135: ((-1, -1), 1, (77, 19), 14),
    225: ((-1, 1), -1, (77, 24), 14),
}


def axis_blade(direction, normal, start, length):
    """Five rows across, as thick as the v13 blade at its middle: the lit
    outline (+2), two stripe rows, the body and the edge.  A one-pixel-wide
    neck would read as an exclamation mark over the knob, so the neck is
    three wide and only two long."""
    colours = {2: FLAG_EDGE, 1: FLAG_STRIPE, 0: FLAG_STRIPE, -1: FLAG_EDGE, -2: FLAG_EDGE}
    (dx, dy), (nx, ny), (x0, y0) = direction, normal, start
    out = {}
    for t in range(length):
        for k, c in colours.items():
            if t < 2 and k not in (1, 0, -1):
                continue  # the neck
            if t < 2:
                c = FLAG_EDGE
            if t == length - 1 and k in (1, 0, -1):
                continue  # the swallowtail's notch
            if t == length - 2 and k == 0:
                continue
            out[(x0 + t * dx + k * nx, y0 + t * dy + k * ny)] = c
    return out


def diagonal_blade(direction, shift, start, steps):
    """Five 45-degree lines: lit outline, two stripe lines, body, edge."""
    colours = {2: FLAG_EDGE, 1: FLAG_STRIPE, 0: FLAG_STRIPE, -1: FLAG_EDGE, -2: FLAG_EDGE}
    (dx, dy), (x0, y0) = direction, start
    out = {}
    for t in range(steps):
        for k, c in colours.items():
            if t < 2 and k not in (0, -1):
                continue
            if t < 2:
                c = FLAG_EDGE
            if t == steps - 1 and k in (1, 0, -1):
                continue
            if t == steps - 2 and k in (0, -1):
                continue
            out[(x0 + t * dx + k * shift, y0 + t * dy)] = c
    return out


def blade(angle, north_flag, south_flag):
    if angle == 158:
        return dict(north_flag)
    if angle == 22:
        return dict(south_flag)
    if angle in AXIS_BLADES:
        return axis_blade(*AXIS_BLADES[angle])
    if angle in DIAGONAL_BLADES:
        return diagonal_blade(*DIAGONAL_BLADES[angle])
    # The right-hand angles mirror the left-hand ones about the knob.
    mirrored = 180 - angle if angle < 90 else 540 - angle
    return mirror_cluster(blade(mirrored, north_flag, south_flag))


def flag_piece(angle, north_flag, south_flag, mast_mask):
    cluster = blade(angle, north_flag, south_flag)
    im = blank()
    # The blade turns behind the mast's knob, like the v13 rest drawings; a
    # blade upright or hanging lies along the mast, so it is drawn in front.
    in_front = angle in (90, 270)
    for (x, y), c in cluster.items():
        if in_front or not mast_mask.get((x, y)):
            put(im, x, y, c)
    return im


def spoke(im, angle, reach, colour, start=0.0):
    cx, cy = WHEEL_CENTRE
    a = math.radians(angle)
    ex, ey = math.cos(a) * reach[0], math.sin(a) * reach[1]
    length = math.hypot(ex, ey)
    ux, uy = ex / length, ey / length
    x0, y0, x1, y1 = WHEEL_BOX
    for y in range(y0, y1):
        for x in range(x0, x1):
            fx, fy = x + 0.5 - cx, y + 0.5 - cy
            along = fx * ux + fy * uy
            across = -fx * uy + fy * ux
            if start <= along <= length and abs(across) <= 1.0:
                im.putpixel((x, y), colour)


def wheel_piece(step):
    """Spokes, pale pointer and hub with the pointer at `step` eighths of a
    turn clockwise from east (screen angles: 90 is down)."""
    im = blank()
    pointer = step * 45
    for k in range(4):
        spoke(im, pointer + 90 * k, WHEEL_R, SPOKE)
    spoke(im, pointer, POINTER_R, POINTER, start=2.0)
    for x, y in [(80, 109), (81, 109), (80, 110), (81, 110)]:
        im.putpixel((x, y), HUB)
    return im


# Where each shaft's warning pointer puts its tip, beside the shaft, one
# column clear of its ink: the left and right shafts' right-hand sides and
# the front shaft's.  The pointer piece is drawn for the left shaft at its
# deep row; the engine shifts it by these columns and by the target level.
POINTER_TIP = {"left": 68, "right": 114, "front": 88}
POINTER_ROW = SIDE_HIGH + 2  # the tip faces the float's middle


def pointer(full):
    """The warning pointer: an amber arrowhead aimed at the shaft, at the row
    its float is about to reach, with a darker back so it reads on sand and
    on water; it pulses between two sizes.  (The v13 pointer's bar is gone:
    with three shafts the gaps are too narrow for it.)"""
    im = blank()
    if full:
        rows = {-3: [3], -2: [2, 3], -1: [1, 2, 3], 0: [0, 1, 2, 3],
                1: [1, 2, 3], 2: [2, 3], 3: [3]}
        back = 3
    else:
        rows = {-2: [2], -1: [1, 2], 0: [0, 1, 2], 1: [1, 2], 2: [2]}
        back = 2
    for dy, xs in rows.items():
        for dx in xs:
            c = AMBER if dx == back else AMBER_LIGHT
            put(im, POINTER_TIP["left"] + dx, POINTER_ROW + dy, c)
    return im


def main():
    src, out = sys.argv[1], sys.argv[2]
    os.makedirs(f"{out}/layers", exist_ok=True)
    os.makedirs(f"{out}/pieces", exist_ok=True)
    rear = load(f"{src}/north_rear.png")
    hulls = load(f"{src}/north_hulls.png")
    cabins = load(f"{src}/north_cabins.png")
    wp_n = load(f"{src}/north_working.png")
    wp_s = load(f"{src}/south_working.png")

    mast = strip_working_parts(wp_n)
    front_pulley(mast)
    front = front_shaft()
    layers = {
        "rear": rear,
        "hulls": hulls,
        "cabins": cabins,
        "front_shaft": front,
        "mast": mast,
    }
    for name, im in layers.items():
        im.save(f"{out}/layers/{name}.png")

    mast_mask = {}
    for y in range(0, 50):
        for x in range(70, 92):
            if mast.getpixel((x, y))[3]:
                mast_mask[(x, y)] = True

    pieces = []

    def piece(key, family, im):
        path = f"{out}/pieces/{key}.png"
        im.save(path)
        pieces.append({"key": key, "family": family, "file": f"pieces/{key}.png"})

    # Floats from one pixel below the dry row to one above the deep row (a
    # switch overshoots a pixel), numbered from the lowest: key n is n - 1
    # pixels above dry.
    for level in range(-1, SIDE_TRAVEL + 2):
        piece(f"station3_float_side_{level + 1}", "float_side", side_float(level))
    for level in range(-1, FRONT_TRAVEL + 2):
        piece(f"station3_float_front_{level + 1}", "float_front", front_float(level))
    north_flag, south_flag = flag_cluster(wp_n), flag_cluster(wp_s)
    for angle in FLAG_ANGLES:
        piece(f"station3_flag_{angle}", "flag", flag_piece(angle, north_flag, south_flag, mast_mask))
    for step in range(8):
        piece(f"station3_wheel_{step}", "wheel", wheel_piece(step))
    for phase, full in enumerate((True, False)):
        piece(f"station3_pointer_{phase}", "pointer", pointer(full))
    json.dump(pieces, open(f"{out}/pieces.json", "w"), indent=1)


if __name__ == "__main__":
    main()
