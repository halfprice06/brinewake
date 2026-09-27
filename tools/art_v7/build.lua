-- Root's authoring/recovery entry point. Later native edits use export_sources.
local root=assert(app.params.root,'root required')
local painter=dofile(root..'/tools/art_v5/painter.lua')
local activity=dofile(root..'/tools/art_v7/activity.lua')
local construction=dofile(root..'/tools/art_v7/construction.lua')
local activities={'union_hq','assembly_hq','union_works','assembly_works','condenser'}
local sites={'union_works','assembly_works','dropoff','condenser','tower'}
local manifest_file=assert(io.open(root..'/art/archive/art-v6/game-assets.json','r'))
local manifest=json.decode(manifest_file:read('*a'));manifest_file:close()
manifest.height=2048
local old=app.open(root..'/art/archive/art-v6/game-assets.aseprite')
local atlas=Sprite(2048,2048,ColorMode.RGB);atlas:setPalette(Palette(old.palettes[1]))
for i,layer in ipairs(old.layers) do
  local target=i==1 and atlas.layers[1] or atlas:newLayer();target.name=layer.name
  local im=Image(2048,2048,ColorMode.RGB);im:clear()
  local cel=layer:cel(1);if cel then im:drawImage(cel.image,cel.position) end
  atlas:newCel(target,1,im,Point(0,0))
end
local count=0
local function insert(sprite,frame,key)
  local x=(count%12)*160;local y=1536+math.floor(count/12)*144;count=count+1
  assert(not manifest.sprites[key],key..' already exists')
  manifest.sprites[key]={x=x,y=y,w=160,h=144,anchor_x=80,anchor_y=128}
  for i,layer in ipairs(sprite.layers) do
    local cel=layer:cel(frame)
    if cel then atlas.layers[i]:cel(1).image:drawImage(cel.image,Point(x+cel.position.x,y+cel.position.y)) end
  end
end
local function document(name,mode,n)
  local base=app.open(root..'/art/archive/art-v6/sources/'..name..'.aseprite')
  assert(base.width==160 and base.height==144 and #base.frames==1 and #base.layers==6)
  local sprite=Sprite(160,144,ColorMode.RGB);sprite:setPalette(Palette(base.palettes[1]))
  for i,layer in ipairs(base.layers) do
    local target=i==1 and sprite.layers[1] or sprite:newLayer();target.name=layer.name
  end
  for f=1,n do
    if f>1 then sprite:newEmptyFrame(f) end
    sprite.frames[f].duration=mode=='activity' and 0.2 or 1
    local images={}
    for i,layer in ipairs(base.layers) do
      local im=Image(160,144,ColorMode.RGB);im:clear()
      local cel=layer:cel(1)
      if cel and (mode=='activity' or f==3 or i==1) then im:drawImage(cel.image,cel.position) end
      images[layer.name]=sprite:newCel(sprite.layers[i],f,im,Point(0,0)).image
    end
    local c=painter.new(images,{key=name..'_'..mode..f,x=0,y=0,w=160,h=144,anchor_x=80,anchor_y=128},0)
    if mode=='activity' then activity[name](c,f-1) else construction.paint(c,name,f-1) end
    insert(sprite,f,name..(mode=='activity' and '_active_' or '_build_')..(f-1))
  end
  if mode=='activity' then sprite:newTag(1,n).name='active'
  else
    for f,tag in ipairs({'foundation','frame','fit_out'}) do sprite:newTag(f,f).name=tag end
  end
  sprite:saveAs(root..'/art/source/v7/'..name..'_'..mode..'.aseprite')
end
for _,name in ipairs(activities) do document(name,'activity',4) end
for _,name in ipairs(sites) do document(name,'construction',3) end
atlas:saveAs(root..'/art/source/game-assets-v7.aseprite')
Image(atlas):saveAs(root..'/art/exports/game-assets-v7.png')
manifest_file=assert(io.open(root..'/art/exports/game-assets-v7.json','w'))
manifest_file:write(json.encode(manifest));manifest_file:close()
print('Root authored '..count..' building-state keys in ten editable native documents.')
