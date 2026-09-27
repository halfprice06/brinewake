# Chart and craft: editable sources

Root personally authored these **56 frames in 16 Aseprite documents** through
scripted native pixel construction, then inspected and revised the exports.
They are original production assets integrated in artistry v10. They are not
image-generation outputs or manually painted GUI strokes.

| Source family | Dimensions | Frames | Editing intent |
|---|---|---|---|
| `water_current_0..2` | 128x64 | 8 each | Separate current body, undersides, traveling crest, small secondary ripples; transparent quiet water |
| `manual_bulwark`, `manual_loom` | 224x144 | 3 each | Mechanical cutaways with editable chassis/frame, feet, vessel/ram, shield/release, annotations |
| `faction_*_emblem` | 24x24 | 1 each | Union enamel/stencil and Assembly reed impression |
| `faction_*_badge` | 16x12 | 1 each | Simplified portrait mark |
| `faction_*_hq` | 24x16 | 1 each | Small maker's mark for a completed headquarters |
| `faction_*_plate` | 96x24 | 1 each | Faction name and emblem in menu/header margins |
| `trace_jack`, `trace_rivet`, `trace_binding` | 32x16 | 6 each | Fresh material, four wash stages, subdued silt residue |

Every source has six semantic paint layers. UI plates are top-left anchored;
water modules use 0,0, traces use 16,8, and HQ markings use 12,14. See
`tools/art_v10/sources.json` for every exact atlas key, frame, anchor and timing.
Manual source holds match the guide: Bulwark .8/1/1 seconds; Loom .8/.8/1.
Trace lifetimes and wash holds come from their observed event clocks in the
desktop, not the source's convenience preview timing.

`tools/art_v10/build.lua` **reconstructs** these sources. Do not run it over
artist edits. `tools/art_v10/export_sources.lua` exports the saved editable
documents into the atlas; `verify_sources.lua` reopens them and checks exact
composites, timing, anchors, and preservation of the v9 atlas region.

The production atlas is 2048x7168 with 1,500 entries and 61 opaque colors.
All 1,444 earlier entries retain their complete metadata and original pixels.
The atlas remains within the engine's existing 16-megapixel ceiling. Its
decoded storage grows by 8 MiB from v9; the game canvas stays 640x360.

Provenance: palette and bitmap typeface reuse root-authored BRINEWAKE sources;
new geometry and pixel decisions are in the root construction script. No
external museum photograph or tutorial pixels were imported. The Aseprite
skill references informed cluster/line design, material construction, contact,
limited animation, UI hierarchy and seam review. Artwork was revised after
native/2x inspection to remove stripe-like water bands, break repeated wave
rows, and expose Bulwark's near jack instead of pointing at its wheels.

Native and enlarged frame inspections, source/export checks, and controlled
engine views are in `output/art-v10/`. GIFs there are exported previews;
actual-speed perceptual review and native interaction remain unverified
because the Mac was locked. Owner art appeal is not inferred from these checks.
