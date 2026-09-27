"""Pixel preservation checks and same-scale proofs for the static base repair."""
from pathlib import Path
import json
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "output/art-v6"
KEYS = ["union_hq", "assembly_hq", "union_works", "assembly_works",
        "dropoff", "condenser", "tower", "gate", "well"]


def load(folder):
    return (Image.open(folder / "game-assets.png").convert("RGBA"),
            json.loads((folder / "game-assets.json").read_text()))


def crop(image, b):
    return image.crop((b["x"], b["y"], b["x"] + b["w"], b["y"] + b["h"]))


old, before = load(ROOT / "art/archive/art-v5")
current, after = load(ROOT / "art/exports")
assert before == after, "atlas rectangles or anchor metadata changed"
assert current.size == (2048, 1536)
assert set(current.getchannel("A").get_flattened_data()) == {0, 255}
assert Image.open(OUT / "reopened.png").convert("RGBA").tobytes() == current.tobytes()
changed = []
for key, b in after["sprites"].items():
    previous, new = crop(old, b), crop(current, b)
    if previous.tobytes() != new.tobytes():
        changed.append(key)
    if key in KEYS:
        # The existing upper architecture is not repainted by a base repair.
        for y in range(96):
            for x in range(160):
                p = previous.getpixel((x, y))
                if p[3]:
                    assert p == new.getpixel((x, y)), (key, x, y)
assert set(changed) == set(KEYS), changed
report = dict(changed_entries=sorted(changed), unchanged_entries=411 - len(KEYS),
              atlas_dimensions=current.size, manifest_and_anchors_unchanged=True,
              opaque_colors=len({p for p in current.get_flattened_data() if p[3]}),
              binary_alpha=True, upper_architecture_preserved=True,
              reopened_source_matches_export=True)
(OUT / "validation-art.json").write_text(json.dumps(report, indent=2) + "\n")

# Each row compares a single building at the same 160x144 native canvas,
# displayed at exactly 2x. No inferred ground or mock scene is added.
sheet = Image.new("RGBA", (1280, 5 * 320), (157, 143, 118, 255))
draw = ImageDraw.Draw(sheet)
for i, key in enumerate(KEYS):
    x, y = i % 2 * 640, i // 2 * 320
    draw.text((x + 8, y + 5), key + " / BEFORE", fill=(23, 35, 46))
    draw.text((x + 328, y + 5), key + " / GROUNDED", fill=(23, 35, 46))
    b = after["sprites"][key]
    for dx, image in [(0, old), (320, current)]:
        tile = crop(image, b).resize((320, 288), Image.Resampling.NEAREST)
        sheet.alpha_composite(tile, (x + dx, y + 24))
sheet.save(OUT / "buildings-before-after.png")

for faction in ("union", "assembly"):
    a = Image.open(OUT / f"{faction}-before.png").convert("RGB")
    b = Image.open(OUT / f"{faction}-after.png").convert("RGB")
    comparison = Image.new("RGB", (1280, 390), (23, 35, 46))
    draw = ImageDraw.Draw(comparison)
    comparison.paste(a, (0, 30)); comparison.paste(b, (640, 30))
    draw.text((10, 9), "BEFORE / same camera and native scale", fill=(231, 217, 181))
    draw.text((650, 9), "GROUNDED / same camera and native scale", fill=(231, 217, 181))
    comparison.save(OUT / f"{faction}-before-after.png")
    b.resize((1920, 1080), Image.Resampling.NEAREST).save(OUT / f"{faction}-1080p.png")
print(json.dumps(report, indent=2))
