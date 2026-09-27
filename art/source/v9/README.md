# V9: what the tide leaves

Root-authored scripted Aseprite artwork for the owner's **Implement all**
request on September 13, 2026. The brief is the seven directions in
`art/explorations/2026-09-13-tidal-artistry/README.md`; the integration contract
was an internal design plan, not published. No drawing was delegated.

All new drawings live in the 24 editable `.aseprite` documents here. They
contain 516 exported frames, appended after the preserved 928-entry v8 atlas.
Six layers separate contacts, structure, working mechanisms, and finish;
terrain, coastal life, and instruments have names suited to those subjects.
The sources use the existing 61-color project palette. Sprite dimensions are
native; the game canvas remains 640×360 with integer display scaling.

| Documents | Native size | Content |
|---|---|---|
| `archaeology_north`, `archaeology_south` | 160×80 | Eight frames each: four dry, four flooded; ferry paving/rails and ropewalk channels |
| `pressure_riveter/bulwark/sounder/skipper/reedguard/loom` | 64×64 | 56 transparent frames each: eight facings, four Surge and three cooling frames |
| `loom_pressure` | 64×64 | 48 complete frames: eight facings of tension, release, recoil, and settling |
| `hook_cargo_cradle`, `wick_cargo_cradle` | 64×64 | 32 transparent cargo-fitting frames each |
| `union/assembly_hauling/fire_control_workshop` | 160×144 | Four-frame HQ attachments for each faction and doctrine |
| `union/assembly_hauling/fire_control_plate` | 32×24 | Four workshop portrait plates |
| `coast_birds`, `coast_crab`, `coast_reeds` | 96×64 | Eight frames each; rooted reeds, paired birds, crab entering a crevice |
| `dock_telegraph` | 48×24 | Idle, submitted, accepted, and rejected instrument states |
| `result_signal` | 32×48 | Four lit harbor signal variants |

The pressure layers attach to the existing machine's ground anchor. Their
steam jets leave a physical valve, and a dark firing shutter accompanies Surge.
The Loom source reuses root's v8 deployment construction and changes the
suspended chamber/bindings while its supports stay fixed. The crab's entire
silhouette is clipped at the crevice entry, including its claws. Coastal
performances receive long quiet holds in the engine.

## Editing and export

Edit the individual source and export without rebuilding its geometry:

```sh
/Applications/Aseprite.app/Contents/MacOS/aseprite --batch \
  --script-param root="$PWD" \
  --script tools/art_v9/export_sources.lua

/Applications/Aseprite.app/Contents/MacOS/aseprite --batch \
  --script-param root="$PWD" \
  --script tools/art_v9/verify_sources.lua

python3 tools/art_v9/review.py
```

`tools/art_v9/build.lua` is the original root construction script. It
**reconstructs and overwrites these v9 documents**, so use it only when
intentionally revising the construction and after preserving any native edits.
`export_sources.lua` reads the edited documents, composites their frames into
the new atlas regions, and preserves the v8 pixels from `art/archive/art-v8/`.
`sources.json` records document/frame, dimensions, anchors, and timing.

Runtime timing: Surge frames 100ms, cooling/HQ/coast frames 200ms, flooded
archaeology frames 400ms; the Loom's 24-tick shot uses 4/4/3/4/4/5 tick holds.
Worker cradle sway is a small cosmetic attachment over the preserved worker
locomotion; it does not move its supporting chassis. GIF review copies round
timings to centiseconds and are not the timing authority.

## Provenance and review

The new art is scripted pixel construction by root, visually reviewed and
revised by root. It is not manually drawn artwork or image-generation output.
The reused base machinery is earlier original BRINEWAKE art by root. External
pixel-art lessons informed cluster, contact, material, and timing decisions;
no tutorial pixels were imported into runtime assets.

The source verifier reopens every document and checks its exported pixels,
dimensions, timing, and anchors. The QA compositor checks every original atlas
entry and the full original pixel region for exact preservation. Contact
sheets and engine fixtures are in `output/art-v9/`; these are labeled controlled
tests. See the [implementation review](../../../design/reviews/artistry/root-review.md)
for actual validation and the limits of native/human review.
