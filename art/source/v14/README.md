# V14 — light on the ground, weight under the machines

Root authored this pass in Aseprite through Lua construction, followed by
native-size, 2x and 3x inspection in engine captures and revision. This is
scripted pixel art, not mouse drawing. No subagent, image-generation tool,
external bitmap or tutorial pixels contributed any delivered pixel. Brief:
"research plan and improve the art and artistry across the whole game";
its plan was an internal document, not published.

40 editable documents contain 841 frames. The atlas stays 2048x7168 and grows
from 1,500 to 1,521 entries; 820 existing entries change pixels in place, 21
are appended below row 6496. Every rectangle and anchor of the 1,500 earlier
entries is unchanged. One palette role is added: `damp` `#736c61` (cool wet
sand), making 64 opaque colours.

| Documents | Native canvas | Content and editing layers |
| --- | --- | --- |
| Eight machines (`hook` … `loom`), 40 frames each | 64x64, anchor 32,50 | Idle and walk facings from the v7 sources; only the first layer, `contact shadows`, is redrawn |
| `bulwark_deployment`, `loom_deployment`, six `*_firing`, two `*_deployed_firing`, eight worker state documents, `loom_pressure` | 64x64, anchor 32,50 | v8/v9 state frames; only `contact shadows` is redrawn |
| `ground_salt_low`, `ground_salt_dim`, `ground_salt_pale`, `ground_salt_high`, `ground_damp` | 40x24, anchor 20,12 | Ground plane, mineral bodies, fracture and wear; four variants each, variant 0 plain |
| `bank_e`, `bank_s`, `bank_w`, `bank_n` | 40x32, anchor 20,12 | Wet foot, eroded bank face, salt cap, sediment and foam; three variants each |
| `water_current_0..2` | 128x64, anchor 0,0 | Quiet transparency, deep shade pools, current ribbons, ribbon shadow, surface sheen, travelling crests; eight 200 ms phases |
| `home_vignette` | 256x124, anchor 0,0 | Deep water, shelf, ground, banks, crossings, water surface, machines behind, station, machines in front, coast life |

## What changed

- **Machine shadows.** Every machine frame's flat oval is replaced by a
  projection of its own silhouette (layers 2 to 6) onto the ground toward the
  lower right, matching the top-left key light used on the sprites, plus a
  two-row contact strip under parts that reach the ground. Overhanging cranes
  and shields cast no floating strip. Structural layers, frames, tags,
  durations and muzzle offsets are untouched; the verifier proves it.
- **Ground light.** Low, dim, mid, pale and high salt families give the floor
  broad tonal planes; the renderer chooses them from a deterministic value
  noise over cell coordinates and from adjacency to deep water (damp). No
  entity or visibility state is read.
- **Waterline.** Banks break their salt cap into runs, vary the eroded face,
  add a dark wet foot and sparse foam. The renderer adds a two-step shallow
  shelf on deep water beside dry ground, wandering along the bank so it does
  not trace the diamond grid.
- **Water.** The current modules gain soft pools of deeper shade with broken
  outlines, two-pixel ribbons with a shadow edge, and crests that travel the
  ribbons across the eight phases. The shared ribbon at y=12 still joins
  neighbouring modules; the runtime sampler is unchanged.
- **Home vignette.** A 256x124 diorama of the sluice station between a dry
  and a flooded crossing, composed on the game's own grid from the v14 ground,
  banks, water and exact copies of project machines and coast life.

Buildings, effects, portraits, interface strips and the manual plates are
outside this pass and keep their v13 pixels. The no-building-shadow rule is
preserved: no building frame changed, and the machine shadow verifier only
covers machine documents.

## Editing and export

```sh
/Applications/Aseprite.app/Contents/MacOS/aseprite --batch \
  --script-param root="$PWD" \
  --script tools/art_v14/export_sources.lua
/Applications/Aseprite.app/Contents/MacOS/aseprite --batch \
  --script-param root="$PWD" \
  --script tools/art_v14/verify_sources.lua
```

The exporter starts from the archived v13 atlas (`art/archive/art-v13/`),
replaces each registered rectangle exactly (transparent pixels included), and
appends the rectangles recorded in `art/exports/game-assets-v14.json`. The
verifier reopens every document and checks pixels, canvas, layer names,
anchors, durations and tags against the active atlas; for machine documents
it also checks that layers 2 to 6 equal the previous sources, that the shadow
uses only the shadow colour, never overlaps the body, and stays at the base.

`shadows.lua`, `ground.lua`, `banks.lua`, `water.lua` and `vignette.lua` are
root's construction scripts. They overwrite the documents they name; do not
run them over later artist edits. `common.lua` holds the shared palette, the
diamond-clipped tile painter and the append-only packer. The per-script
registries (`sources-*.json`) are merged into `sources.json` on export.

[Root review](../../../design/reviews/artistry/v14-root-review.md) ·

[Shadow before/after](../../../output/art-v14/shadow-before-after.png) ·
[Water modules](../../../output/art-v14/water-modules-comparison.png) ·
[Banks](../../../output/art-v14/banks-comparison.png) ·
[Ground families](../../../output/art-v14/ground-families.png)
