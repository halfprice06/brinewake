"""Compose three-arm station poses from the v22 pieces, as the engine does.

Usage: python3 compose.py <piece dir> <out.png> [scale]
Writes a strip of the five rest poses (W, E, S dry, neutral, flood), one
warning frame and the semaphore's in-between angles, over a flat water
colour, at `scale`.  It is a review aid: the engine composes the same
pieces in crates/bw_desktop/src/station3.rs.
"""

import sys

from PIL import Image

BASE_LAYERS = ["rear", "hulls", "cabins", "front_shaft", "mast"]
RIGHT_DX = 46  # the right shaft's float is the left one's, 46 columns over
TIPS = {"left": 68, "right": 114, "front": 88}


def float_top(slot, twentieths):
    travel = 16 if slot == "front" else 20
    return 83 - round(twentieths * travel / 20)


def pose_image(d, levels, flag, wheel, pointers=()):
    """levels: (left, right, front) in twentieths of full; pointers: list of
    (slot, target twentieths, phase)."""
    im = Image.new("RGBA", (160, 144), (0, 0, 0, 0))
    for name in BASE_LAYERS:
        im.alpha_composite(Image.open(f"{d}/layers/{name}.png").convert("RGBA"))
    left, right, front = levels
    # Float keys are numbered from one pixel below the dry row.
    side = lambda t: round(t * 20 / 20) + 1
    fr = lambda t: round(t * 16 / 20) + 1
    im.alpha_composite(Image.open(f"{d}/pieces/station3_float_side_{side(left)}.png"))
    im.alpha_composite(
        Image.open(f"{d}/pieces/station3_float_side_{side(right)}.png"), (RIGHT_DX, 0)
    )
    im.alpha_composite(Image.open(f"{d}/pieces/station3_float_front_{fr(front)}.png"))
    im.alpha_composite(Image.open(f"{d}/pieces/station3_flag_{flag}.png"))
    im.alpha_composite(Image.open(f"{d}/pieces/station3_wheel_{wheel}.png"))
    for slot, target, phase in pointers:
        dx, dy = TIPS[slot] - 68, float_top(slot, target) + 2 - 65
        piece = Image.open(f"{d}/pieces/station3_pointer_{phase}.png")
        shifted = Image.new("RGBA", (160, 144), (0, 0, 0, 0))
        shifted.paste(piece, (dx, dy))
        im.alpha_composite(shifted)
    return im


POSES = {
    # levels left(W) right(E) front(S); flag angle; wheel step (eighths
    # clockwise from east)
    "w_dry": ((0, 20, 20), 158, 5),
    "e_dry": ((20, 0, 20), 22, 7),
    "s_dry": ((20, 20, 0), 270, 2),
    "neutral": ((10, 10, 10), 90, 6),
    "flood": ((20, 20, 20), 90, 6),
}


def main():
    d, out = sys.argv[1], sys.argv[2]
    k = int(sys.argv[3]) if len(sys.argv) > 3 else 1
    ims = [pose_image(d, *POSES[n]) for n in POSES]
    # A warning from W dry to S dry: W will fill, S will drain.
    ims.append(pose_image(d, (0, 20, 20), 158, 6, [("left", 20, 0), ("front", 0, 0)]))
    for a in (180, 225, 135, 45, 0, 315):
        ims.append(pose_image(d, (10, 10, 10), a, 0))
    sheet = Image.new("RGBA", (160 * len(ims), 144), (46, 88, 98, 255))
    for i, im in enumerate(ims):
        sheet.alpha_composite(im, (160 * i, 0))
    sheet.resize((sheet.width * k, sheet.height * k), Image.NEAREST).save(out)


if __name__ == "__main__":
    main()
