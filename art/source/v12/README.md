# Coastal materials v12

Root personally authored this pass in Aseprite using Lua construction, followed
by native-scale visual inspection and in-game revisions. This is scripted pixel
art, not mouse-drawn art. No subagent, image-generation tool, external bitmap,
or tutorial pixels authored the delivered assets. Brief: “Please improve the
game art,” interpreted as a focused coastal environment refinement.

Ten editable documents contain 33 static material/shape variants:

| Documents | Native canvas | Content and editing layers |
| --- | --- | --- |
| `ground_salt`, `ground_silt`, `ground_causeway` | 40×24 | Ground plane, mineral bodies, fracture/wear; four variants each |
| `salt_shelves` | 96×64 | Recesses, mineral plates, sunlit chips, seams; three variants |
| `coastal_rock` | 64×64 | Contact, stone masses, fracture planes, crevices, salt/lichen; three variants |
| `shore_plants` | 64×64 | Root contact, back leaves, stems, front leaves, seed heads; reed, grass, bush |
| `bank_e`, `bank_s`, `bank_w`, `bank_n` | 40×32 | Wet foot, eroded face, salt cap, sediment/foam; three variants each |

The frames are static variants, not an animation cycle; their 1-second source
durations are editor holds. Native bounds and ground anchors are unchanged.
The 2048×7168 runtime atlas has 1,500 entries and still uses 63 opaque colors.
31 entries have changed pixels; two plain ground fills match their old pixels.
The other 1,467 entries are outside this pass and remain exact.

Use the saved documents as the editable source of truth:

```sh
/Applications/Aseprite.app/Contents/MacOS/aseprite --batch \
  --script-param root="$PWD" \
  --script tools/art_v12/export_sources.lua
/Applications/Aseprite.app/Contents/MacOS/aseprite --batch \
  --script-param root="$PWD" \
  --script tools/art_v12/verify_sources.lua
```

`build.lua` reconstructs the initial drawings and overwrites these sources;
do not run it over subsequent artist edits. The exporter replaces exact RGBA,
including transparent pixels, in the v11 baseline atlas. Older sources are
preserved, and `art/archive/art-v11/` retains the exact task-entry atlas.

Visual study used the skill's complete Markdown references, the relevant
Saint11 material/terrain sheets and neighboring sampled frames, and the
line/curve/banding/directional-light/hue-ramp diagrams. Applied lessons: planes
before texture, quiet terrain below machines, irregular interlocking mineral
shapes, upper-left light, root contact, and variation without noisy speckling.
The reference sheets are Pedro Medeiros / Saint11, CC BY 4.0, studied locally
only; see the skill's attribution. No new external reference was downloaded.

Root inspected equal-scale comparisons, both factions at 1×/2×/3× in engine
fixtures, a grayscale diagnostic, and native Union practice with selection,
group movement and zoom. These inspections support delivery; owner appeal is
not inferred. See the [root review](../../../design/reviews/artistry/v12-root-review.md)
and [comparison](../../../output/art-v12/asset-comparison.png).
