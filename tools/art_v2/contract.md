# Art v2 contribution contract

The user requests a substantial visual upgrade: richer, higher-quality native Aseprite pixel art modeled from new image-generated references. Keep the original BRINEWAKE factions/roles and game rules. Root integrates and visually reviews. Luna max agents own disjoint modules.

## Shared module API

Each file returns a table mapping asset keys to `function(ctx, phase)`. Phase is nil for idle, 0..3 for movement. Root invokes each unit for eight screen facings. `ctx` stores `cx`, `cy`, `facing`, `phase`, and current source-layer image. Unit cells are 64×64, ground anchor (32,50); aim for body width 28–38 and height 30–42 with room for raised crane/mast. The integrated building allocation is 160×144 for every structure, anchor (80,128), with body width 80–116. Exact native body detail matters more than cell dimensions.

Root supplies these methods (colon calls):

- `ctx:layer(name)` chooses shadows, far, body, upper, front, or details.
- `ctx:p(f,s,z)` returns integer screen x,y. Local forward/side ground axes rotate by facing; screen ground y is compressed to1/2. Height z ALWAYS remains screen-up. No raster rotations.
- `ctx:pixel(x,y,color)` and `ctx:line(x0,y0,x1,y1,color,width)` use absolute screen pixels. Width defaults1.
- `ctx:poly(points,color)` fills absolute screen2D vertices; `ctx:poly3(points,color)` maps local {f,s,z} vertices first.
- `ctx:line3(a,b,color,width)` connects local3D triples.
- `ctx:ellipse(x,y,rx,ry,color)` uses screen coordinates.
- `ctx:box(f,s,z,hf,hs,height,top,light,dark)` draws a grounded local cuboid; hf/hs are half-lengths. Automatically chooses visible front walls and shades consistently.
- `ctx:extrude(footprint,z,height,top,light,dark)` extrudes a local {f,s} polygon; use bevels rather than only boxes.
- `ctx:disc(center,axisA,axisB,r,color)` filled16-point disc in local3D plane. Axes are local3D unit vectors; useful for wheel faces, pressure tanks, spools, gauges.
- `ctx:shadow(rx,ry)` adds a short screen-ground contact shadow at anchor.
- `ctx:near_side()` returns +1/-1 for which local side projects toward the viewer.

Palette role strings:

ink, deep, shadow; steel0..3; cream0..3; orange0..3; jade0..3; straw0..3; amber0..3; glass0..3; plum0..2; silt0..3; ground0..3; water0..3; foam.

Color0 is darkest, color3 is brightest. Use shared ink/deep occlusion and 3–4 material steps. One upper-left light; selective surface highlights, real contact, no pillow shading or all-over noise. Root supplies exact palette values. Request a missing method/role instead of guessing one.

## Creative bar

Study the new reference image(s) once root posts their path. Name what you borrow: material construction, silhouette, cab, joints, pressure/drivetrain components. Redraw from geometry and clusters in native pixels; no importing/downsampling generated concept pixels. The current assets are too flat and schematic; achieve visible solidity, bevels, glazing, structural thickness, functional machinery and deliberate wear.

Preserve worker, line-fighter, protector, scout/support identity at native1x. Keep negative spaces and original faction identity. Movement must preserve volume/anchor: wheel phases and articulated contacts; no whole-body bob as a substitute. Use the Aseprite skill, including required reference reads/images. Clearly label Lua-authored art and actual tests.

Do not alter Rust, the current atlas, other modules, or simulation. Root owns the compiler/atlas source and runtime integration. Each artist owns only its named Lua module and a short review note. Build one representative facing first, then expand the working design. Root will render and return concrete feedback.
