# Art

Everything the game draws comes from one atlas, `exports/game-assets.png`
with its index `exports/game-assets.json`, plus the interface icons in
`exports/ui-icons.png`. They are built in Aseprite from the editable
documents in `source/`.

## How the art is made

The art is native pixel art on the game's pixel grid, shown at whole-number
scales. Most of it was built by Aseprite Lua scripts that place every pixel
(`tools/art_v*`), written and reviewed pass by pass with AI agents; some
passes were edited directly in Aseprite. Each pass has a folder in `source/`
with its editable documents and a README that says what the pass changed,
how it was made and how to rebuild it. The passes build on each other, so
the older scripts stay: `tools/art_v19` loads v4, v5, v8, v9, v15 and v18,
and the current passes (v21–v23) load v5, v8, v18 and v19.

Standing rules for new art:

- **No shadows beneath buildings.** Buildings, the sluice and the wells
  sit on fitted foundations in direct contact with the ground. Shading on a
  building's own surfaces is fine; a shadow patch under it is not, in any
  state or preview, baked or drawn by the renderer.
- Native grid, integer scaling, a fixed light direction (top-left).
- Every existing rectangle in the atlas keeps its exact position; new art
  is added, never packed over old.

## Rebuilding

Aseprite 1.3.x is needed. Run the scripts from the repository root, for
example:

```sh
/Applications/Aseprite.app/Contents/MacOS/aseprite --batch \
  --script-param root="$PWD" \
  --script tools/art_v23/build.lua
```

Each pass README gives its exact commands. The exporters paint into the live
`exports/game-assets.png`, so rebuild from a clean checkout.

## Studies

`source/battlefield-pixel-study.aseprite` is the first pixel study
(`tools/create_pixel_study.lua`), and its enlargements are
`exports/battlefield-*.png`.

The pixel-art tutorials of Pedro Medeiros (Saint11,
[Saint11Tutorials](https://github.com/saint11/Saint11Tutorials), CC BY 4.0)
were studied while making this art. None of their pixels are in it.
