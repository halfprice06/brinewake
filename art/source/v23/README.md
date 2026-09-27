# V23: the Saltglass Compact's missing pieces

A Claude agent authored this pass on 2026-09-23 as stream R3 (Compact art) of
trial 10's second round of fixes. Before starting, it read the
`aseprite-pixel-art` skill and all of the skill's references,
`tools/art_v2/contract.md`, the v19/v21 art tools and CONTRIBUTING.md's no-shadow
rule.

It fills the gaps that `design/reviews/three-seats/step-4-compact-rules.md`
and `design/reviews/artistry/v21-compact-review.md` left open. Before this
pass, the Compact had:

- no doctrine plates;
- no doctrine headquarters keys;
- no Raker Hauling kit;
- no `trace_glaze`;
- no GLINT effect sprite, so the ray was drawn with two lines and borrowed
  the Heliostat's flare.

## Provenance

This is **scripted pixel art**. None of it was drawn with a mouse, and no
image generation, external bitmap or resampling contributed a pixel.

- **Hand-placed literal rows** (`tools/art_v23/pieces.py`) make up the
  plates, the headquarters annexes, the trace and the three GLINT effects.
  - Each pixel is a character in a row the agent wrote, and each character
    maps to a palette role.
  - The palette is the v21 painter's (glaze, crust, copper, glass, amber,
    glint, UI). The trace adds the four water and silt tones of the v10
    traces.
- **Painter construction** (`tools/art_v23/cradle.lua`) makes the Raker's
  panniers. They are built through the v21 painter, in the Raker's own
  projection, materials and light.
- **Headless Aseprite 1.3.15** (source build, `LAF_BACKEND=none`) runs
  `tools/art_v23/build.lua`. The script:
  - builds each piece as a layered, tagged document in this folder;
  - paints every frame into free 16-pixel cells of `art/exports/game-assets.png`
    with the v21 packer;
  - writes the manifest in the earlier passes' sorted-key layout.

  No earlier rectangle or pixel moves.

Rebuild:

    python3 tools/art_v23/pieces.py PIECES.json
    aseprite -b --script-param root=$PWD --script-param pieces=PIECES.json \
      --script tools/art_v23/build.lua
    python3 tools/art_v23/verify_atlas.py OLD.png OLD.json .

`verify_atlas.py` checks against the atlas as it was before the build. Its
checks:

- the build added 59 keys, all in v23's families;
- every earlier key keeps its rectangle and anchor;
- no pixel outside the new rectangles changed;
- every added key matches its entry in `tools/art_v23/sources.json`.

`sources.json` maps each key to its document, frame, size, anchor, duration
and layers.

## Keys

| Document | Keys | Size, anchor | What |
|---|---|---|---|
| `doctrine_compact_hauling` | `doctrine_compact_hauling` | 32x24, 0,0 | Dock plate: a salt bin on stilts under a copper yoke. |
| `doctrine_compact_fire_control` | `doctrine_compact_fire_control` | 32x24, 0,0 | Dock plate: a mirror mast with its glass disc and a sun. |
| `doctrine_compact_hauling_hq` | `doctrine_compact_hauling_hq_{0..3}` | 160x144, 80,128 | A salt crib on four stilts on the Kiln's pad, left of the kiln. A glint crosses its shaded face on frame 1. |
| `doctrine_compact_fire_control_hq` | `doctrine_compact_fire_control_hq_{0..3}` | 160x144, 80,128 | A signal plinth with two small mirrors on the same pad spot. The left mirror catches on frame 1, the right on frame 3. |
| `raker_hauling_panniers` | `doctrine_raker_{0..7}_hauling_{0..3}` | 64x64, 32,50 | Two glazed panniers slung on copper straps, swaying one unit along the heading. See below. |
| `trace_glaze` | `trace_glaze_{0..5}` | 32x16, 16,8 | A fused glass bead with a dark silt edge and two drops. Frames 1-4 are the wash, in the v10 traces' water tones. Frame 5 is a fleck of glass in a residue line. |
| `fx_glint_flash` | `fx_glint_flash_{0..3}` | 32x32, 16,16 | The heliograph flash at the mirror: a long level streak and a short upright one, shrinking. |
| `fx_glint_spot` | `fx_glint_spot_{0..3}` | 32x16, 16,8 | A ring of light spreading on the ground's 2:1 diamond at the end of the ray. |
| `fx_glint_mote` | `fx_glint_mote_{0..2}` | 16x16, 8,8 | A spark running out along the ray. |

About the Raker panniers:

- The far pannier is cut away wherever the Raker covers it in any of its 18
  poses. The Raker's contact shadow is its own; the kit adds none.
- The document has a "face N" tag per facing.

The Glinter's mirror is the muzzle on its `glinter_{f}_fire_0` pose.

## No shadows

Nothing is drawn beneath the crib or the plinth: no oval, drop, cast or
contact patch. Their stilts and plinth stand directly on the Kiln's salt
pad, and the only shading is on their own faces. The panniers add no shadow
to the Raker.

## Where the engine draws them

- **Plates:** `dock_log::doctrine_plate_key`.
- **Annexes and panniers:** `artistry::headquarters_doctrine_key` and
  `artistry::doctrine_key`. These were already wired; the keys were missing.
- **Glaze traces:** `tidal_traces` now leaves a `Glaze` trace where an
  observed Compact shot lands on a lane. `damage::RepairMarks` already named
  `trace_glaze` for a Glazier's patch.
- **GLINT:** `glint_fx` and `game.rs` draw the following:
  - the flash at the casting Glinter's mirror for 8 ticks, falling back to
    40 px above the ray's root;
  - a glint-white spine with pale glaze edges, fading;
  - three motes easing out along the ray;
  - the ring at the ray's end.

## Status

Checked in headless frames at 1X and 2X world zoom; see
`design/reviews/artistry/v23-compact-missing-art.md`. These frames come from
controlled fixtures and a practice field, not from human play, and they are
not a judgment on final art quality.
