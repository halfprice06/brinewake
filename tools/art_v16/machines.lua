-- Root-authored v16 machine bodies: the tier-two roles of both factions,
-- drawn in the v4 painter's projection and materials so they stand beside
-- the original eight without a seam. Coordinates are (forward, side, up)
-- local units; layers are the six semantic planes of every machine document.
--
--   Union, heavy on dry ground:
--     tidewatch  a light wheeled buggy under a tall periscope mast
--     caulker    a tracked mender with a spool and a rivet torch on a jib
--     caisson    a wheeled flatbed carrying a plug that lowers to the ground
--   Assembly, tide and siege:
--     lampwright a slim two-legged walker with a lantern on a reed pole
--     tender     a four-legged mender with a cordage basket and a bobbin arm
--     dredger    a wide paddle-walker with a bucket chain and a hopper
local out={}
local function sign(n) return n>=0 and 1 or -1 end

-- The v4 helpers, kept verbatim so materials and joints match.
local function cabin(c,f,s,z,hf,hs,h,material)
  c:box(f,s,z,hf,hs,h,material,2)
  local front=sign(c.si);local side=sign(c.co)
  local panes={
    {{f+front*hf,s-hs+2,z+3},{f+front*hf,s+hs-2,z+3},
     {f+front*hf,s+hs-2,z+h-2},{f+front*hf,s-hs+2,z+h-2}},
    {{f-hf+2,s+side*hs,z+3},{f+hf-2,s+side*hs,z+3},
     {f+hf-2,s+side*hs,z+h-2},{f-hf+2,s+side*hs,z+h-2}},
  }
  for _,p in ipairs(panes) do
    c:poly3(p,'glass0')
    local a,b=p[4],p[3]
    c:line3({a[1],a[2],a[3]-1},{b[1],b[2],b[3]-1},'glass2')
    local mid={(a[1]+b[1])/2,(a[2]+b[2])/2,a[3]}
    c:line3(a,mid,'glass3')
    local low=p[1]
    c:line3({low[1],low[2],low[3]-1},{p[2][1],p[2][2],p[2][3]-1},material..'0')
    c:line3({a[1],a[2],a[3]-1},{a[1],a[2],a[3]-3},'glass2')
  end
  c:box(f-1,s,z+h,math.max(2,hf-2),math.max(2,hs-2),1,material,1)
  c:line3({f-hf+2,s-1,z+h+1},{f-1,s-1,z+h+1},material..'1')
  c:line3({f-hf+2,s,z+h+1},{f-2,s,z+h+1},material..'4')
  c:line3({f+front*hf,s-hs+1,z+1},{f+front*hf,s-hs+3,z+1},'rust2')
  c:line3({f-hf+1,s+side*hs,z+2},{f-hf+3,s+side*hs,z+2},material..'4')
end

local function tracks(c,phase,hf,hs)
  local near=sign(c.co)
  c:shadow(hf+2,5)
  for _,side in ipairs({-near,near}) do
    c:layer(side==near and 3 or 2)
    c:box(0,side*hs,1,hf,3,6,'steel',3)
    for f=-hf+3,hf-2,4 do
      local shift=phase and phase%2 or 0
      c:line3({f+shift,side*hs-2,7},{f+shift,side*hs+2,7},'steel0')
    end
    if side==near then
      for _,f in ipairs({-hf+4,0,hf-4}) do
        c:disc({f,side*(hs+3),4},{1,0,0},{0,0,1},3,'steel0')
        c:disc({f,side*(hs+3),4},{1,0,0},{0,0,1},2,'steel2')
        local p=c:point(f,side*(hs+3),4);c:pixel(p[1],p[2],'rust3')
      end
    end
  end
end

local function wheel(c,f,s,z,r,phase)
  c:disc({f,s,z},{1,0,0},{0,0,1},r+1,'ink')
  c:disc({f,s,z},{1,0,0},{0,0,1},r,'steel1')
  c:disc({f,s,z},{1,0,0},{0,0,1},r-2,'rust2')
  local p=c:point(f-r/3,s,z+r/3)
  c:line(p,{p[1]+2,p[2]},'rust4')
  local a=(phase or 0)*math.pi/4
  for k=0,3 do
    local t=a+k*math.pi/2
    c:line3({f,s,z},{f+math.cos(t)*(r-3),s,z+math.sin(t)*(r-3)},'rust0')
  end
  c:disc({f,s,z},{1,0,0},{0,0,1},2,'steel3')
end

local function leg(c,root,foot,phase,offset,material,paddle)
  local gait=phase and (phase+offset)%4 or 0
  local step=c.face%2==0 and 4 or 6
  local travel=phase and ({step,0,-step,0})[gait+1] or 0
  local lift=phase and gait==3 and 3 or 0
  local toe={foot[1]+travel,foot[2],lift+1}
  local knee={(root[1]+toe[1])/2-2,(root[2]+toe[2])/2,root[3]/2+1}
  c:beam(root,knee,4,'steel');c:dot(root[1],root[2],root[3],2,'steel')
  c:beam(knee,toe,3,material);c:dot(knee[1],knee[2],knee[3],2,'steel')
  c:box(toe[1]+(paddle and 2 or 0),toe[2],lift,paddle and 7 or 3,paddle and 3 or 2,2,material,1)
end

local function assembly_feet(c,phase,width,length,height,paddles)
  c:shadow(length+3,5)
  local positions={{-length,-width,0,0},{length,-width,0,2},{-length,width,0,2},{length,width,0,0}}
  table.sort(positions,function(a,b)return c:point(a[1],a[2],0)[2]<c:point(b[1],b[2],0)[2] end)
  for _,p in ipairs(positions) do
    c:layer(2)
    leg(c,{p[1]*0.6,p[2]*0.6,height},p,phase,p[4],'jade',paddles)
  end
end

local function two_feet(c,phase,length,height)
  c:shadow(length+2,4)
  -- A biped stands with its feet apart, so the legs read at every facing.
  local positions={{-length,-5,0,0},{length,5,0,2}}
  table.sort(positions,function(a,b)return c:point(a[1],a[2],0)[2]<c:point(b[1],b[2],0)[2] end)
  for _,p in ipairs(positions) do
    c:layer(2)
    leg(c,{p[1]*0.5,p[2]*0.5,height},p,phase,p[4],'jade',false)
  end
end

-- TIDEWATCH: a buggy that is mostly mast. Two big wheels, a low rust
-- chassis, an open ivory seat box, and the periscope with its glass eye.
out.tidewatch=function(c,phase)
  local near=sign(c.co)
  c:shadow(14,5)
  c:layer(2);wheel(c,-5,-near*7,6,6,phase)
  c:layer(3);c:box(1,0,5,11,4,4,'rust',2)
  c:box(9,0,9,3,3,3,'ivory',1)
  c:layer(4);c:box(-5,0,9,4,4,5,'ivory',2)
  c:line3({-8,-3,14},{-2,-3,14},'ivory1')
  c:layer(5);wheel(c,-5,near*7,6,6,phase)
  c:beam({2,0,12},{2,0,35},2,'steel')
  c:beam({2,0,26},{7,0,29},1,'rust')
  c:dot(2,0,35,3,'steel')
  local eye=c:point(2,near*1,37)
  c:ellipse(eye[1],eye[2],2,2,'glass1');c:pixel(eye[1]-1,eye[2]-1,'glass4')
  c:layer(6)
  c:line3({-4,near*4,8},{6,near*4,8},'rust3')
  c:line3({4,-2,12},{9,-2,12},'ivory4')
  c:line3({2,near*1,18},{2,near*1,24},'steel4')
end

-- LAMPWRIGHT: a slim two-legged walker carrying a lantern on a curved
-- reed pole, the amber chamber small and the lamp glass large.
out.lampwright=function(c,phase)
  two_feet(c,phase,7,11)
  c:layer(3);c:box(0,0,10,6,4,5,'jade',3)
  c:layer(4);c:box(-2,0,15,4,3,4,'reed',2)
  c:cylinder(-4,sign(c.co)*2,16,2,4,'amber')
  c:layer(5)
  c:beam({-1,0,19},{3,0,30},2,'reed');c:beam({3,0,30},{9,0,33},2,'reed')
  c:cylinder(9,0,27,3,5,'glass');c:cylinder(9,0,26,4,1,'steel');c:cylinder(9,0,32,4,1,'steel')
  c:layer(6)
  c:line3({8,-2,29},{8,-2,31},'glass4')
  c:line3({-4,3,12},{2,3,12},'jade4')
  c:line3({-5,-3,10},{-1,-3,10},'jade0')
end

-- CAULKER: a short tracked mender. A spool of wire on the rear deck feeds
-- a jib that ends in a rivet torch; the cabin sits forward of the spool.
out.caulker=function(c,phase)
  local near=sign(c.co)
  tracks(c,phase,12,7)
  c:layer(3);c:box(0,0,6,11,7,5,'ivory',3)
  c:box(-7,0,11,4,5,6,'rust',2)
  c:layer(4);cabin(c,4,1,11,5,4,9,'ivory')
  c:cylinder(-7,0,17,3,3,'steel')
  c:layer(5)
  c:beam({-7,-3,19},{3,-4,27},3,'rust');c:beam({3,-4,27},{12,-3,23},2,'steel')
  c:dot(12,-3,23,2,'amber')
  c:line3({12,-3,23},{15,-3,21},'amber3')
  c:line3({-7,-1,20},{-7,-1,14},'steel3')
  c:layer(6)
  c:line3({-11,near*5,13},{-3,near*5,13},'rust4')
  c:line3({2,-4,22},{6,-4,22},'ivory4')
  c:line3({-9,-4,12},{-5,-4,12},'rust0')
end

-- TENDER: a four-legged mender with a reed cordage basket behind a small
-- cabin and a bobbin arm that reaches out over the front.
out.tender=function(c,phase)
  local near=sign(c.co)
  assembly_feet(c,phase,9,9,12,false)
  c:layer(3);c:box(-1,0,10,10,7,6,'jade',3)
  c:box(-7,0,16,4,5,5,'reed',2)
  for f=-10,-4,3 do c:line3({f,-5,21},{f,5,21},'reed0') end
  c:layer(4);cabin(c,4,-1,16,5,4,8,'ivory')
  c:cylinder(-3,near*5,17,2,5,'amber')
  c:layer(5)
  c:beam({-6,3,21},{5,3,28},3,'jade');c:beam({5,3,28},{12,2,25},2,'steel')
  c:cylinder(12,2,23,2,3,'reed')
  c:line3({12,2,26},{14,2,22},'amber3')
  c:layer(6)
  c:line3({-9,near*6,14},{-4,near*6,14},'jade4')
  c:line3({-3,near*5,23},{-3,near*5,19},'amber4')
  c:line3({6,-3,26},{9,-3,26},'jade2')
end

-- CAISSON: a heavy flatbed under a plug, a steel-strapped ivory box that
-- stands proud of the deck and lowers to the ground when deployed.
local function caisson_body(c,phase,drop,plates)
  local near=sign(c.co)
  c:shadow(20,7)
  c:layer(2);wheel(c,-8,-near*9,7,7,phase);wheel(c,8,-near*9,7,7,phase)
  c:box(0,-near*7,1,14,3,5,'steel',2)
  c:layer(3);c:box(0,0,8,15,7,4,'rust',3)
  c:layer(4)
  local z=13-drop
  c:box(0,0,z,10,7,16,'ivory',2)
  c:layer(5);wheel(c,-8,near*9,7,7,phase);wheel(c,8,near*9,7,7,phase)
  c:box(0,0,z+16,11,8,2,'steel',1)
  for _,f in ipairs({-7,0,7}) do
    c:line3({f,-8,z},{f,-8,z+16},'steel2');c:line3({f,8,z},{f,8,z+16},'steel2')
  end
  c:line3({-10,near*8,z+9},{10,near*8,z+9},'rust1',2)
  if plates>0 then
    for _,side in ipairs({-1,1}) do
      c:box(0,side*(9+plates),0,9,1,3+plates*2,'steel',1)
    end
    c:box(13+plates,0,0,1,7,3+plates*2,'steel',1)
    c:box(-13-plates,0,0,1,7,3+plates*2,'steel',1)
  end
  c:layer(6)
  c:line3({-9,near*8,z+3},{-4,near*8,z+3},'ivory4')
  c:line3({3,near*8,z+13},{8,near*8,z+13},'ivory1')
  c:line3({-12,near*6,10},{-6,near*6,10},'rust3')
end
out.caisson=function(c,phase) caisson_body(c,phase,0,0) end
-- Deploy stages: the plug settles onto the ground and the side plates rise.
function out.caisson_deploy(c,stage)
  caisson_body(c,nil,({4,8,12})[stage+1],stage)
end

-- DREDGER: a wide paddle-walker, a low jade hull with a bucket chain on a
-- steel boom over the bow and a reed hopper astern.
local function dredger_body(c,phase,fill,dig)
  local near=sign(c.co)
  assembly_feet(c,phase,10,11,9,true)
  c:layer(3)
  local shell={{-14,-8},{-8,-12},{8,-12},{15,-6},{15,6},{8,12},{-8,12},{-14,8}}
  c:solid(shell,8,6,'jade')
  c:layer(4);cabin(c,-2,0,14,5,4,7,'ivory')
  c:box(-10,0,14,3,6,4,'reed',2)
  if fill>0 then c:box(-10,0,18,2,5,fill,'rust',1) end
  c:cylinder(-4,near*5,15,2,4,'amber')
  c:layer(5)
  c:beam({4,0,17},{15,0,12-dig},3,'steel')
  for i=0,3 do
    local t=i/3
    local f=6+9*t;local z=16-(4+dig)*t
    c:box(f,0,z-2,2,3,2,'steel',1)
  end
  c:line3({4,0,19},{16,0,14-dig},'reed2')
  c:layer(6)
  c:line3({-12,near*6,12},{-6,near*6,12},'jade4')
  c:line3({0,-5,15},{6,-5,15},'jade2')
  c:line3({-6,near*4,21},{-4,near*4,21},'amber4')
end
out.dredger=function(c,phase) dredger_body(c,phase,0,0) end
-- The worker set: loaded, loaded walk, gather (the chain digs), unload.
function out.dredger_mode(c,mode,phase)
  if mode=='loaded' then dredger_body(c,nil,3,0)
  elseif mode=='loaded_walk' then dredger_body(c,phase,3,0)
  elseif mode=='gather' then dredger_body(c,nil,({1,2,3})[phase+1],({2,4,3})[phase+1])
  else dredger_body(c,nil,({3,2,0})[phase+1],0) end
end

-- Material overpaint, as the v4 finish: contours attached to surfaces.
local ivory={'ivory1','ivory2','ivory3','ivory4'}
local rust={'rust1','rust2','rust3'}
local jade={'jade1','jade2','jade3','jade4'}
out.finish={}
out.finish.tidewatch=function(c) c:layer(6);c:overpaint3({{-8,-3,10},{-2,-3,10},{-2,-3,13},{-8,-3,13}},'ivory1',ivory) end
out.finish.lampwright=function(c) c:layer(6);c:overpaint3({{-5,-3,11},{4,-3,11},{4,-3,14},{-5,-3,14}},'jade1',jade) end
out.finish.caulker=function(c) c:layer(6);c:overpaint3({{-10,6,8},{-2,6,8},{-2,6,11},{-10,6,11}},'ivory1',ivory);c:overpaint3({{-10,-6,12},{-5,-6,12},{-5,-6,16},{-10,-6,16}},'rust1',rust) end
out.finish.tender=function(c) c:layer(6);c:overpaint3({{-10,7,12},{-2,7,12},{-2,7,15},{-10,7,15}},'jade1',jade) end
out.finish.caisson=function(c) c:layer(6);c:overpaint3({{-9,-7,16},{-3,-7,16},{-3,-7,24},{-9,-7,24}},'ivory2',ivory);c:overpaint3({{-14,5,9},{-8,5,9},{-8,5,12},{-14,5,12}},'rust1',rust) end
out.finish.dredger=function(c) c:layer(6);c:overpaint3({{-12,-9,10},{-2,-11,10},{-2,-11,13},{-12,-9,13}},'jade2',jade);c:overpaint3({{-9,7,10},{2,9,10},{8,7,10},{1,11,10}},'jade1',jade) end
return out
