# Root-authored Aseprite sources

Every drawing in this folder was authored by root for the owner's v3 art request.
No Luna model drew or refined these assets.

Units: [Hook](hook.aseprite), [Riveter](riveter.aseprite),
[Bulwark](bulwark.aseprite), [Sounder](sounder.aseprite),
[Wick](wick.aseprite), [Skipper](skipper.aseprite),
[Reedguard](reedguard.aseprite), [Loom](loom.aseprite).
Each is 64×64 with six layers and 40 frames. Eight `idle_*` tags contain one
frame each; eight `walk_*` tags contain four frames each. Preview holds are
140 ms. The game advances those poses by distance traveled.

Buildings/props: [Union HQ](union_hq.aseprite), [Assembly HQ](assembly_hq.aseprite),
[Union Works](union_works.aseprite), [Assembly Works](assembly_works.aseprite),
[yard](dropoff.aseprite), [condenser](condenser.aseprite), [tower](tower.aseprite),
[gate](gate.aseprite), [salvage](salvage.aseprite), [well](well.aseprite).
Each is 160×144 with six layers and one frame.

The project palette is installed in every document. RGB pixel colors are
independent of palette entry order. Unit pivots are (32,50); structure pivots
are (80,128). Retain dimensions, layers and frame layout for the current exporter.

To export edits from these native files, run from the project root:

```sh
/Applications/Aseprite.app/Contents/MacOS/aseprite --batch \
  --script-param root="$PWD" --script tools/art_v3/export_sources.lua
./tools/package_macos.sh --reuse-build
```

`build.lua` reconstructs the original root drawings; `split_sources.lua`
recreates these documents. Those are authoring/recovery tools and would replace
subsequent manual edits. Use `export_sources.lua` for normal source editing.
