"""Build the working-parts layer of the lock switch frames.

The v13 gate has two resting drawings, north dry and south dry, that differ
only in the working parts: the two shaft floats and their water, the
semaphore flag on the mast and the pointer spoke on the handwheel.  This
script is the procedural half of the v20 switch: it takes those two layers
and generates the in-between frames, so the gate is seen to turn when the
tide changes.  The build script (gate_switch.lua) puts the layers into an
editable Aseprite document next to the untouched v13 layers.

Frames are script-generated from the v13 clusters, not hand drawn.
Usage: python3 gate_switch.py <wp_north.png> <wp_south.png> <out_dir>
"""

import math
import sys

from PIL import Image

A = (85, 53, 58, 255)  # flag outline
B = (231, 217, 181, 255)  # flag stripe
C = (38, 56, 68, 255)  # ink
E = (158, 180, 181, 255)  # hub
H = (125, 99, 78, 255)  # rope
I = (157, 87, 56, 255)  # float body
J = (242, 195, 101, 255)  # float top
K = (77, 124, 130, 255)  # shaft water
L = (233, 153, 78, 255)  # spokes
O = (190, 183, 155, 255)  # pointer spoke
CLEAR = (0, 0, 0, 0)

# Geometry of the v13 drawing (pixel coordinates in the 160x144 frame).
SHAFTS = {"north": 51, "south": 97}  # float left edge; rope at +6
FLOAT_LOW, FLOAT_HIGH = 83, 63  # float top row, dry and wet
WATER_BOTTOM = 93
ROPE_TOP = 43
PIVOT = (80.5, 21.5)  # flag mast knob, pixel centre coordinates
FLAG_BOX = (56, 12, 106, 24)
WHEEL_CENTRE = (81.0, 110.0)  # corner between the four spoke pixels
WHEEL_BOX = (66, 98, 96, 123)
WHEEL_R = (10.5, 8.5)  # spoke reach, east and south
POINTER_R = (12.5, 9.5)  # pointer reach through the rim
SWITCH_FRAMES = 12


def ease(p):
    return p * p * (3 - 2 * p)


def float_travel(frame):
    """Float travel in twentieths of the shaft for each frame: still while
    the wheel starts, fast mid-stroke, a one-pixel overshoot, then rest."""
    table = [0, 0, 1, 2, 5, 8, 12, 15, 18, 20, 21, 20]
    return table[frame]


def flag_angle(frame):
    """0 is the old side, 1 the new; the arm lifts, passes upright, and
    lands a frame before the floats start their fast stroke."""
    return [0.25, 0.5, 0.75, 1, 1, 1, 1, 1, 1, 1, 1, 1][frame]


def wheel_turn(frame):
    """Eighths of a turn from the old pointer toward the new one: a turn
    and a quarter, slowing into the stop."""
    return [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 10, 10][frame]


def clear_box(im, box, colours):
    x0, y0, x1, y1 = box
    for y in range(y0, y1):
        for x in range(x0, x1):
            if im.getpixel((x, y)) in colours:
                im.putpixel((x, y), CLEAR)


def draw_shaft(im, side, top):
    x = SHAFTS[side]
    for y in range(ROPE_TOP, top):
        im.putpixel((x + 6, y), H)
    for dx in range(13):
        edge = dx in (0, 12)
        im.putpixel((x + dx, top), I if edge else J)
        im.putpixel((x + dx, top + 1), I if edge else J)
        im.putpixel((x + dx, top + 2), I)
        im.putpixel((x + dx, top + 3), I)
    for y in range(top + 4, WATER_BOTTOM + 1):
        for dx in range(4, 10):
            im.putpixel((x + dx, y), K)


def clear_shaft(im, side):
    x = SHAFTS[side]
    for y in range(ROPE_TOP, WATER_BOTTOM + 1):
        for dx in range(13):
            im.putpixel((x + dx, y), CLEAR)


def flag_cluster(src):
    """The flag arm alone (outline and stripe), as a sparse dict."""
    x0, y0, x1, y1 = FLAG_BOX
    out = {}
    for y in range(y0, y1):
        for x in range(x0, x1):
            p = src.getpixel((x, y))
            if p in (A, B):
                out[(x, y)] = p
    return out


def rotate_cluster(cluster, degrees, pivot, scale=8):
    """RotSprite-style: sample each output pixel centre at 1/scale steps and
    keep the colour most of its samples land on, so edges stay clean."""
    a = math.radians(degrees)
    ca, sa = math.cos(a), math.sin(a)
    px, py = pivot
    out = {}
    for y in range(0, 40):
        for x in range(40, 122):
            votes = {}
            for sy in range(scale):
                for sx in range(scale):
                    fx = x + (sx + 0.5) / scale - px
                    fy = y + (sy + 0.5) / scale - py
                    # inverse rotation
                    ux = fx * ca + fy * sa + px
                    uy = -fx * sa + fy * ca + py
                    p = cluster.get((math.floor(ux - 0.0), math.floor(uy)))
                    if p:
                        votes[p] = votes.get(p, 0) + 1
            if votes:
                p, n = max(votes.items(), key=lambda kv: kv[1])
                total = sum(votes.values())
                if total * 2 >= scale * scale:
                    out[(x, y)] = p if n * 3 >= total else A
    return out


def mirror_cluster(cluster):
    return {(160 - 1 - x + 1, y): p for (x, y), p in cluster.items()}


def blank_wheel(src):
    """The handwheel with its spokes and pointer painted out, each hidden
    pixel taking the ring colour found at the same radius on a diagonal."""
    im = src.copy()
    cx, cy = WHEEL_CENTRE
    x0, y0, x1, y1 = WHEEL_BOX

    def rho(x, y):
        return math.hypot((x + 0.5 - cx) / 13.0, (y + 0.5 - cy) / 11.0)

    samples = []
    for y in range(y0, y1):
        for x in range(x0, x1):
            p = src.getpixel((x, y))
            if p[3] and p not in (L, O, E):
                dx, dy = abs(x + 0.5 - cx), abs(y + 0.5 - cy)
                if dx > 2 and dy > 2:
                    samples.append((rho(x, y), p))
    for y in range(y0, y1):
        for x in range(x0, x1):
            if src.getpixel((x, y)) in (L, O, E):
                r = rho(x, y)
                im.putpixel((x, y), min(samples, key=lambda s: abs(s[0] - r))[1])
    return im


def spoke(im, angle, reach, colour, start=0.0):
    """A two-pixel spoke from the hub along `angle` (degrees, 0 east, 90
    south), laid on the wheel's slightly flattened ellipse."""
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


def hub(im, src):
    for y in range(106, 113):
        for x in range(76, 86):
            if src.getpixel((x, y)) == E:
                im.putpixel((x, y), E)
    # The hub is a small diamond: fill its centre four.
    for x, y in [(80, 109), (81, 109), (80, 110), (81, 110)]:
        im.putpixel((x, y), E)


def frame(north, south, to_south, i, flag_n, flag_s, blank):
    src_from, src_to = (north, south) if to_south else (south, north)
    im = src_from.copy()
    # Floats: the side that was dry fills, the other drains.
    travel = float_travel(i)
    rising, falling = ("north", "south") if to_south else ("south", "north")
    clear_shaft(im, "north")
    clear_shaft(im, "south")
    draw_shaft(im, rising, FLOAT_LOW - travel)
    draw_shaft(im, falling, FLOAT_HIGH + travel)
    # Flag: the arm swings over the top of the mast to the new side.
    t = flag_angle(i)
    clear_box(im, FLAG_BOX, (A, B))
    # 71 degrees takes the north arm upright.
    arm = rotate_cluster(flag_n, 142 * t, PIVOT) if to_south else mirror_cluster(
        rotate_cluster(flag_n, 142 * t, PIVOT)
    )
    if t == 1:
        arm = flag_s if to_south else flag_n
    for (x, y), p in arm.items():
        if im.getpixel((x, y))[3] == 0:
            im.putpixel((x, y), p)
    # Wheel: the pointer turns clockwise to the south to drain the south
    # shaft, anticlockwise back to the east for the north.
    wx0, wy0, wx1, wy1 = WHEEL_BOX
    for y in range(wy0, wy1):
        for x in range(wx0, wx1):
            im.putpixel((x, y), blank.getpixel((x, y)))
    if i >= 10:
        # At rest the wheel is the v13 drawing of the new side, exactly.
        for y in range(wy0, wy1):
            for x in range(wx0, wx1):
                im.putpixel((x, y), src_to.getpixel((x, y)))
        return im
    turn = wheel_turn(i) * 45
    pointer = (0 + turn) if to_south else (90 - turn)
    for k in range(4):
        spoke(im, pointer + 90 * k, WHEEL_R, L)
    spoke(im, pointer, POINTER_R, O, start=2.0)
    hub(im, src_to)
    return im


def main():
    north = Image.open(sys.argv[1]).convert("RGBA")
    south = Image.open(sys.argv[2]).convert("RGBA")
    out = sys.argv[3]
    flag_n, flag_s = flag_cluster(north), flag_cluster(south)
    blank = blank_wheel(north)
    for to_south in (True, False):
        name = "south" if to_south else "north"
        for i in range(SWITCH_FRAMES):
            frame(north, south, to_south, i, flag_n, flag_s, blank).save(
                f"{out}/wp_to_{name}_dry_{i}.png"
            )


if __name__ == "__main__":
    main()
