-- Root-authored pressure anatomy and workshop fittings.
local root=assert(app.params.root)
local previous=dofile(root..'/tools/art_v8/units.lua')
local h=dofile(root..'/tools/art_v8/common.lua')
local M={}
local function union(n)return n=='riveter' or n=='bulwark' or n=='sounder'end
function M.pressure(c,name,p,cool)
 local steel=union(name);local near=c.co>=0 and 1 or -1
 local f,s,z=-9,near*6,21
 if name=='sounder' then f,s,z=-10,near*3,18 end
 if name=='skipper' then f,s,z=-8,0,22 end
 if name=='loom' then f,s,z=0,0,28 end
 c:layer(4)
 -- Open steel louvers or a reed-bound expansion sleeve occupy the machinery's
 -- pressure housing. They never translate the chassis or its foot contacts.
 if steel then
  c:box(f,s,z-5,4,3,7,'steel',1)
  for dz=0,4,2 do c:line3({f-3,s+near*3,z-4+dz},{f+3,s+near*3,z-4+dz},cool and 'steel1' or 'amber2')end
  c:line3({f-3,s+near*3,z+1},{f+3,s+near*3,z+1},'steel3')
 else
  local swell=not cool and p%3==1 and 1 or 0
  c:cylinder(f,s,z-7,3+swell,7,cool and 'reed' or 'amber')
  c:cylinder(f,s,z-8,4,1,'steel');c:cylinder(f,s,z,4,1,'steel')
  c:line3({f-3,s,z-6},{f-3,s,z-1},'reed1')
  c:line3({f+3,s,z-6},{f+3,s,z-1},'reed2')
 end
 c:layer(5)
 local valve={f-2,s,z+2};local endp={f-2+(cool and 1 or p%2*2),s-3,z+2}
 c:beam(valve,endp,2,steel and 'rust' or 'reed')
 if not cool then
  -- One short jet rooted in the open valve, followed by a larger cool lobe.
  local mouth=c:point(f-3,s,z+2);local tip=c:point(f-9-p*2,s,z+3+p)
  c:line(mouth,tip,'steel2',2)
  c:ellipse(tip[1],tip[2],2+p%2,2,'steel3')
  if p<2 then c:line({mouth[1]-1,mouth[2]-1},{mouth[1]+1,mouth[2]-1},'foam')end
  if p==3 then c:line({tip[1]+3,tip[2]-2},{tip[1]+5,tip[2]-2},'steel2')end
  -- A dark mechanical shutter on the firing housing reads "weapons off".
  local barrel=name=='riveter' and {21,0,18} or name=='bulwark' and {15,0,17}
   or name=='sounder' and {11,0,17} or name=='skipper' and {16,0,14}
   or name=='reedguard' and {12,-6,28} or {7,near*8,27}
  c:layer(6);c:box(barrel[1],barrel[2],barrel[3]-2,2,3,4,'steel',1)
  c:line3({barrel[1],barrel[2]-2,barrel[3]+2},{barrel[1],barrel[2]+2,barrel[3]+2},'steel0')
 else
  -- Cooling stays subdued and is visibly distinct from the sustained jet.
  local a=c:point(f-5-p*2,s,z+3+p*2)
  c:line({a[1]-1,a[2]},{a[1]+1,a[2]},p==0 and 'steel2' or 'steel1')
 end
end
function M.loom(c,p)
 -- Reuse root's deployed structural construction; move only the suspended
 -- chamber and working binding. Its feet and the outer arch remain anchored.
 local point=c.point
 local shift=({-2,-3,2,3,1,0})[p+1]
 c.point=function(self,f,s,z)
  z=z or 0
  if self.image==self.images['cabins and shells'] and z>=18 then
   f=f+shift;z=z+(p<2 and 1 or 0)
  end
  return point(self,f,s,z)
 end
 previous.deploy(c,'loom',2);c.point=point
 c:layer(6)
 local near=c.co>=0 and 1 or -1
 c:line3({-8,near*9,36},{shift,0,32},p<2 and 'reed4' or 'reed2')
 c:line3({8,near*9,36},{shift,0,32},p<2 and 'reed3' or 'reed1')
 if p==2 then
  c:box(11,near*7,35,3,2,4,'amber',1)
  c:line3({7,near*7,34},{12,near*7,36},'amber3',2)
 elseif p==3 then
  local a=c:point(8,near*8,34);c:ellipse(a[1],a[2],3,2,'steel3')
 elseif p==4 then
  local a=c:point(10,near*8,36);c:line({a[1]-2,a[2]},{a[1]+2,a[2]},'steel2')
 end
 local muzzle=c:point(11,near*7,35);c.muzzle={muzzle[1]-c.cx,muzzle[2]-c.cy}
end
function M.cargo(c,name,p)
 local steel=name=='hook';local near=c.co>=0 and 1 or -1
 c:layer(5)
 if steel then
  -- Two lipped side baskets are bolted to the rear chassis, clear of the
  -- crane cable and original front cargo animation.
  for _,s in ipairs({-near*10,near*10}) do
   c:box(-8,s,8,5,3,5,'steel',1)
   c:line3({-13,s-near*2,13},{-3,s-near*2,13},'ivory2')
   c:line3({-12,s+near*3,10},{-4,s+near*3,10},'rust3')
  end
 else
  local sway=({0,0,1,0})[p+1]
  c:box(-6,near*8+sway,8,6,3,5,'reed',1)
  for f=-10,-2,4 do c:line3({f,near*11+sway,8},{f,near*11+sway,13},'reed0')end
  c:beam({-9,near*5,17},{-11,near*8+sway,13},1,'steel')
  c:beam({-2,near*5,17},{0,near*8+sway,13},1,'steel')
 end
end
function M.workshop(c,faction,doctrine,p)
 local steel=faction=='union';local x,y=steel and -36 or -32,-22
 c:layer(5)
 if doctrine=='hauling' then
  -- A rack of the same cargo cradles beside the shipping door.
  h.block(c,x,y,11,14,9,steel and 'steel' or 'reed')
  h.line(c,x-10,y-10,x+12,y-11,steel and 'ivory2' or 'reed3',2)
  for i=0,2 do h.line(c,x-8+i*5,y-9-i*2.5,x-8+i*5,y-3-i*2.5,steel and 'rust2' or 'reed0')end
  h.line(c,x+11,y-13,x+11,y-4,steel and 'rust3' or 'jade3',2)
 else
  -- A governor in a fixed bracket; arms rotate but mounting holes do not.
  h.block(c,x,y,9,12,9,steel and 'steel' or 'jade')
  h.line(c,x+1,y-11,x+1,y-26,'steel3',2)
  local arms=({7,4,1,4})[p+1]
  h.line(c,x+1,y-23,x-arms,y-19,'reed2',2)
  h.line(c,x+1,y-23,x+arms+2,y-19,'reed3',2)
  h.oval(c,x-arms,y-18,2,2,'rust3');h.oval(c,x+arms+2,y-18,2,2,'amber3')
  h.line(c,x-7,y-10,x-3,y-12,'steel4')
 end
end
return M
