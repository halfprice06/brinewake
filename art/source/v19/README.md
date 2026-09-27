# V19 — machines at the engine's own speed, and the original eight in budget

A Claude agent authored this pass on 2026-09-22, after reading the
`aseprite-pixel-art` skill and every one of its references in full. It is
scripted pixel art: Lua construction through the v5 painter, run in a
headless Aseprite built from source, not mouse drawing. No image-generation
tool, external bitmap or tutorial pixel contributed any delivered pixel.
Every key keeps its rectangle, anchor and muzzle; the v18 atlas is archived
under `art/archive/art-v18/`. No engine or rules code changed.

## What changed and why

| Finding | Change | Where |
| --- | --- | --- |
| The eight original machines (v4–v14) used 19–29 colours against the 8–16 contract; v18 fixed only the later roles | Each is folded to its budget, role to role, after drawing (so the v4 overpaint masks still see their roles): the Union four to 16 with ink, the Assembly four to 14–16. Rest, fire, deployment, worker poses, the Loom's pressure strip, pressure and cargo-cradle overlays and damage overlays all take the same fold, so no state changes colour when it plays | `tools/art_v19/common.lua` (`folds`), `build_machines.lua` §1, §4 |
| Feet skate. The engine advances a walk frame per quarter cell of Manhattan travel, which is 4 painter units on a screen-axis facing and 5.66 on a diagonal; v18 stepped 6 on every facing, so on four facings the planted feet slid back 2 px a frame; v4 stepped 6 on diagonals | The planted foot moves back exactly the engine's travel on its facing, for all six legged machines and the three loaded walks | `common.lua` (`travel`), `originals.lua` (`leg`), `roles.lua` (`leg`) |
| Wheels, the Barge's stern wheel and the Lifter's screw turned half their symmetry period a frame (four spokes at 45°, three blades at 60°): no direction, and at the Tidewatch's 1.6 frames a tick they read backwards | Every rotation turns a quarter period a frame (22.5°, 30°), top forward, and stays under half a period per 30 Hz tick at every machine's full speed; the one exception is a surging Tidewatch on a screen-axis facing (2.4 frames a tick) | `motion.lua` |
| Tracks shimmered (bars shifted 0,1,0,1) and showed no drive side-on, where the hull hides the top run | The top-run bars advance a quarter pitch a frame and wrap; each road wheel carries a hub bar that turns while driving | `motion.lua` (`tracks`) |
| The original eight kept the v15 idle: a one-pixel dip and three orphan pixels at the middle of the cabin | The v18 acting: the Union four vent one five-pixel cluster from the stack v4 built on them (the Hook's engine stack, the Riveter's boiler, the Bulwark's roof vent, the Sounder's rear stack), first seen where it clears the silhouette, then risen and drifting; the Assembly four breathe inside the budget, the glow peaking at ivory3 | `build_machines.lua` (`idle_frame`, `VENT`) |
| Legs of the originals: the knee never changed and the swing foot barely cleared the ground | Passing over the planted foot the leg straightens; on the swing the knee folds up and the foot lifts four units. The body stays level, because the engine draws the pressure sleeves, cargo cradles and damage marks of these machines at the rest position over every walk frame | `originals.lua` (`leg`) |

The documents' walk-frame durations are the engine's own time per frame at
full speed on that facing (`common.lua` `phase_ms`), so Aseprite's preview
plays at game speed; the idle frames are 200 ms, the engine's 6 ticks.

## Documents

60 documents, 1,728 frames under `art/source/v19/`:

- redrawn: `{role}` for all sixteen machines (rest and four walk frames per
  facing; the v18 roles' rest frames are their v18 pixels), `hook_loaded_walk`,
  `wick_loaded_walk`, `dredger_loaded_walk`, `{original}_idle`;
- recoloured from the shipped documents, layer by layer: the original eight's
  rest frames, `hook_gather`, `hook_loaded`, `hook_unload`, the `wick_`
  equivalents, `{role}_firing`, `bulwark_deployment`,
  `bulwark_deployed_firing`, `loom_deployment`, `loom_deployed_firing`,
  `loom_pressure`, `pressure_{role}` for the six armed originals,
  `hook_cargo_cradle`, `wick_cargo_cradle` and `{original}_damage`.

## Build, export, verify

```sh
aseprite --batch --script-param root=$PWD --script tools/art_v19/build_machines.lua
aseprite --batch --script-param root=$PWD --script tools/art_v19/export_sources.lua
aseprite --batch --script-param root=$PWD --script tools/art_v19/verify_sources.lua
```

The build checks every source document it reads against the archived v18
atlas and reports how far the v4 and v18 constructions reproduce the shipped
rest frames: the v18 roles exactly, the originals within 3 px in total (the
Hook's face 5 and the Riveter's face 0 round one or two pixels differently
from the shipped documents; their rest frames are the shipped pixels). The
verifier proves the 717 rectangles v19 did not touch exact, every re-authored key in its old rectangle, anchor and
muzzle, no new key, every recoloured frame equal to its v18 pixels through
its machine's fold and nothing else, the v18 roles' rest frames unchanged,
shadows in the shadow colour off the body, no shadow beneath a building, and
damage marks on the base silhouette and never on glass; and it prints the
colour count of every document.

## What was reviewed, and what was not

Sheets at 1x, 2x, 3x and 7x, before and after, grayscale, and walk loops
played at engine speed: the sim's Chebyshev movement at each machine's spec
speed, the desktop's walk phase per quarter cell, one frame per 30 Hz tick,
over scrolling ground, all sixteen machines in all eight facings. Engine
captures of the starting bases through the existing v16 review mode. No
human has judged the result at speed, and there is no appeal evidence.
Still open: the fastest walkers (Tidewatch and Lampwright, six cells a
second) cycle 1.6 walk frames a tick on the screen-axis facings, which no
four-frame drawing can make read as steps; the engine's frame rule would
have to change for that, and this pass changed no engine code. The v18
roles' idle and damage documents are v18's.
