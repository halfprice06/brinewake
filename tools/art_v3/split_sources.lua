-- Create convenient editable native documents from the reviewed layered atlas.
-- This never reads a PNG or generated reference: every cel comes from Aseprite.
local root=assert(app.params.root,'root required')
local source=app.open(root..'/art/source/game-assets-v3.aseprite')
local names={'hook','riveter','bulwark','sounder','wick','skipper','reedguard','loom'}
local facings={'e','ne','n','nw','w','sw','s','se'}
local function layers_for(sprite)
  sprite:setPalette(Palette(source.palettes[1]))
  local result={}
  for i,layer in ipairs(source.layers) do
    local target=i==1 and sprite.layers[1] or sprite:newLayer()
    target.name=layer.name;result[i]=target
  end
  return result
end
local function copy_cell(target,layers,frame,x,y,w,h)
  if frame>1 then target:newEmptyFrame(frame) end
  target.frames[frame].duration=0.14
  for i,layer in ipairs(source.layers) do
    local cel=layer:cel(1)
    local crop=Image(cel.image,Rectangle(x,y,w,h))
    target:newCel(layers[i],frame,crop,Point(0,0))
  end
end
for i,name in ipairs(names) do
  local sprite=Sprite(64,64,ColorMode.RGB);local layers=layers_for(sprite)
  local bx=((i-1)%4)*512;local by=math.floor((i-1)/4)*320
  for face=0,7 do
    local frame=face*5+1
    copy_cell(sprite,layers,frame,bx+face*64,by,64,64)
    for phase=0,3 do copy_cell(sprite,layers,frame+phase+1,bx+face*64,by+(phase+1)*64,64,64) end
  end
  -- Create tags only after inserting all frames. Aseprite extends an existing
  -- end-of-timeline tag when new frames are inserted beside its end boundary.
  for face=0,7 do
    local frame=face*5+1
    sprite:newTag(frame,frame).name='idle_'..facings[face+1]
    sprite:newTag(frame+1,frame+4).name='walk_'..facings[face+1]
  end
  sprite:saveAs(root..'/art/source/v3/'..name..'.aseprite')
end
local structures={'union_hq','assembly_hq','union_works','assembly_works','dropoff','condenser','tower','gate','salvage','well'}
for i,name in ipairs(structures) do
  local sprite=Sprite(160,144,ColorMode.RGB);local layers=layers_for(sprite)
  copy_cell(sprite,layers,1,(i-1)*160,640,160,144)
  sprite:saveAs(root..'/art/source/v3/'..name..'.aseprite')
end
print('Saved eight 40-frame unit documents and ten structure documents, all with six native layers.')
