"""Validate native exports and compose clearly labeled art/engine studies."""
from pathlib import Path
import json
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "output/art-v7"
ACTIVE = ["union_hq", "assembly_hq", "union_works", "assembly_works", "condenser"]
SITES = ["union_works", "assembly_works", "dropoff", "condenser", "tower"]


def load(folder):
    return Image.open(folder / "game-assets.png").convert("RGBA"), json.loads((folder / "game-assets.json").read_text())


def crop(image, b):
    return image.crop((b["x"], b["y"], b["x"] + b["w"], b["y"] + b["h"]))


atlas, manifest = load(ROOT / "art/exports")
previous, before = load(ROOT / "art/archive/art-v6")
entries = manifest["sprites"]
new_keys = {f"{name}_active_{p}" for name in ACTIVE for p in range(4)}
new_keys |= {f"{name}_build_{p}" for name in SITES for p in range(3)}
assert set(entries) - set(before["sprites"]) == new_keys
assert len(entries) == 446 and atlas.size == (2048, 2048)
for key, b in before["sprites"].items():
    assert entries[key] == b, key
    assert crop(atlas, b).tobytes() == crop(previous, b).tobytes(), key
for key, b in entries.items():
    assert b["x"] >= 0 and b["y"] >= 0
    assert b["x"] + b["w"] <= atlas.width and b["y"] + b["h"] <= atlas.height
    assert crop(atlas, b).getbbox(), key
for key in new_keys:
    b = entries[key]
    assert (b["w"], b["h"], b["anchor_x"], b["anchor_y"]) == (160, 144, 80, 128)
assert set(atlas.getchannel("A").get_flattened_data()) == {0, 255}
assert Image.open(OUT / "reopened.png").convert("RGBA").tobytes() == atlas.tobytes()
report = dict(atlas_dimensions=atlas.size, entries=len(entries), new_state_keys=len(new_keys),
              original_entries_preserved=411, binary_alpha=True,
              used_opaque_colors=len({p for p in atlas.get_flattened_data() if p[3]}),
              reopened_source_equals_export=True)
(OUT / "validation-art.json").write_text(json.dumps(report, indent=2) + "\n")

activity_frames = []
for phase in range(4):
    image = Image.new("RGBA", (480, 364), (157, 143, 118, 255))
    draw = ImageDraw.Draw(image)
    draw.text((8, 4), "BUILDING ACTIVITY / native art study / 200 ms keys", fill=(23, 35, 46))
    for i, name in enumerate(ACTIVE):
        x, y = i % 3 * 160, i // 3 * 170 + 26
        image.alpha_composite(crop(atlas, entries[f"{name}_active_{phase}"]), (x, y))
        draw.text((x + 6, y + 147), name.replace("_", " "), fill=(23, 35, 46))
    activity_frames.append(image.resize((960, 728), Image.Resampling.NEAREST))
activity_frames[0].save(OUT / "building-activity.gif", save_all=True,
                        append_images=activity_frames[1:], duration=200, loop=0, disposal=2)
for i, image in enumerate(activity_frames):
    image.save(OUT / f"activity-phase-{i}.png")

sheet = Image.new("RGBA", (1280, 1600), (157, 143, 118, 255))
draw = ImageDraw.Draw(sheet)
for row, name in enumerate(SITES):
    for p, label in enumerate(["FOUNDATION", "FRAME", "FIT-OUT", "COMPLETE"]):
        key = f"{name}_build_{p}" if p < 3 else name
        image = crop(atlas, entries[key]).resize((320, 288), Image.Resampling.NEAREST)
        sheet.alpha_composite(image, (p * 320, row * 320 + 22))
        draw.text((p * 320 + 8, row * 320 + 7), name + " / " + label, fill=(23, 35, 46))
sheet.save(OUT / "construction-stages.png")

# These use the same game renderer as the application, with explicit state
# fixtures. The source captures already carry an ART STATE STUDY label.
for name in ACTIVE:
    frames = [Image.open(OUT / "engine" / f"{name}-activity-{p}.png").convert("RGB")
              .resize((1280, 720), Image.Resampling.NEAREST) for p in range(4)]
    frames[0].save(OUT / f"{name}-engine-study.gif", save_all=True,
                   append_images=frames[1:], duration=200, loop=0, disposal=2)
for faction in ["union", "assembly"]:
    images = [Image.open(OUT / f"{faction}-{state}.png").convert("RGB") for state in ["before", "after"]]
    proof = Image.new("RGB", (1280, 390), (23, 35, 46))
    draw = ImageDraw.Draw(proof)
    for i, image in enumerate(images):
        proof.paste(image, (i * 640, 30))
        draw.text((i * 640 + 8, 9), "V6 / BEFORE" if i == 0 else "V7 / ACTIVE HEADQUARTERS", fill=(231, 217, 181))
    proof.save(OUT / f"{faction}-before-after.png")
print(json.dumps(report, indent=2))
