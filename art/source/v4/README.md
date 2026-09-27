# Editable v4 pixel art

Root personally drew the v4 interface, portraits, effects, shoreline assets and
material refinements. The unit/building drawings retain the root-authored v3
forms and gain surface overpainting. No art was delegated.

- Units: `hook`, `riveter`, `bulwark`, `sounder`, `wick`, `skipper`, `reedguard`,
  and `loom` `.aseprite` files: 64×64, six layers, 40 frames, 16 idle/walk tags.
- The ten building/prop `.aseprite` files: 160×144, six layers, one frame.
- [Interface](interface.aseprite): 1280×160 native working sheet containing the
  command console, ten 56×56 portraits and three resource icons. Backgrounds,
  console surfaces and portrait machinery occupy separate existing layers.
- [Full atlas](../game-assets-v4.aseprite): 2048×1536, including terrain,
  shoreline strips, vegetation, debris, and thirteen effect keys.

Normal export from the project root:

```sh
/Applications/Aseprite.app/Contents/MacOS/aseprite --batch \
  --script-param root="$PWD" --script tools/art_v4/export_sources.lua
./tools/package_macos.sh
```

The exporter reads the individual unit/building/interface documents and retains
terrain/effects from the full v4 atlas template. Keep the existing layer order,
names, dimensions, pivots and frame layout. `build.lua` and `split_sources.lua`
reconstruct this delivered drawing pass and would replace later native edits;
they are separate authoring/recovery tools.

The source is RGB with a 63-slot project palette, including transparency and
unused swatches. The final composite uses 57 opaque colors. Unit preview holds
are 140 ms; locomotion in the game is driven by distance traveled. Impact and
muzzle effects play quickly, while wreck bursts settle into smoke. The effect
study GIF is at ../../../output/art-v4/effects-preview.gif.
