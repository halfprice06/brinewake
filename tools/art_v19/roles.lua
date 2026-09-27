-- v19 bodies for the v16 tier-two roles and the v17 transports: the v18
-- construction (tools/art_v18/machines.lua) verbatim, budgets included,
-- with the v19 motion. What changed from v18:
--   * The planted foot moves back the engine's own travel per phase: four
--     units on a screen-axis facing, 5.7 on a diagonal. v18 moved it six on
--     every facing, so on the four screen-axis facings the feet slid back
--     two units a frame.
--   * Wheels, tracks, the Barge's stern wheel and the Lifter's screw turn a
--     quarter of their symmetry period per phase (tools/art_v19/motion.lua);
--     at v18's half period they strobed with no direction.
-- The rest pose (phase nil) of every machine is v18's.
local root=assert(app.params.root)
local C=dofile(root..'/tools/art_v19/common.lua')
local motion=dofile(root..'/tools/art_v19/motion.lua')
local out={}
local function sign(n) return n>=0 and 1 or -1 end

-- Colour budgets per machine: allowed shades per family, and where a family
-- outside the budget folds. Ink, contact and cast always pass.
out.budget={
  tidewatch ={allowed={rust={0,1,2,3,4},ivory={1,2,3},steel={0,1,2,3},glass={1,4}},fallback={amber='rust',reed='ivory',jade='steel'}},
  lampwright={allowed={jade={0,1,2,3,4},reed={1,2,3},steel={1,2},glass={1,3,4},amber={1,3}},fallback={ivory='reed',rust='reed'}},
  caulker   ={allowed={ivory={1,2,3,4},rust={1,2,3},steel={0,1,2,3},glass={0,2,3},amber={3}},fallback={reed='ivory',jade='steel'}},
  tender    ={allowed={jade={0,1,2,3,4},reed={0,1,2},ivory={1,2,3},glass={0,2},amber={1,3}},fallback={steel='jade',rust='reed'}},
  caisson   ={allowed={steel={0,1,2,3},rust={1,2,3},ivory={1,2,3,4}},fallback={amber='rust',glass='steel',reed='ivory',jade='steel'}},
  dredger   ={allowed={jade={0,1,2,3,4},ivory={1,2,3},glass={0,2},reed={1,2,3},steel={0,2},rust={2}},fallback={amber='rust'}},
  barge     ={allowed={jade={0,1,2,3,4},reed={1,2},ivory={1,2,3},glass={0,2},amber={3}},fallback={steel='jade',rust='reed'}},
  lifter    ={allowed={rust={0,1,2,3},ivory={1,2,3,4},steel={1,3},glass={0,2},amber={3}},fallback={reed='ivory',jade='steel'}},
}

-- The v4 cabin, kept verbatim so materials and joints match.
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

local tracks=motion.tracks
local function wheel(c,f,s,z,r,phase) motion.wheel(c,f,s,z,r,phase) end

-- The walk. Four engine phases, one quarter cell each. A leg with offset 0
-- is at front contact on phase 0, planted and vertical on 1 (the body is
-- highest here), at back contact on 2, swinging forward and lifted on 3.
-- The stroke is the engine's travel on this facing (common.lua `travel`).
local function gait_of(phase,offset) return phase and (phase+offset)%4 or nil end
-- Body rise for a set of legs on this phase: one unit on the passing frames.
function out.rise(phase,amount) if phase==nil then return 0 end return (phase%2==1) and amount or 0 end
local function leg(c,hip,foot,gait,material,opts)
  opts=opts or {}
  local step=C.travel(c.face)
  local travel=({step,0,-step,0})[(gait or 1)+1]
  if gait==nil then travel=0 end
  local lift=(gait==3) and (opts.lift or 4) or 0
  local toe={foot[1]+travel,foot[2],lift+1}
  local kz=hip[3]*0.55
  local knee
  if gait==1 then
    knee={toe[1],toe[2]+(hip[2]-toe[2])*0.4,kz+1}          -- straight, planted
  elseif gait==3 then
    knee={(hip[1]+toe[1])/2+3,(hip[2]+toe[2])/2,kz+3}       -- bent, carried forward
  else
    knee={(hip[1]+toe[1])/2-1,(hip[2]+toe[2])/2,kz}          -- contact and standing
  end
  local joint=opts.joint or 'steel'
  c:beam(hip,knee,4,joint);c:dot(hip[1],hip[2],hip[3],2,joint)
  c:beam(knee,toe,3,material);c:dot(knee[1],knee[2],knee[3],2,joint)
  if opts.paddle then
    c:box(toe[1]+1,toe[2],lift,6,3,2,material,1)
  else
    c:box(toe[1],toe[2],lift,3,2,2,material,1)
  end
end
-- Draw a set of legs: far legs on layer 2 (behind the hull), near legs on
-- layer 5 (in front of it). `positions` are {f,s,offset}; hips sit at
-- `height` over a point pulled toward the body centre.
local function legs(c,positions,height,phase,material,opts,rise)
  local centre_y=c:point(0,0,0)[2]
  table.sort(positions,function(a,b)return c:point(a[1],a[2],0)[2]<c:point(b[1],b[2],0)[2] end)
  for _,p in ipairs(positions) do
    local near=c:point(p[1],p[2],0)[2]>centre_y
    c:layer(near and 5 or 2)
    local pull=opts.pull or 0.55
    leg(c,{p[1]*pull,p[2]*pull,height+(rise or 0)},p,gait_of(phase,p[3]),material,opts)
  end
end

-- TIDEWATCH: a buggy that is mostly mast, now as wide as its role. Two big
-- wheels on a long rust chassis, an ivory housing and seat, the periscope.
out.tidewatch=function(c,phase)
  local near=sign(c.co)
  c:shadow(15,6)
  c:layer(2);wheel(c,-4,-near*9,7,7,phase)
  c:layer(3);c:box(0,0,5,13,6,5,'rust',2)
  c:box(9,0,10,4,4,3,'ivory',1)
  c:layer(4);c:box(-5,0,10,5,5,6,'ivory',2)
  c:line3({-9,-4,15},{-1,-4,15},'ivory1')
  c:layer(5);wheel(c,-4,near*9,7,7,phase)
  c:beam({1,0,13},{1,0,34},3,'steel')
  c:beam({1,0,26},{7,0,29},2,'rust')
  c:dot(1,0,34,3,'steel')
  local eye=c:point(1,near*1,36)
  c:ellipse(eye[1],eye[2],2,2,'glass1');c:pixel(eye[1]-1,eye[2]-1,'glass4');c:pixel(eye[1],eye[2]-1,'glass4')
  c:layer(6)
  c:line3({-6,near*5,8},{8,near*5,8},'rust3')
  c:line3({5,-3,13},{11,-3,13},'ivory3')
  c:line3({1,near*1,17},{1,near*1,24},'steel3')
end

-- LAMPWRIGHT: a two-legged walker with a wide stance, a jade body, and a
-- big lantern on a curved reed pole.
out.lampwright=function(c,phase)
  c:shadow(12,5)
  local rise=out.rise(phase,1)
  legs(c,{{-6,-7,0},{6,7,2}},12,phase,'jade',{pull=0.5,joint='steel'},rise)
  c.dz=rise
  c:layer(3);c:box(0,0,12,7,5,6,'jade',3)
  c:layer(4);c:box(-2,0,18,4,3,4,'reed',2)
  c:cylinder(-5,sign(c.co)*2,19,2,4,'amber')
  c:layer(5)
  c:beam({-1,0,22},{3,0,31},2,'reed');c:beam({3,0,31},{9,0,34},2,'reed')
  c:cylinder(9,0,27,3,6,'glass');c:cylinder(9,0,26,4,1,'steel');c:cylinder(9,0,33,4,1,'steel')
  c:layer(6)
  c:line3({8,-2,29},{8,-2,32},'glass4')
  c:line3({-5,3,14},{3,3,14},'jade4')
  c:line3({-6,-4,12},{-1,-4,12},'jade0')
  c.dz=0
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
  c:line3({-11,near*5,13},{-3,near*5,13},'rust3')
  c:line3({2,-4,22},{6,-4,22},'ivory4')
  c:line3({-9,-4,12},{-5,-4,12},'rust1')
end

-- TENDER: a four-legged mender with a reed cordage basket behind a small
-- cabin and a bobbin arm that reaches out over the front. The legs trot as
-- two bipeds: the diagonal pairs share a phase.
out.tender=function(c,phase)
  local near=sign(c.co)
  c:shadow(12,5)
  local rise=out.rise(phase,2)
  legs(c,{{-9,-9,0},{9,-9,2},{-9,9,2},{9,9,0}},12,phase,'jade',{pull=0.6,joint='jade'},rise)
  c.dz=rise
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
  c.dz=0
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
function out.caisson_deploy(c,stage)
  caisson_body(c,nil,({4,8,12})[stage+1],stage)
end

-- DREDGER: a wide paddle-walker. The hull sits lower than in v16 and the
-- paddles stand under its rim, the near pair in front of the hull, so the
-- feet stay attached at every facing.
local function dredger_body(c,phase,fill,dig)
  local near=sign(c.co)
  c:shadow(16,6)
  local rise=out.rise(phase,1)
  legs(c,{{-10,-9,0},{10,-9,2},{-10,9,2},{10,9,0}},7,phase,'jade',{pull=0.7,joint='jade',paddle=true,lift=3},rise)
  c.dz=rise
  c:layer(3)
  local shell={{-14,-8},{-8,-12},{8,-12},{15,-6},{15,6},{8,12},{-8,12},{-14,8}}
  c:solid(shell,6,6,'jade')
  c:layer(4);cabin(c,-2,0,12,5,4,7,'ivory')
  c:box(-10,0,12,3,6,4,'reed',2)
  if fill>0 then c:box(-10,0,16,2,5,fill,'rust',1) end
  c:cylinder(-4,near*5,13,2,4,'amber')
  c:layer(5)
  c:beam({4,0,15},{15,0,10-dig},3,'steel')
  for i=0,3 do
    local t=i/3
    local f=6+9*t;local z=14-(4+dig)*t
    c:box(f,0,z-2,2,3,2,'steel',1)
  end
  c:line3({4,0,17},{16,0,12-dig},'reed2')
  c:layer(6)
  c:line3({-12,near*6,10},{-6,near*6,10},'jade4')
  c:line3({0,-5,13},{6,-5,13},'jade2')
  c:line3({-6,near*4,19},{-4,near*4,19},'amber4')
  c.dz=0
end
out.dredger=function(c,phase) dredger_body(c,phase,0,0) end
function out.dredger_mode(c,mode,phase)
  if mode=='loaded' then dredger_body(c,nil,3,0)
  elseif mode=='loaded_walk' then dredger_body(c,phase,3,0)
  elseif mode=='gather' then dredger_body(c,nil,({1,2,3})[phase+1],({2,4,3})[phase+1])
  else dredger_body(c,nil,({3,2,0})[phase+1],0) end
end

-- BARGE: a long low jade hull, a reed cargo well under an ivory tarp bar,
-- the wheelhouse aft with its lamp on the roof, a stern paddle that turns.
out.barge=function(c,phase)
  local near=sign(c.co)
  c:shadow(16,6)
  c:layer(2)
  motion.stern_wheel(c,-19,4,phase)
  c:layer(3)
  local hull={{-16,-7},{-12,-9},{12,-9},{17,-5},{17,5},{12,9},{-12,9},{-16,7}}
  c:solid(hull,0,5,'jade')
  for s=-6,6,3 do c:line3({-13,s,5},{13,s,5},'reed1') end
  c:line3({-14,-8,5},{14,-8,5},'jade4');c:line3({-14,8,5},{14,8,5},'jade1')
  c:line3({-16,near*7,1},{16,near*6,1},'jade0')
  c:layer(4)
  c:box(3,0,5,7,5,2,'reed',1)
  c:line3({-3,-4,7},{9,-4,7},'ivory3');c:line3({-3,4,7},{9,4,7},'ivory2')
  cabin(c,-9,0,5,4,4,7,'ivory')
  c:layer(5)
  -- The lamp: a squat housing on the wheelhouse roof with an amber pane.
  c:box(-10,0,13,2,2,3,'steel',1)
  local lamp=c:point(-10,near*2,15);c:ellipse(lamp[1],lamp[2],1,1,'amber3')
  for _,f in ipairs({-6,2,10}) do c:dot(f,near*9,5,1,'steel') end
  c:layer(6)
  c:line3({-12,near*8,4},{12,near*8,4},'jade3')
  c:line3({4,-4,7},{9,-4,7},'ivory4')
end

-- LIFTER: one big ivory envelope over a rust gondola, four rigging lines,
-- the screw astern. The body floats eleven units above its ground shadow.
local LIFT=11
out.lifter=function(c,phase)
  local near=sign(c.co)
  c:shadow(13,5)
  c:layer(2)
  motion.screw(c,-13,LIFT+4,phase)
  c:layer(3)
  c:box(0,0,LIFT,10,4,5,'rust',2)
  c:line3({-10,near*4,LIFT},{10,near*4,LIFT},'rust0')
  c:layer(4)
  cabin(c,6,0,LIFT+5,3,3,5,'ivory')
  -- The envelope: a long rounded mass, tapered at the stern.
  c:box(0,0,LIFT+12,13,6,8,'ivory',4)
  c:box(-14,0,LIFT+14,3,4,4,'ivory',2)
  c:layer(5)
  for _,p in ipairs({{-8,-3},{8,-3},{-8,3},{8,3}}) do
    c:line3({p[1],p[2],LIFT+5},{p[1],p[2]*1.6,LIFT+12},'steel1')
  end
  c:cylinder(-4,near*3,LIFT+5,1,3,'steel')
  c:layer(6)
  c:line3({-8,near*4,LIFT+3},{8,near*4,LIFT+3},'rust3')
  c:line3({-9,-near*4,LIFT+19},{9,-near*4,LIFT+19},'ivory4')
  c:line3({-6,near*5,LIFT+14},{6,near*5,LIFT+14},'ivory1')
end

-- Material overpaint, as the v4 finish: contours attached to surfaces.
local ivory={'ivory1','ivory2','ivory3','ivory4'}
local rust={'rust1','rust2','rust3'}
local jade={'jade1','jade2','jade3','jade4'}
out.finish={}
out.finish.tidewatch=function(c) c:layer(6);c:overpaint3({{-9,-4,11},{-1,-4,11},{-1,-4,14},{-9,-4,14}},'ivory1',ivory) end
out.finish.lampwright=function(c) c:layer(6);c:overpaint3({{-6,-4,13},{5,-4,13},{5,-4,16},{-6,-4,16}},'jade1',jade) end
out.finish.caulker=function(c) c:layer(6);c:overpaint3({{-10,6,8},{-2,6,8},{-2,6,11},{-10,6,11}},'ivory1',ivory);c:overpaint3({{-10,-6,12},{-5,-6,12},{-5,-6,16},{-10,-6,16}},'rust1',rust) end
out.finish.tender=function(c) c:layer(6);c:overpaint3({{-10,7,12},{-2,7,12},{-2,7,15},{-10,7,15}},'jade1',jade) end
out.finish.caisson=function(c) c:layer(6);c:overpaint3({{-9,-7,16},{-3,-7,16},{-3,-7,24},{-9,-7,24}},'ivory2',ivory);c:overpaint3({{-14,5,9},{-8,5,9},{-8,5,12},{-14,5,12}},'rust1',rust) end
out.finish.dredger=function(c) c:layer(6);c:overpaint3({{-12,-9,8},{-2,-11,8},{-2,-11,11},{-12,-9,11}},'jade2',jade);c:overpaint3({{-9,7,8},{2,9,8},{8,7,8},{1,11,8}},'jade1',jade) end
out.finish.barge=function(c) c:layer(6);c:overpaint3({{-12,-9,1},{12,-9,1},{12,-9,4},{-12,-9,4}},'jade2',jade);c:overpaint3({{-13,0,6},{-5,0,6},{-5,0,12},{-13,0,12}},'ivory2',ivory) end
out.finish.lifter=function(c) c:layer(6);c:overpaint3({{-10,-4,LIFT+1},{10,-4,LIFT+1},{10,-4,LIFT+3},{-10,-4,LIFT+3}},'rust1',rust);c:overpaint3({{-12,-7,LIFT+14},{12,-7,LIFT+14},{12,-7,LIFT+18},{-12,-7,LIFT+18}},'ivory2',ivory) end
return out
