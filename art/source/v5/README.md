# Editable v5 art

Root authored the new landscape and salvage artwork personally through
Aseprite Lua. Existing units, buildings, interface, effects and shore strips
are retained. The full source is [game-assets-v5.aseprite](../game-assets-v5.aseprite).

| Document | Native canvas | Frames / editing purpose |
| --- | --- | --- |
| `salvage.aseprite` | 160×144 | Tug wreck; one frame |
| `salvage_turbine.aseprite` | 160×144 | Broken turbine; one frame |
| `salvage_barge.aseprite` | 160×144 | Cargo wreck; one frame |
| `terrain.aseprite` | 40×24 | Twenty tagged terrain keys, four each for salt, silt, causeway, water and shallow water |
| `coastal_props.aseprite` | 64×64 | Three rock variants and reeds; four tagged frames |
| `salt_crust.aseprite` | 96×64 | Three tagged ground overlays |
| Eight unit documents | 64×64 | Forty frames and sixteen idle/walk tags each, retained from v4 |
| Nine other building/object documents | 160×144 | One frame each, retained from v4 |
| `interface.aseprite` | 1280×160 | Console, portraits and icons, retained from v4 |

All use the shared six-layer contract. For the wrecks, contact/shadow,
rear/debris, hull, cabin/casing, working/broken parts and surface finish remain
separate. Terrain bases and surface accents occupy separate layers. Empty
contract layers are retained for export compatibility. The 63-slot project
palette is unchanged; the final atlas uses 58 opaque colors.

From the project root, export native edits and package them:

```sh
/Applications/Aseprite.app/Contents/MacOS/aseprite --batch \
  --script-param root="$PWD" --script tools/art_v5/export_sources.lua
/Applications/Aseprite.app/Contents/MacOS/aseprite --batch \
  --script-param root="$PWD" --script tools/art_v5/verify_sources.lua
./tools/package_macos.sh
```

Preserve dimensions, layer names/order, frame order and anchor metadata.
The exporter reads these documents, and takes untouched shores/effects from
the full v5 template. It saves the assembled versioned and active atlas.
`build.lua` is a reconstruction tool: it starts from the archived live v4 atlas
and replaces v5 landscape drawings and their native documents. Do not run it
over later artist edits; use the normal exporter above.

The water [GIF](../../../output/art-v5/water-study.gif) is a 3× presentation
study reconstructed from the runtime tile sequence. Its underlying composition
is 320×144, with three 800 ms phases.
