# Coherent chart-and-craft art

Root personally reworked every v10 art family in response to the owner's
coherence request and their photo feedback: the old Bulwark diagram was
confusing. These sources replace the 56 v10 atlas entries in place. V10's
sources and exports remain preserved.

- **Manual comparisons:** complete mobile and deployed machines appear side
  by side. These reuse the actual earlier root-authored in-game source layers
  at exact 2x, keeping the same silhouette, lighting and attachments. Each
  source has 14 editable layers: comparison ground/divider, six layers for
  each machine, and state headings. Component cutaways and RAM/JACK/RIG labels
  have been removed. Native size stays 224x144.
- **Faction identity:** the Union mark suggests a gantry and hook; Assembly's
  arch encloses a pressure chamber. Emblems, small badges, headquarters marks
  and nameplates share enamel edge weight, corner treatment and upper-left
  light. Small sizes have simplified symbols rather than resampled detail.
- **Water:** grouped surface shapes, quiet gaps, compatible seam fragments and
  small overlapping glints replace the uniformly thin wave rows. Two muted
  water shades reduce the contrast of paired dark/light outlines. All three
  128x64 variants retain eight 200ms frames.
- **Traces:** imprints, metal rivets and torn bindings keep distinct material
  shapes. Their wash crests use the water's revised tones; the final remnant
  is a few neutral silt strokes rather than a solid blue puddle. All three
  32x16 families retain six frames and their existing ground anchors.

The active atlas remains 2048x7168 and 1,500 entries. Every rectangle and anchor
is unchanged, every one of the 56 revised entries has different pixels, and
the other 1,444 entries remain exact. It now uses 63 opaque colors: the earlier
61 plus two subdued water midtones. The game remains 640x360 with integer scaling.

Root used Aseprite Lua construction and source-layer recomposition, followed
by visual review and revisions. This is scripted pixel art; no subagent or
image-generation service authored this pass. No external reference pixels were
imported. The user photo supplied the clarity problem, not production artwork.

`tools/art_v11/build.lua` reconstructs these sources and should not be run
over later manual edits. Use `export_sources.lua` to export saved sources;
`verify_sources.lua` reopens each file and compares its full RGBA composite,
layer names, timing and anchors against the active atlas. The exporter copies
transparent pixels explicitly so old artwork cannot remain under a redraw.

See `output/art-v11/` for the native comparisons, integer enlargements, phase
sheets, animation previews, engine views and verification. The Mac remained
locked, so actual-speed perceptual playback and native hands-on review are
still unavailable. Numeric checks do not establish owner approval or clarity.
