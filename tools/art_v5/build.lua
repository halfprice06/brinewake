-- Reconstruct only the v5 landscape drawings over the preserved live v4 atlas.
-- Normal later artist edits should use export_sources.lua, never this builder.
local root=assert(app.params.root,'root required')
local painter=dofile(root..'/tools/art_v5/painter.lua')
local drawings=dofile(root..'/tools/art_v5/landscape.lua')
local file=assert(io.open(root..'/art/archive/art-v4/game-assets.json','r'))
local manifest=json.decode(file:read('*a'));file:close()
local atlas=app.open(root..'/art/archive/art-v4/game-assets.aseprite')
local images={}
for i,layer in ipairs(atlas.layers) do
  assert(layer.name==painter.layers[i],'layer contract changed')
  -- Normalize storage to full canvas without changing any composited pixels.
  local im=Image(atlas.width,atlas.height,ColorMode.RGB);im:clear()
  local cel=layer:cel(1)
  if cel then im:drawImage(cel.image,cel.position);atlas:deleteCel(cel) end
  images[layer.name]=atlas:newCel(layer,1,im,Point(0,0)).image
end
local function add(key,x,y,w,h,ax,ay)
  assert(not manifest.sprites[key],key..' already exists')
  manifest.sprites[key]={x=x,y=y,w=w,h=h,anchor_x=ax,anchor_y=ay}
end
add('salvage_turbine',0,1264,160,144,80,128)
add('salvage_barge',160,1264,160,144,80,128)
add('salt_rock_1',320,1264,64,64,32,50)
add('salt_rock_2',384,1264,64,64,32,50)
for i=0,2 do add('salt_crust_'..i,i*96,1424,96,64,48,32) end
local keys={};for key in pairs(drawings) do keys[#keys+1]=key end;table.sort(keys)
for _,key in ipairs(keys) do
  local entry=assert(manifest.sprites[key],key..' has no atlas rectangle')
  local b={key=key}
  for _,field in ipairs({'x','y','w','h','anchor_x','anchor_y'}) do b[field]=entry[field] end
  for _,im in pairs(images) do im:clear(Rectangle(b.x,b.y,b.w,b.h),0) end
  drawings[key](painter.new(images,b,0))
end
atlas:saveAs(root..'/art/source/game-assets-v5.aseprite')
Image(atlas):saveAs(root..'/art/exports/game-assets-v5.png')
file=assert(io.open(root..'/art/exports/game-assets-v5.json','w'))
file:write(json.encode(manifest));file:close()

local function editable(name,rects,w,h,tags)
  local sprite=Sprite(w,h,ColorMode.RGB);sprite:setPalette(Palette(atlas.palettes[1]))
  local layers={}
  for i,layer in ipairs(atlas.layers) do
    layers[i]=i==1 and sprite.layers[1] or sprite:newLayer();layers[i].name=layer.name
  end
  for f,b in ipairs(rects) do
    if f>1 then sprite:newEmptyFrame(f) end
    sprite.frames[f].duration=0.18
    for i,layer in ipairs(atlas.layers) do
      local crop=Image(layer:cel(1).image,Rectangle(b.x,b.y,b.w,b.h))
      sprite:newCel(layers[i],f,crop,Point(0,0))
    end
  end
  if tags then for i,name in ipairs(tags) do sprite:newTag(i,i).name=name end end
  sprite:saveAs(root..'/art/source/v5/'..name..'.aseprite')
end
for _,key in ipairs({'salvage','salvage_turbine','salvage_barge'}) do
  editable(key,{manifest.sprites[key]},160,144)
end
local ground_keys={}
for _,family in ipairs({'salt','silt','causeway','water','shallow'}) do
  for i=0,3 do ground_keys[#ground_keys+1]='terrain_'..family..'_'..i end
end
local rects={};for _,key in ipairs(ground_keys) do rects[#rects+1]=manifest.sprites[key] end
editable('terrain',rects,40,24,ground_keys)
rects={};local props={'salt_rock','salt_rock_1','salt_rock_2','reed_clump'}
for _,key in ipairs(props) do rects[#rects+1]=manifest.sprites[key] end
editable('coastal_props',rects,64,64,props)
rects={};local crusts={}
for i=0,2 do local key='salt_crust_'..i;rects[#rects+1]=manifest.sprites[key];crusts[#crusts+1]=key end
editable('salt_crust',rects,96,64,crusts)
print('Root v5 landscape: '..#keys..' authored entries, native sources saved.')
