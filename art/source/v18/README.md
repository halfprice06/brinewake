# V18 — the audit fixes: budgets, walks, marks and walls

A Fable agent authored this pass on 2026-09-18 from the audit in the
project's art thread, after reading the `aseprite-pixel-art` skill and all
sixteen of its references in full and its walk, idle, smoke and ruins
sheets. It is scripted pixel art, Lua construction through the v4 painter
run in a headless Aseprite build, not mouse drawing; no image-generation
tool, external bitmap or tutorial pixel contributed any delivered pixel.
The audit that drove it, with its own sheets, is in the project files
(`art-audit-v15-v17`). Every key keeps its rectangle and anchor; the v17
atlas is archived under `art/archive/art-v17/`.

## What changed and why

| Finding | Change | Where |
| --- | --- | --- |
| 19–35 colours per machine against the 8–16 contract | Each machine draws inside a budget of two or three ramps, one accent, ink; missing families fold into the nearest ramp. Now 12–17 (Dredger 17, a heavy 64-cell unit); Drydocks 19–20; Palisades 12–13 | `tools/art_v18/machines.lua` (`out.budget`), `common.lua` (`M.budget`), `tools/art_v5/painter.lua` (`remap`) |
| Tidewatch and Lampwright under the light-unit width | Wheels on a 26-unit chassis; a wide stance and a larger lantern. Both fill 28+ units | `machines.lua` |
| Walks: level bodies, legs jumping 4–6 px, dredger paddles floating | Legs have a stance and a swing: the planted foot moves back six units a frame (about a quarter cell, the engine's walk phase), the swing foot lifts four with the knee forward, the body rises one or two units over the straight planted leg. Near legs draw in front of the hull, far legs behind, and the dredger hull sits lower over its paddles | `machines.lua` (`leg`, `legs`, `out.rise`), `painter.lua` (`dz`) |
| Idle: a one-pixel twitch and three orphan pixels | Union machines settle a pixel while a five-pixel puff forms in one cluster, then rises, spreads and drifts right; Assembly glow kept | `build_machines.lua` (`idle_frame`) |
| Damage: hash speckle on glass and roof planes, vertical rust on flat tops | Marks read the base drawing's planes from its shading roles and sit on wall planes only: a 3x2 (4x3 on buildings) pit with a lit lip and a narrowing rust run, spaced apart; a torn opening with a rib and a lit rim; one connected soot smudge. Re-authored for every machine and building, v4 to v17 | `damage.lua`, `build_damage.lua` |
| Effects translate without evolving; lone pixels | VENT forms, splits, thins and breaks up downwind; SOUND is a ring of short arcs; MEND is a burst, travelling streaks with bright heads, then embers | `portraits_effects.lua` |
| Palisades read as crates | One wall segment along the up-right diagonal, post to post, so a line joins into a wall | `buildings.lua` |
| Barge lamp an orphan speck; Lifter scattered blocks | The lamp is a housing on the wheelhouse roof; the Lifter carries one envelope over its gondola | `machines.lua` |
| Portrait and effect documents on machine layer names | Portraits save on `plate` and `machine`; effects on one `effect` layer | `portraits_effects.lua` |

## Documents

69 documents, 753 frames, all under `art/source/v18/`: eight machine sets
(`{role}`, `{role}_idle`, `{role}_damage`), `caisson_deployment`, four
`dredger_*` worker sets, two Drydocks and two Palisades with construction,
activity and damage, damage overlays for the seven earlier buildings and
the eight earlier machines, eight portraits, three effects.

## Build, export, verify

```sh
aseprite --batch --script-param root=$PWD --script tools/art_v18/build_machines.lua
aseprite --batch --script-param root=$PWD --script tools/art_v18/build_buildings.lua
aseprite --batch --script-param root=$PWD --script tools/art_v18/build_damage.lua
aseprite --batch --script-param root=$PWD --script tools/art_v18/portraits_effects.lua
aseprite --batch --script-param root=$PWD --script tools/art_v18/export_sources.lua
aseprite --batch --script-param root=$PWD --script tools/art_v18/verify_sources.lua
```

The verifier proves every v17 rectangle that v18 did not re-author exact,
every re-authored key in its old rectangle and anchor, no shadow beneath a
building, machine shadows in the shadow colour off the body, damage marks
on the base silhouette and never on glass, and prints the colour count of
every re-authored document. Engine captures:
`target/debug/brinewake --art-v16-review output/art-v18/review`.

## What was reviewed, and what was not

Contact sheets at 1x, 2x, 3x and 6x, before/after pairs, grayscale, and
engine captures at 1x, 2x and 3x from both seats. Walk, idle and effect
loops were exported as GIFs at 140 ms per walk frame for review; the
engine advances walk frames by distance (one per quarter cell), so a
Tidewatch or Lampwright at six cells a second cycles far faster than that
and its feet still skate some. No human appeal evidence. The eight original
machines keep their v4–v14 colour counts; only their damage overlays were
re-authored.
