-- Root's native screen-space drawings. These are composed in explicit 2:1
-- pixel planes, with attached panels and asymmetrical masses drawn individually.
local out={}
local function point(c,x,y) return {c.cx+x,c.cy+y} end
local function poly(c,p,color)
  local q={};for _,v in ipairs(p) do q[#q+1]=point(c,v[1],v[2]) end;c:poly(q,color)
end
local function line(c,x,y,xx,yy,color,width) c:line(point(c,x,y),point(c,xx,yy),color,width) end
local function rect(c,x,y,w,h,color) poly(c,{{x,y},{x+w,y},{x+w,y+h},{x,y+h}},color) end
local function ellipse(c,x,y,rx,ry,col) c:ellipse(c.cx+x,c.cy+y,rx,ry,col) end

-- Front corner (x,y), lengths along the left/up and right/up ground axes.
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
  if length>12 then line(c,x+side*length/2,y-length/4,x+side*length/2,y-length/4-h,'ivory1') end
end

local function ribbed_roof(c,x,y,w,d,mat)
  for a=6,w-3,7 do
    line(c,x-a,y-a/2,x-a+d,y-(a+d)/2,mat..'1')
    line(c,x-a+1,y-a/2-1,x-a+d-1,y-(a+d)/2-1,mat..'3')
  end
end

local function vent(c,x,y,w,d)
  block(c,x,y,w,d,3,'steel')
  for i=2,w-1,3 do line(c,x-i,y-i/2-3,x-i+d-1,y-(i+d-1)/2-3,'steel0') end
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

local function cargo(c,x,y,w,d,h)
  block(c,x,y,w,d,h,'rust')
  panel(c,x-2,y-3,w-4,h-5,-1,'rust1','rust3')
  panel(c,x+2,y-3,d-4,h-5,1,'rust0','rust2')
  line(c,x-w+3,y-w/2-1,x-3,y-h-1,'rust3')
  line(c,x-3,y-2,x-w+3,y-w/2-h+2,'rust0')
end

local function footing(c,x,y,w,d)
  block(c,x+3,y+2,w+5,d+5,3,'earth')
  block(c,x,y-1,w,d,4,'steel')
end

out.union_hq=function(c)
  c:shadow(59,9)
  c:layer(2)
  -- Low dock platform and the rear communications wheelhouse.
  footing(c,-2,5,53,59)
  block(c,-16,-42,22,34,18,'rust')
  block(c,-16,-60,23,35,4,'ivory')
  window(c,-18,-47,18,10,-1)
  window(c,-13,-47,28,10,1)
  block(c,-17,-64,20,32,2,'ivory')
  vent(c,-28,-72,8,9)
  pipe(c,2,-78,2,-98,'steel',2)
  line(c,-4,-92,9,-95,'steel3')
  rect(c,0,-100,3,3,'rust3')
  c:layer(3)
  -- A repurposed hull to the left and a larger engine-room hull to the right.
  block(c,-22,-3,32,23,29,'rust')
  block(c,-22,-31,33,24,5,'ivory')
  block(c,28,-9,26,23,33,'rust')
  block(c,28,-41,27,24,5,'ivory')
  ribbed_roof(c,-22,-36,32,23,'ivory')
  ribbed_roof(c,28,-46,26,23,'ivory')
  -- Rounded pressure stacks and roof-mounted cooling equipment.
  cylinder(c,39,-49,6,17,'steel')
  cylinder(c,39,-66,7,3,'rust')
  vent(c,-34,-41,12,8)
  vent(c,16,-51,10,8)
  c:layer(4)
  -- Wheelhouse glazing and reinforced door panels belong to their hull faces.
  window(c,-25,-14,24,10,-1)
  window(c,31,-18,16,12,1)
  panel(c,-19,-5,18,18,1,'rust0','rust3')
  for z=0,2 do line(c,-16,-10-z*4,0,-18-z*4,'steel2') end
  panel(c,25,-14,20,19,-1,'ivory2','ivory4')
  line(c,8,-25,24,-17,'ivory1')
  -- A deep central maintenance entrance with a wide steel lintel.
  poly(c,{{-13,-23},{3,-31},{16,-24},{16,-6},{3,1},{-13,-7}},'ink')
  poly(c,{{-9,-18},{3,-24},{12,-19},{12,-8},{3,-3},{-9,-9}},'steel0')
  block(c,3,-27,18,18,4,'steel')
  block(c,3,2,19,20,3,'steel')
  for i=0,3 do line(c,-10+i*6,-4-i*2,-6+i*6,-2-i*2,'rust3',2) end
  c:layer(5)
  -- Asymmetric short crane. Every support reaches the dock or a bearing.
  pipe(c,49,-17,49,-60,'rust',5)
  pipe(c,49,-60,26,-75,'rust',5)
  pipe(c,26,-75,7,-66,'ivory',5)
  pipe(c,46,-37,34,-65,'steel',2)
  ellipse(c,49,-58,3,3,'steel0');ellipse(c,49,-59,1,1,'steel4')
  line(c,8,-64,8,-48,'ink')
  rect(c,6,-48,5,5,'rust2')
  line(c,8,-43,8,-40,'steel3')
  line(c,8,-40,12,-40,'steel2')
  line(c,12,-40,12,-43,'steel3')
  c:layer(6)
  -- Localized wear, cap bolts and identification paint; no scattered noise.
  for _,p in ipairs({{-49,-37},{-27,-27},{31,-39},{47,-46}}) do rect(c,p[1],p[2],2,1,'ivory4') end
  line(c,-48,-22,-43,-20,'rust0')
  line(c,36,-24,41,-27,'rust1')
  rect(c,21,-24,2,5,'rust0');rect(c,17,-24,2,5,'rust0')
  rect(c,-39,-48,6,2,'steel0')
  -- The yard remains visibly industrial at native size: a roof extractor,
  -- a stacked freight crate, a tank, and repaired panel patches.
  cylinder(c,-33,-39,7,3,'steel')
  ellipse(c,-33,-43,5,2,'steel0')
  line(c,-37,-43,-29,-43,'steel2')
  line(c,-33,-45,-33,-41,'steel2')
  cargo(c,41,1,13,13,11)
  cylinder(c,-41,-7,5,10,'steel')
  cylinder(c,-41,-15,6,2,'rust')
  panel(c,-29,-17,8,5,-1,'rust2','rust3')
  line(c,-38,-19,-34,-17,'rust0')
  line(c,33,-53,38,-55,'ivory1')
  line(c,-14,-71,-5,-75,'ivory2')
  line(c,-9,-72,-2,-76,'ivory4')
  line(c,4,-42,8,-44,'steel0')
  line(c,4,-41,9,-43,'steel3')
end

out.assembly_hq=function(c)
  c:shadow(56,8)
  c:layer(2)
  footing(c,0,5,49,51)
  -- Woven shell: a continuous dark interior with two thick flanking vaults.
  poly(c,{{-49,-8},{-48,-39},{-38,-65},{-18,-86},{4,-94},{30,-76},{48,-46},{51,-12},{25,1},{-9,7}},'jade0')
  poly(c,{{-46,-17},{-44,-41},{-33,-63},{-16,-79},{5,-87},{24,-72},{39,-48},{45,-20},{23,-8},{-9,-1}},'jade1')
  c:layer(3)
  -- The camera-left shell catches broad warm daylight; roof planes change
  -- value across their curvature instead of outlining every rib equally.
  poly(c,{{-49,-8},{-48,-37},{-39,-62},{-21,-81},{2,-94},{11,-87},{-9,-70},{-26,-44},{-33,-13}},'jade3')
  poly(c,{{-48,-11},{-41,-17},{-35,-41},{-21,-66},{-3,-85},{2,-94},{-21,-81},{-39,-62}},'jade2')
  poly(c,{{2,-94},{25,-79},{43,-53},{51,-25},{51,-12},{40,-18},{34,-44},{19,-70},{4,-82}},'jade2')
  poly(c,{{5,-83},{19,-70},{34,-44},{40,-18},{32,-13},{25,-44},{13,-65},{0,-74}},'jade0')
  -- Broad woven panels at the feet of the shell, with a few bands following form.
  block(c,-23,-4,22,12,25,'reed')
  block(c,31,-10,12,16,28,'reed')
  for i=0,3 do
    panel(c,-25,-8-i*5,17,2,-1,'reed3')
    panel(c,33,-14-i*5,12,2,1,'reed1')
  end
  c:layer(4)
  -- Central amber pressure vessel: asymmetric light, collars and suspension.
  cylinder(c,3,-19,12,32,'amber')
  cylinder(c,3,-49,13,4,'steel')
  cylinder(c,3,-20,13,4,'steel')
  pipe(c,2,-54,2,-77,'steel',4)
  ellipse(c,3,-54,8,4,'steel2')
  rect(c,-5,-45,3,19,'amber4')
  cylinder(c,-24,-25,6,15,'amber')
  cylinder(c,28,-30,6,18,'amber')
  c:layer(5)
  -- Lit leading rib and structurally attached cross bindings.
  for _,seg in ipairs({{-38,-16,-32,-42},{-32,-42,-17,-65},{-17,-65,1,-80},{1,-80,15,-68},{15,-68,29,-43},{29,-43,36,-18}}) do
    line(c,seg[1],seg[2],seg[3],seg[4],'jade4',2)
  end
  for _,p in ipairs({{-33,-40},{-18,-65},{2,-83},{21,-62},{35,-39}}) do
    line(c,p[1]-3,p[2]+2,p[1]+4,p[2]-2,'reed2',3)
    line(c,p[1]-2,p[2]+1,p[1]+3,p[2]-2,'reed4')
  end
  -- Entry bridge and open floor reveal interior depth.
  block(c,2,4,20,24,4,'jade')
  for i=0,3 do line(c,-12+i*7,-3-i*2,-3+i*7,1-i*2,'reed3',2) end
  pipe(c,-20,-10,-11,-25,'steel',2)
  pipe(c,26,-14,19,-28,'steel',2)
  c:layer(6)
  line(c,-43,-34,-38,-47,'jade4')
  line(c,-27,-68,-19,-77,'jade3')
  rect(c,-5,-73,3,3,'reed4')
  ellipse(c,30,-29,3,4,'glass0');line(c,29,-31,31,-33,'glass3')
  rect(c,-8,-36,21,3,'steel0')
  rect(c,-7,-37,8,1,'steel3')
  pipe(c,-23,-22,-14,-18,'steel',2)
  pipe(c,26,-24,16,-20,'steel',2)
  line(c,-30,-20,-25,-27,'reed0')
  line(c,-42,-28,-37,-27,'reed1')
  line(c,36,-25,41,-28,'reed3')
end

out.union_works=function(c)
  c:shadow(56,8);c:layer(2)
  footing(c,2,5,49,48)
  -- Long factory hall with three stepped sawtooth roof monitors.
  block(c,2,0,43,44,31,'rust')
  block(c,2,-31,44,45,4,'ivory')
  c:layer(3)
  block(c,-17,-47,17,29,10,'steel')
  window(c,-14,-51,23,6,1)
  block(c,-17,-57,18,30,3,'ivory')
  block(c,9,-35,13,26,8,'steel')
  window(c,12,-38,20,5,1)
  block(c,9,-43,14,27,3,'ivory')
  cylinder(c,-15,-62,8,12,'steel')
  cylinder(c,-15,-74,9,3,'rust')
  ellipse(c,-15,-78,6,3,'steel0')
  line(c,-20,-78,-10,-78,'steel2')
  line(c,-15,-80,-15,-76,'steel2')
  c:layer(4)
  panel(c,-1,-5,32,23,-1,'steel0','rust3')
  for i=0,4 do line(c,-30,-20-i*3,-4,-7-i*3,'steel2') end
  line(c,-33,-23,-6,-10,'steel0',2)
  line(c,-32,-26,-6,-13,'steel0',2)
  window(c,6,-14,31,11,1)
  panel(c,6,-3,12,9,1,'ivory1','ivory3')
  cylinder(c,44,-19,6,31,'steel')
  cylinder(c,44,-49,7,4,'rust')
  ellipse(c,44,-54,4,2,'steel0')
  pipe(c,39,-23,30,-27,'steel',3)
  pipe(c,30,-27,30,-35,'steel',3)
  c:layer(5)
  block(c,-10,6,27,22,3,'steel')
  for i=0,4 do line(c,-33+i*6,-8+i*3,-28+i*6,-10+i*3,'rust3',2) end
  pipe(c,-40,-13,-40,-44,'steel',3)
  pipe(c,-40,-44,-14,-31,'rust',4)
  line(c,-16,-32,-16,-16,'ink')
  rect(c,-18,-17,5,5,'rust3')
  vent(c,6,-66,8,10)
  c:layer(6)
  line(c,13,-47,18,-49,'ivory1')
  line(c,-34,-36,-29,-33,'rust0')
  line(c,29,-9,34,-12,'rust0')
  rect(c,6,-21,2,2,'amber4')
  cargo(c,41,3,11,11,10)
end

out.assembly_works=function(c)
  c:shadow(51,7);c:layer(2)
  footing(c,0,4,44,46)
  block(c,1,-3,30,33,21,'jade')
  c:layer(3)
  -- Low folded composite canopy. Multiple broad planes distinguish the loom
  -- workshop from the tall headquarters vault.
  poly(c,{{-44,-31},{-29,-54},{-7,-66},{17,-60},{39,-38},{46,-22},{12,-7},{-11,-10}},'jade0')
  poly(c,{{-44,-34},{-28,-56},{-6,-68},{3,-60},{-13,-39},{-31,-25}},'jade3')
  poly(c,{{-6,-68},{17,-60},{39,-39},{45,-25},{22,-21},{3,-60}},'jade2')
  poly(c,{{-13,-39},{3,-60},{22,-21},{12,-8},{-11,-11}},'jade1')
  for _,p in ipairs({{-33,-42,-17,-50},{-24,-29,-10,-39},{10,-58,27,-34},{20,-48,35,-29}}) do
    line(c,p[1],p[2],p[3],p[4],'reed2',2)
    line(c,p[1],p[2]-1,p[3],p[4]-1,'reed3')
  end
  c:layer(4)
  panel(c,-2,-5,24,18,-1,'reed0','reed3')
  for i=0,3 do line(c,-25,-17-i*3,-4,-6-i*3,'reed2') end
  panel(c,5,-7,26,13,1,'steel0','jade3')
  cylinder(c,-32,-8,8,20,'reed')
  cylinder(c,-32,-29,10,3,'steel')
  cylinder(c,-32,-9,10,3,'steel')
  for i=0,3 do line(c,-39,-14-i*3,-25,-14-i*3,'reed1') end
  cylinder(c,34,-15,8,22,'reed')
  cylinder(c,34,-36,10,3,'steel')
  cylinder(c,34,-15,10,3,'steel')
  for i=0,3 do line(c,27,-20-i*3,41,-20-i*3,'reed1') end
  c:layer(5)
  cylinder(c,6,-16,6,18,'amber')
  cylinder(c,6,-33,7,2,'steel')
  pipe(c,-24,-14,-13,-20,'steel',2)
  pipe(c,26,-20,13,-20,'steel',2)
  block(c,0,4,21,25,3,'jade')
  for i=0,2 do line(c,-17+i*8,-7+i*3,-6+i*8,-12+i*3,'reed3',2) end
  c:layer(6)
  line(c,-35,-41,-30,-48,'jade4')
  line(c,4,-32,4,-24,'amber4')
  rect(c,-4,-61,4,2,'reed4')
end

out.dropoff=function(c)
  c:shadow(49,7);c:layer(2)
  footing(c,0,5,40,45)
  block(c,15,-19,18,24,7,'steel')
  cargo(c,26,-23,15,16,17)
  cargo(c,-9,-12,20,17,18)
  c:layer(3)
  cargo(c,14,-4,20,18,17)
  cargo(c,11,-22,15,13,12)
  block(c,-22,0,16,14,4,'reed')
  for i=0,2 do line(c,-35+i*5,-7+i*2,-24+i*5,-13+i*2,'reed0') end
  cylinder(c,-25,-2,6,13,'steel')
  cylinder(c,-25,-14,6,2,'rust')
  c:layer(4)
  pipe(c,-39,-13,-39,-53,'steel',4)
  pipe(c,38,-15,38,-54,'steel',4)
  pipe(c,-39,-53,38,-54,'rust',5)
  line(c,-39,-57,38,-58,'rust4',2)
  line(c,-35,-51,-27,-41,'steel3')
  line(c,35,-52,27,-42,'steel3')
  c:layer(5)
  rect(c,2,-55,12,6,'steel0');rect(c,3,-55,9,3,'ivory2')
  line(c,8,-49,8,-34,'ink')
  rect(c,5,-35,7,5,'rust2')
  line(c,8,-30,8,-26,'steel3')
  line(c,8,-26,12,-27,'steel2')
  c:layer(6)
  panel(c,11,-8,8,5,1,'ivory2','ivory3')
  line(c,11,-11,15,-13,'steel0')
  line(c,-23,-23,-18,-21,'rust0')
end

out.condenser=function(c)
  c:shadow(42,7);c:layer(2)
  footing(c,0,5,34,37)
  cylinder(c,-16,-11,12,40,'steel')
  cylinder(c,-16,-48,13,5,'ivory')
  cylinder(c,-16,-54,8,5,'steel')
  pipe(c,-18,-59,-18,-68,'steel',4)
  ellipse(c,-17,-69,5,2,'steel0')
  c:layer(3)
  cylinder(c,17,-5,14,39,'steel')
  cylinder(c,17,-43,15,5,'ivory')
  cylinder(c,17,-50,9,5,'steel')
  -- Framed liquid chamber, with shaded glass and a bright meniscus.
  poly(c,{{7,-38},{23,-38},{26,-33},{26,-15},{23,-10},{7,-10},{4,-15},{4,-33}},'steel0')
  rect(c,8,-35,13,23,'glass0')
  rect(c,9,-31,11,18,'glass1')
  rect(c,10,-28,4,15,'glass2')
  line(c,9,-31,20,-31,'glass3')
  line(c,10,-33,10,-26,'glass4')
  cylinder(c,17,-6,15,4,'steel')
  c:layer(4)
  pipe(c,-28,-12,-35,-16,'steel',4)
  pipe(c,-35,-16,-35,-32,'steel',4)
  pipe(c,-35,-32,-27,-35,'steel',4)
  pipe(c,30,-12,36,-17,'rust',4)
  pipe(c,36,-17,36,-42,'rust',4)
  pipe(c,36,-42,26,-48,'rust',4)
  pipe(c,-5,-38,6,-43,'steel',3)
  c:layer(5)
  ellipse(c,-16,-30,7,7,'steel0')
  ellipse(c,-16,-31,5,5,'ivory2')
  ellipse(c,-17,-32,3,3,'ivory4')
  line(c,-16,-31,-13,-34,'rust1')
  rect(c,-19,-18,6,5,'rust2')
  c:layer(6)
  line(c,-23,-43,-23,-38,'steel3')
  rect(c,6,-6,4,2,'rust3')
  rect(c,24,-9,3,2,'rust3')
end

out.tower=function(c)
  c:shadow(41,7);c:layer(2)
  footing(c,0,5,34,36)
  block(c,0,0,25,27,16,'steel')
  block(c,0,-16,26,28,5,'ivory')
  for i=0,2 do panel(c,-3,-5-i*4,17,1,-1,'steel0') end
  panel(c,4,-5,16,8,1,'rust1','rust3')
  c:layer(3)
  cylinder(c,0,-25,16,7,'steel')
  block(c,1,-33,16,18,21,'rust')
  block(c,1,-53,17,19,5,'ivory')
  window(c,-2,-40,12,8,-1)
  window(c,4,-41,13,8,1)
  vent(c,-3,-62,8,9)
  c:layer(4)
  pipe(c,-9,-60,-9,-82,'steel',3)
  rect(c,-11,-84,5,4,'rust3')
  line(c,-14,-75,-4,-77,'steel3')
  -- Offset heavy barrel, recoil sleeve and dark open muzzle.
  poly(c,{{7,-37},{39,-59},{47,-56},{15,-30}},'steel0')
  poly(c,{{9,-38},{39,-58},{43,-56},{13,-35}},'steel3')
  poly(c,{{13,-35},{43,-56},{43,-51},{13,-30}},'steel1')
  line(c,12,-38,36,-54,'ivory4')
  poly(c,{{26,-50},{30,-52},{36,-49},{32,-47}},'rust3')
  poly(c,{{32,-47},{36,-49},{36,-44},{32,-42}},'rust1')
  poly(c,{{39,-60},{48,-57},{48,-50},{40,-54}},'rust1')
  poly(c,{{41,-59},{46,-57},{46,-53},{41,-55}},'ink')
  line(c,40,-60,47,-57,'rust3')
  c:layer(5)
  ellipse(c,6,-36,7,7,'steel0')
  ellipse(c,5,-37,5,5,'steel2')
  ellipse(c,4,-38,2,2,'steel4')
  for _,p in ipairs({{-25,-8},{26,-7}}) do cylinder(c,p[1],p[2],5,6,'steel') end
  c:layer(6)
  line(c,-17,-22,-11,-19,'ivory1')
  line(c,9,-18,15,-21,'rust4')
  rect(c,-9,-82,2,2,'amber4')
end

out.gate=function(c)
  c:shadow(57,8);c:layer(2)
  footing(c,0,6,53,54)
  poly(c,{{-36,-13},{0,5},{39,-15},{5,-32}},'sea0')
  line(c,-30,-14,-3,0,'sea2')
  line(c,-7,-21,19,-8,'sea1',2)
  -- The operating wheel stands behind the bridge's structural lintel.
  ellipse(c,1,-41,25,25,'steel0')
  ellipse(c,0,-42,22,23,'rust1')
  ellipse(c,-1,-43,18,19,'steel0')
  for _,p in ipairs({{0,-61},{15,-52},{17,-35},{0,-25},{-15,-35},{-17,-51}}) do
    pipe(c,0,-43,p[1],p[2],'rust',3)
  end
  ellipse(c,0,-43,7,7,'steel1');ellipse(c,-1,-44,4,4,'ivory3')
  line(c,-17,-57,-9,-63,'rust4',2)
  c:layer(3)
  block(c,-30,1,15,16,49,'earth')
  block(c,34,-1,15,16,49,'earth')
  block(c,-30,-48,17,18,6,'ivory')
  block(c,34,-50,17,18,6,'ivory')
  -- A shallow lintel, not a full roof slab: the operating wheel must remain
  -- exposed beneath it. The extrusion recedes along a 2:1 depth edge.
  poly(c,{{-44,-51},{43,-51},{43,-59},{-44,-59}},'earth2')
  poly(c,{{-44,-59},{-36,-63},{51,-63},{43,-59}},'earth4')
  poly(c,{{43,-51},{51,-55},{51,-63},{43,-59}},'earth1')
  line(c,-44,-51,43,-51,'earth0')
  line(c,-42,-58,40,-58,'earth3')
  c:layer(4)
  poly(c,{{-34,1},{33,1},{40,-6},{-27,-6}},'steel2')
  poly(c,{{-34,1},{33,1},{33,5},{-34,5}},'steel0')
  line(c,-31,0,30,0,'steel3')
  for i=-2,2 do line(c,i*10,-5,i*10-4,0,'steel0') end
  pipe(c,35,-63,35,-87,'steel',3)
  block(c,38,-75,7,5,13,'ivory')
  panel(c,37,-78,5,7,-1,'glass0','steel2')
  line(c,32,-81,35,-79,'glass3')
  c:layer(5)
  cylinder(c,-32,-60,5,6,'rust')
  ellipse(c,-32,-67,4,3,'amber3')
  rect(c,-34,-69,3,2,'amber4')
  pipe(c,34,-44,25,-38,'steel',2)
  c:layer(6)
  for _,p in ipairs({{-43,-35},{-39,-19},{39,-33},{43,-17}}) do
    line(c,p[1],p[2],p[1]+4,p[2]+2,'earth0')
    line(c,p[1]+1,p[2]-1,p[1]+5,p[2]+1,'earth3')
  end
end

out.salvage=function(c)
  c:shadow(45,6);c:layer(2)
  -- A collapsed tug hull: torn contour, split cab, exposed turbine and beams.
  poly(c,{{-44,-8},{-29,-29},{-12,-33},{-2,-29},{10,-37},{30,-26},{43,-13},{28,2},{-2,6},{-31,0}},'earth1')
  poly(c,{{-39,-12},{-30,-25},{-12,-28},{-6,-24},{9,-31},{32,-20},{27,-5},{-3,0}},'rust0')
  c:layer(3)
  poly(c,{{-33,-15},{-27,-31},{-13,-35},{-6,-30},{1,-34},{18,-24},{15,-7},{-4,0}},'rust2')
  poly(c,{{-33,-15},{-4,0},{-4,-12},{-26,-25}},'rust1')
  poly(c,{{-26,-25},{-13,-35},{-6,-30},{1,-34},{17,-24},{-4,-12}},'ivory1')
  block(c,10,-12,15,17,17,'steel')
  poly(c,{{-5,-29},{-1,-38},{9,-42},{17,-36},{25,-26},{10,-20}},'ivory2')
  poly(c,{{3,-30},{9,-36},{18,-31},{17,-27},{11,-27},{9,-24}},'glass0')
  line(c,7,-33,13,-34,'glass2')
  c:layer(4)
  ellipse(c,-15,-11,12,10,'steel0')
  ellipse(c,-16,-12,9,8,'rust2')
  ellipse(c,-17,-13,5,5,'steel1')
  line(c,-23,-16,-11,-9,'rust0',2)
  line(c,-21,-8,-12,-18,'rust0',2)
  ellipse(c,-17,-13,2,2,'steel3')
  pipe(c,-35,-18,-10,-39,'steel',4)
  pipe(c,13,-34,37,-49,'steel',4)
  c:layer(5)
  cargo(c,30,0,13,12,10)
  line(c,8,-17,8,-4,'ink',2)
  line(c,8,-4,17,-3,'steel1',2)
  for i=0,3 do line(c,-3+i*4,-16+i*2,-3+i*4,-9+i*2,'steel3') end
  c:layer(6)
  line(c,-29,-23,-23,-21,'rust3')
  line(c,-4,-30,0,-28,'ivory3')
  line(c,20,-17,25,-20,'steel3')
end

out.well=function(c)
  c:shadow(31,5);c:layer(2)
  block(c,0,4,25,26,5,'earth')
  ellipse(c,0,-10,20,10,'earth0')
  ellipse(c,0,-12,17,8,'ivory2')
  ellipse(c,0,-13,12,5,'sea0')
  line(c,-8,-13,2,-17,'sea2')
  c:layer(3)
  block(c,-7,-8,10,11,20,'steel')
  block(c,-7,-28,11,12,3,'ivory')
  pipe(c,-7,-32,-7,-48,'steel',4)
  pipe(c,-7,-48,17,-39,'steel',3)
  ellipse(c,18,-38,3,3,'rust3')
  pipe(c,4,-22,13,-18,'steel',3)
  pipe(c,13,-18,13,-11,'steel',3)
  c:layer(4)
  ellipse(c,-12,-22,5,5,'steel0')
  ellipse(c,-12,-23,3,3,'ivory3')
  line(c,-12,-23,-10,-25,'rust1')
  c:layer(6)
  line(c,-23,-8,-19,-6,'earth3')
  line(c,18,-9,22,-11,'earth3')
end

return out
