"""Inspect exports and compose proofs. Never author or modify source pixels."""
from pathlib import Path
import json
from PIL import Image, ImageDraw, ImageOps

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "output/art-v5"


def load(folder):
    return (Image.open(folder / "game-assets.png").convert("RGBA"),
            json.loads((folder / "game-assets.json").read_text())["sprites"])


def crop(image, entry):
    x, y, w, h = (entry[k] for k in ("x", "y", "w", "h"))
    return image.crop((x, y, x + w, y + h))


atlas, entries = load(ROOT / "art/exports")
old, old_entries = load(ROOT / "art/archive/art-v4")
changed = []
for key, entry in entries.items():
    x, y, w, h = (entry[k] for k in ("x", "y", "w", "h"))
    assert 0 <= x < atlas.width and 0 <= y < atlas.height
    assert x + w <= atlas.width and y + h <= atlas.height
    assert crop(atlas, entry).getbbox(), key
    if key not in old_entries or crop(atlas, entry).tobytes() != crop(old, old_entries[key]).tobytes():
        changed.append(key)
assert len(entries) == 411
assert all(entries.get(k) == v for k, v in old_entries.items())
assert set(atlas.getchannel("A").get_flattened_data()) == {0, 255}
assert Image.open(OUT / "reopened.png").convert("RGBA").tobytes() == atlas.tobytes()
protected = [k for k in old_entries if not k.startswith("terrain_")
             and k not in ("salvage", "salt_rock", "reed_clump")]
assert all(crop(atlas, entries[k]).tobytes() == crop(old, old_entries[k]).tobytes()
           for k in protected)
colors = {p for p in atlas.get_flattened_data() if p[3]}
report = dict(entries=len(entries), dimensions=atlas.size, opaque_colors=len(colors),
              binary_alpha=True, changed_or_added_keys=changed,
              all_old_rectangles_and_anchors_preserved=True,
              unchanged_protected_entries=len(protected), reopened_source_equals_png=True)
(OUT / "validation-art.json").write_text(json.dumps(report, indent=2) + "\n")

proof = Image.new("RGBA", (960, 660), (157, 143, 118, 255))
draw = ImageDraw.Draw(proof)
for i, key in enumerate(("salvage", "salvage_turbine", "salvage_barge")):
    for y, source, name, table, label in (
        (24, old, "salvage", old_entries, "V4 / repeated salvage"),
        (348, atlas, key, entries, "V5 / " + key),
    ):
        draw.text((i * 320 + 8, y - 16), label, fill=(23, 35, 46))
        image = crop(source, table[name]).resize((320, 288), Image.Resampling.NEAREST)
        proof.alpha_composite(image, (i * 320, y))
proof.save(OUT / "wrecks-before-after.png")

# This reconstructs the existing renderer's three 800 ms water phases.
# It is an atlas study, not a recording from the live game window.
frames = []
for phase in range(3):
    image = Image.new("RGBA", (320, 144), (38, 61, 68, 255))
    for diagonal in range(35):
        for x in range(22):
            y = diagonal - x
            if not 0 <= y < 14:
                continue
            variation = (x * 13 + y * 17 + x * y * 3) % 11
            p = 0 if variation < 8 else 1 + (phase + x * 5 + y * 3) % 3
            entry = entries[f"terrain_water_{p}"]
            image.alpha_composite(crop(atlas, entry),
                                  ((x - y) * 16 + 100 - entry["anchor_x"],
                                   (x + y) * 8 - 90 - entry["anchor_y"]))
    draw = ImageDraw.Draw(image)
    draw.rectangle((0, 0, 320, 15), fill=(23, 35, 46))
    draw.text((5, 3), "ATLAS WATER STUDY / 800 ms per phase", fill=(196, 212, 196))
    frames.append(image.resize((960, 432), Image.Resampling.NEAREST))
frames[0].save(OUT / "water-study.gif", save_all=True, append_images=frames[1:],
               duration=800, loop=0, disposal=2)
strip = Image.new("RGBA", (960, 144))
for i, image in enumerate(frames):
    strip.alpha_composite(image.resize((320, 144), Image.Resampling.NEAREST), (i * 320, 0))
strip.save(OUT / "water-phases.png")

for faction in ("union", "assembly"):
    images = [Image.open(OUT / f"{faction}-{state}.png").convert("RGB")
              for state in ("before", "after")]
    comparison = Image.new("RGB", (1280, 390), (23, 35, 46))
    draw = ImageDraw.Draw(comparison)
    for i, image in enumerate(images):
        comparison.paste(image, (i * 640, 30))
        draw.text((i * 640 + 12, 10), f"{'V4 / BEFORE' if i == 0 else 'V5 / AFTER'} - same start, camera and native scale", fill=(231, 217, 181))
    comparison.save(OUT / f"{faction}-before-after.png")
    images[1].resize((1920, 1080), Image.Resampling.NEAREST).save(OUT / f"{faction}-1080p.png")
    ImageOps.grayscale(comparison).save(OUT / f"{faction}-grayscale.png")
print(json.dumps(report, indent=2))
