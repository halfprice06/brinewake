-- Root pixel construction: old civic surfaces and a few living shore vignettes.
local root=assert(app.params.root)
local h=dofile(root..'/tools/art_v8/common.lua')
local M={}
local function q(c,u,v) return {c.cx+16*(u-v),c.cy+8*(u+v)} end
local function quad(c,u,v,w,d,col)
 c:poly({q(c,u,v),q(c,u+w,v),q(c,u+w,v+d),q(c,u,v+d)},col)
end
function M.archaeology(c,lane,wet,phase)
 c:layer(3)
 -- Break all motifs into large remnants. Crossbars survive in the lane tiles
 -- beyond this transparent footprint, retaining the original route grammar.
 if lane=='north' then
  quad(c,-1.9,-1.8,3.8,3.6,'earth1')
  quad(c,-1.7,-1.6,3.4,3.2,'salt2')
  quad(c,-1.4,-1.3,2.8,2.6,'wet1')
  quad(c,-1.15,-1.05,2.3,2.1,'salt2')
  quad(c,-.75,-.7,.4,1.4,'wet1');quad(c,.1,-.7,.4,1.4,'wet1')
  -- Two obsolete loading rails, split into distinct surviving segments.
  c:layer(5)
  for _,v in ipairs({-1.65,-1.42}) do
   c:line(q(c,-2.1,v),q(c,-.4,v),'earth0')
   c:line(q(c,-.15,v),q(c,1.8,v),'earth0')
   c:line(q(c,-1.7,v+.04),q(c,-.6,v+.04),'ivory1')
  end
 else
  -- The ropewalk uses long paired troughs and worn sleepers, not another
  -- ornamental square. Its historical purpose reads at a different scale.
  quad(c,-2,-1.5,4,3,'earth1')
  quad(c,-1.8,-1.3,3.6,2.6,'salt1')
  for _,v in ipairs({-.8,.45}) do
   quad(c,-1.85,v,3.65,.3,'wet0');quad(c,-1.7,v+.15,3.4,.15,'earth0')
   for _,u in ipairs({-1.35,-.3,.8,1.4}) do quad(c,u,v-.12,.15,.63,'ivory1') end
  end
  c:layer(5)
  for _,u in ipairs({-1.5,1.4}) do
   local p=q(c,u,1.1);c:ellipse(p[1],p[2],4,2,'earth0')
   c:line({p[1]-2,p[2]-1},{p[1]+1,p[2]-1},'ivory1')
  end
 end
 c:layer(6)
 -- Silt takes whole corners. No uniformly scattered highlights.
 h.poly(c,{{-10,27},{8,20},{24,18},{31,22},{17,28},{5,34}},'salt1')
 h.poly(c,{{-52,-3},{-36,-10},{-29,-7},{-22,-9},{-17,-6},{-33,2},{-45,2}},'salt1')
 h.poly(c,{{5,-30},{18,-23},{17,-20},{9,-18},{-1,-22}},'salt2')
 for _,p in ipairs({{-21,20},{-15,22},{32,4},{35,7}}) do
  h.poly(c,{{p[1],p[2]},{p[1]+4,p[2]-2},{p[1]+7,p[2]},{p[1]+3,p[2]+2}},'earth3')
 end
 h.line(c,-41,4,-34,1,'earth1');h.line(c,-32,1,-28,2,'earth1')
 if wet then
  local palette=dofile(root..'/tools/art_v5/painter.lua').palette
  local function rgba(hex)return app.pixelColor.rgba(tonumber(hex:sub(1,2),16),tonumber(hex:sub(3,4),16),tonumber(hex:sub(5,6),16),255)end
  local map={earth0='sea0',earth1='sea1',salt1='sea1',salt2='sea2',wet0='sea0',wet1='sea1',ivory1='sea2',earth3='sea2'}
  local lookup={};for a,b in pairs(map)do lookup[rgba(palette[a])]=rgba(palette[b])end
  for _,im in pairs(c.images)do for it in im:pixels()do local value=lookup[it()];if value then it(value)end end end
  c:layer(6)
  for i,p in ipairs({{-38,-5,14},{-16,9,11},{14,-9,17},{23,17,9}}) do
   local shift=({0,1,2,1})[(phase+i)%4+1]
   h.line(c,p[1]+shift,p[2],p[1]+p[3]+shift,p[2],'sea2')
   if i%2==0 then h.line(c,p[1]+3+shift,p[2]-1,p[1]+7+shift,p[2]-1,'sea3')end
  end
 end
end
local function bird(c,x,y,pose)
 -- Grounded feet and two clear wing extremes; head remains attached.
 c:layer(1);h.oval(c,x+1,y+1,5,2,'cast')
 c:layer(5)
 local up=pose==2 or pose==3
 local lift=up and (pose==2 and 5 or 9) or 0
 h.line(c,x-2,y-lift,x-2,y-4-lift,'reed1');h.line(c,x+1,y-lift,x+1,y-4-lift,'reed1')
 h.poly(c,{{x-5,y-8-lift},{x-2,y-10-lift},{x+3,y-9-lift},{x+5,y-6-lift},{x+1,y-3-lift},{x-4,y-4-lift}},'ivory2')
 h.line(c,x-4,y-8-lift,x+1,y-8-lift,'ivory3',2)
 local head=pose==1 and 2 or 0
 h.rect(c,x+2,y-12-lift+head,4,4,'ivory3')
 h.line(c,x+5,y-10-lift+head,x+8,y-9-lift+head,'reed2')
 h.rect(c,x+4,y-11-lift+head,1,1,'ink')
 if up then
  local z=pose==2 and 10 or -2
  h.poly(c,{{x-1,y-7-lift},{x-11,y-8-lift-z},{x-13,y-5-lift-z},{x-4,y-3-lift}},'steel2')
  h.line(c,x-3,y-6-lift,x-11,y-7-lift-z,'ivory3',2)
 end
end
function M.coast(c,kind,p)
 if kind=='birds' then
  bird(c,-12,0,({0,0,1,0,2,3,2,0})[p+1])
  bird(c,10,4,({0,1,0,0,0,2,3,0})[p+1])
 elseif kind=='crab' then
  c:layer(3)
  h.poly(c,{{-17,2},{-10,-4},{1,-5},{10,0},{7,5},{-9,6}},'earth1')
  h.poly(c,{{-9,1},{-5,-2},{2,-1},{6,2},{1,4},{-7,4}},'contact')
  c:layer(5)
  local x=({-14,-12,-9,-6,-3,-6,-9,-12})[p+1]
  -- The whole animal passes behind one entry plane. Do not leave an exposed
  -- claw floating on top of the rock while its body has entered the crevice.
  local pixel=c.pixel
  c.pixel=function(self,xx,yy,color)
   if xx<self.cx-7 then pixel(self,xx,yy,color) end
  end
  if p~=4 then
   h.oval(c,x,0,4,2,'rust1');h.line(c,x-2,-2,x+1,-2,'rust3')
   for _,s in ipairs({-1,1}) do
    h.line(c,x+s*3,0,x+s*6,2+p%2,'reed1')
    h.line(c,x+s*3,-1,x+s*5,-4,'rust2')
   end
   h.rect(c,x+1,-3,1,1,'ink')
  end
  c.pixel=pixel
 else
  c:layer(1);h.oval(c,0,1,19,4,'cast')
  -- A pulse travels across the stems; root contacts stay fixed throughout.
  for i=-3,3 do
   local x=i*5;local delay=(p-i+8)%8
   local bend=({0,0,1,2,3,2,1,0})[delay+1]
   local height=20+(i*i+2)%9
   c:layer(3);h.line(c,x,0,x+1,-height/2,'moss0',2)
   c:layer(5);h.line(c,x+1,-height/2,x+bend,-height,'moss1')
   h.line(c,x+bend,-height,x+bend+1,-height-5,'reed2',2)
   h.line(c,x,-4,x-3+bend,-12,'moss1')
  end
 end
end
return M
