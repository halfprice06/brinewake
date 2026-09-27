"""Pack the three-arm station's exported frames into the game atlas.

Usage: python3 pack.py <root> <out dir>
Reads <out dir>/pieces.json and <out dir>/export/*.png (written by
station3.lua from the saved art/source/v22/station3.aseprite) and adds the
keys to art/exports/game-assets.{png,json}.

Every piece of a family is cropped to the family's common box, so its
anchor is the same (the station's anchor, (80, 128) of the 160x144 frame,
expressed in the crop).  New rectangles go in free 16-pixel cells, as the
v21 packer places them; no earlier rectangle moves, and a rebuild reuses a
key's rectangle (its size and anchor must not change).  The script checks
every packed key against its export afterwards.
"""

import json
import sys

from PIL import Image

ANCHOR = (80, 128)
G = 16


def bbox_union(images):
    boxes = [im.getbbox() for im in images if im.getbbox()]
    return (
        min(b[0] for b in boxes),
        min(b[1] for b in boxes),
        max(b[2] for b in boxes),
        max(b[3] for b in boxes),
    )


def main():
    root, out = sys.argv[1], sys.argv[2]
    pieces = json.load(open(f"{out}/pieces.json"))
    families = {"base": [{"key": "station3"}]}
    for p in pieces:
        families.setdefault(p["family"], []).append(p)
    manifest_path = f"{root}/art/exports/game-assets.json"
    atlas_path = f"{root}/art/exports/game-assets.png"
    manifest = json.load(open(manifest_path))
    atlas = Image.open(atlas_path).convert("RGBA")
    width, height = manifest["width"], manifest["height"]
    gw, gh = width // G, height // G
    used = bytearray(gw * gh)

    def mark(b):
        for gy in range(b["y"] // G, (b["y"] + b["h"] - 1) // G + 1):
            for gx in range(b["x"] // G, (b["x"] + b["w"] - 1) // G + 1):
                used[gy * gw + gx] = 1

    for b in manifest["sprites"].values():
        mark(b)

    def alloc(key, w, h, ax, ay):
        old = manifest["sprites"].get(key)
        if old:
            assert (old["w"], old["h"]) == (w, h), f"{key} size changed"
            assert (old["anchor_x"], old["anchor_y"]) == (ax, ay), f"{key} anchor changed"
            return old
        cw, ch = -(-w // G), -(-h // G)
        for gy in range(gh - ch + 1):
            for gx in range(gw - cw + 1):
                if all(
                    not used[(gy + y) * gw + gx + x] for y in range(ch) for x in range(cw)
                ):
                    b = {"anchor_x": ax, "anchor_y": ay, "h": h, "w": w, "x": gx * G, "y": gy * G}
                    manifest["sprites"][key] = b
                    mark(b)
                    return b
        raise SystemExit(f"atlas full for {key}")

    crops = {}
    for family, members in families.items():
        images = [Image.open(f"{out}/export/{m['key']}.png").convert("RGBA") for m in members]
        x0, y0, x1, y1 = bbox_union(images)
        for m, im in zip(members, images):
            crops[m["key"]] = (im.crop((x0, y0, x1, y1)), ANCHOR[0] - x0, ANCHOR[1] - y0)

    for key, (im, ax, ay) in crops.items():
        b = alloc(key, im.width, im.height, ax, ay)
        # Clear the rectangle first so a rebuild leaves no stale pixels.
        atlas.paste(Image.new("RGBA", (b["w"], b["h"])), (b["x"], b["y"]))
        atlas.paste(im, (b["x"], b["y"]))

    assert len(manifest["sprites"]) <= 4096
    atlas.save(atlas_path, optimize=True)
    with open(manifest_path, "w") as f:
        f.write(json.dumps(manifest, sort_keys=True))

    check = Image.open(atlas_path).convert("RGBA")
    for key, (im, _, _) in crops.items():
        b = manifest["sprites"][key]
        got = check.crop((b["x"], b["y"], b["x"] + b["w"], b["y"] + b["h"]))
        assert got.tobytes() == im.tobytes(), f"{key} differs in the atlas"
    print(f"packed {len(crops)} keys; atlas holds {len(manifest['sprites'])}")


if __name__ == "__main__":
    main()
