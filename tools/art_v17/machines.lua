-- Root-authored v17 machine bodies: the two transports, drawn in the v4
-- painter's projection and materials so they stand beside the rest.
--
--   Assembly:  barge   a flat jade hull with a reed cargo well, an ivory
--                      wheelhouse aft and a stern paddle that turns as it
--                      sails; the water is the field's, the barge never
--                      stands on dry ground
--   Union:     lifter  a rust gondola under twin ivory gasbags, rigged with
--                      steel lines, a stern screw that spins in flight; the
--                      body floats eleven units above its ground shadow
local out={}
local function sign(n) return n>=0 and 1 or -1 end

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

-- BARGE: the hull is a long low shell; the deck carries a reed cargo well
-- under an ivory tarp bar; the wheelhouse sits aft; the paddle turns.
out.barge=function(c,phase)
  local near=sign(c.co)
  c:shadow(16,6)
  c:layer(2)
  local a=(phase or 0)*math.pi/4
  c:disc({-19,0,4},{1,0,0},{0,0,1},5,'steel0')
  c:disc({-19,0,4},{1,0,0},{0,0,1},4,'jade1')
  for k=0,3 do
    local t=a+k*math.pi/2
    c:line3({-19,0,4},{-19+math.cos(t)*4,0,4+math.sin(t)*4},'steel3')
  end
  c:dot(-19,0,4,1,'steel')
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
  c:line3({15,0,5},{15,0,16},'reed2');c:dot(15,0,16,2,'amber')
  for _,f in ipairs({-6,2,10}) do c:dot(f,near*9,5,1,'steel') end
  c:cylinder(-13,-near*3,12,1,3,'steel')
  c:layer(6)
  c:line3({-12,near*8,4},{12,near*8,4},'jade3')
  c:line3({4,-4,7},{9,-4,7},'ivory4')
end

-- LIFTER: the gondola and its cabin hang from twin gasbags; rigging holds
-- them; the screw spins astern.  Everything floats above the anchor.
local LIFT=11
out.lifter=function(c,phase)
  local near=sign(c.co)
  c:shadow(13,5)
  c:layer(2)
  local a=(phase or 0)*math.pi/3
  c:disc({-12,0,LIFT+3},{0,1,0},{0,0,1},4,'steel0')
  for k=0,2 do
    local t=a+k*2*math.pi/3
    c:line3({-12,0,LIFT+3},{-12,math.cos(t)*4,LIFT+3+math.sin(t)*4},'steel3')
  end
  for _,s in ipairs({-5,5}) do
    c:box(-12,s,LIFT+13,4,2,3,'ivory',1)
  end
  c:layer(3)
  c:box(0,0,LIFT,9,4,4,'rust',2)
  c:line3({-9,near*4,LIFT},{9,near*4,LIFT},'rust0')
  c:layer(4)
  cabin(c,5,0,LIFT+4,3,3,5,'ivory')
  for _,s in ipairs({-5,5}) do
    c:box(0,s,LIFT+12,9,3,5,'ivory',1)
    c:box(12,s,LIFT+13,4,2,3,'ivory',1)
  end
  c:layer(5)
  for _,p in ipairs({{-8,-3},{8,-3},{-8,3},{8,3}}) do
    c:line3({p[1],p[2],LIFT+4},{p[1],p[2]*1.6,LIFT+12},'steel1')
  end
  c:cylinder(-4,near*3,LIFT+4,1,3,'steel')
  c:dot(9,0,LIFT+1,1,'amber')
  c:layer(6)
  c:line3({-8,near*4,LIFT+3},{8,near*4,LIFT+3},'rust3')
  c:line3({-8,-near*3,LIFT+17},{8,-near*3,LIFT+17},'ivory4')
end

-- Material overpaint, as the v4 finish: contours attached to surfaces.
local ivory={'ivory1','ivory2','ivory3','ivory4'}
local jade={'jade1','jade2','jade3','jade4'}
local rust={'rust1','rust2','rust3'}
out.finish={}
out.finish.barge=function(c) c:layer(6);c:overpaint3({{-12,-9,1},{12,-9,1},{12,-9,4},{-12,-9,4}},'jade2',jade);c:overpaint3({{-13,0,6},{-5,0,6},{-5,0,12},{-13,0,12}},'ivory2',ivory) end
out.finish.lifter=function(c) c:layer(6);c:overpaint3({{-9,-4,LIFT+1},{9,-4,LIFT+1},{9,-4,LIFT+3},{-9,-4,LIFT+3}},'rust1',rust);c:overpaint3({{-9,-8,LIFT+13},{9,-8,LIFT+13},{9,-8,LIFT+16},{-9,-8,LIFT+16}},'ivory2',ivory) end
return out
