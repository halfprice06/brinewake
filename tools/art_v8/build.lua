-- Root authoring entry point for v8. Rebuilds only the new v8 documents.
-- Later native edits should use export_sources.lua, not this reconstruction.
local root=assert(app.params.root,'root required')
local painter=dofile(root..'/tools/art_v5/painter.lua')
local units=dofile(root..'/tools/art_v8/units.lua')
local env=dofile(root..'/tools/art_v8/environment.lua')
local fx=dofile(root..'/tools/art_v8/effects.lua')
local ui=dofile(root..'/tools/art_v8/interface.lua')
local function read_json(path)
  local f=assert(io.open(path));local j=json.decode(f:read('*a'));f:close();return j
end
local function save_json(path,value)
  local f=assert(io.open(path,'w'));f:write(json.encode(value));f:close()
end
local manifest=read_json(root..'/art/archive/art-v7/game-assets.json')
manifest.width=2048;manifest.height=4096
local baseline=app.open(root..'/art/archive/art-v7/game-assets.aseprite')
local atlas_images={}
for i,name in ipairs(painter.layers) do
  local im=Image(2048,4096,ColorMode.RGB);im:clear()
  local cel=baseline.layers[i]:cel(1)
  if cel then im:drawImage(cel.image,cel.position) end
  atlas_images[name]=im
end
local shelf_x,shelf_y,shelf_h=0,2048,0
local sources={};local count=0
local function insert(name,im,w,h,ax,ay,doc,frame,muzzle,duration)
  if shelf_x+w>2048 then shelf_x=0;shelf_y=shelf_y+shelf_h;shelf_h=0 end
  assert(shelf_y+h<=4096,'atlas out of room at '..name)
  assert(not manifest.sprites[name],'duplicate key '..name)
  local b={x=shelf_x,y=shelf_y,w=w,h=h,anchor_x=ax,anchor_y=ay}
  b.muzzle=muzzle
  manifest.sprites[name]=b
  for _,layer in ipairs(painter.layers) do atlas_images[layer]:drawImage(im[layer],Point(shelf_x,shelf_y)) end
  sources[#sources+1]={key=name,document=doc,frame=frame,w=w,h=h,anchor_x=ax,anchor_y=ay,muzzle=muzzle,duration_ms=math.floor(duration*1000+.5)}
  shelf_x=shelf_x+w;shelf_h=math.max(shelf_h,h);count=count+1
end
local bases={}
local function base(name)
  if not bases[name] then bases[name]=app.open(root..'/art/archive/art-v7/sources/'..name..'.aseprite') end
  return bases[name]
end
local function document(name,w,h,ax,ay,frames,draw,tags,base_name)
  local sprite=Sprite(w,h,ColorMode.RGB);painter.install_palette(sprite)
  for i,layer in ipairs(painter.layers) do local l=i==1 and sprite.layers[1] or sprite:newLayer();l.name=layer end
  for f,spec in ipairs(frames) do
    if f>1 then sprite:newEmptyFrame(f) end
    sprite.frames[f].duration=spec.duration or .1
    local ims={}
    for i,layer in ipairs(painter.layers) do
      local im=Image(w,h,ColorMode.RGB);im:clear()
      if base_name then
        local cel=base(base_name).layers[i]:cel(1)
        if cel then im:drawImage(cel.image,cel.position) end
      end
      ims[layer]=im
    end
    local c=painter.new(ims,{key=spec.key,x=0,y=0,w=w,h=h,anchor_x=ax,anchor_y=ay},spec.face or 0)
    draw(c,spec)
    for i,layer in ipairs(painter.layers) do sprite:newCel(sprite.layers[i],f,ims[layer],Point(0,0)) end
    insert(spec.key,ims,w,h,ax,ay,name,f,c.muzzle,sprite.frames[f].duration)
  end
  for _,t in ipairs(tags or {}) do sprite:newTag(t[2],t[3]).name=t[1] end
  sprite:saveAs(root..'/art/source/v8/'..name..'.aseprite')
  -- One preview per native source. Timeline inspection uses metadata below.
  Image(sprite):saveAs(root..'/output/art-v8/'..name..'-first.png')
  sprite:close()
end

for _,name in ipairs({'bulwark','loom'}) do
  local n=name;local frames,tags={},{}
  for face=0,7 do
    for stage=0,2 do frames[#frames+1]={key=n..'_'..face..'_deploy_'..stage,face=face,stage=stage,duration=stage==2 and .334 or .333} end
    tags[#tags+1]={'deploy_'..face,face*3+1,face*3+3}
  end
  document(n..'_deployment',64,64,32,50,frames,function(c,s)units.deploy(c,n,s.stage)end,tags)
end
for _,name in ipairs({'riveter','bulwark','sounder','skipper','reedguard','loom'}) do
  local n=name
  for mode=0,((n=='bulwark' or n=='loom') and 1 or 0) do
    local deployed=mode==1;local frames,tags={},{}
    for face=0,7 do
      for phase=0,2 do frames[#frames+1]={key=n..'_'..face..(deployed and '_deployed_fire_' or '_fire_')..phase,face=face,phase=phase,duration=({.067,.100,.133})[phase+1]} end
      tags[#tags+1]={'fire_'..face,face*3+1,face*3+3}
    end
    document(n..(deployed and '_deployed_firing' or '_firing'),64,64,32,50,frames,function(c,s)units.fire(c,n,s.phase,deployed)end,tags)
  end
end
for _,name in ipairs({'hook','wick'}) do
  local n=name
  for _,mode in ipairs({'loaded','loaded_walk','gather','unload'}) do
    local m=mode;local total=m=='loaded' and 1 or (m=='loaded_walk' and 4 or 3)
    local frames,tags={},{}
    for face=0,7 do
      for phase=0,total-1 do frames[#frames+1]={key=n..'_'..face..'_'..m..(total>1 and '_'..phase or ''),face=face,phase=phase,duration=m=='loaded_walk' and .14 or (m=='gather' and (phase==2 and .334 or .333) or .1)} end
      tags[#tags+1]={m..'_'..face,face*total+1,face*total+total}
    end
    document(n..'_'..m,64,64,32,50,frames,function(c,s)units.worker(c,n,m,s.phase)end,tags)
  end
end
-- Larger environmental source cells are grouped for economical packing.
for _,name in ipairs({'salvage','salvage_turbine','salvage_barge'}) do
  local n=name;local frames,tags={},{}
  for stage=0,3 do frames[#frames+1]={key=n..'_stage_'..stage,stage=stage,duration=1};tags[#tags+1]={({'intact','stripped','skeleton','exhausted'})[stage+1],stage+1,stage+1} end
  document(n..'_dismantling',160,144,80,128,frames,function(c,s)env.salvage(c,n,s.stage)end,tags,n)
end
for _,north in ipairs({true,false}) do
  local dry=north;local n=dry and 'gate_north_dry' or 'gate_south_dry'
  local frames={{key=n,phase=0,warning=false,duration=1}}
  for phase=0,3 do frames[#frames+1]={key=n..'_warning_'..phase,warning=true,phase=phase,duration=.2} end
  document(n,160,144,80,128,frames,function(c,s)env.gate(c,dry,s.warning,s.phase)end,{{'stable',1,1},{'warning',2,5}})
end
for _,name in ipairs({'tide_gauge','ferry_stairs','hull_ribs'}) do
  local n=name
  document('landmark_'..n,160,144,80,128,{{key='landmark_'..n,duration=1}},function(c)env.landmark(c,n)end,{{'landmark',1,1}})
end
for _,material in ipairs({'steel','reed'}) do
  local m=material
  local frames={};for phase=0,3 do frames[#frames+1]={key='fx_impact_'..m..'_'..phase,phase=phase,duration=.1} end
  document('impact_'..m,48,48,24,32,frames,function(c,s)fx.impact(c,m,s.phase)end,{{'impact',1,4}})
  frames={};for phase=0,5 do frames[#frames+1]={key='fx_wreck_'..m..'_'..phase,phase=phase,duration=phase%3==2 and .134 or .133} end
  document('breakdown_'..m,64,64,32,50,frames,function(c,s)fx.wreck(c,m,s.phase)end,{{'breakdown',1,6}})
end
for _,faction in ipairs({'union','assembly'}) do
  local f=faction
  document('wreck_'..f,64,64,32,50,{{key='wreck_'..f,duration=1}},function(c)fx.residue(c,f)end,{{'residue',1,1}})
end
for _,wet in ipairs({false,true}) do
  local w=wet;local n=w and 'terrain_lane_wet' or 'terrain_lane_dry';local frames={}
  for phase=0,3 do frames[#frames+1]={key=n..'_'..phase,phase=phase,duration=.8} end
  document(n,32,16,16,8,frames,function(c,s)env.lane(c,w,s.phase)end,{{w and 'shallow' or 'crossbars',1,4}})
end
local foam={};for phase=0,5 do foam[#foam+1]={key='fx_sluice_'..phase,phase=phase,duration=.2} end
document('sluice_foam',96,64,48,48,foam,function(c,s)env.sluice(c,s.phase)end,{{'switch_outflow',1,6}})
for _,faction in ipairs({'union','assembly'}) do
  local f=faction
  document('console_bottom_'..f,640,72,0,0,{{key='hud_bottom_'..f,duration=1}},function(c)ui.bottom(c,f)end,{{'console',1,1}})
  document('console_top_'..f,640,24,0,0,{{key='hud_top_'..f,duration=1}},function(c)ui.top(c,f)end,{{'header',1,1}})
end
local font=read_json(root..'/tools/art_v8/font7.json');local chars={}
for ch in pairs(font) do chars[#chars+1]=ch end;table.sort(chars)
save_json(root..'/tools/art_v8/font-map.json',chars)
document('console_type',80,50,0,0,{{key='console_font',duration=1}},function(c)
  c:layer(5)
  for i,ch in ipairs(chars) do
    local x=((i-1)%10)*8;local y=math.floor((i-1)/10)*10
    for yy,row in ipairs(font[ch]) do for xx=1,7 do
      if row:sub(xx,xx)=='1' then c:pixel(x+xx-1,y+yy-1,'ivory3') end
    end end
  end
end,{{'7x9_glyphs',1,1}})

local atlas=Sprite(2048,4096,ColorMode.RGB);painter.install_palette(atlas)
for i,layer in ipairs(painter.layers) do
  local l=i==1 and atlas.layers[1] or atlas:newLayer();l.name=layer
  atlas:newCel(l,1,atlas_images[layer],Point(0,0))
end
atlas:saveAs(root..'/art/source/game-assets-v8.aseprite')
Image(atlas):saveAs(root..'/art/exports/game-assets-v8.png')
save_json(root..'/art/exports/game-assets-v8.json',manifest)
save_json(root..'/tools/art_v8/sources.json',sources)
print('Root authored '..count..' new entries; packing ends at row '..(shelf_y+shelf_h)..' of 4096. Active assets await export_sources.lua.')
