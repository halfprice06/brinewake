-- Root's localized machine animation. Rigid buildings and grounded bases stay
-- fixed; only the device that causes the motion is redrawn in each key.
local out={}
local function poly(c,p,col)
  local q={};for _,v in ipairs(p) do q[#q+1]={c.cx+v[1],c.cy+v[2]} end;c:poly(q,col)
end
local function line(c,x,y,xx,yy,col,w) c:line({c.cx+x,c.cy+y},{c.cx+xx,c.cy+yy},col,w) end
local function rect(c,x,y,w,h,col) poly(c,{{x,y},{x+w,y},{x+w,y+h},{x,y+h}},col) end
local function oval(c,x,y,rx,ry,col) c:ellipse(c.cx+x,c.cy+y,rx,ry,col) end
local function fan(c,x,y,r,phase)
  oval(c,x,y,r,math.ceil(r/2),'steel0')
  for i=0,2 do
    local a=(phase/4+i)*2*math.pi/3
    local ca,sa=math.cos(a),math.sin(a)
    poly(c,{{x,y},{x+ca*(r-1)-sa,y+sa*(r-1)/2+ca},
      {x+ca*r+sa,y+sa*r/2-ca/2}},ca<0 and 'steel3' or 'steel2')
  end
  oval(c,x,y,1,1,'steel4')
end
local function pennant(c,phase)
  -- One fixed halyard; the free end and the inner fold reach their extremes
  -- at different times. A small double-stripe identifies the Union.
  local old_y=c.cy;c.cy=c.cy+5
  local tip=({0,-2,-1,1})[phase+1]
  local fold=({1,0,-1,0})[phase+1]
  poly(c,{{4,-98},{9,-100+fold},{14,-99},{24,-100+tip},
    {20,-95+tip},{24,-91+tip},{14,-92},{9,-91+fold},{4,-93}},'rust0')
  poly(c,{{4,-98},{9,-99+fold},{14,-98},{22,-99+tip},
    {18,-95+tip},{22,-92+tip},{14,-93},{9,-92+fold},{4,-94}},'rust2')
  line(c,5,-98,9,-99+fold,'rust3')
  line(c,9,-99+fold,9,-93+fold,'ivory2')
  line(c,12,-98,12,-93,'ivory2')
  line(c,4,-98,4,-93,'steel1')
  c.cy=old_y
end
out.union_hq=function(c,p)
  c:layer(6)
  fan(c,-33,-43,5,p)
  pennant(c,p)
end
out.assembly_hq=function(c,p)
  c:layer(6)
  -- Bubbles travel inside the amber sight glass; the reservoir's shell and
  -- highlights stay still. Opaque glazing and belt masks protect the structure.
  local amber={'amber1','amber2','amber3','amber4'}
  local y=({-26,-31,-37,-43})[p+1]
  c:overpaint({{c.cx-3,c.cy-46},{c.cx+8,c.cy-46},{c.cx+8,c.cy-22},{c.cx-3,c.cy-22}},'amber2',amber)
  c:overpaint({{c.cx-2,c.cy+y},{c.cx+2,c.cy+y-1},{c.cx+4,c.cy+y+1},{c.cx+2,c.cy+y+3},{c.cx-1,c.cy+y+2}},'amber3',amber)
  c:overpaint({{c.cx+4,c.cy+y+9},{c.cx+7,c.cy+y+8},{c.cx+8,c.cy+y+10},{c.cx+5,c.cy+y+11}},'amber3',amber)
  -- A woven pennon fastened to the right vault. The hem lags the upper fold.
  local dx=({0,1,2,1})[p+1];local hem=({1,0,1,2})[p+1]
  line(c,37,-53,48,-53,'reed0',2)
  poly(c,{{47,-53},{54,-51},{53+dx,-42},{54+hem,-31},{50+hem,-34},{46+hem,-32},{47+dx,-43}},'reed0')
  poly(c,{{48,-52},{53,-50},{52+dx,-42},{53+hem,-33},{50+hem,-36},{47+hem,-34},{48+dx,-43}},'reed2')
  line(c,49,-50,51,-49,'reed3');line(c,49+dx,-42,51+dx,-43,'jade1',2)
end
out.union_works=function(c,p)
  c:layer(6);fan(c,-15,-78,6,p)
  -- An exposed crank and push rod within the recessed shutter bay.
  poly(c,{{-29,-29},{-7,-18},{-7,-9},{-29,-20}},'steel0')
  line(c,-27,-26,-9,-17,'steel1')
  local shift=({0,2,4,2})[p+1]
  line(c,-23,-21,-12+shift,-15+shift/2,'steel3',2)
  oval(c,-23,-23,4,4,'steel1');oval(c,-23,-23,3,3,'rust2')
  local x=({-25,-23,-21,-23})[p+1];local y=({-23,-25,-23,-21})[p+1]
  line(c,-23,-23,x,y,'steel4');oval(c,x,y,1,1,'steel3')
  poly(c,{{-14+shift,-18+shift/2},{-8+shift,-15+shift/2},{-8+shift,-10+shift/2},{-14+shift,-13+shift/2}},'steel2')
  line(c,-14+shift,-18+shift/2,-8+shift,-15+shift/2,'steel3')
end
out.assembly_works=function(c,p)
  c:layer(6)
  -- The shuttle traverses a fixed reed warp. Woven material bends locally,
  -- while the canopy, pressure drums and recent foundation remain anchored.
  poly(c,{{-26,-26},{-4,-15},{-4,-7},{-26,-18}},'reed0')
  for i=0,6 do line(c,-24+i*3,-24+i*1.5,-24+i*3,-18+i*1.5,'reed2') end
  local x=({-24,-16,-7,-15})[p+1];local y=-20+(x+24)/2
  poly(c,{{x-2,y},{x+3,y-1},{x+6,y+2},{x+1,y+3}},'jade0')
  line(c,x-1,y,x+3,y+1,'jade3',2)
  rect(c,4,-28+p%2,3,5,'amber3')
end
out.condenser=function(c,p)
  c:layer(6)
  -- Two staggered puffs evolve from the same pipe mouth. They widen, drift
  -- and fragment; the base structure is never translated or scaled.
  for i=0,1 do
    local age=(p*2+i*3)%8
    local x=-17+math.floor(age*age/13);local y=-72-age*2
    if age<5 then
      local r=age<2 and 2 or 3
      oval(c,x,y,r,math.max(1,r-1),age<2 and 'steel3' or 'steel2')
      oval(c,x-2,y-2,r-1,r-1,'steel3')
      line(c,x-2,y-3,x,y-3,'steel4')
    else
      line(c,x-3,y,x-1,y-1,'steel2')
      line(c,x+2,y-3,x+4,y-3,'steel2')
    end
  end
  -- Liquid movement stays inside the framed glass.
  local glass={'glass1','glass2','glass3'}
  c:overpaint({{c.cx+9,c.cy-31},{c.cx+20,c.cy-31},{c.cx+20,c.cy-13},{c.cx+9,c.cy-13}},'glass1',glass)
  local y=({-29,-27,-25,-27})[p+1]
  c:overpaint({{c.cx+9,c.cy+y},{c.cx+20,c.cy+y},{c.cx+20,c.cy+y+1},{c.cx+9,c.cy+y+1}},'glass3',glass)
  c:overpaint({{c.cx+10,c.cy+y+2},{c.cx+13,c.cy+y+2},{c.cx+13,c.cy-14},{c.cx+10,c.cy-14}},'glass2',glass)
end
return out
