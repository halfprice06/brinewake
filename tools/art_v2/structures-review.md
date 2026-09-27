# Structures art v2 review note

This module is a Lua-authored native pixel geometry pass for the ten structure
and world-object keys required by the art-v2 runner: `union_hq`, `assembly_hq`,
`union_works`, `assembly_works`, `dropoff`, `condenser`, `tower`, `gate`,
`salvage`, and `well`.  It uses only the shared context methods and palette
roles from `tools/art_v2/contract.md`; it does not change Rust, the current
atlas, simulation, or another artist's module.

The supplied faction board and battlefield concept were used to study the
setting and silhouette language.  `art/references/union-detail-v2.png` guided
the cream/orange/steel ramps, repurposed tug hulls, glazing, repair seams,
winches, gantry, pipes, rails, and maintenance machinery.  `assembly-detail-v2`
guided the jade/straw/plum/amber ramps, thick woven ribs, suspended pressure
heart, open floors, stilt feet, spools, and hose joints.  The images were
redrawn from local geometry; no generated or reference pixels were imported,
sampled, rotated, or downsampled.

The layer plan is deliberately structural:

- `shadows` holds the short anchor contact shadow;
- `far` holds rear pads, undercarriages, back braces, and quiet ground planes;
- `body` holds the main beveled hull, factory, arch, tank, pylon, or wreck mass;
- `upper` holds cabs, gantries, ribs, spines, pressure chambers, and hoists;
- `front` holds visible glazing, hatches, rails, wheels, gauges, and gun parts;
- `details` holds sparse seams, bolts, cross-weave, needles, light accents, and
  functional wear.

The buildings are rendered by the root runner at fixed facing 1, which exposes
both local ground faces in the intended 2:1 diagonal presentation.  All large
spans remain local: f=s paths form screen-horizontal rails and arches, z is
always screen-up, and no finished raster is rotated.  Main footprints are
shifted slightly toward the rear local f axis so their near ground corners stay
inside the requested bottom anchor margin.  The current harness and runner use
160x144 cells with anchor (80,128) for all ten structures.

The revision also converts `screen_panel` and `hatch` to explicit local f/s
centers so glazing, doors, and cargo faces stay attached to their host planes.
Wheel spokes now use the same screen/up plane as their discs.  Union Works has
attached rail spans, roof ribs, vent housings, a framed roller bay, and paired
inspection windows.  Assembly arch and canopy highlights follow real ribs and
canopy segments; no detached vertical straw strokes remain.

Structure identity is carried by working construction rather than decoration:
the Union HQ has three low tug-hull masses, a clear open maintenance court,
offset wheelhouses with cyan glazing, a suspended hook block, and a two-tier
gantry.  Assembly HQ has two parallel thick woven arches, a suspended amber
pressure heart, open floor, cross-braces, capsule collars, and stilt feet.
Union Works is a squat press factory with a ribbed roller bay, chimney, trolley,
service housings, and hazard accents; Assembly Works is a lower woven canopy
with side spools, reed curtains, pressure hoses, and a seed chamber.  Dropoff
is a hoist yard with framed, cross-braced cargo crates.  Condenser is a tank and
pump with liquid core, feed/return elbows, and two readable gauges.  Tower is a
defense machine with an armored turret, sensor dish, tall spine, long gun, and
threat lamp.  Gate is a concrete/silt pair of pylons around a large mechanical
wheel, dark water channel, sill, and separate level gauge.  Salvage is a
grounded wreck pile of split hull, exposed engine block, flywheel, fallen plate,
bent mast, and hanging cable.  Well is a stone-ringed water opening with pump
body, handle, and small gauge.

Validation performed here: the module was executed by Aseprite 1.3.18.3-arm64
through a disposable native raster harness against `core.lua`; all ten keys
rendered in 160x144 cells, and phases 0, 1, 2, and 3 each completed without a
syntax, unknown-role, layer, or bounds error.  The resulting contact sheet was
inspected at native size and at 4x nearest-neighbor, including separate crops
for every key.  The measured opaque extents in the 160x144 harness were:

| Key | Opaque extent (w x h) | Opaque pixels |
| --- | ---: | ---: |
| union_hq | 116 x 86 | 7,146 |
| assembly_hq | 103 x 93 | 6,211 |
| union_works | 101 x 95 | 6,752 |
| assembly_works | 95 x 69 | 4,002 |
| dropoff | 85 x 67 | 3,225 |
| condenser | 77 x 83 | 3,360 |
| tower | 81 x 101 | 4,512 |
| gate | 98 x 96 | 6,156 |
| salvage | 85 x 59 | 2,841 |
| well | 65 x 71 | 2,010 |

These are raster measurements and construction evidence, not a claim of final
art approval.  Root should render the module through the actual combined atlas
and inspect it against the new unit modules at 1x/2x, on land and water, with
all fixed-facing structures in world context.  In particular, check whether
the compact Works/dropoff/well silhouettes need more screen coverage, whether
the tall tower and HQ arches clear nearby units, and whether the gate wheel and
level pointer remain legible at ordinary monitor distance.

Root final corrections: projected the Union Works left-wall windows and ribs on its actual f=-16 face, attached the rail to the s=18 wall lip, and aligned Works/gate spoke centers with their wheel rims. The final combined atlas was regenerated and reviewed after these corrections.
