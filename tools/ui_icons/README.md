# Command-card icons

The 24x24 icons on the lean HUD's command card, quick-select column,
selection roster and production queue: 78 of them (orders, machines,
buildings, upgrades, doctrines).  `stats.py` adds nine 8x8 marks for the
hover card's number rows (salvage, pressure, crew, time, damage, reach,
hull, speed, sight), drawn in the corner of a 24x24 frame and cropped to
8x8 in the manifest.

They are **scripted pixel art**: `icons.py`, `units.py`, `buildings.py`,
`upgrades.py`, `compact.py` (the Saltglass Compact) and `stats.py` place every pixel with the helpers in `draw.py` (rectangles,
hand-written rows, hand-chosen circle widths, a full dark outline), using
the game atlas palette plus the interface red.  Nothing is generated from
images or resampled.

Rules they follow:

- Light from the upper left; one dark outline so an icon reads on every
  button state.
- The top-right corner (columns 17-23, rows 0-8) stays quiet where it can:
  the button's key chip sits there.
- Each machine shows its cube head and the one part that names it (the
  crane, the lamp, the reed shield), in its side's colour: Union orange,
  Assembly jade.

## Build

    python3 tools/ui_icons/preview.py review.png          # 1x, 2x, 3x and a 2x button
    python3 tools/ui_icons/build.py /path/to/aseprite

`build.py` writes the editable source `art/source/ui/command_icons.aseprite`
(one frame and one tag per icon) through Aseprite, then exports
`art/exports/ui-icons.png` and `ui-icons.json` from it.  The game appends
that sheet below the main atlas at load (`Atlas::merge_ui_icons`), so the
two rebuild apart; without it the card falls back to text buttons.
