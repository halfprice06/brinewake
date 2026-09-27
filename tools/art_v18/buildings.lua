-- Root-authored v18 buildings in the v4 structures vocabulary: the v16
-- Drydocks unchanged in form but drawn inside a colour budget, and the
-- Palisades redrawn as wall segments (v16 drew a 12x12 cube that read as a
-- crate). A segment runs the cell's up-right diagonal from post to post, so
-- a line of them along that axis joins into one wall. No shadow is
-- drawn beneath a building: the contact layer stays empty: screen-space
-- 2:1 planes composed with the front corner at (x,y) below the anchor.
--   Drydock (160x144, anchor 80,128): a long shed open at the front over a
--     slipway cradle, a gantry crane, and the faction's pressure plant.
--   Palisade (64x64, anchor 32,50): one wall segment between posts, steel
--     plate for the Union, a bound reed hurdle for the Assembly.
local out={}
local function point(c,x,y) return {c.cx+x,c.cy+y} end
local function poly(c,p,color)
  local q={};for _,v in ipairs(p) do q[#q+1]=point(c,v[1],v[2]) end;c:poly(q,color)
end
local function line(c,x,y,xx,yy,color,width) c:line(point(c,x,y),point(c,xx,yy),color,width) end
local function rect(c,x,y,w,h,color) poly(c,{{x,y},{x+w,y},{x+w,y+h},{x,y+h}},color) end
local function ellipse(c,x,y,rx,ry,col) c:ellipse(c.cx+x,c.cy+y,rx,ry,col) end
local function block(c,x,y,w,d,h,mat)
  poly(c,{{x-w,y-w/2-h},{x,y-h},{x,y},{x-w,y-w/2}},mat..'2')
  poly(c,{{x,y-h},{x+d,y-d/2-h},{x+d,y-d/2},{x,y}},mat..'1')
  poly(c,{{x-w,y-w/2-h},{x-w+d,y-(w+d)/2-h},{x+d,y-d/2-h},{x,y-h}},mat..'3')
  line(c,x-w,y-w/2,x,y,mat..'0')
  line(c,x,y,x+d,y-d/2,mat..'0')
  line(c,x-w,y-w/2-h,x,y-h,mat..'4')
  line(c,x,y-h,x+d,y-d/2-h,mat..'2')
  line(c,x,y-h+1,x,y-1,mat..'0')
end
local function panel(c,x,y,length,h,side,col,rim)
  poly(c,{{x,y},{x+side*length,y-length/2},{x+side*length,y-length/2-h},{x,y-h}},col)
  if rim then
    line(c,x,y-h,x+side*length,y-length/2-h,rim)
    line(c,x,y,x+side*length,y-length/2,rim)
  end
end
local function window(c,x,y,length,h,side)
  panel(c,x,y,length,h,side,'glass0','steel0')
  panel(c,x+side,y-2,length-2,h-3,side,'glass1')
  panel(c,x+side*2,y-h+3,length-4,1,side,'glass3')
end
local function cylinder(c,x,y,r,h,mat)
  ellipse(c,x,y,r,math.floor(r/2),mat..'0')
  rect(c,x-r,y-h,2*r,h,mat..'1')
  rect(c,x-r+2,y-h,r,h,mat..'2')
  rect(c,x-r+3,y-h,math.max(2,math.floor(r/2)),h,mat..'3')
  ellipse(c,x,y-h,r,math.floor(r/2),mat..'3')
  line(c,x-r+2,y-h-1,x-1,y-h-math.floor(r/2)+1,mat..'4')
end
local function pipe(c,x,y,xx,yy,mat,width)
  line(c,x+1,y+1,xx+1,yy+1,'ink',(width or 3)+1)
  line(c,x,y,xx,yy,mat..'1',width or 3)
  line(c,x,y,xx,yy,mat..'3')
end
local function footing(c,x,y,w,d)
  block(c,x+3,y+2,w+5,d+5,3,'earth')
  block(c,x,y-1,w,d,4,'steel')
end
local function ribbed_roof(c,x,y,w,d,mat)
  for a=6,w-3,7 do
    line(c,x-a,y-a/2,x-a+d,y-(a+d)/2,mat..'1')
    line(c,x-a+1,y-a/2-1,x-a+d-1,y-(a+d)/2-1,mat..'3')
  end
end

-- Foundation specs (front corner, ground lengths) for construction sites.
out.spec={
  union_drydock={x=2,y=5,w=43,d=44,h=36},
  assembly_drydock={x=0,y=4,w=40,d=42,h=34},
  union_palisade={x=-11,y=6,w=4,d=26,h=13},
  assembly_palisade={x=-11,y=6,w=4,d=26,h=12},
}

-- The Union Drydock: a riveted steel hall, its front open over a cradle on
-- rails, a gantry crane with a chain block, a rust water tank and a stack.
out.union_drydock=function(c,phase)
  c:layer(2)
  footing(c,2,5,49,48)
  block(c,2,0,43,44,30,'rust')
  block(c,2,-30,44,45,4,'ivory')
  ribbed_roof(c,2,-34,44,45,'ivory')
  c:layer(3)
  -- The open front: a dark bay under a steel lintel, the cradle inside.
  poly(c,{{-30,-14},{0,-29},{0,-3},{-30,12}},'ink')
  poly(c,{{-27,-13},{-3,-25},{-3,-8},{-27,4}},'steel0')
  block(c,2,-32,34,2,4,'steel')
  -- The cradle on its rails.
  line(c,-26,2,-4,-9,'steel1',2);line(c,-24,6,-2,-5,'steel1',2)
  block(c,-8,-6,10,6,5,'rust')
  block(c,-8,-11,10,6,2,'ivory')
  -- Rear plant: a tank and a stack.
  cylinder(c,32,-38,7,14,'rust')
  cylinder(c,32,-52,8,3,'steel')
  cylinder(c,12,-46,4,22,'steel')
  ellipse(c,12,-69,3,2,'steel0')
  c:layer(4)
  window(c,8,-16,28,9,1)
  panel(c,8,-4,10,8,1,'ivory1','ivory3')
  c:layer(5)
  -- The gantry: two legs, a beam, and a chain block whose hook moves.
  pipe(c,-34,-2,-34,-52,'steel',3)
  pipe(c,-2,-18,-2,-60,'steel',3)
  pipe(c,-34,-52,-2,-60,'rust',4)
  local drop=phase and ({0,3,6,3})[phase+1] or 0
  line(c,-18,-56,-18,-40+drop,'ink')
  rect(c,-20,-40+drop,5,4,'steel2');line(c,-19,-36+drop,-16,-36+drop,'rust3')
  c:layer(6)
  for _,p in ipairs({{-46,-30},{-22,-19},{30,-36},{44,-42}}) do rect(c,p[1],p[2],2,1,'ivory4') end
  line(c,-44,-20,-39,-18,'rust0')
  line(c,36,-22,41,-25,'rust1')
  rect(c,18,-22,2,5,'rust0')
  line(c,-12,-27,-3,-31,'ivory2')
end

-- The Assembly Drydock: a woven vault over a slip, reed-bound gantry
-- poles, an amber pressure vessel and a bobbin winch that turns.
out.assembly_drydock=function(c,phase)
  c:layer(2)
  footing(c,0,4,46,48)
  poly(c,{{-46,-8},{-45,-36},{-34,-58},{-14,-74},{6,-78},{28,-64},{44,-40},{47,-12},{22,0},{-8,6}},'jade0')
  c:layer(3)
  poly(c,{{-46,-8},{-45,-34},{-36,-56},{-18,-72},{4,-78},{12,-72},{-8,-58},{-24,-40},{-31,-12}},'jade3')
  poly(c,{{4,-78},{26,-64},{42,-42},{47,-24},{47,-12},{37,-17},{32,-40},{18,-60},{6,-70}},'jade2')
  poly(c,{{6,-70},{18,-60},{32,-40},{37,-17},{30,-12},{24,-40},{12,-56},{0,-63}},'jade0')
  -- The open slip at the front, a woven lintel above it.
  poly(c,{{-30,-12},{0,-27},{0,-3},{-30,12}},'ink')
  poly(c,{{-27,-11},{-3,-23},{-3,-7},{-27,5}},'jade0')
  line(c,-30,-12,0,-27,'reed3',2)
  block(c,-8,-6,10,6,5,'reed')
  line(c,-26,2,-4,-9,'reed1',2);line(c,-24,6,-2,-5,'reed1',2)
  block(c,29,-8,10,14,24,'reed')
  for i=0,3 do panel(c,31,-12-i*5,10,2,1,'reed1') end
  c:layer(4)
  cylinder(c,24,-24,9,24,'amber')
  cylinder(c,24,-47,10,3,'steel')
  cylinder(c,24,-25,10,3,'steel')
  pipe(c,23,-51,23,-66,'steel',3)
  c:layer(5)
  -- Reed-bound gantry poles with a bobbin winch that turns.
  pipe(c,-34,-2,-34,-50,'reed',3)
  pipe(c,-2,-18,-2,-58,'reed',3)
  pipe(c,-34,-50,-2,-58,'jade',4)
  local turn=phase or 0
  ellipse(c,-18,-54,4,2,'steel0')
  local a=turn*math.pi/2
  line(c,-18,-54,-18+math.floor(math.cos(a)*3+.5),-54+math.floor(math.sin(a)*1.5+.5),'steel3')
  line(c,-18,-52,-18,-42+({0,2,4,2})[turn+1],'ink')
  rect(c,-20,-42+({0,2,4,2})[turn+1],5,3,'reed3')
  c:layer(6)
  line(c,-40,-30,-35,-43,'jade4')
  line(c,-24,-62,-16,-71,'jade3')
  rect(c,-4,-67,3,3,'reed4')
  line(c,-28,-16,-23,-23,'reed0')
  line(c,34,-20,39,-23,'reed3')
end

-- Palisades: one wall segment along the up-right diagonal, 30 units post
-- to post, thick enough to shade as a wall and tall enough to read as a
-- barrier. Both posts stand on their own footing pads.
out.union_palisade=function(c)
  c:layer(2)
  block(c,-11,8,6,28,2,'earth')
  c:layer(3)
  block(c,-11,6,4,26,13,'steel')
  -- Bolted ivory plates on the visible face, a rust rail along the middle.
  panel(c,-10,4,11,8,1,'ivory1','ivory2');panel(c,3,-3,11,8,1,'ivory1','ivory2')
  line(c,-10,-1,15,-13,'rust2',1)
  c:layer(5)
  block(c,-14,8,4,4,16,'steel');block(c,12,-5,4,4,16,'steel')
  for i=0,3 do local x=-8+i*6;local y=1-i*3;line(c,x,y,x+1,y,'rust3') end
  c:layer(6)
  line(c,-15,-7,-13,-8,'steel4');line(c,-10,-7,14,-19,'steel4')
  rect(c,-9,2,2,1,'ivory4');rect(c,5,-5,2,1,'ivory4')
end
out.assembly_palisade=function(c)
  c:layer(2)
  block(c,-11,8,6,28,2,'earth')
  c:layer(3)
  block(c,-11,6,4,26,12,'reed')
  -- The weave: alternating light and dark withies bound between the poles.
  for i=0,2 do panel(c,-10,4-i*5,25,1,1,'reed0');panel(c,-10,3-i*5,25,1,1,'reed3') end
  c:layer(5)
  pipe(c,-14,9,-14,-8,'jade',2);pipe(c,0,2,0,-15,'jade',2);pipe(c,13,-5,13,-22,'jade',2)
  line(c,-13,-8,13,-21,'jade4')
  c:layer(6)
  line(c,-9,4,-5,2,'reed4');line(c,4,-3,8,-5,'reed4');rect(c,-1,-16,2,1,'jade4')
end

-- Construction: a laid-out site, a load-bearing frame, then the finished
-- drawing on its pad. Uprights end at foundation points.
local function post(c,x,y,h,mat)
  block(c,x+2,y,3,3,h,mat)
  block(c,x+2,y+1,5,5,2,'earth')
end
function out.construction(c,name,stage)
  local s=out.spec[name];local x,y,w,d,h=s.x,s.y,s.w,s.d,s.h
  local woven=name:find('assembly',1,true)==1;local mat=woven and 'reed' or 'steel'
  if stage==2 then out[name](c,nil);return end
  local left={x-w,y-w/2};local right={x+d,y-d/2};local back={x-w+d,y-(w+d)/2}
  c:layer(2)
  block(c,x,y,w,d,2,'earth')
  line(c,left[1]+4,left[2]-3,x,y-3,'steel0',2)
  line(c,x,y-3,right[1]-4,right[2]-3,'steel0',2)
  if w>20 then
    block(c,-14,-8,11,8,5,woven and 'reed' or 'rust')
    for i=0,2 do block(c,10+i*4,-16+i*2,4,16,3,mat) end
  end
  c:layer(3)
  local core=stage==0 and math.floor(h/5) or math.floor(h/2)
  if w>20 then
    block(c,8,-18,14,12,core,woven and 'jade' or 'steel')
    if stage==1 then
      if woven then c:layer(4);cylinder(c,24,-24,8,core,'amber') else block(c,29,-8,8,12,core,'rust') end
    end
  else
    block(c,x,y,w,d,core,woven and 'reed' or 'steel')
  end
  if stage==1 then
    c:layer(5)
    for _,p in ipairs({left,right,back}) do post(c,p[1],p[2],h,mat) end
    line(c,left[1]+3,left[2]-h,back[1]+3,back[2]-h,mat..'3',2)
    line(c,back[1]+3,back[2]-h,right[1]+3,right[2]-h,mat..'3',2)
  end
end
return out
