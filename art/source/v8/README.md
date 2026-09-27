# V8: machines and landscape show their state

The user approved all six art directions on September 13, 2026. Root personally
authored the Aseprite artwork through scripted pixel construction and visual
revision. No artwork was delegated and no generated-image pixels or tutorial
pixels entered the game assets.

The 40 new documents provide 482 new atlas entries. The earlier native
documents are retained beside them. The atlas is 2048×4096 with 928 entries;
the game canvas remains 640×360. All 446 v7 entries retain their exact pixels,
rectangles, and anchors. The final atlas uses 61 opaque colors and binary alpha.

| Documents | Native size / ground anchor | Editing purpose |
| --- | --- | --- |
| `bulwark_deployment`, `loom_deployment` | 64×64 / 32,50 | Eight facings, three deployment keys each; 333/333/334 ms preview |
| Six `*_firing` plus Bulwark/Loom `*_deployed_firing` | 64×64 / 32,50 | Eight facings, recoil/recovery keys at 67/100/133 ms |
| Hook/Wick `*_loaded`, `*_loaded_walk`, `*_gather`, `*_unload` | 64×64 / 32,50 | Cargo, distance-driven walking, working tools, unloading |
| Three `*_dismantling` documents | 160×144 / 80,128 | Intact, stripped, skeleton, exhausted ground residue |
| `gate_north_dry`, `gate_south_dry` | 160×144 / 80,128 | Stable water-level gauges and four 200 ms warning poses |
| `terrain_lane_dry`, `terrain_lane_wet` | 32×16 / 16,8 | Exposed and submerged crossbars; dry variants stay static |
| `sluice_foam` | 96×64 / 48,48 | Six 200 ms outflow keys after the actual switch |
| `impact_steel`, `impact_reed` | 48×48 / 24,32 | Four 100 ms material-contact keys |
| `breakdown_steel`, `breakdown_reed` | 64×64 / 32,50 | Six breakup keys, four simulation ticks each |
| `wreck_union`, `wreck_assembly` | 64×64 / 32,50 | Quiet, nonblocking ground debris |
| Three `landmark_*` documents | 160×144 / 80,128 | Old tide gauge, ferry stairs, and hull ribs |
| `console_bottom_*`, `console_top_*` | 640×72 and 640×24 / 0,0 | Faction-specific console edges with plain content areas |
| `console_type` | 80×50 / 0,0 | Fifty root-authored 7×9 glyphs, ten columns of 8×10 cells |

The six layers retain the existing editing contract: contact shadows, rear
mechanisms, hulls, cabins and shells, working parts, and surface finish. Stable
registration is important: the runtime uses the selected state for both drawing
and opaque-pixel selection.

Normal export from the project root:

```sh
/Applications/Aseprite.app/Contents/MacOS/aseprite --batch \
  --script-param root="$PWD" --script tools/art_v8/export_sources.lua
/Applications/Aseprite.app/Contents/MacOS/aseprite --batch \
  --script-param root="$PWD" --script tools/art_v8/verify_sources.lua
python3 tools/art_v8/preview.py
./tools/package_macos.sh
```

The exporter reads the native documents named in `tools/art_v8/sources.json`;
it never reconstructs their drawing from geometry. Editing the native font
document also updates `font7.rs` and the JSON pixel rows on export. Firing
assets have authored `muzzle` offsets in the manifest; keep these offsets in
step with a nozzle redesign. Frame durations, tags, semantic layers, bounds,
and anchors are checked against the source manifest.

`tools/art_v8/build.lua` is the root's reconstruction/recovery entry point.
It overwrites the 40 new documents. Do not use it over later native edits.
The v7 source/atlas backup is in `art/archive/art-v7`; the earlier code and
notes are preserved in `output/art-v8/baseline-code-and-notes.tar.gz`.

The preview GIFs are art presentations, with extra holds where helpful for
inspection. Loaded walking uses actual distance in the game rather than a
fixed timer. Runtime durations are expressed in 30 Hz ticks; source durations
and GIF rounding do not change the simulation.

[Root review](../../review-v8.md) · [Overview](../../../output/art-v8/overview.png)
