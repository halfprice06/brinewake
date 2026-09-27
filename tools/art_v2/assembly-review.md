# Assembly v2 contribution review

This module is Lua-authored native pixel construction for the Silt Assembly
units. It does not import, sample, rotate, or downsample the generated
`art/references/assembly-detail-v2.png` concept board. The board was used to
choose the construction language: thick pale-jade composite panels, warm
woven reed ribs, amber banded pressure hardware, dark articulated sockets,
and open silhouettes with functional gaps.

## Visual diagnosis addressed

The previous Assembly sprites read as small flat wedges and dots. In
particular, Wick lacked a convincing work basket, Skipper had little structure
under its fan, Reedguard was a generic block, and Loom collapsed into a
triangle with a flat central mark. The redraw establishes a separate machine
identity for each key:

- Wick uses an open crescent carrier, exposed cross-lattice basket, three
  stilt chains, pressure tank, instrument cab, hose, counterweight, and a
  separated reed gripper.
- Skipper uses a low fan-shaped jade hull, selective roof ribs, underside
  intake vents, central ivory pod, amber service tank, and two long folding
  paddle legs with clear negative gaps below the hull.
- Reedguard uses a heavier low pressure chamber, four broad mechanical legs,
  a thick interlocking woven shield, ivory crest, amber chamber, and shoulder
  spool repeater. It has no insect face or antenna silhouette.
- Loom uses two segmented open arches with thick beveled jade beams, warm
  inner ribs, cross braces, a suspended banded amber drum, side-offset spool,
  tension struts, and three grounded spread legs. It intentionally avoids the
  former triangle-and-dot construction.

## Animation and construction checks

Each exported key is a function `(ctx, phase)` and supports phase `nil` or
`0..3`. All four functions are safe for eight facings through
`tools/art_v2/core.lua`; the ground axes are only transformed by `ctx:p`, and
vertical heights are always passed as the third coordinate. Main hulls,
chambers, tanks, pods, shields, and arches remain at a fixed body anchor.
Movement changes articulated knees, ankles, feet, paddles, and small service
contacts, rather than bobbing the entire body. Lift and recover phases carry
the toe and its low foot plate at `z=2`; contact phases return the plate to
`z=0`, so the planted foot is grounded while a swinging foot clears the floor.

The code uses only the exact shared palette roles and the six shared semantic
layers (`shadows`, `far`, `body`, `upper`, `front`, `details`). Large connected
planes are drawn before material cues; warm ribs and bright accents are sparse
and directional. Contact shadows, dark overlap sockets, top-left jade edges,
and pressure bands provide the value hierarchy at native 1x.

## Verification performed

`/Applications/Aseprite.app/Contents/MacOS/aseprite --batch` ran a temporary
in-memory harness that loaded `core.lua` and this module, cleared all six
layers, and rendered every key for facings `0..7` and phases `0..3`. The run
completed without Lua errors or cell-bound escapes. This is a structural and
native-bounds check; root must still render the atlas and inspect silhouettes,
material readability, phase contacts, and final in-game scale before approval.
