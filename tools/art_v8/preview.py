"""Assemble inspection sheets from actual Aseprite exports; no art painting."""
from pathlib import Path
import json
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "output/art-v8"
atlas = Image.open(ROOT / "art/exports/game-assets-v8.png").convert("RGBA")
entries = json.loads((ROOT / "art/exports/game-assets-v8.json").read_text())["sprites"]
BG = (157, 143, 118, 255)
INK = (23, 35, 46, 255)


def crop(key):
    b = entries[key]
    return atlas.crop((b["x"], b["y"], b["x"] + b["w"], b["y"] + b["h"]))


def board(name, rows, cell=(160, 164), scale=2):
    width = max(len(r) for r in rows) * cell[0]
    sheet = Image.new("RGBA", (width, len(rows) * cell[1]), BG)
    draw = ImageDraw.Draw(sheet)
    for row, keys in enumerate(rows):
        for col, key in enumerate(keys):
            im = crop(key)
            x, y = col * cell[0], row * cell[1]
            sheet.alpha_composite(im, (x + (cell[0] - im.width) // 2, y + 18))
            label = key.replace("salvage_", "").replace("terrain_", "").replace("bulwark_", "B ").replace("loom_", "L ").replace("_deploy_", " D")
            max_chars = max(8, (cell[0] - 6) // 6)
            draw.text((x + 3, y + 3), label[:max_chars], fill=INK)
    sheet.save(OUT / f"{name}-native.png")
    sheet.resize((sheet.width * scale, sheet.height * scale), Image.Resampling.NEAREST).save(OUT / f"{name}.png")


for role in ["bulwark", "loom"]:
    rows = [[f"{role}_{d}" for d in range(8)]]
    rows += [[f"{role}_{d}_deploy_{s}" for d in range(8)] for s in range(3)]
    board(f"{role}-deployment", rows, (92, 86))

board("gate-states", [
    [f"gate_{state}"] + [f"gate_{state}_warning_{p}" for p in range(4)]
    for state in ["north_dry", "south_dry"]
])
board("salvage-stages", [[f"{r}_stage_{p}" for p in range(4)] for r in ["salvage", "salvage_turbine", "salvage_barge"]])
board("coastal-landmarks", [[f"landmark_{n}" for n in ["tide_gauge", "ferry_stairs", "hull_ribs"]]])
board("combat-reactions", [[f"{r}_7"] + [f"{r}_7_fire_{p}" for p in range(3)] for r in ["riveter", "bulwark", "sounder", "skipper", "reedguard", "loom"]], (132, 84))
board("worker-cargo", [[f"{r}_7", f"{r}_7_loaded"] + [f"{r}_7_loaded_walk_{p}" for p in range(4)] for r in ["hook", "wick"]], (145, 84))
board("worker-tools", [[f"{r}_7_{m}_{p}" for p in range(3)] for r in ["hook", "wick"] for m in ["gather", "unload"]], (160, 84))
board("material-impacts", [[f"fx_impact_{m}_{p}" for p in range(4)] for m in ["steel", "reed"]], (128, 74))
board("material-breakdowns", [[f"fx_wreck_{m}_{p}" for p in range(6)] for m in ["steel", "reed"]], (128, 84))
board("crossing-materials", [[f"terrain_lane_{m}_{p}" for p in range(4)] for m in ["dry", "wet"]], (120, 42), 3)


def gif(name, keys, durations, scale=3, padding=10):
    frames = []
    for key in keys:
        im = crop(key)
        frame = Image.new("RGBA", (im.width + padding * 2, im.height + padding * 2), BG)
        frame.alpha_composite(im, (padding, padding))
        frames.append(frame.resize((frame.width * scale, frame.height * scale), Image.Resampling.NEAREST).convert("RGB"))
    frames[0].save(OUT / f"{name}.gif", save_all=True, append_images=frames[1:], duration=durations, loop=0, disposal=2)


for role in ["bulwark", "loom"]:
    keys = [f"{role}_7"] + [f"{role}_7_deploy_{p}" for p in range(3)] + [f"{role}_7_deploy_{p}" for p in [1, 0]]
    gif(f"{role}-deploy-preview", keys, [600, 330, 330, 930, 330, 330])
for role in ["riveter", "loom"]:
    gif(f"{role}-fire-preview", [f"{role}_7"] + [f"{role}_7_fire_{p}" for p in range(3)], [500, 70, 100, 130])
for role in ["hook", "wick"]:
    gif(f"{role}-loaded-preview", [f"{role}_7_loaded_walk_{p}" for p in range(4)], [140] * 4)
gif("gate-warning-preview", [f"gate_north_dry_warning_{p}" for p in range(4)], [200] * 4, 2)
print("Generated labeled native/2x inspection sheets and presentation GIFs from exported Aseprite art.")
