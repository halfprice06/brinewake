-- Root authoring entry point for the Saltglass Compact's eight machines
-- (compact.lua). Adds new keys to the runtime atlas, after the v19 layout:
--   {n}_{f}                 rest, and `{n}` as the east-facing alias
--   {n}_{f}_walk_{0..3}     walk at the engine's ground speed
--   {n}_{f}_idle_{0,1}      settle onto the knees, then a shine across the glass
--   {n}_{f}_damage          chips and cracks overlay (damage.lua)
--   brander, glinter:       {n}_{f}_fire_{0..2} with muzzles
--   heliostat:              _deploy_{0..2} (2 is fully braced, the rest pose
--                           of a deployed machine), _deployed_fire_{0..2}
--   raker:                  _gather_{0..2}, _loaded, _loaded_walk_{0..3},
--                           _unload_{0..2}
--   pan:                    _deploy_{0..2}, and _deployed_{0..3} (steam off
--                           the brine while it works)
--   salter:                 _lay_{0..2} (salt pouring from the chute)
-- Writes editable documents under art/source/v21/, review PNGs under
-- output/art-v21/machines/ and the registry tools/art_v21/sources-machines.json.
-- `only=<name>` builds one machine's documents without saving the atlas.
local root=assert(app.params.root,'root required')
local C=dofile(root..'/tools/art_v21/common.lua')
local M=dofile(root..'/tools/art_v21/compact.lua')
local D=dofile(root..'/tools/art_v21/damage.lua')
local painter=C.painter
local layers=painter.layers
local A=app.pixelColor.rgbaA
local W,H,AX,AY=64,64,32,50
local only=app.params.only
local out=C.OUT..'machines';os.execute('mkdir -p '..out..' '..C.SOURCE)
local atlas,m,packer=C.open_atlas()
local ctx={atlas=atlas,packer=packer,entries={},out=out}
local report={}
local total=0

local function blank() local im=Image(W,H,ColorMode.RGB);im:clear();return im end
local function paint(name,face,st)
  local ims={}
  for _,l in ipairs(layers) do ims[l]=blank() end
  local c=painter.new(ims,{key=name,x=0,y=0,w=W,h=H,anchor_x=AX,anchor_y=AY,remap=M.BUDGET},face)
  M[name](c,st)
  c.dz=0
  local list={};for i,l in ipairs(layers) do list[i]=ims[l] end
  local bodies={};for i=2,#list do bodies[#bodies+1]=list[i] end
  list[1]=C.grounded_shadow(bodies,list[1],W,H)
  return list,c
end
-- A local point on a facing, as a muzzle offset from the anchor.
local function muzzle(name,face,p)
  local probe={};for _,l in ipairs(layers) do probe[l]=blank() end
  local c=painter.new(probe,{key='probe',x=0,y=0,w=W,h=H,anchor_x=AX,anchor_y=AY},face)
  if type(p)=='function' then p=p(c) end
  local q=c:point(p[1],p[2],p[3])
  return {q[1]-AX,q[2]-AY}
end

-- The idle's second frame: a shine crossing the glass. Glass pixels in one
-- diagonal band step up a shade; glass3 in the band becomes the glint.
local shine_map=C.pixmap{glass1='glass2',glass2='glass3',glass3='glint'}
local function shine(list)
  local sum,n=0,0
  for i=2,#list do local im=list[i]
    for y=0,H-1 do for x=0,W-1 do local v=im:getPixel(x,y)
      if A(v)>0 and shine_map[v] then sum=sum+x+y;n=n+1 end
    end end
  end
  if n==0 then return list end
  local mid=math.floor(sum/n+.5)
  local o={list[1]}
  for i=2,#list do local im=Image(list[i])
    for y=0,H-1 do for x=0,W-1 do local v=im:getPixel(x,y)
      local d=x+y-mid
      if A(v)>0 and shine_map[v] and d>=-1 and d<=1 then im:drawPixel(x,y,shine_map[v]) end
    end end
    o[i]=im
  end
  return o
end

local MACHINES={'raker','brander','heliostat','glinter','stilt','glazier','salter','pan'}
local function sign(n) return n>=0 and 1 or -1 end
local MUZZLE={
  brander=function(k) return function(c) return {22-(({2,1,0})[k+1]),sign(c.co)*7,25} end end,
  glinter=function() return M.muzzle.glinter end,
}
-- A state strip: `frames` per facing from `st(k)`, one document, a tag a facing.
local function strip(name,doc,suffix,count,st,opts)
  opts=opts or {}
  local frames,tags={},{}
  for face=0,7 do
    for k=0,count-1 do
      local list=paint(name,face,st(k))
      if opts.post then list=opts.post(list,k) end
      local key=name..'_'..face..suffix..k
      if opts.single then key=name..'_'..face..suffix end
      local mz=opts.muzzle and muzzle(name,face,opts.muzzle(k)) or nil
      frames[#frames+1]={key=key,duration=opts.duration and opts.duration(face,k) or 0.1,images=list,muzzle=mz}
    end
    tags[#tags+1]={doc..'_'..face,face*count+1,face*count+count}
  end
  C.document(ctx,doc,W,H,AX,AY,layers,frames,tags)
  total=total+#frames
end

for _,n in ipairs(MACHINES) do if not only or only==n then
  -- Rest and walk: one document, five frames a facing.
  local frames,tags,maxc={},{},0
  local rests={}
  for face=0,7 do
    local rest=paint(n,face,{})
    rests[face]=rest
    local cnt=C.colours({rest[2],rest[3],rest[4],rest[5],rest[6]},true);if cnt>maxc then maxc=cnt end
    frames[#frames+1]={key=n..'_'..face,duration=1,images=rest}
    for p=0,3 do
      local walk=paint(n,face,{phase=p})
      local cnt2=C.colours({walk[2],walk[3],walk[4],walk[5],walk[6]},true);if cnt2>maxc then maxc=cnt2 end
      frames[#frames+1]={key=n..'_'..face..'_walk_'..p,duration=C.phase_ms(n,face)/1000,images=walk}
    end
    tags[#tags+1]={'face_'..face,face*5+1,face*5+5}
  end
  C.document(ctx,n,W,H,AX,AY,layers,frames,tags)
  m.sprites[n]=m.sprites[n..'_0']
  total=total+#frames
  report[#report+1]=n..': at most '..maxc..' colours a frame (shadows aside)'

  strip(n,n..'_idle','_idle_',2,function(k) return {mode='idle',k=k} end,{
    post=function(list,k) if k==1 then return shine(list) end return list end,
    duration=function() return 0.2 end})

  -- Damage: marks on the rest body, shadows left out.
  local dframes={}
  for face=0,7 do
    local body=C.composite(rests[face],2)
    dframes[#dframes+1]={key=n..'_'..face..'_damage',duration=1,images=D.marks(C,body,1,face+#n*3,{count=2,spacing=9})}
  end
  C.document(ctx,n..'_damage',W,H,AX,AY,{'chips','cracks','soot'},dframes,'damage_facings')
  total=total+#dframes

  if MUZZLE[n] then
    strip(n,n..'_firing','_fire_',3,function(k) return {mode='fire',k=k} end,{muzzle=MUZZLE[n]})
  end
  if n=='heliostat' then
    strip(n,'heliostat_deployment','_deploy_',3,function(k) return {mode='deploy',k=k} end,{duration=function() return 1/3 end})
    strip(n,'heliostat_deployed_firing','_deployed_fire_',3,function(k) return {mode='deployed_fire',k=k} end,
      {muzzle=function() return M.heliostat_focus() end})
  elseif n=='pan' then
    strip(n,'pan_deployment','_deploy_',3,function(k) return {mode='deploy',k=k} end,{duration=function() return 1/3 end})
    strip(n,'pan_deployed','_deployed_',4,function(k) return {mode='deployed',k=k} end,{duration=function() return 0.25 end})
  elseif n=='salter' then
    strip(n,'salter_laying','_lay_',3,function(k) return {mode='lay',k=k} end,{duration=function() return 0.15 end})
  elseif n=='raker' then
    strip(n,'raker_gather','_gather_',3,function(k) return {mode='gather',k=k} end,{duration=function() return 1/3 end})
    strip(n,'raker_loaded','_loaded',1,function() return {mode='loaded'} end,{single=true,duration=function() return 1 end})
    strip(n,'raker_unload','_unload_',3,function(k) return {mode='unload',k=k} end,{duration=function() return 0.1 end})
    strip(n,'raker_loaded_walk','_loaded_walk_',4,function(k) return {mode='loaded_walk',phase=k} end,
      {duration=function(face) return C.phase_ms(n,face)/1000 end})
  end
end end

if not only then
  -- Keep the registry of every other v21 document (buildings, icons) and
  -- replace this script's entries.
  local path=root..'/tools/art_v21/sources-machines.json'
  table.sort(ctx.entries,function(a,b) if a.document~=b.document then return a.document<b.document end return a.frame<b.frame end)
  C.write(path,json.encode(ctx.entries))
  C.save_atlas(atlas,m)
end
for _,r in ipairs(report) do print(r) end
print('Authored '..total..' machine frames.')
