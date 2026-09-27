# Editable v7 building activity and construction

The pass adds two forms of visual storytelling to the v6 art: small devices
show a building operating, and construction visibly progresses from a prepared
site through a supporting frame to final installation work.

All native state documents are **160×144**, use the six shared semantic layers,
and retain the **80,128** ground anchor.

| Documents | Frames and tags | Runtime use |
| --- | --- | --- |
| `union_hq_activity`, `assembly_hq_activity` | Four frames, `active`, 200 ms each | Finished HQ signal cloth, fan or pressure activity |
| `union_works_activity`, `assembly_works_activity` | Four frames, `active`, 200 ms each | Factory crank/piston or loom shuttle during progressing production |
| `condenser_activity` | Four frames, `active`, 200 ms each | Steam and movement inside the pressure glass |
| `union_works_construction`, `assembly_works_construction`, `dropoff_construction`, `condenser_construction`, `tower_construction` | Three individually tagged frames: `foundation`, `frame`, `fit_out` | Construction thirds, selected from remaining build time |

All listed documents have the `.aseprite` extension. Activity is isolated on the
surface-finish layer; the first five layers and foundation pixels are identical
to the completed v6 source. Construction keeps the grounded source foundation
and adds staged machinery and scaffolds with feet on that foundation. The last
construction key retains the finished architecture beneath installation work.

The existing v6 documents are retained unchanged in this directory. The full
atlas is [game-assets-v7.aseprite](../game-assets-v7.aseprite), 2048×2048 with
446 entries. The 35 added keys occupy three new rows below the old atlas.
Every original key, rectangle, anchor and exported sprite remains unchanged.

Normal export from the project root:

```sh
/Applications/Aseprite.app/Contents/MacOS/aseprite --batch \
  --script-param root="$PWD" --script tools/art_v7/export_sources.lua
/Applications/Aseprite.app/Contents/MacOS/aseprite --batch \
  --script-param root="$PWD" --script tools/art_v7/verify_sources.lua
./tools/package_macos.sh
```

`build.lua`, `activity.lua` and `construction.lua` reconstruct this drawing pass
from the archived v6 source. They replace the ten state documents; use normal
export for later native edits. Frame order, tags, durations, layer names and
anchors are part of the renderer contract.

The optional command `target/release/brinewake --art-review output/art-v7/engine`
exports explicitly labeled offscreen state fixtures. These are renderer studies,
not a recorded match. After ordinary faction-start captures, `review.py` makes
the comparison sheets and GIFs. [Activity preview](../../../output/art-v7/building-activity.gif)
uses the same four 200 ms keys as the game, at 2× display scale.
