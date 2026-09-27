# V15 — the chart, the lanes, the wind and the day

Root authored this pass in Aseprite through Lua construction, followed by
contact-sheet, 2x and 3x inspection in engine captures and revision. This is
scripted pixel art, not mouse drawing. No subagent, image-generation tool,
external bitmap or tutorial pixels contributed any delivered pixel. Brief:
"Implement all" of `art/art-direction-opportunities-2026-09-14.md`; the plan
was an internal design plan, not published.

66 editable documents contain 1,041 frames: 257 new frames and 784 machine
frames whose shadow layer was rebuilt in place after the owner found the
v14 projection made machines look lifted. The atlas grows from 2048x7168 to
2048x8192 and from 1,521 to 1,778 entries; every earlier rectangle and anchor
is unchanged, the 792 machine rectangles carry the grounded shadow, and every
other pixel is exact against the v14 atlas archived under
`art/archive/art-v14/`. The palette stays at 65 opaque colours.

| Documents | Native canvas | Content and editing layers |
| --- | --- | --- |
| `lane_drying` | 32x16, anchor 16,8 | Lane floor (exact copy of the dry lane tile), damp wash, standing pools, salt rime; four stages, four variants each |
| `shore_plants_sway` | 64x64, anchor 32,50 | The v12 plant layers sheared by height; two lean frames per plant |
| `submerged` | 64x32, anchor 32,16 | Deep silhouette, softer edge; wall, roof, hull, posts, stair, ring |
| Eight `{role}_idle` | 64x64, anchor 32,50 | The v7 layers with the acted dip or flex, puff or glow, and a reprojected shadow; two frames per facing |
| Seven `{building}_damage` | 160x144, anchor 80,128 | Dents and tears, streaks and strands, soot; two tiers |
| Eight `{role}_damage` | 64x64, anchor 32,50 | The same layers; one tier per facing |
| Four `{hq,works}_lamps` | 160x144, anchor 80,128 | Lit glass, lamp bloom |
| `coast_birds_fly`, `coast_crab_hide`, `home_birds` | 96x64, anchor 48,52 | The v9 site layers with the animals replaced by the take-off, the hide, and gulls alone |
| `home_vignette_live` | 256x124, anchor 0,0 | The v14 vignette layers with the water surface at each of the eight phases |
| 32 re-grounded machine documents (`hook` … `loom_pressure`) | 64x64, anchor 32,50 | The v14 structural layers exact; `contact shadows` rebuilt as the authored oval, a contact strip and a short flat cast. The idle documents received the same shadow |

## What the engine does with them

- **Lane memory.** After a public gate change the drained lane shows the
  drying stages for a minute; a foam front and a receding sheen are code.
- **Wind.** Bank plants pick their lean from the gust field at their own
  world position; the water crests take the same field.
- **Drowned town.** The silhouettes draw under the crests at fixed deep
  cells, in chart ink when hidden.
- **Idle acting.** A machine with no order plays its strip on an id-keyed
  period; a busy machine holds still.
- **Damage and lamps.** Overlays draw over the unchanged base frame by hull
  tier; lamps only in the evening key. Nothing is drawn beneath a building.
- **Living coast and home.** The take-off and hide strips play when a
  machine arrives; the home page cycles the vignette phases and the gulls.

## Editing and export

```sh
/Applications/Aseprite.app/Contents/MacOS/aseprite --batch \
  --script-param root="$PWD" \
  --script tools/art_v15/export_sources.lua
/Applications/Aseprite.app/Contents/MacOS/aseprite --batch \
  --script-param root="$PWD" \
  --script tools/art_v15/verify_sources.lua
```

The exporter starts from the archived v14 atlas, rewrites the 792 machine
rectangles in place with their grounded shadows, grows the canvas and appends
the rectangles recorded in `art/exports/game-assets-v15.json`. The verifier
reopens every document and checks pixels, canvas, layer names, anchors,
durations and tags against the active atlas, proves every other archived
entry unchanged, proves the re-grounded frames keep their structural layers
exact with a shadow in the shadow colour only and no shadow colour in any
body, checks that idle frames keep their shadow at the base, and checks that
damage and lamp overlays never draw outside or below their base silhouette.

`lane.lua`, `plants.lua`, `submerged.lua`, `idle.lua`, `damage.lua`,
`lamps.lua`, `coast.lua`, `vignette.lua` and `shadows.lua` are root's
construction scripts.
They overwrite the documents they name; do not run them over later artist
edits. `common.lua` holds the palette, the painter, exact atlas copies and the
append-only packer. The per-script registries (`sources-*.json`) are merged
into `sources.json` on export.

[Root review](../../../design/reviews/artistry/v15-root-review.md) ·

[Engine captures](../../../output/art-v15/engine/) ·
[Sheets](../../../output/art-v15/)
