# Union art v2 review note

This is a Lua-authored native geometry pass for the Breakwater Union units. The
generated concept at `art/references/union-detail-v2.png` was used for study of
cream/orange/steel material ramps, cyan cab glazing, beveled hull construction,
tracks and flywheels, crane joints and hydraulic pistons, barrier hinges, and
the Sounder mast/dish silhouette. Its pixels were not imported, sampled, or
downscaled. The old `art/exports/game-assets.png` was inspected as the baseline;
its units read as small flat schematics, so this pass rebuilds the masses and
working parts from local 3D primitives.

The module returns the contract keys `hook`, `riveter`, `bulwark`, and `sounder`.
Each function accepts `(ctx, phase)` and uses only the shared context methods and
palette role names from `tools/art_v2/contract.md`. The layer plan is:

- `shadows`: ground contact ellipse;
- `far`: far track, rear rail, far flywheel or tripod leg;
- `body`: beveled crawler/chassis and major wheels;
- `upper`: cab, hammerhead, mast, shield structure;
- `front`: near machinery, glazing, shield plates, jacks, hook and dish;
- `details`: seams, rivets, toothed-wheel spokes, highlights, and the small
  receiver glint.

The 64x64 cells keep the ground anchor at the context anchor. Hook uses a thick
raised boom, piston, cable block, and a hook that projects beyond the cab.
Riveter uses two visible flywheels, rotating spoke clusters, a short riveter
nozzle, and a raised hammerhead. The flywheel faces are built in the local
forward/up plane, normal to the side axle; the Sounder receiver remains a
side/up disc. Bulwark keeps a broad split two-panel barrier,
hinges, braces, and four jack feet. Sounder keeps the body fixed while three
legs alternate a one-pixel lift and a screen-up asymmetric mast, antennae, and
receiver dish. Track shoes and wheel spokes use the four phases without bobbing
the complete unit.

Validation performed here: the two supplied reference images were viewed at
native resolution; the required pixel-art references were read and the relevant
metal, tech, spaceships, walk/run, top-down, modular, tripod, and illumination
sheets were inspected. An Aseprite batch harness exercised every Union key over
all eight facings and four movement phases; the core assertions verified palette
roles and bounds with no escapes. Root still needs to render the module through
the final raster/atlas runner and inspect all eight facings at 1x/2x, including
silhouette crops, phase contacts, palette compliance, and any
projection-specific visibility issues.
