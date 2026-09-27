# V21: the Saltglass Compact

A Claude agent authored this pass on 2026-09-23, after reading the
`aseprite-pixel-art` skill and its references in full, docs/DESIGN.md §8 and §20,
`tools/art_v2/contract.md` and `docs/THIRD-FACTION-AND-CONFLUENCE.md`. It
is scripted pixel art: Lua construction through the v21 painter (the v5
painter with the Compact's ramps) run in a headless Aseprite built from
source, and hand-placed Python pixel rows for the command-card icons. No
image-generation tool, external bitmap or resampling contributed a pixel.

Every key is new. No earlier rectangle moved: the packer
(`tools/art_v21/common.lua` `open_atlas`) places new keys in free 16-pixel
cells and a rebuild reuses a key's rectangle. No engine or rules code
changed; the "Third faction and three-player map" thread wires the keys.

## The look

The Compact is the fast, fragile side, so its machines are tall and thin:
stilt legs with backward knees and salt shoes, masts, mirror dishes on
yokes, lenses. Materials: cobalt-violet glaze plates for the body mass, a
salt-white crust at the joints, caps and feet, copper collars and lances,
clear glass that carries the one bright accent, the glint.

| Role | Hex |
| --- | --- |
| glaze0–4 | 221733 33295c 4b3f8f 6d69bd a4ade0 |
| crust0–2 | 8d88a3 c7c5cf efeee4 |
| glint | fffbe6 |

Copper uses rust1/rust2, brass reed3, glass glass1–3, the kiln and lance
glow amber2/amber3. Machines fold rust0 to glaze0 and rust3 to rust2
(`compact.lua` `BUDGET`), so every frame stays within 12–16 colours;
buildings stay within 8–16 (`buildings.lua` `BUDGET`). In greyscale the
Compact reads darker and thinner than the Union (pale blocks) and the
Assembly (mid-tone domes): see the review's lineup sheets.

## Machines (`build_machines.lua`, `compact.lua`)

Eight machines, eight facings, 64x64 at anchor (32,50):

- Raker (worker, rake), Brander (fire-lance), Heliostat (mirror dish),
  Glinter (heliograph mast), Stilt (scout), Glazier (kiln pot and
  blowpipe), Salter (salt hopper and chute), Pan (brine pan and petals).
- Every machine has `{n}_{f}`, `_walk_{0..3}`, `_idle_{0,1}` and
  `_damage`, plus the bare `{n}` alias.
- The Brander and the Glinter have `_fire_{0..2}` with muzzles.
- The Heliostat has `_deploy_{0..2}` and `_deployed_fire_{0..2}`, with
  its muzzle at the focus lens. `_deploy_2` is its braced pose.
- The Pan has `_deploy_{0..2}` and `_deployed_{0..3}`, which is steam off
  the brine.
- The Raker has the gatherer set: `_gather`, `_loaded`, `_loaded_walk`
  and `_unload`.
- The Salter has `_lay_{0..2}`, which is salt pouring from the chute.

The walks follow the art v19 rules. A planted foot moves back exactly the
engine's travel on its facing (`common.lua` `travel`), and the body rises
a unit on the passing frames. Frame durations are the engine's time per
frame at full speed (`phase_ms`).

The idle settles the machine onto its knees, then runs a shine across its
glass. Damage is glaze chips and cracks (`damage.lua`): a chip shows the
pale body under the glaze, and a crack runs from it.

Surge overlays (`build_pressure.lua`): `pressure_{brander,glinter,heliostat}_{f}_surge_{0..3}`
and `_cool_{0..2}`, drawn over the machine like the other sides'. A copper
nozzle and glass sight bulb on the back vent a white brine jet.

## Buildings (`build_buildings.lua`, `buildings.lua`)

These are 160x144 at anchor (80,128). Each stands on a tiled salt pad, and
nothing is drawn beneath it.

- `compact_hq` is the Kiln: a banded bottle kiln with a glowing mouth, two
  heliostats throwing sun into it, salt pans and a packing shed.
- `compact_works` is the Glassworks: a glass-roofed hall on stilts, a
  chimney and a cooling rack of panes.
- `compact_drydock` is a stilted gantry over a brine slip with a new pod in
  the cradle.
- `compact_dropoff` is the Rake shed.
- `compact_tower` is the mirror nest.
- `compact_palisade` is salt blocks between glazed posts, 64x64 at anchor
  (32,50).

The key sets match the other two sides:

- construction `_build_{0..2}`, except the HQ;
- activity `_active_{0..3}` for the Kiln, the Glassworks and the Drydock;
- `_lamps` for the Kiln and the Glassworks;
- `_damage_{1,2}` on every building.

## Console (`build_portraits.lua`)

- `portrait_{machine}` and `portrait_compact` (56x56). A machine portrait
  is the machine built again at a larger scale with the painter's `scale`
  (constructed, not resampled) on the v17 console plate.
- `faction_compact_badge`, `_emblem`, `_hq` and `_plate` (the plate reads
  COMPACT in the v8 font).
- `hud_top_compact` and `hud_bottom_compact`: the Union bars with the trim
  recoloured role for role to violet and salt.
- `wreck_compact`.

## Effects (`build_effects.lua`)

- `terrain_crust_{0..3}` (32x16, anchor 16,8) are laid causeway cells. Set
  cells join without a seam, so a strip reads as one road.
- `terrain_crust_fresh_{0..2}` is the row being laid, as wet salt, then
  setting, then set.
- `fx_beam_flare_{0..2}` and `fx_beam_hit_{0..3}` (32x32, anchor 16,16) are
  the Heliostat's beam. The game draws the line between them: a one-pixel
  glint core (fffbe6) with glass3 (91ccd0) edges.
- `fx_impact_glaze_{0..3}` and `fx_wreck_glaze_{0..5}` follow the
  `fx_impact_{material}` and `fx_wreck_{material}` patterns for a glaze
  material.

## Command-card icons (`tools/ui_icons/compact.py`)

These are 24x24 in the lean HUD's icon sheet:

- `ui_unit_{machine}` for the eight machines;
- `ui_build_compact_{hq,works,drydock,palisade,dropoff,tower}`;
- `ui_cmd_glint` and `ui_cmd_lay`;
- `ui_cmd_dry_{e,w,s}` and `ui_cmd_flood`, the Confluence station's
  tide orders.

The palette gains the glaze, crust and glint letters in `draw.py`.

## Rebuild

    aseprite -b --script-param root=$PWD --script tools/art_v21/build_machines.lua
    aseprite -b --script-param root=$PWD --script tools/art_v21/build_buildings.lua
    aseprite -b --script-param root=$PWD --script tools/art_v21/build_portraits.lua
    aseprite -b --script-param root=$PWD --script tools/art_v21/build_effects.lua
    aseprite -b --script-param root=$PWD --script tools/art_v21/build_pressure.lua
    python3 tools/ui_icons/build.py aseprite
    python3 tools/art_v21/review.py        # sheets and engine-speed walk GIFs

Registries: `tools/art_v21/sources-{machines,buildings,console,effects,pressure}.json`.
