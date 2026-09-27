# V13 — buildings without ground shadows

Root applied the owner's standing no-building-shadow rule to the shipped art.
This is an Aseprite Lua refinement of the earlier root-authored layered sources,
not mouse drawing or generated concept art. Prior versions are preserved.

23 editable documents contain 56 frames:

- Nine base structures: both headquarters and Works appearances, dropoff,
  condenser, tower, legacy gate and well.
- Five four-frame activity documents and five three-stage construction documents.
- Both current gate settings with all four warning phases (10 frames).
- The fixed tide-gauge and ferry-stair landmarks.

Broad cast shadows, surrounding under-building soil patches, and detached dark
gate marks are removed. Actual masonry foundation tops/sides, entrance ramps,
wall shading and machine parts remain. The base documents name their first
layer `fitted masonry foundation`. The current gate and fixed landmarks have
an explicitly empty `ground shadows forbidden - keep empty` layer.

Every frame retains its original 160×144 canvas, anchor, duration and tags.
See `sources.json` for the per-document records. Structural layers 2–5 are pixel-identical to
the previous sources in every frame. Layer 6 contains localized base cleanup;
upper activity/production drawing remains intact. The atlas still has 1,500
entries at 2048×7168 with 63 opaque colors; exactly 56 entries change.

## Editing and export

Use these saved sources for further building edits. Export and verify them with:

```sh
/Applications/Aseprite.app/Contents/MacOS/aseprite --batch \
  --script-param root="$PWD" \
  --script tools/art_v13/export_sources.lua
/Applications/Aseprite.app/Contents/MacOS/aseprite --batch \
  --script-param root="$PWD" \
  --script tools/art_v13/verify_sources.lua
```

`build.lua` reconstructs this correction from preserved earlier sources and
will overwrite v13 source edits. `footings.lua` draws actual foundation geometry
without an underlying shadow or ground perimeter. The reconstruction propagates
only the changed base pixels that still match the old base, preserving
frame-specific scaffolding and activity. The exporter copies exact RGBA,
including cleared pixels, into the v12 baseline atlas.

Earlier version exporters are historical and can restore superseded art.
Future atlas passes must start from the corrected v13 baseline, or reapply this
correction before activation. Verify the standing rule in `CONTRIBUTING.md` and the
regressions in `game.rs`; the known cast-shadow palette color is forbidden in
these structure frames. These guards cover concrete past failures; visual
review still has to catch new ways of drawing a shadow.

Root inspected all 56 revised native frames, same-scale before/after images,
engine construction/activity fixtures and the native Metal game. The skill's
previously studied construction, layering, material and contact guidance was
applied subject to the owner's stricter rule: **no shadows beneath buildings**.
No external image pixels or subagent artwork were used.

[Gate comparison](../../../output/art-v13/gate-comparison.png) ·
[Building comparison](../../../output/art-v13/building-comparison.png) ·
[Root review](../../../design/reviews/artistry/v13-root-review.md)
