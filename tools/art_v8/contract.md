# V8 implementation contract

The user authorized all six directions in `art/art-direction-opportunities-2026-09-13.md` with “Do it all thank you”. Root authors all Aseprite artwork. Engineering delegation follows AGENTS.md. Existing rules and replay hashes remain authoritative; art must not invent wind-up damage timing, impassable water, collision, income, or hidden information.

The old 446 atlas entries remain available. New entries are appended in a 2048×4096 atlas (within the current loader limit), with RGBA binary alpha and existing material colors. Existing 640×360 rendering and integer scaling remain.

New names and source sizes (all indices zero based):

- `gate_north_dry`, `gate_south_dry`: stable gauge states, 160×144, anchor (80,128).
- `gate_north_dry_warning_0..3`, `gate_south_dry_warning_0..3`: current state named in key; target pointers show its opposite. Four 200 ms phases; gauges show current water truth.
- `terrain_lane_dry_0..3`, `terrain_lane_wet_0..3`: 32×16, anchor (16,8), exposed/submerged bars, restrained reflections.
- `fx_sluice_0..5`: 96×64, anchor (48,48), foam pulse immediately after actual switch, not before.
- `bulwark_{face}_deploy_{stage}`, `loom_{face}_deploy_{stage}`: 64×64 anchor (32,50), faces 0..7 in existing atlas convention, stages 0..2 (partial, middle, fully deployed). Packed uses previous idle. Match existing 30-tick deployment/packing state.
- For six combat roles (`riveter`, `bulwark`, `sounder`, `skipper`, `reedguard`, `loom`), `{role}_{face}_fire_{phase}`: 64×64, anchor (32,50), 0..2 phases, initial recoil then recovery. For `bulwark` and `loom`, also `{role}_{face}_deployed_fire_{phase}`. Use Shot events/cooldown and current deployment state. Local effects only.
- For `hook` and `wick`, `{role}_{face}_loaded` and `{role}_{face}_loaded_walk_{phase}` (0..3), `{role}_{face}_gather_{phase}` and `{role}_{face}_unload_{phase}` (0..2). 64×64 anchor (32,50). Cargo follows existing carried amount; gather/unload effects follow actual activity/deposit, not arbitrary continuous idle.
- `salvage_stage_0..3`, `salvage_turbine_stage_0..3`, `salvage_barge_stage_0..3`: 160×144 anchor (80,128), intact, stripped, skeleton, exhausted residue. Stage 0 copies existing drawing. Current Split Basin nodes start at 2400. Do not expose hidden changes; observation cache must freeze unseen stages and reset safely on start/load.
- `fx_impact_steel_0..3`, `fx_impact_reed_0..3`: 48×48 anchor (24,32), localized hit material based on target.
- `fx_wreck_steel_0..5`, `fx_wreck_reed_0..5`: 64×64 anchor (32,50); `wreck_union`, `wreck_assembly`: 64×64 anchor (32,50), quiet nonblocking short-lived residue created only for observed deaths.
- `landmark_tide_gauge`, `landmark_ferry_stairs`, `landmark_hull_ribs`: 160×144 anchor (80,128). Deterministic placement around existing boundary/approach regions, no simulation topology changes. Depth sort, dim/hide according to appropriate map visibility; do not obscure units or order targets.
- `hud_bottom_union`, `hud_bottom_assembly`: 640×72 anchor (0,0). `hud_top_union`, `hud_top_assembly`: 640×24 anchor (0,0). Shared control geometry, plain centers, faction edge detailing.

Root will supply readable 7×9 glyph drawings in `font7.rs` and editable Aseprite proof. Engineering may add rasterization/layout APIs; do not draw new font glyphs or art independently.

Engineering ownership:

- Rendering agent: `game.rs`, optionally new `presentation.rs`, `art_review.rs`, `tests.rs`, plus necessary module declarations in `main.rs`. State selection, gate world/minimap forecast, cargo/resource memory, combat, landmarks, fixtures/regressions. Coordinate console invocation with UI agent.
- UI agent: `canvas.rs` rasterization utilities (preserve original API), new `console.rs`. New UI functions with clear data inputs, no game.rs writes. Root owns `font7.rs` and Aseprite UI art. Coordinate with rendering agent.
- Root: all `tools/art_v8`, `art/`, root review/docs/package and final integration review. No agent may approve own contribution.

Expected verification: focused state and fog regressions, whole workspace tests/Clippy/fmt, saved-source/PNG equality and old-entry comparison, fixtures for every requested system and both factions, historical replay hashes, packaging equality and signature, performance smoke. Try native play; report actual lock/tool barriers without claiming live verification if unavailable.
