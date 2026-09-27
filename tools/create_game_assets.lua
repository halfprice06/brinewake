-- Export editable v4 sources to the active runtime atlas.
local root = assert(app.params.root, 'root parameter required')
dofile(root .. '/tools/art_v4/export_sources.lua')
