"""Check that the v22 pack changed only its own rectangles.

Usage: python3 verify_atlas.py <old atlas.png> <old manifest.json> <root>
Every earlier key keeps its rectangle and anchor, and every pixel outside
the new station3 rectangles is unchanged (compared as RGBA, with fully
transparent pixels treated as equal).
"""

import json
import sys

from PIL import Image, ImageChops


def main():
    old_png, old_json, root = sys.argv[1:4]
    old = json.load(open(old_json))["sprites"]
    new = json.load(open(f"{root}/art/exports/game-assets.json"))["sprites"]
    for key, b in old.items():
        assert new.get(key) == b, f"{key} moved or changed"
    added = sorted(set(new) - set(old))
    assert all(k.startswith("station3") for k in added), added
    a = Image.open(old_png).convert("RGBA")
    b = Image.open(f"{root}/art/exports/game-assets.png").convert("RGBA")
    assert a.size == b.size
    mask = Image.new("L", a.size, 255)
    for key in added:
        r = new[key]
        mask.paste(0, (r["x"], r["y"], r["x"] + r["w"], r["y"] + r["h"]))
    # Transparent pixels compare equal whatever their colour channels.
    a_clear = Image.new("RGBA", a.size)
    a2 = Image.composite(a, a_clear, a.getchannel("A").point(lambda v: 255 if v else 0))
    b2 = Image.composite(b, a_clear, b.getchannel("A").point(lambda v: 255 if v else 0))
    diff = ImageChops.difference(a2, b2)
    outside = Image.composite(diff, Image.new("RGBA", a.size), mask)
    assert outside.getbbox() is None, f"pixels changed outside the new keys: {outside.getbbox()}"
    print(f"{len(added)} keys added; no earlier rectangle or pixel changed")


if __name__ == "__main__":
    main()
