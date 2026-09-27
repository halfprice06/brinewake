-- Art v22: the three-arm sluice station's editable Aseprite document.
--
-- Scripted pixel art (see station3.py and art/source/v22/README.md).
-- Run with a headless Aseprite from the project root:
--
--   aseprite --batch --script-param mode=layers --script-param root=$PWD \
--     --script-param out=<dir> --script tools/art_v22/station3.lua
--       writes the v13 lock's rest-frame layers as
--       <dir>/{north,south}_{rear,hulls,cabins,working}.png
--   aseprite --batch --script-param mode=build --script-param root=$PWD \
--     --script-param out=<dir> --script tools/art_v22/station3.lua
--       reads <dir>/layers, <dir>/pieces and <dir>/pieces.json (written by
--       station3.py), saves art/source/v22/station3.aseprite, reopens it and
--       exports every frame the atlas takes to <dir>/export/<key>.png
--   aseprite --batch --script-param mode=export ... re-exports a saved,
--       hand-edited document the same way without rebuilding it.

local root = app.params.root
local out = app.params.out
local mode = app.params.mode
assert(root and out and mode, "root, out and mode are required")

local V13 = root .. "/art/source/v13/"
local DOC = root .. "/art/source/v22/station3.aseprite"

local V13_LAYERS = {
  ["rear mechanisms"] = "rear",
  ["hulls"] = "hulls",
  ["cabins and shells"] = "cabins",
  ["working parts"] = "working",
}

-- The base's layers, bottom to top, and their files from station3.py.
local BASE_LAYERS = {
  { "ground shadows forbidden - keep empty", nil },
  { "rear mechanisms", "rear" },
  { "hulls", "hulls" },
  { "cabins and shells", "cabins" },
  { "front shaft (S arm)", "front_shaft" },
  { "mast, bar, pulleys and wheel rim", "mast" },
}

-- One layer per piece family; each piece is a frame with a cel on its
-- family's layer only.
local FAMILY_LAYERS = {
  float_side = "floats: W and E shafts (drawn at W, E is 46 px right)",
  float_front = "floats: S shaft",
  flag = "semaphore",
  wheel = "handwheel spokes and pointer",
  pointer = "warning pointer (drawn at W's deep row)",
}
local FAMILY_ORDER = { "float_side", "float_front", "flag", "wheel", "pointer" }

local function read(path)
  local f = assert(io.open(path, "rb"))
  local s = f:read("a")
  f:close()
  return s
end

local function layer_png(sprite, name, path)
  for _, layer in ipairs(sprite.layers) do
    if layer.name == name then
      local image = Image(sprite.width, sprite.height, sprite.colorMode)
      local cel = layer:cel(1)
      if cel then
        image:drawImage(cel.image, cel.position)
      end
      image:saveAs(path)
      return
    end
  end
  error("missing layer " .. name)
end

if mode == "layers" then
  for _, side in ipairs({ "north", "south" }) do
    local sprite = app.open(V13 .. "gate_" .. side .. "_dry.aseprite")
    assert(sprite.width == 160 and sprite.height == 144, "v13 gate size")
    for name, file in pairs(V13_LAYERS) do
      layer_png(sprite, name, out .. "/" .. side .. "_" .. file .. ".png")
    end
    sprite:close()
  end
  return
end

local function export(sprite, pieces)
  -- Frame 1 is the base: every base layer.  A piece frame shows its
  -- family's layer only.
  local function composite(frame, only)
    local image = Image(sprite.width, sprite.height, sprite.colorMode)
    for _, layer in ipairs(sprite.layers) do
      if (only == nil and layer.data ~= "piece") or layer.name == only then
        local cel = layer:cel(frame)
        if cel then
          image:drawImage(cel.image, cel.position, cel.opacity, BlendMode.NORMAL)
        end
      end
    end
    return image
  end
  composite(1, nil):saveAs(out .. "/export/station3.png")
  for i, p in ipairs(pieces) do
    local frame = i + 1
    composite(frame, FAMILY_LAYERS[p.family]):saveAs(out .. "/export/" .. p.key .. ".png")
  end
end

local pieces = json.decode(read(out .. "/pieces.json"))
os.execute('mkdir -p "' .. out .. '/export" "' .. root .. '/art/source/v22"')

if mode == "export" then
  local sprite = app.open(DOC)
  export(sprite, pieces)
  return
end

assert(mode == "build", "mode is layers, build or export")
local sprite = Sprite(160, 144, ColorMode.RGB)
sprite.filename = DOC
-- The first frame is the base; each piece adds a frame after it.
for _ = 1, #pieces do
  sprite:newEmptyFrame()
end
for _, frame in ipairs(sprite.frames) do
  frame.duration = 0.166
end
local first = sprite.layers[1]
first.name = BASE_LAYERS[1][1]
for i = 2, #BASE_LAYERS do
  local name, file = BASE_LAYERS[i][1], BASE_LAYERS[i][2]
  local layer = sprite:newLayer()
  layer.name = name
  layer.data = "base"
  sprite:newCel(layer, 1, Image { fromFile = out .. "/layers/" .. file .. ".png" }, Point(0, 0))
end
first.data = "base"
local family_layer = {}
for _, family in ipairs(FAMILY_ORDER) do
  local layer = sprite:newLayer()
  layer.name = FAMILY_LAYERS[family]
  layer.data = "piece"
  family_layer[family] = layer
end
local ranges = {}
for i, p in ipairs(pieces) do
  local frame = i + 1
  local image = Image { fromFile = out .. "/" .. p.file }
  sprite:newCel(family_layer[p.family], frame, image, Point(0, 0))
  local r = ranges[p.family] or { first = frame }
  r.last = frame
  ranges[p.family] = r
end
local base_tag = sprite:newTag(1, 1)
base_tag.name = "base"
for _, family in ipairs(FAMILY_ORDER) do
  local r = ranges[family]
  local tag = sprite:newTag(r.first, r.last)
  tag.name = family
end
sprite:saveAs(DOC)
sprite:close()

-- Reopen the saved document and export from it, so the atlas is made from
-- what was saved.
local saved = app.open(DOC)
assert(#saved.frames == #pieces + 1, "frame count after save")
export(saved, pieces)
saved:close()
