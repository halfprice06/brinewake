"""Check that the v23 build changed only its own rectangles.

Usage: python3 verify_atlas.py <old atlas.png> <old manifest.json> <root>

Every earlier key keeps its rectangle and anchor, every added key is one
of v23's families, and every pixel outside the added rectangles is
unchanged (compared as RGBA, fully transparent pixels treated as equal).
Each added key is also compared with its entry in tools/art_v23/
sources.json.
"""

import json
import sys

from PIL import Image, ImageChops

FAMILIES = (
    "doctrine_compact_",
    "doctrine_raker_",
    "trace_glaze_",
    "fx_glint_flash_",
    "fx_glint_spot_",
    "fx_glint_mote_",
)


def opaque(im):
    clear = Image.new("RGBA", im.size)
    return Image.composite(im, clear, im.getchannel("A").point(lambda v: 255 if v else 0))


def main():
    old_png, old_json, root = sys.argv[1:4]
    old = json.load(open(old_json))["sprites"]
    new = json.load(open(f"{root}/art/exports/game-assets.json"))["sprites"]
    for key, b in old.items():
        assert new.get(key) == b, f"{key} moved or changed"
    added = sorted(set(new) - set(old))
    assert all(k.startswith(FAMILIES) for k in added), [k for k in added if not k.startswith(FAMILIES)]
    sources = {e["key"]: e for e in json.load(open(f"{root}/tools/art_v23/sources.json"))}
    assert set(added) <= set(sources), sorted(set(added) - set(sources))
    for key in added:
        b, s = new[key], sources[key]
        assert (b["w"], b["h"], b["anchor_x"], b["anchor_y"]) == (
            s["w"],
            s["h"],
            s["anchor_x"],
            s["anchor_y"],
        ), key
    a = opaque(Image.open(old_png).convert("RGBA"))
    b = opaque(Image.open(f"{root}/art/exports/game-assets.png").convert("RGBA"))
    assert a.size == b.size
    mask = Image.new("L", a.size, 255)
    for key in added:
        r = new[key]
        mask.paste(0, (r["x"], r["y"], r["x"] + r["w"], r["y"] + r["h"]))
    outside = Image.composite(ImageChops.difference(a, b), Image.new("RGBA", a.size), mask)
    assert outside.getbbox() is None, f"pixels changed outside the new keys: {outside.getbbox()}"
    print(f"{len(added)} keys added; no earlier rectangle or pixel changed")


if __name__ == "__main__":
    main()
