"""Build the command-card icons through Aseprite.

    python3 tools/ui_icons/build.py path/to/aseprite

Writes art/source/ui/command_icons.aseprite (one frame and one tag per
icon, on a layer named "icon") and exports art/exports/ui-icons.png and
ui-icons.json, the sheet the game merges into its atlas at load.  The
icons are scripted pixel art: icons.py and its modules place every pixel.
"""

import json
import os
import subprocess
import sys
import tempfile

from draw import PALETTE, N
from icons import ICONS
from stats import SMALL

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
SOURCE = os.path.join(ROOT, "art", "source", "ui", "command_icons.aseprite")
SHEET = os.path.join(ROOT, "art", "exports", "ui-icons.png")
MANIFEST = os.path.join(ROOT, "art", "exports", "ui-icons.json")
COLUMNS = 8

LUA = r"""
local data = dofile(app.params.data)
local spr = Sprite(24, 24, ColorMode.RGB)
spr.layers[1].name = "icon"
local pal = Palette(#data.palette + 1)
pal:setColor(0, Color{r=0, g=0, b=0, a=0})
for i, c in ipairs(data.palette) do
  pal:setColor(i, Color{r=c[1], g=c[2], b=c[3], a=255})
end
spr:setPalette(pal)
for i, icon in ipairs(data.icons) do
  if i > 1 then spr:newEmptyFrame(i) end
  local img = Image(24, 24, ColorMode.RGB)
  for _, p in ipairs(icon.pixels) do
    img:drawPixel(p[1], p[2], app.pixelColor.rgba(p[3], p[4], p[5], 255))
  end
  spr:newCel(spr.layers[1], i, img, Point(0, 0))
  local tag = spr:newTag(i, i)
  tag.name = icon.key
end
spr:saveAs(app.params.out)
"""


def lua_table(icons):
    out = ["return {", "palette = {"]
    colours = sorted({c for c in PALETTE.values() if c})
    out += ["{%d,%d,%d}," % c for c in colours]
    out.append("}, icons = {")
    for key in icons:
        grid = ICONS[key]().text()
        px = []
        for y, row in enumerate(grid):
            for x, ch in enumerate(row):
                c = PALETTE.get(ch)
                if c:
                    px.append("{%d,%d,%d,%d,%d}" % (x, y, *c))
        out.append('{key="%s", pixels={%s}},' % (key, ",".join(px)))
    out.append("}}")
    return "\n".join(out)


def main(aseprite):
    keys = sorted(ICONS)
    os.makedirs(os.path.dirname(SOURCE), exist_ok=True)
    with tempfile.TemporaryDirectory() as tmp:
        data = os.path.join(tmp, "icons.lua")
        script = os.path.join(tmp, "build.lua")
        open(data, "w").write(lua_table(keys))
        open(script, "w").write(LUA)
        subprocess.run([aseprite, "-b", "--script-param", "data=" + data,
                        "--script-param", "out=" + SOURCE, "--script", script], check=True)
        sheet_json = os.path.join(tmp, "sheet.json")
        subprocess.run([aseprite, "-b", SOURCE, "--sheet", SHEET, "--sheet-type", "rows",
                        "--sheet-columns", str(COLUMNS), "--format", "json-array",
                        "--data", sheet_json], check=True)
        frames = json.load(open(sheet_json))["frames"]
    sprites = {}
    for key, frame in zip(keys, frames):
        f = frame["frame"]
        size = SMALL.get(key)
        w, h = (size, size) if size else (f["w"], f["h"])
        sprites[key] = {"x": f["x"], "y": f["y"], "w": w, "h": h, "anchor_x": 0, "anchor_y": 0}
    rows = (len(keys) + COLUMNS - 1) // COLUMNS
    manifest = {"width": COLUMNS * N, "height": rows * N, "sprites": sprites}
    with open(MANIFEST, "w") as fh:
        json.dump(manifest, fh, indent=1, sort_keys=True)
        fh.write("\n")
    print(f"{len(keys)} icons -> {SHEET}")


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "aseprite")
