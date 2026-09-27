-- Root-authored scripted pixel art. Reconstruct NEW v9 sources only.
-- Subsequent artist edits are exported with export_sources.lua, not this file.
local root=assert(app.params.root)
local painter=dofile(root..'/tools/art_v5/painter.lua')
local env=dofile(root..'/tools/art_v9/environment.lua')
local units=dofile(root..'/tools/art_v9/machines.lua')
local ui=dofile(root..'/tools/art_v9/interface.lua')
local function read(p)local f=assert(io.open(p));local s=f:read('*a');f:close();return s end
local function write(p,s)local f=assert(io.open(p,'w'));f:write(s);f:close()end
local base=json.decode(read(root..'/art/archive/art-v8/game-assets.json'))
local sources={};local manifest=base;manifest.height=6144
local sx,sy,sh,count=0,4096,0,0
local function layer_names(name)
 if name:match('^archaeology') then
  return {'ground contacts','buried edges','paving and channel beds','surface shade','loading rails and sleepers','silt salt and reflections'}
 elseif name:match('^coast') then
  return {'ground contacts','rear silhouettes','bank and rooted stems','body underpainting','animals and moving reed tips','edge accents'}
 elseif name:match('^pressure_') then
  return {'contact reference','rear pipes','mountings','valves and pressure sleeves','jets and valve linkages','weapon shutters'}
 elseif name:match('plate$') or name=='dock_telegraph' or name=='result_signal' then
  return {'ground contacts','rear bracket','instrument casing','glass shade','dial lamp and symbols','rivets and highlights'}
 end
 return painter.layers
end
local function document(name,w,h,ax,ay,frames,draw,tags)
 local sprite=Sprite(w,h,ColorMode.RGB);painter.install_palette(sprite)
 for i,n in ipairs(layer_names(name)) do local l=i==1 and sprite.layers[1] or sprite:newLayer();l.name=n end
 for f,s in ipairs(frames) do
  if f>1 then sprite:newEmptyFrame(f) end;sprite.frames[f].duration=s.duration or .2
  local ims={};for _,n in ipairs(painter.layers) do ims[n]=Image(w,h,ColorMode.RGB);ims[n]:clear() end
  local c=painter.new(ims,{key=s.key,x=0,y=0,w=w,h=h,anchor_x=ax,anchor_y=ay},s.face or 0)
  draw(c,s)
  for i,n in ipairs(painter.layers) do sprite:newCel(sprite.layers[i],f,ims[n],Point(0,0)) end
  if sx+w>2048 then sx=0;sy=sy+sh;sh=0 end
  assert(sy+h<=6144,'v9 atlas overflow');assert(not manifest.sprites[s.key],'duplicate '..s.key)
  manifest.sprites[s.key]={x=sx,y=sy,w=w,h=h,anchor_x=ax,anchor_y=ay,muzzle=c.muzzle}
  sources[#sources+1]={key=s.key,document=name,frame=f,w=w,h=h,anchor_x=ax,anchor_y=ay,duration_ms=math.floor(sprite.frames[f].duration*1000+.5)}
  sx=sx+w;sh=math.max(sh,h);count=count+1
 end
 for _,t in ipairs(tags or {}) do sprite:newTag(t[2],t[3]).name=t[1] end
 sprite:saveAs(root..'/art/source/v9/'..name..'.aseprite')
 local preview=Image(w,h,ColorMode.RGB);preview:drawSprite(sprite,1)
 preview:saveAs(root..'/output/art-v9/'..name..'-first.png');sprite:close()
end
for _,lane in ipairs({'north','south'}) do
 local l=lane;local frames={}
 for _,wet in ipairs({false,true}) do for phase=0,3 do
  frames[#frames+1]={key='archaeology_'..l..(wet and '_wet_' or '_dry_')..phase,phase=phase,wet=wet,duration=.4}
 end end
 document('archaeology_'..l,160,80,80,40,frames,function(c,s)env.archaeology(c,l,s.wet,s.phase)end,{{'drained',1,4},{'flooded',5,8}})
end
for _,name in ipairs({'riveter','bulwark','sounder','skipper','reedguard','loom'}) do
 local n=name;local frames,tags={},{}
 for face=0,7 do
  local start=#frames+1
  for phase=0,3 do frames[#frames+1]={key='pressure_'..n..'_'..face..'_surge_'..phase,phase=phase,face=face,cool=false,duration=.1} end
  tags[#tags+1]={'surge_'..face,start,#frames}
  start=#frames+1
  for phase=0,2 do frames[#frames+1]={key='pressure_'..n..'_'..face..'_cool_'..phase,phase=phase,face=face,cool=true,duration=.2} end
  tags[#tags+1]={'cool_'..face,start,#frames}
 end
 document('pressure_'..n,64,64,32,50,frames,function(c,s)units.pressure(c,n,s.phase,s.cool)end,tags)
end
local frames,tags={},{}
for face=0,7 do
 for phase=0,5 do frames[#frames+1]={key='loom_'..face..'_pressure_'..phase,face=face,phase=phase,duration=({.133,.134,.100,.133,.133,.167})[phase+1]} end
 tags[#tags+1]={'committed_shot_'..face,face*6+1,face*6+6}
end
document('loom_pressure',64,64,32,50,frames,function(c,s)units.loom(c,s.phase)end,tags)
for _,name in ipairs({'hook','wick'}) do
 local n=name;local frames,tags={},{}
 for face=0,7 do
  for phase=0,3 do frames[#frames+1]={key='doctrine_'..n..'_'..face..'_hauling_'..phase,face=face,phase=phase,duration=.14} end
  tags[#tags+1]={'cargo_'..face,face*4+1,face*4+4}
 end
 document(n..'_cargo_cradle',64,64,32,50,frames,function(c,s)units.cargo(c,n,s.phase)end,tags)
end
for _,faction in ipairs({'union','assembly'}) do for _,doctrine in ipairs({'hauling','fire_control'}) do
 local f,d=faction,doctrine;local frames={}
 for phase=0,3 do frames[#frames+1]={key='doctrine_'..f..'_'..d..'_hq_'..phase,phase=phase,duration=.2} end
 document(f..'_'..d..'_workshop',160,144,80,128,frames,function(c,s)units.workshop(c,f,d,s.phase)end,{{'working',1,4}})
 document(f..'_'..d..'_plate',32,24,0,0,{{key='doctrine_'..f..'_'..d,duration=1}},function(c)ui.doctrine(c,f,d)end,{{'plate',1,1}})
end end
for _,kind in ipairs({'birds','crab','reeds'}) do
 local k=kind;local frames={}
 for phase=0,7 do frames[#frames+1]={key='coast_'..k..'_'..phase,phase=phase,duration=.2} end
 document('coast_'..k,96,64,48,52,frames,function(c,s)env.coast(c,k,s.phase)end,{{'coast',1,8}})
end
frames={};for _,s in ipairs({'idle','pending','accepted','rejected'}) do frames[#frames+1]={key='dock_telegraph_'..s,state=s,duration=.25} end
document('dock_telegraph',48,24,0,0,frames,function(c,s)ui.telegraph(c,s.state)end,{{'states',1,4}})
frames={};for p=0,3 do frames[#frames+1]={key='result_signal_'..p,phase=p,duration=.3} end
document('result_signal',32,48,16,44,frames,function(c,s)ui.signal(c,s.phase)end,{{'harbor_signal',1,4}})
write(root..'/tools/art_v9/sources.json',json.encode(sources))
write(root..'/art/exports/game-assets-v9.json',json.encode(manifest))
print('Root authored '..count..' new entries; final atlas row '..(sy+sh)..' of6144. Run export_sources.lua to integrate.')
