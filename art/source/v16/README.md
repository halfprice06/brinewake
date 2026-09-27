# V16 — the tide roles, the Drydock and the Palisade

Root authored this pass in Aseprite through Lua construction in the v4
painter's projection, palette and six layers, followed by contact-sheet
inspection at 1x, 2x and 3x, engine captures from both seats and revision
(the Lampwright's stance; damage marks and strands kept on the silhouette;
no shadow beneath a building). This is scripted pixel art, not mouse
drawing. No subagent, image-generation tool, external bitmap or tutorial
pixels contributed any delivered pixel. Brief: the seats' proposals after
the third two-agent trial; its plan was an internal document, not published.

46 editable documents contain 545 frames. The atlas grows from 2048x8192 to
4096x8192 and from 1,778 to 2,329 entries; every earlier rectangle, anchor
and pixel is exact against the v15 atlas archived under
`art/archive/art-v15/`, and every v16 rectangle packs into the right half.
The palette stays at 65 opaque colours.

| Documents | Native canvas | Content |
| --- | --- | --- |
| Six `{role}` (tidewatch, caulker, caisson, lampwright, tender, dredger) | 64x64, anchor 32,50 | Base facing and four walk frames per facing, eight facings; grounded shadow |
| Six `{role}_idle` | 64x64 | Two acted frames per facing: Union settle and puff, Assembly flex and glow |
| Six `{role}_damage` | 64x64 | Dents and streaks on steel, creases and strands on the weave, one facing each |
| `caisson_deployment` | 64x64 | Three stages per facing: the plug settles and the side plates rise |
| Four `dredger_{loaded, loaded_walk, gather, unload}` | 64x64 | The worker set: the hopper fills, the chain digs, the hopper empties |
| `union_drydock`, `assembly_drydock` with `_construction`, `_activity`, `_damage` | 160x144, anchor 80,128 | The shed over the slipway with a gantry that lifts (Union) or a bobbin winch that turns (Assembly); site, frame, fit-out; two damage tiers |
| `union_palisade`, `assembly_palisade` with `_construction`, `_damage` | 64x64, anchor 32,50 | A plate wall on a footing; a bound reed hurdle between jade poles |
| Six `portrait_{role}` | 56x56 | Console close-ups on the faction plate |
| `fx_sound`, `fx_vent`, `fx_mend` | 64x32, 48x48, 24x24 | The SOUND ring, the VENT plume, mending sparks |

## What the engine does with them

- The Drydock trains the roles and researches Tracks; its gantry or winch
  moves while it produces. The Palisade blocks a cell.
- The Caisson's deployment frames play on D; deployed it holds `_deploy_2`.
- The Dredger draws the worker set when loaded, gathering and unloading.
- SOUND draws its ring on the ground for five seconds; VENT its plume over
  the headquarters for ten; mending draws the repair marks.

## Editing and export

```sh
/Applications/Aseprite.app/Contents/MacOS/aseprite --batch \
  --script-param root="$PWD" \
  --script tools/art_v16/export_sources.lua
/Applications/Aseprite.app/Contents/MacOS/aseprite --batch \
  --script-param root="$PWD" \
  --script tools/art_v16/verify_sources.lua
```

The exporter starts from the archived v15 atlas in the left half of a
4096x8192 canvas and paints the registered v16 frames into their rectangles.
The verifier reopens every document and checks pixels, canvas, layer names,
anchors, durations and tags, proves every archived rectangle exact, keeps
machine shadows in the shadow colour off the body, and keeps damage marks
on their base's silhouette above its lowest row. Engine review captures:
`target/debug/brinewake --art-v16-review output/art-v16/review`.
