# V22: the three-arm sluice station

A Claude agent (fix stream S6 of the trial 10 fixes) authored this pass on
2026-09-23. Before it started, it read the `aseprite-pixel-art` skill and every one of its references in full,
`tools/art_v2/contract.md`, the art v19/v20 tools and CONTRIBUTING.md's
no-shadow rule. This is scripted pixel art. `tools/art_v22/station3.py` builds
the layers and pieces from the v13 lock's own layers, clusters and palette.
`tools/art_v22/station3.lua` then assembles them into an editable Aseprite
document and exports it, in a headless Aseprite (1.3.15 source, built with
`LAF_BACKEND=none`). None of it is mouse drawing. No image generation, external bitmap or
resampling contributed a pixel.

## The problem

In the tenth self-play trial, all three seats reported the same thing: the Confluence's
sluice station was the Split Basin's two-shaft lock, with **N** and **S**
painted above its shafts. Its two gauges were north and south too (see the trial's frames
`union/0058`, `union/0177`, `assembly/0097` and `assembly/0123`). The Confluence's
arms are E, W and S, so on that map the letters named nothing, and the lock
showed only whether arm 0 was dry.

## The station

The Split Basin keeps the v13/v20 lock unchanged. On a map with three arms, the
engine draws this station instead (`crates/bw_desktop/src/station3.rs`).
It is built on the same hull, walls, shafts, mast and handwheel, so it stays the
same landmark:

- **One gauge shaft per arm, standing where the arm leaves the station on
  screen.** The left shaft is the arm running up-left (W) and the right shaft
  is the arm running up-right (E). A new, shorter shaft stands on the deck's front lip for the arm
  that runs down toward the viewer (S). The engine works out the slots from the
  map's arm centres, not from arm names.
- **Floats show the water.** A float is low when its arm is dry, high when the arm is deep, and
  half way on the neutral tide. All three are high in a flood. Every shaft has the same dry
  row, so a dry arm reads on one line.
- **The semaphore and the wheel's pale pointer point along the dry arm**
  (up-left, up-right, or hanging straight down for S). They stand upright
  when no arm is dry.
- **No letters.** The N and S signs are painted out of the base.
- **A change is seen.** The semaphore swings first. Then the wheel turns a
  full turn and on to its new pointer, and the floats ease to their rows
  with a one-pixel overshoot, in the two seconds the lane's water takes (v20's
  timing). While a switch or flood warning counts down, an amber pointer
  beside each shaft that will move marks the row its float is about to
  reach, and pulses. The wheel rocks an eighth on alternate pulses.
- **No shadow beneath the station.** The layer
  `ground shadows forbidden - keep empty` is empty. The masonry meets the
  ground. `station3.rs` tests that nothing is painted in the band where the
  lock's old cast shadow lay.

### What is new, pixel by pixel

- The front shaft uses the side shafts' construction at 15 columns:
  - an ink border;
  - a slate frame with the cap slab and pale rim;
  - a 7-column glass window with a teal glint;
  - tick marks on the side shafts' rows.
- A small pulley block under the rope bar at the mast carries the S rope.
- The semaphore's new angles are not rotations. A rotated 23-pixel blade
  breaks into ragged steps, so each blade is laid on a straight or 45° line
  and has an even run rhythm. It has the v13 blade's cross-section:
  - a maroon outline on the lit (upper-left) side;
  - the cream stripe;
  - a maroon body and edge;
  - a swallowtail at the tail.

  The upright and hanging blades are five pixels wide, with a three-wide neck.
  An earlier one-pixel neck read as an exclamation mark over the knob, so it was widened.
- The W and E rest blades are the v13 drawings, unchanged.
- The wheel's rim is the v13 rim with spokes painted out by radius (v20's
  method). Spokes and pointer are drawn at eight turns.
- The warning pointer is the v13 arrowhead without its bar, because the three shafts leave no
  room for the bar. It has a darker back column so it reads on sand and on water.

## Document

`station3.aseprite` is 160×144 with anchor (80, 128), like every gate key, and has 63 frames.

- Frame 1 (tag `base`) holds the base layers:
  - `ground shadows forbidden - keep empty`;
  - `rear mechanisms`, `hulls` and `cabins and shells` (v13, pixel-identical);
  - `front shaft (S arm)`;
  - `mast, bar, pulleys and wheel rim`.
- Each later frame is one piece, on its family's layer and tagged by family:
  - `float_side`: 23 frames. `station3_float_side_{n}` is n−1 pixels above the dry row, from −1 to 21. The engine draws the E shaft's float 46 px to the right.
  - `float_front`: 19 frames. `station3_float_front_{n}` runs from −1 to 17.
  - `flag`: `station3_flag_{0,22,45,90,135,158,180,225,270,315}` (degrees
    counter-clockwise from screen east).
  - `wheel`: `station3_wheel_{0..7}`, the pointer in eighths clockwise from east.
  - `pointer`: `station3_pointer_{0,1}` (full and small pulse), drawn at W's
    deep row. The engine shifts it to each shaft and target row.

The atlas keys are cropped to each family's common box, so a family shares one
anchor. They were packed into free 16-pixel cells of
`art/exports/game-assets.png`. No earlier rectangle or pixel moved, and
`tools/art_v22/verify_atlas.py` checks this.

## Build

    aseprite -b --script-param mode=layers --script-param root=$PWD \
      --script-param out=OUT/v13 --script tools/art_v22/station3.lua
    python3 tools/art_v22/station3.py OUT/v13 OUT/build
    aseprite -b --script-param mode=build --script-param root=$PWD \
      --script-param out=OUT/build --script tools/art_v22/station3.lua
    python3 tools/art_v22/pack.py $PWD OUT/build
    python3 tools/art_v22/verify_atlas.py OLD_ATLAS.png OLD_ATLAS.json $PWD
    python3 tools/art_v22/compose.py OUT/build poses.png 2   # review strip

`mode=build` overwrites `station3.aseprite`. After hand edits to the saved
document, run `mode=export` (with the same `out`) and then `pack.py`.

## Review

The stream's hand-back lists the engine frames, at world zoom 1× and 2×, under
`output/s6-station3/`. None of them is a mock-up. There are two kinds:

- `live-match-*` frames are captures from a headless `--map confluence`
  match. The match was at the neutral tide.
- The other frames come from the test
  `the_confluence_field_shows_the_station_without_north_or_south_signs`. It renders
  through the real renderer with the gate state set directly. Run it with
  `BW_STATION3_FRAMES=<dir>` to save the crops.

`design/reviews/three-seats/trial10-s6-station3.md` records the findings
and the questions left open.
