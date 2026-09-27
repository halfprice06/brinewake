local root=assert(app.params.root,'root parameter required')
local painter=dofile(root..'/tools/art_v4/painter.lua')
local units=dofile(root..'/tools/art_v4/units.lua')
local finish=dofile(root..'/tools/art_v4/finish.lua')
local width,height=2048,1536
local sprite=Sprite(width,height,ColorMode.RGB)
painter.install_palette(sprite)
local images={}
for i,name in ipairs(painter.layers) do
  local layer=i==1 and sprite.layers[1] or sprite:newLayer()
  layer.name=name
  local im=Image(width,height,ColorMode.RGB);im:clear()
  images[name]=sprite:newCel(layer,1,im,Point(0,0)).image
end
local entries={}
local function draw(key,x,y,w,h,ax,ay,fn,face,phase)
  local bounds={key=key,x=x,y=y,w=w,h=h,anchor_x=ax,anchor_y=ay}
  local context=painter.new(images,bounds,face)
  fn(context,phase)
  local detail=finish[key] or finish[key:match('^([a-z]+)_%d')]
  if detail then detail(context,phase) end
  entries[key]=bounds
end
local names={'hook','riveter','bulwark','sounder','wick','skipper','reedguard','loom'}
for i,name in ipairs(names) do
  local bx=((i-1)%4)*512;local by=math.floor((i-1)/4)*320
  for face=0,7 do
    draw(name..'_'..face,bx+face*64,by,64,64,32,50,units[name],face,nil)
    if face==0 then entries[name]=entries[name..'_0'] end
    for phase=0,3 do
      draw(name..'_'..face..'_walk_'..phase,bx+face*64,by+(phase+1)*64,64,64,32,50,units[name],face,phase)
    end
  end
end
local file=loadfile(root..'/tools/art_v4/structures.lua')
if file then
  local structures=file()
  local keys={'union_hq','assembly_hq','union_works','assembly_works','dropoff','condenser','tower','gate','salvage','well'}
  for i,name in ipairs(keys) do
    if structures[name] then draw(name,(i-1)*160,640,160,144,80,128,structures[name],1,nil) end
  end
end
local environment=loadfile(root..'/tools/art_v4/environment.lua')
if environment then
  local env=environment();local keys={}
  for name in pairs(env) do keys[#keys+1]=name end
  table.sort(keys)
  for i,name in ipairs(keys) do
    local tile=name:find('terrain_',1,true)==1
    local shore=name:find('shore_',1,true)==1
    local small=tile or shore
    draw(name,((i-1)%24)*80,800+math.floor((i-1)/24)*80,small and 40 or 64,shore and 32 or (tile and 24 or 64),small and 20 or 32,small and 12 or 50,env[name],0,nil)
  end
end
local interface=dofile(root..'/tools/art_v4/interface.lua')
draw('hud_bottom',0,944,640,72,0,0,interface.hud_bottom,0,nil)
draw('hud_top',640,944,640,24,0,0,interface.hud_top,0,nil)
local portrait_keys={}
for name in pairs(interface) do if name:match('^portrait_') then portrait_keys[#portrait_keys+1]=name end end
table.sort(portrait_keys)
for i,name in ipairs(portrait_keys) do draw(name,(i-1)*64,1040,56,56,0,0,interface[name],0,nil) end
for i,name in ipairs({'icon_salvage','icon_pressure','icon_crew'}) do draw(name,704+(i-1)*24,1040,16,16,0,0,interface[name],0,nil) end
local effects=dofile(root..'/tools/art_v4/effects.lua')
local effect_keys={};for key in pairs(effects) do effect_keys[#effect_keys+1]=key end;table.sort(effect_keys)
for i,key in ipairs(effect_keys) do
  local wreck=key:find('fx_wreck_',1,true)==1
  draw(key,((i-1)%16)*64,1136+math.floor((i-1)/16)*64,wreck and 48 or 32,wreck and 48 or 32,wreck and 24 or 16,wreck and 36 or 16,effects[key],0,nil)
end
sprite:saveAs(root..'/art/source/game-assets-v4.aseprite')
Image(sprite):saveAs(root..'/art/exports/game-assets-v4.png')
local keys={};for key in pairs(entries) do keys[#keys+1]=key end;table.sort(keys)
local manifest=assert(io.open(root..'/art/exports/game-assets-v4.json','w'))
manifest:write('{"width":',width,',"height":',height,',"sprites":{\n')
for i,key in ipairs(keys) do
  local b=entries[key]
  manifest:write(string.format('%q:{"x":%d,"y":%d,"w":%d,"h":%d,"anchor_x":%d,"anchor_y":%d}',key,b.x,b.y,b.w,b.h,b.anchor_x,b.anchor_y))
  manifest:write(i<#keys and ',\n' or '\n')
end
manifest:write('}}\n');manifest:close()
print('Root art v4: '..#keys..' entries in six semantic layers')
