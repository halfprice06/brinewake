# Current editable art: v6

This version repairs the bases of `union_hq`, `assembly_hq`, `union_works`,
`assembly_works`, `dropoff`, `condenser`, `tower`, `gate`, and `well`. Each remains
160×144, with one frame and the shared six-layer contract. The first layer now
holds a fitted ground foundation instead of a detached oval shadow. Ground
contact and entrance approach edits occupy the surface-finish layer.

The other native documents, frames, tags, palette and canvas sizes are retained
from [v5](../v5/README.md). The full atlas is
[game-assets-v6.aseprite](../game-assets-v6.aseprite). Previous source and exports
are preserved under `art/archive/art-v5/`.

Export native edits from the project root:

```sh
/Applications/Aseprite.app/Contents/MacOS/aseprite --batch \
  --script-param root="$PWD" --script tools/art_v6/export_sources.lua
/Applications/Aseprite.app/Contents/MacOS/aseprite --batch \
  --script-param root="$PWD" --script tools/art_v6/verify_sources.lua
./tools/package_macos.sh
```

Keep dimensions, layer names/order, frame order and anchor metadata. The exporter
reads the individual native documents and retains template-only shores/effects.
`ground_buildings.lua` reconstructs these nine base repairs from the archived v5
sources, replacing the nine v6 documents. Use the normal exporter for later
native edits; the reconstruction script would replace those edits.
