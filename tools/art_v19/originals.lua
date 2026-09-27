-- v19 bodies for the eight original machines. Every body, the cabin and
-- the assembly stance below are the v4 construction verbatim (root-authored
-- in tools/art_v4/units.lua); the rest pose is unchanged pixel for pixel.
-- Only the moving parts are new: the wheels and tracks come from
-- tools/art_v19/motion.lua, and the leg below carries a stance and a swing
-- at the engine's own ground speed.
local root=assert(app.params.root)
local C=dofile(root..'/tools/art_v19/common.lua')
local motion=dofile(root..'/tools/art_v19/motion.lua')
local out = {}
local function sign(n) return n>=0 and 1 or -1 end

local function cabin(c,f,s,z,hf,hs,h,material)
  c:box(f,s,z,hf,hs,h,material,2)
  local front=sign(c.si);local side=sign(c.co)
  -- Glass sits on the actual two visible walls, inset from roof and pillars.
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
    -- One reflected sky wedge and a recessed lower sill keep the pane from
    -- reading as a flat cyan stripe. The reflections stay inside the glazing.
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

-- The leg. A leg with offset 0 is at front contact on phase 0, planted
-- under its hip on 1, at back contact on 2 and swinging on 3. The planted
-- foot moves back exactly the engine's travel per phase (four units on a
-- screen-axis facing, 5.7 on a diagonal; v4 used six there), so it holds
-- the ground. Passing over it, the leg straightens under the load; on the
-- swing the knee folds up and the foot clears the ground by four units
-- (v4: three, with the knee fixed). The body stays level: the engine draws
-- the pressure sleeves, cargo cradles and damage marks of these machines
-- at the rest position over every walk frame, and a rising body would slide
-- under them. The rest pose (phase nil) is v4's.
local function leg(c,root,foot,phase,offset,material,paddle)
  local gait=phase and (phase+offset)%4 or nil
  local step=C.travel(c.face)
  local travel=gait and ({step,0,-step,0})[gait+1] or 0
  local lift=gait==3 and 4 or 0
  local toe={foot[1]+travel,foot[2],lift+1}
  local knee
  if gait==1 then
    knee={(root[1]+toe[1])/2,(root[2]+toe[2])/2,(root[3]+toe[3])/2}
  elseif gait==3 then
    knee={(root[1]+toe[1])/2-2,(root[2]+toe[2])/2,root[3]/2+3}
  else
    knee={(root[1]+toe[1])/2-2,(root[2]+toe[2])/2,root[3]/2+1}
  end
  c:beam(root,knee,4,'steel');c:dot(root[1],root[2],root[3],2,'steel')
  c:beam(knee,toe,3,material);c:dot(knee[1],knee[2],knee[3],2,'steel')
  c:box(toe[1]+(paddle and 2 or 0),toe[2],lift,paddle and 7 or 3,paddle and 3 or 2,2,material,1)
end

out.hook=function(c,phase)
  tracks(c,phase,15,8)
  c:layer(3)
  c:box(0,0,6,14,8,5,'rust',3)
  c:box(-8,0,11,5,6,7,'steel',2)
  c:box(10,0,10,5,7,5,'ivory',2)
  c:layer(4)
  cabin(c,4,2,12,6,5,12,'ivory')
  c:box(-7,-2,18,4,4,5,'rust',2)
  c:layer(5)
  c:beam({-7,-2,19},{-6,-2,31},6,'rust')
  c:beam({-6,-2,31},{17,-2,38},5,'ivory')
  c:beam({-5,0,20},{1,0,31},2,'steel')
  c:dot(-6,-2,30,3,'rust')
  c:dot(16,-2,37,2,'steel')
  c:line3({17,-2,36},{17,-2,23},'ink')
  local hook=c:point(17,-2,23)
  c:line(hook,{hook[1],hook[2]+3},'rust3',2)
  c:line({hook[1],hook[2]+3},{hook[1]+3,hook[2]+4},'rust2',2)
  c:line({hook[1]+3,hook[2]+4},{hook[1]+4,hook[2]+1},'rust3')
  c:layer(6)
  c:line3({-11,-5,19},{-6,-5,19},'steel3')
  c:line3({8,2,26},{11,2,26},'rust2')
  c:box(-11,4,13,2,2,5,'steel',1)
  c:line3({-13,-4,17},{-9,-4,17},'steel0')
  c:line3({-13,-2,17},{-9,-2,17},'steel0')
  c:line3({8,-5,15},{11,-5,15},'ivory1')
  c:line3({1,-2,35},{5,-2,36},'ivory1')
  c:line3({-5,2,22},{-2,2,28},'steel4')
end

out.riveter=function(c,phase)
  local near=sign(c.co)
  c:shadow(19,6)
  c:layer(2);wheel(c,-3,-near*8,8,8,phase)
  c:box(4,-near*7,1,10,3,5,'steel',2)
  c:layer(3);c:box(-1,0,8,14,7,7,'rust',3)
  c:box(-9,0,15,6,6,8,'rust',1)
  c:box(-9,0,23,5,5,1,'steel',1)
  for _,p in ipairs({{-11,-2},{-7,1},{-10,3}}) do c:dot(p[1],p[2],25,2,'steel') end
  c:layer(4);cabin(c,4,0,15,6,5,11,'ivory')
  c:layer(5);wheel(c,-3,near*9,8,8,phase)
  c:box(12,0,9,4,4,8,'steel',1)
  c:beam({10,0,18},{22,0,18},5,'steel')
  c:box(22,0,15,2,3,5,'rust',1)
  c:layer(6)
  c:line3({-14,near*7,15},{-7,near*7,15},'rust4')
  c:line3({1,-3,28},{6,-3,28},'ivory4')
  c:cylinder(-4,-3,26,2,4,'steel')
  c:line3({-13,near*6,21},{-8,near*6,21},'rust0')
  c:line3({-13,near*6,19},{-9,near*6,19},'rust3')
  c:line3({13,-3,17},{18,-3,17},'steel4')
end

out.bulwark=function(c,phase)
  tracks(c,phase,13,9)
  c:layer(3);c:box(-3,0,7,11,9,10,'rust',3)
  c:layer(4);cabin(c,-4,0,17,7,6,10,'ivory')
  c:box(-8,-8,15,4,3,8,'steel',1)
  c:box(-8,8,15,4,3,8,'steel',1)
  c:layer(5)
  for _,side in ipairs({-1,1}) do
    c:beam({1,side*7,12},{12,side*9,15},4,'steel')
    c:box(12,side*6,4,3,5,23,'ivory',1)
    c:box(12,side*6,4,3,5,7,'rust',1)
    c:line3({15,side*6-3,19},{15,side*6+3,19},'ivory1')
    c:dot(12,side*11,24,2,'steel')
    c:dot(12,side*11,7,2,'steel')
  end
  c:layer(6);c:box(-4,0,28,3,3,2,'rust',1)
  c:line3({-11,sign(c.co)*9,17},{-4,sign(c.co)*9,17},'rust4')
  for _,side in ipairs({-1,1}) do
    c:line3({15,side*7,8},{15,side*7,11},'rust0')
    c:line3({15,side*4,17},{15,side*4,20},'ivory1')
    c:line3({15,side*8,23},{15,side*5,23},'ivory4')
  end
end

out.sounder=function(c,phase)
  c:shadow(17,5)
  local feet={{-12,-9,0},{-12,9,0},{13,0,0}}
  for i,p in ipairs(feet) do c:layer(2);leg(c,{-2,p[2]/2,12},p,phase,i,'ivory',false) end
  c:layer(3);c:box(-1,0,10,10,6,6,'rust',3)
  c:layer(4);cabin(c,3,0,16,6,5,8,'ivory')
  c:layer(5)
  c:beam({-7,-2,16},{-7,-2,40},3,'steel')
  c:box(-7,-2,25,2,2,8,'ivory',1)
  c:line3({-7,-2,38},{-7,-2,44},'steel3')
  c:line3({-7,-2,44},{-4,-2,43},'rust3')
  local p=c:point(-7,-2,35)
  c:ellipse(p[1]+3,p[2],5,6,'steel0')
  c:ellipse(p[1]+2,p[2]-1,4,5,'ivory2')
  c:ellipse(p[1]+1,p[2]-2,2,3,'ivory4')
  c:line({p[1]+4,p[2]+1},{p[1]+7,p[2]+3},'steel2')
  c:layer(6);c:box(-10,3,12,3,3,5,'steel',1)
  c:line3({-12,5,18},{-8,5,18},'steel0')
  c:line3({-7,-2,26},{-7,-2,30},'rust2')
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

out.wick=function(c,phase)
  assembly_feet(c,phase,10,10,13,false)
  c:layer(3);c:box(0,0,11,11,8,6,'jade',4)
  c:box(-2,0,17,8,6,2,'reed',3)
  c:box(-2,0,19,6,4,1,'steel',2)
  for f=-5,2,3 do c:line3({f,-3,21},{f,3,21},'reed2') end
  c:layer(4)
  cabin(c,5,-2,18,5,4,9,'ivory')
  c:cylinder(-7,-3,19,3,12,'amber')
  c:cylinder(-7,-3,20,4,2,'steel')
  c:cylinder(-7,-3,30,4,2,'steel')
  c:layer(5)
  c:beam({2,5,17},{10,7,24},4,'jade')
  c:beam({10,7,24},{19,7,23},3,'steel')
  c:line3({19,7,23},{22,5,25},'reed3',2)
  c:line3({19,7,23},{22,9,21},'reed2',2)
  c:layer(6)
  c:line3({-9,4,18},{-4,7,18},'jade4')
  c:line3({-7,-5,22},{-7,-5,28},'amber4')
  c:line3({-8,5,15},{-3,7,15},'jade0')
  c:line3({0,7,16},{3,7,16},'jade4')
  c:line3({10,7,23},{15,7,23},'steel3')
end

out.skipper=function(c,phase)
  assembly_feet(c,phase,9,10,10,true)
  c:layer(3)
  local shell={{-16,-6},{-11,-12},{1,-13},{12,-8},{17,0},{12,8},{1,13},{-11,12},{-16,6}}
  c:solid(shell,10,6,'jade')
  c:layer(4)
  -- Broad radiating fan plates, substantial at every view.
  for _,s in ipairs({-9,-5,5,9}) do
    c:line3({-10,s,17},{9,s*0.6,17},'jade1')
    c:line3({-9,s+1,17},{8,s*0.6+1,17},'jade3')
  end
  cabin(c,3,0,17,7,4,7,'ivory')
  c:cylinder(-8,0,17,4,5,'amber')
  c:layer(5);c:box(13,0,11,4,3,5,'steel',2)
  c:layer(6);c:line3({11,-2,17},{16,-2,15},'ivory4')
  c:line3({-14,-4,14},{-14,4,14},'jade4')
  c:line3({-4,-10,16},{1,-11,16},'jade2')
  c:line3({-4,10,16},{1,11,16},'jade2')
  c:line3({14,-2,14},{14,2,14},'steel0')
  c:line3({-10,-2,23},{-6,-2,23},'amber4')
end

out.reedguard=function(c,phase)
  assembly_feet(c,phase,10,11,12,false)
  c:layer(3);c:box(-3,0,11,11,9,9,'jade',4)
  c:layer(4);cabin(c,-4,0,20,6,5,8,'ivory')
  c:cylinder(-9,sign(c.co)*6,18,4,9,'amber')
  c:layer(5)
  c:box(10,0,8,3,10,19,'reed',2)
  c:box(10,0,25,3,8,3,'jade',2)
  for s=-7,7,4 do c:line3({13,s,10},{13,s,25},'reed0',2) end
  for z=13,22,4 do c:line3({13,-8,z},{13,8,z},'reed3',2) end
  c:beam({-4,-6,25},{9,-6,29},4,'steel')
  c:box(9,-6,26,4,3,5,'jade',2)
  c:layer(6);c:line3({10,-8,28},{10,4,28},'jade4')
  c:line3({13,-5,11},{13,-5,15},'reed0')
  c:line3({13,4,20},{13,7,20},'reed4')
  c:line3({-10,sign(c.co)*7,22},{-10,sign(c.co)*7,26},'amber3')
end

out.loom=function(c,phase)
  assembly_feet(c,phase,11,12,12,false)
  c:layer(3);c:box(0,0,11,9,8,6,'jade',3)
  local near=sign(c.co)
  local arch={{-11,15},{-11,24},{-7,34},{0,39},{7,34},{11,24},{11,15}}
  for _,side in ipairs({-near,near}) do
    c:layer(side==near and 5 or 3)
    for i=1,#arch-1 do
      c:beam({arch[i][1],side*6,arch[i][2]},{arch[i+1][1],side*6,arch[i+1][2]},5,'jade')
    end
  end
  c:layer(4)
  c:cylinder(0,0,19,5,12,'amber')
  c:cylinder(0,0,18,6,2,'steel')
  c:cylinder(0,0,31,6,2,'steel')
  c:beam({0,0,32},{0,0,38},3,'steel')
  c:box(0,0,37,3,6,3,'jade',2)
  c:layer(5);c:beam({-5,near*8,17},{8,near*8,24},4,'steel')
  c:cylinder(7,near*8,23,3,5,'reed')
  c:layer(6);c:line3({-2,-3,22},{-2,-3,28},'amber4')
  c:line3({-7,near*6,33},{-5,near*6,35},'reed2',2)
  c:line3({7,near*6,33},{5,near*6,35},'reed2',2)
  c:line3({-4,near*6,38},{0,near*6,39},'jade4')
end
return out
