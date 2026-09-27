-- Root authoring entry point for the v19 machines. Re-authors, in place
-- (same keys, same rectangles, same anchors):
--   * the walk of all sixteen machines, drawn at the engine's ground speed
--     (originals.lua, roles.lua, motion.lua), and the loaded walks of the
--     Hook, the Wick and the Dredger;
--   * every state of the eight original machines, recoloured to its colour
--     budget (common.lua `folds`): rest, fire, deployment, worker poses, the
--     Loom's pressure strip, their pressure and cargo-cradle overlays and
--     their damage overlays;
--   * the idle acting of the eight original machines, rebuilt with the v18
--     acting: a vent puff that forms as one cluster, rises and drifts on the
--     Union machines, the pressure glow inside the budget on the Assembly's.
-- Writes editable documents under art/source/v19/, review PNGs under
-- output/art-v19/machines/, the registry tools/art_v19/sources-machines.json
-- and the v19 manifest. `only=<name>` builds one machine's documents for
-- review without touching the registry or the manifest.
local root=assert(app.params.root,'root required')
local C=dofile(root..'/tools/art_v19/common.lua')
local painter=C.painter
local originals=dofile(root..'/tools/art_v19/originals.lua')
local finish4=dofile(root..'/tools/art_v4/finish.lua')
local roles=dofile(root..'/tools/art_v19/roles.lua')
local poses=dofile(root..'/tools/art_v19/poses.lua')
local m,packer=C.manifest()
local entries={}
local out=root..'/output/art-v19/machines';os.execute('mkdir -p '..out)
local layers=painter.layers
local A=app.pixelColor.rgbaA
local W,H,AX,AY=64,64,32,50
local only=app.params.only
local report={}
local function note(s) report[#report+1]=s end

local function blank() local im=Image(W,H,ColorMode.RGB);im:clear();return im end
local function paint(draw,face,remap)
  local ims={}
  for _,layer in ipairs(layers) do ims[layer]=blank() end
  local c=painter.new(ims,{key='frame',x=0,y=0,w=W,h=H,anchor_x=AX,anchor_y=AY,remap=remap},face)
  draw(c)
  c.dz=0
  local list={};for i,layer in ipairs(layers) do list[i]=ims[layer] end
  return list
end
local function grounded(list,oval)
  local bodies={};for i=2,#list do bodies[#bodies+1]=list[i] end
  local o={C.grounded_shadow(bodies,oval or list[1],W,H)}
  for i=2,#list do o[i]=list[i] end
  return o
end
local function folded(list,map)
  local o={};for i,im in ipairs(list) do o[i]=C.recolour(im,map) end;return o
end
local function composite(list,from)
  local im=blank();for i=from or 1,#list do im:drawImage(list[i],Point(0,0)) end;return im
end
local function differing(a,b)
  local n=0;for y=0,H-1 do for x=0,W-1 do if a:getPixel(x,y)~=b:getPixel(x,y) then n=n+1 end end end;return n
end
-- The frame of a document as a list of full-canvas layer images, checked
-- against the archived v18 atlas so every source used is the shipped one.
local function doc_frame(s,f,key)
  local list={};for i=1,#s.layers do list[i]=C.full(s.layers[i],f,W,H) end
  local shipped=C.sprite_image(m,key)
  assert(composite(list):isEqual(shipped),key..': source document differs from the v18 atlas')
  return list
end

-- Idle acting (v18 build_machines.lua `idle_frame`, with the v19 budget).
local function cabin_top(im)
  for y=0,im.height-1 do local xs={}
   for x=0,im.width-1 do if A(im:getPixel(x,y))>0 then xs[#xs+1]=x end end
   if #xs>0 then return xs[math.ceil(#xs*0.7)],y end
  end
end
-- The Assembly glow runs on the drawing's own amber roles before the fold,
-- so rust folded into amber never glows, and peaks at ivory3, the palest
-- warm colour inside the budget (v15 peaked at amber4, which the budget
-- folds away).
local function pixmap(t) local o={};for a,b in pairs(t) do o[C.rgba[a]]=C.rgba[b] end;return o end
local glow1=pixmap{amber1='amber2',amber2='amber3'}
local glow2=pixmap{amber1='amber2',amber2='amber3',amber3='ivory3',amber4='ivory3'}
-- Where each original Union machine vents: the mouth of the stack v4 built
-- on it (hook: the engine stack on the rear deck; riveter: the boiler
-- cylinder behind the cabin; bulwark: the roof vent; sounder: the rear
-- stack). v18 put the puff on the highest row of the cabin layer, which on
-- the Hook is the crane jib, so the puff hung off the hook.
local VENT={hook={-11,4,18},riveter={-4,-3,30},bulwark={-4,0,30},sounder={-10,3,17}}
local function idle_frame(base,oval,kind,k,mouth)
  local images={}
  images[1]=blank()
  if kind=='union' then
    local dip=(k==0) and 1 or 0
    images[2]=Image(base[2]);images[3]=Image(base[3])
    for i=4,6 do images[i]=C.shifted(base[i],0,dip) end
    local body=composite(images,2)
    local function free(x,y) return x>=0 and y>=0 and x<W and y<H and A(body:getPixel(x,y))==0 end
    local cx,cy
    if mouth and mouth.cx then cx,cy=mouth.cx,mouth.cy
    elseif mouth then
      -- The fresh puff sits over the stack's mouth, whole, with nothing of
      -- the machine through it. A stack behind the hull or the crane is
      -- first seen where its puff clears the silhouette: the search rises
      -- from the mouth and may lean two pixels either way. The risen puff
      -- of the next frame starts from the same place.
      local cluster={{1,-2},{2,-2},{3,-2},{2,-1},{3,-1},{2,-3}}
      for lift=0,16 do
        for _,dx in ipairs({0,-1,1,-2,2}) do
          local x,y=mouth[1]-2+dx,mouth[2]+dip-lift;local ok=true
          for _,o in ipairs(cluster) do if not free(x+o[1],y+o[2]) then ok=false;break end end
          if ok then cx,cy=x,y;break end
        end
        if cx then break end
      end
      mouth.cx,mouth.cy=cx,cy
    end
    if not cx then cx,cy=cabin_top(base[4]) end
    if cx then
      local finish=images[6]
      local function puff(x,y,col) if free(x,y) then finish:drawPixel(x,y,C.rgba[col]) end end
      if k==0 then
        -- A fresh puff: dense, one cluster at the mouth, lit on top.
        puff(cx+1,cy-2,'steel3');puff(cx+2,cy-2,'steel3');puff(cx+3,cy-2,'steel3')
        puff(cx+2,cy-1,'steel2');puff(cx+3,cy-1,'steel2')
        puff(cx+2,cy-3,'steel4')
      else
        -- Risen and thinning: wider, paler, leaning right, one piece
        -- breaking away downwind.
        for x=cx+2,cx+6 do puff(x,cy-5,'steel3') end
        for x=cx+3,cx+5 do puff(x,cy-6,'steel3') end
        puff(cx+4,cy-4,'steel2');puff(cx+3,cy-4,'steel2')
        puff(cx+7,cy-7,'steel2');puff(cx+8,cy-7,'steel2')
      end
    end
  else
    images[2]=Image(base[2]);images[3]=Image(base[3])
    images[4]=C.recolour(base[4],(k==0) and glow1 or glow2)
    images[5]=(k==0) and C.shifted(C.recolour(base[5],glow1),0,-1) or C.recolour(base[5],glow2)
    images[6]=Image(base[6])
  end
  return grounded(images,oval)
end

local ORIGINALS={'hook','riveter','bulwark','sounder','wick','skipper','reedguard','loom'}
local ROLES={
 {name='tidewatch',kind='union'},{name='caulker',kind='union'},{name='caisson',kind='union'},
 {name='lampwright',kind='assembly'},{name='tender',kind='assembly'},{name='dredger',kind='assembly'},
 {name='barge',kind='assembly'},{name='lifter',kind='air'},
}
local total=0

-- 1. The eight original machines: rest recoloured, walk redrawn, idle rebuilt.
for _,n in ipairs(ORIGINALS) do if not only or only==n then
  local fold=C.fold_pixels(C.folds[C.faction[n]])
  local src=assert(app.open(root..'/art/source/v15/'..n..'.aseprite'))
  local frames,tags,idle={},{},{}
  local rest_diff,maxc=0,0
  for face=0,7 do
    local key=n..'_'..face
    local base=doc_frame(src,face*5+1,key)
    local drawn=paint(function(c) originals[n](c,nil);finish4[n](c) end,face)
    for i=2,6 do rest_diff=rest_diff+differing(drawn[i],base[i]) end
    local oval=drawn[1]
    local fbase=folded(base,fold)
    frames[#frames+1]={key=key,duration=1,images=fbase}
    local cnt=C.colours({fbase[2],fbase[3],fbase[4],fbase[5],fbase[6]},true);if cnt>maxc then maxc=cnt end
    for phase=0,3 do
      local walk=paint(function(c) originals[n](c,phase);finish4[n](c) end,face)
      frames[#frames+1]={key=key..'_walk_'..phase,duration=C.phase_ms(n,face)/1000,images=folded(grounded(walk,oval),fold)}
    end
    tags[#tags+1]={'face_'..face,face*5+1,face*5+5}
    local mouth
    if VENT[n] then
      local probe={};for _,layer in ipairs(layers) do probe[layer]=blank() end
      mouth=painter.new(probe,{key='probe',x=0,y=0,w=W,h=H,anchor_x=AX,anchor_y=AY},face):point(table.unpack(VENT[n]))
    end
    for k=0,1 do
      idle[#idle+1]={key=key..'_idle_'..k,duration=.2,images=folded(idle_frame(base,oval,C.faction[n],k,mouth),fold)}
    end
  end
  src:close()
  note(n..': rest '..maxc..' colours, v4 construction vs shipped rest differs in '..rest_diff..' px')
  C.document_images(n,W,H,AX,AY,layers,frames,tags,m,packer,out,entries)
  m.sprites[n]=m.sprites[n..'_0']
  C.document_images(n..'_idle',W,H,AX,AY,layers,idle,'idle_acting',m,packer,out,entries)
  total=total+#frames+#idle
end end

-- 2. Loaded walks: the Hook and the Wick carry through the v8 worker pose,
-- the Dredger through its v18 body; same ground speed as the walk.
for _,n in ipairs({'hook','wick'}) do if not only or only==n then
  local fold=C.fold_pixels(C.folds[C.faction[n]])
  local frames,tags={},{}
  for face=0,7 do
    local oval=paint(function(c) poses.worker(c,n,'loaded',0) end,face)[1]
    for phase=0,3 do
      local list=paint(function(c) poses.worker(c,n,'loaded_walk',phase) end,face)
      frames[#frames+1]={key=n..'_'..face..'_loaded_walk_'..phase,duration=C.phase_ms(n,face)/1000,images=folded(grounded(list,oval),fold)}
    end
    tags[#tags+1]={'loaded_walk_'..face,face*4+1,face*4+4}
  end
  C.document_images(n..'_loaded_walk',W,H,AX,AY,layers,frames,tags,m,packer,out,entries)
  total=total+#frames
end end

-- 3. The v18 roles: rest kept exactly, walk redrawn.
for _,mc in ipairs(ROLES) do local n=mc.name;if not only or only==n then
  local b=roles.budget[n]
  local remap=b and C.budget(b.allowed,b.fallback) or nil
  local finish=roles.finish[n]
  local function draw_with(phase) return function(c) roles[n](c,phase);c.dz=0;if finish then finish(c) end end end
  local src=assert(app.open(root..'/art/source/v18/'..n..'.aseprite'))
  local frames,tags={},{}
  local rest_diff=0
  for face=0,7 do
    local key=n..'_'..face
    local base=doc_frame(src,face*5+1,key)
    local drawn=paint(draw_with(nil),face,remap)
    for i=2,6 do rest_diff=rest_diff+differing(drawn[i],base[i]) end
    frames[#frames+1]={key=key,duration=1,images=base}
    for phase=0,3 do
      local walk=paint(draw_with(phase),face,remap)
      if mc.kind~='air' then walk=grounded(walk) end
      frames[#frames+1]={key=key..'_walk_'..phase,duration=C.phase_ms(n,face)/1000,images=walk}
    end
    tags[#tags+1]={'face_'..face,face*5+1,face*5+5}
  end
  src:close()
  note(n..': v18 construction vs shipped rest differs in '..rest_diff..' px')
  C.document_images(n,W,H,AX,AY,layers,frames,tags,m,packer,out,entries)
  m.sprites[n]=m.sprites[n..'_0']
  total=total+#frames
  if n=='dredger' then
    local frames2,tags2={},{}
    for face=0,7 do
      for phase=0,3 do
        local list=grounded(paint(function(c) roles.dredger_mode(c,'loaded_walk',phase);c.dz=0;roles.finish.dredger(c) end,face,remap))
        frames2[#frames2+1]={key='dredger_'..face..'_loaded_walk_'..phase,duration=C.phase_ms(n,face)/1000,images=list}
      end
      tags2[#tags2+1]={'loaded_walk_'..face,face*4+1,face*4+4}
    end
    C.document_images('dredger_loaded_walk',W,H,AX,AY,layers,frames2,tags2,m,packer,out,entries)
    total=total+#frames2
  end
end end

-- 4. Every other state of the originals: the shipped document recoloured to
-- the budget, layer by layer. Silhouettes, poses, timings and tags stay.
local function recoloured(path,name,keys,fold)
  local s=assert(app.open(root..'/'..path))
  local names={};for i,l in ipairs(s.layers) do names[i]=l.name end
  for f,key in pairs(keys) do
    local list={};for i=1,#s.layers do list[i]=C.full(s.layers[i],f,s.width,s.height) end
    local im=Image(s.width,s.height,ColorMode.RGB);for _,l in ipairs(list) do im:drawImage(l,Point(0,0)) end
    assert(im:isEqual((C.sprite_image(m,key))),key..': source document differs from the v18 atlas')
  end
  for _,cel in ipairs(s.cels) do cel.image=C.recolour(cel.image,fold) end
  C.install_palette(s)
  s:saveAs(C.SOURCE..name..'.aseprite')
  local count=0
  for f,key in pairs(keys) do
    local b=packer:alloc(key,s.width,s.height,m.sprites[key].anchor_x,m.sprites[key].anchor_y)
    entries[#entries+1]={key=key,document=name,frame=f,w=s.width,h=s.height,anchor_x=b.anchor_x,anchor_y=b.anchor_y,
      duration_ms=math.floor(s.frames[f].duration*1000+.5),layers=names,muzzle=b.muzzle}
    local frame=Image(s.width,s.height,ColorMode.RGB);frame:drawSprite(s,f);frame:saveAs(out..'/'..key..'.png')
    count=count+1
  end
  s:close()
  total=total+count
end
local function registry(path)
  local by={}
  for _,e in ipairs(json.decode(C.read(root..'/'..path))) do
    by[e.document]=by[e.document] or {};by[e.document][e.frame]=e.key
  end
  return by
end
local v15=registry('tools/art_v15/sources-shadows.json')
local v18=registry('tools/art_v18/sources-damage.json')
local v9=registry('tools/art_v9/sources.json')
local states={
 hook={'hook_gather','hook_loaded','hook_unload'},
 wick={'wick_gather','wick_loaded','wick_unload'},
 riveter={'riveter_firing'},sounder={'sounder_firing'},skipper={'skipper_firing'},reedguard={'reedguard_firing'},
 bulwark={'bulwark_firing','bulwark_deployment','bulwark_deployed_firing'},
 loom={'loom_firing','loom_deployment','loom_deployed_firing','loom_pressure'},
}
for _,n in ipairs(ORIGINALS) do if not only or only==n then
  local fold=C.fold_pixels(C.folds[C.faction[n]])
  for _,doc in ipairs(states[n]) do recoloured('art/source/v15/'..doc..'.aseprite',doc,assert(v15[doc],doc),fold) end
  recoloured('art/source/v18/'..n..'_damage.aseprite',n..'_damage',assert(v18[n..'_damage']),fold)
  if v9['pressure_'..n] then recoloured('art/source/v9/pressure_'..n..'.aseprite','pressure_'..n,v9['pressure_'..n],fold) end
  if v9[n..'_cargo_cradle'] then recoloured('art/source/v9/'..n..'_cargo_cradle.aseprite',n..'_cargo_cradle',v9[n..'_cargo_cradle'],fold) end
end end

if not only then
  table.sort(entries,function(a,b) if a.document~=b.document then return a.document<b.document end return a.frame<b.frame end)
  C.write(root..'/tools/art_v19/sources-machines.json',json.encode(entries))
  C.save_manifest(m)
end
for _,r in ipairs(report) do print(r) end
print('Authored '..total..' machine frames; manifest has '..C.count(m)..' entries.')
