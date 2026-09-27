-- The v8 machine poses (tools/art_v8/units.lua, root-authored) verbatim,
-- over the v19 bodies: deployments, recoil, and the worker's gather, carry
-- and unload poses. v19 uses it for the loaded walk, whose legs and tracks
-- now move at the engine's ground speed; every other pose is the v8 drawing
-- recoloured to its budget.
local root=assert(app.params.root)
local original=dofile(root..'/tools/art_v19/originals.lua')
local finish=dofile(root..'/tools/art_v4/finish.lua')
local M={}

local function pose(c,name,deploy,kick,work)
  local point=c.point
  c.point=function(self,f,s,z)
    z=z or 0
    local working=self.image==self.images['working parts']
    local detail=self.image==self.images['surface finish']
    local shell=self.image==self.images['cabins and shells']
    local hull=self.image==self.images['hulls']
    if deploy and name=='bulwark' then
      -- Hinged front plates fan out while the wheelbase keeps its contacts.
      if (working or detail) and f>=8 then s=s*(1+deploy*.13) end
      if z>8 and (shell or hull or working or detail) then z=z-deploy*.5 end
    elseif deploy and name=='loom' then
      if (working or hull) and z>=22 then f=f*(1+deploy*.07);s=s*(1+deploy*.13);z=z+deploy end
      if shell and z>=18 then z=z+deploy end
    end
    if kick and kick>0 then
      if name=='riveter' and (working or detail) and f>=10 and z>=14 then f=f-kick
      elseif name=='bulwark' and (working or detail) and f>=8 and z>8 then f=f-kick*.5
      elseif name=='sounder' and working and z>=25 then f=f-kick*.35
      elseif name=='skipper' and working and f>=10 then f=f-kick*.6
      elseif name=='reedguard' and working and s<=-4 and z>=24 then f=f-kick*.7
      elseif name=='loom' and shell and z>=18 then z=z-kick*.5 end
    end
    if work and working then
      if name=='hook' and f>=15 and z<=26 then z=z-work.lower end
      if name=='wick' and f>9 then f=9+(f-9)*(work.fold or 1) end
    end
    return point(self,f,s,z)
  end
  original[name](c,work and work.walk or nil)
  finish[name](c)
  c.point=point
end

local function brace(c,name,stage)
  local d=stage+1
  local steel=name=='bulwark'
  local sides={{-10,-12},{11,-12},{-10,12},{11,12}}
  table.sort(sides,function(a,b)return c:point(a[1],a[2],0)[2]<c:point(b[1],b[2],0)[2] end)
  for _,p in ipairs(sides) do
    local f,s=p[1],p[2]+(p[2]<0 and -1 or 1)*(d-1)
    local lift=({4,2,0})[d]
    -- Feet stop at the same ground plane; the strut changes length above it.
    c:layer(c:point(f,s,0)[2]>c.cy and 5 or 2)
    c:beam({f*.6,s*.55,steel and 12 or 14},{f,s,lift+3},steel and 3 or 2,'steel')
    c:box(f,s,lift,3,2,2,steel and 'ivory' or 'jade',1)
    c:dot(f*.6,s*.55,steel and 12 or 14,2,steel and 'rust' or 'reed')
    if stage==2 then
      c:layer(1);local a=c:point(f-2,s,0);local b=c:point(f+3,s,0);c:line(a,b,'contact',2)
    end
  end
  if not steel then
    c:layer(5)
    for _,s in ipairs({-1,1}) do
      c:line3({-10,s*14,2},{-10,s*(7+d),29+d},'reed1')
      c:line3({10,s*14,2},{10,s*(7+d),29+d},'reed2')
    end
  else
    c:layer(6)
    -- Two strong stripe groups stay on the front-facing plate, not the rear.
    for _,side in ipairs({-1,1}) do
      c:line3({15,side*(9+d),9},{15,side*(9+d)-side*3,9},'rust3',2)
    end
  end
end

function M.deploy(c,name,stage)
  pose(c,name,stage+1,nil,nil)
  brace(c,name,stage)
end

function M.fire(c,name,phase,deployed)
  local kick=({3,2,0})[phase+1]
  pose(c,name,deployed and 3 or nil,kick,nil)
  if deployed then brace(c,name,2) end
  -- Local discharge and recovery. The frame starts on the actual Shot event;
  -- there is no invented pre-damage anticipation in the authoritative game.
  c:layer(6)
  if name=='loom' then
    local f,s,z=7,(c.co>=0 and 1 or -1)*8,30+(deployed and 3 or 0)
    local a=c:point(f,s,z+phase*2)
    if phase<2 then
      c:ellipse(a[1]+phase,a[2],2+phase,1+phase,'steel3')
      c:line({a[1]-1,a[2]-1},{a[1]+1,a[2]-1},'foam')
    else c:line({a[1]+3,a[2]-2},{a[1]+5,a[2]-2},'steel2') end
  elseif name=='riveter' then
    c:line3({11-kick,0,20},{17-kick,0,20},phase==0 and 'amber3' or 'steel3')
    c:line3({11-kick,1,16},{16-kick,1,16},'steel0')
  elseif name=='skipper' then
    c:line3({12,0,18},{14,0,18},phase==0 and 'amber4' or 'jade2',2)
  elseif name=='reedguard' then
    c:line3({6-kick,-6,30},{10-kick,-6,30},phase==0 and 'amber3' or 'steel3')
  elseif name=='sounder' then
    local p=c:point(11,0,17)
    c:line({p[1],p[2]},{p[1]+1,p[2]},phase==0 and 'amber3' or 'steel3')
  end
  local muzzle
  if name=='riveter' then muzzle=c:point(24-kick,0,18)
  elseif name=='bulwark' then muzzle=c:point(16-kick*.5,0,17-(deployed and 1.5 or 0))
  elseif name=='sounder' then muzzle=c:point(11,0,17)
  elseif name=='skipper' then muzzle=c:point(17-kick*.6,0,14)
  elseif name=='reedguard' then muzzle=c:point(13-kick*.7,-6,28)
  else muzzle=c:point(7*(deployed and 1.21 or 1),(c.co>=0 and 1 or -1)*8*(deployed and 1.39 or 1),28+(deployed and 3 or 0)) end
  c.muzzle={muzzle[1]-c.cx,muzzle[2]-c.cy}
end

local function cargo(c,name,lower,sway)
  c:layer(5)
  if name=='hook' then
    local f,s,z=19,-2+(sway or 0),8-(lower or 0)
    -- A heavy folded plate hangs from the existing cable and hook eye.
    c:box(f,s,z,4,6,10,'steel',1)
    c:beam({17,-2,21-(lower or 0)},{f,s,z+10},1,'rust')
    c:line3({f-3,s-5,z+9},{f+3,s-5,z+9},'ivory3')
    c:line3({f+4,s-4,z+2},{f+4,s+4,z+2},'rust2',2)
    c:line3({f+4,s,z+1},{f+4,s,z+8},'ivory2')
  else
    -- Tightly bound scrap is cradled inside the open rear basket.
    local z=20-(lower or 0)
    c:box(-1,4,z,6,3,6,'reed',1)
    for _,f in ipairs({-4,2}) do
      c:line3({f,1,z+6},{f,7,z+6},'steel1')
      c:line3({f,7,z+6},{f,7,z},'steel1')
    end
    c:line3({-5,2,z+7},{2,2,z+7},'steel3')
    c:line3({-3,4,z+7},{4,4,z+7},'rust2')
    c:line3({6,5,24},{4,5,z+4},'reed3',2)
  end
end

function M.worker(c,name,mode,phase)
  local loaded=mode=='loaded' or mode=='loaded_walk'
  local lower=mode=='gather' and ({2,5,3})[phase+1] or (mode=='unload' and ({0,3,5})[phase+1] or 0)
  local work={lower=lower,fold=loaded and -.25 or (mode=='unload' and -.25 or 1),walk=mode=='loaded_walk' and phase or nil}
  pose(c,name,nil,nil,work)
  if loaded or (mode=='unload' and phase<2) then cargo(c,name,mode=='unload' and lower or 0,mode=='loaded_walk' and ({0,0,1,0})[phase+1] or 0) end
  if mode=='gather' then
    c:layer(5)
    local p=c:point(name=='hook' and 18 or 22,name=='hook' and -2 or 7,name=='hook' and 18-lower or 22)
    if phase==1 then
      c:line({p[1]-2,p[2]+3},{p[1],p[2]+1},'amber3')
      c:line({p[1]+2,p[2]+2},{p[1]+4,p[2]+3},'steel3')
    end
  elseif mode=='unload' and phase==2 then
    c:layer(5)
    if name=='hook' then c:box(18,-2,6,3,4,3,'steel',1)
    else c:box(-1,4,17,6,3,2,'reed',1) end
  end
end
return M
