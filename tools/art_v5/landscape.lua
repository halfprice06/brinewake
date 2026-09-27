-- Root-authored Aseprite drawing pass: salt strata, fractured seawall stone,
-- tidal growth and three broken reclamation wreck silhouettes.
-- No imported image pixels. Screen coordinates are relative to ground anchors.
local out={}
local function poly(c,p,color)
  local q={};for _,v in ipairs(p) do q[#q+1]={c.cx+v[1],c.cy+v[2]} end
  c:poly(q,color)
end
local function line(c,x,y,xx,yy,color,w)
  c:line({c.cx+x,c.cy+y},{c.cx+xx,c.cy+yy},color,w)
end
local function oval(c,x,y,rx,ry,color) c:ellipse(c.cx+x,c.cy+y,rx,ry,color) end
local function ground(c,color)
  c:layer(3)
  for y=-8,8 do line(c,-16+2*math.abs(y),y,16-2*math.abs(y),y,color) end
end
local function fracture(c,x,y,w)
  line(c,x,y,x+w,y-2,'salt0');line(c,x+w,y-2,x+w+6,y,'salt0')
  line(c,x+1,y-1,x+w-1,y-3,'salt2')
end
for i=0,3 do
  local v=i
  out['terrain_salt_'..i]=function(c)
    ground(c,'salt1');c:layer(6)
    if v==1 then fracture(c,-10,1,7)
    elseif v==2 then
      poly(c,{{-9,0},{-6,-3},{0,-4},{9,-1},{6,2},{-1,2}},'salt2')
      line(c,-9,0,-3,1,'salt0');line(c,2,3,7,1,'salt0')
    elseif v==3 then
      line(c,-8,-2,-2,-4,'salt2');line(c,-2,-4,3,-3,'salt2')
      line(c,2,2,9,0,'salt0');line(c,-6,3,-1,4,'salt2')
    end
  end
  out['terrain_silt_'..i]=function(c)
    ground(c,'earth1');c:layer(6)
    if v==1 then
      line(c,-11,0,-5,-2,'earth0');line(c,-5,-2,1,-2,'earth0')
      line(c,-7,-3,-2,-4,'earth2')
    elseif v==2 then
      poly(c,{{-5,2},{0,-1},{7,0},{4,2},{-1,3}},'earth0')
      line(c,0,-2,4,-1,'earth2');line(c,-8,-2,-5,-3,'earth2')
    elseif v==3 then
      line(c,-10,0,-4,2,'earth2');line(c,3,-3,8,-1,'earth0')
    end
  end
  out['terrain_causeway_'..i]=function(c)
    ground(c,'earth2');c:layer(6)
    -- The paving changes within each tile, with a common quiet boundary.
    poly(c,{{-13,0},{-1,-6},{11,0},{0,5}},v==2 and 'earth2' or 'earth3')
    line(c,-13,1,0,7,'earth1');line(c,1,7,14,0,'earth1')
    line(c,-12,0,-3,-4,'earth4')
    if v==1 then
      line(c,-2,-3,0,0,'earth1');line(c,0,0,5,1,'earth1')
      poly(c,{{3,2},{7,0},{10,1},{6,3}},'earth2')
    elseif v==2 then
      poly(c,{{-4,-3},{2,-4},{8,-1},{4,1},{-2,0}},'earth1')
      line(c,-4,-3,1,-4,'earth3');line(c,-7,1,-2,3,'earth3')
    elseif v==3 then
      line(c,-5,0,-1,2,'earth2');line(c,-1,2,4,1,'earth2')
    end
  end
  out['terrain_water_'..i]=function(c)
    ground(c,'sea1');c:layer(6)
    if v>0 then
      local dx=({0,1,2})[v]
      line(c,-12+dx,0,-7+dx,-2,'sea2');line(c,-7+dx,-2,0+dx,-2,'sea2')
      line(c,-9+dx,1,-5+dx,0,'sea0')
      line(c,3-dx,4,10-dx,2,'sea2')
    end
  end
  out['terrain_shallow_'..i]=function(c)
    ground(c,'sea2');c:layer(6)
    if v>0 then
      local dx=v-1
      line(c,-11+dx,0,-6+dx,-2,'sea3');line(c,-6+dx,-2,1+dx,-2,'sea3')
      line(c,-7+dx,1,-2+dx,0,'sea1')
      line(c,4-dx,4,9-dx,2,'sea3')
    end
  end
end

-- Larger, transparent ground patches introduce spatial scale between the
-- tiny repeating texture and the architecture. Values stay within the salt ramp.
out.salt_crust_0=function(c)
  c:layer(3)
  poly(c,{{-42,0},{-31,-6},{-23,-5},{-13,-11},{0,-14},{16,-9},{30,-10},{42,-4},{34,0},{18,1},{6,8},{-8,10},{-20,7},{-32,8}},'salt0')
  poly(c,{{-42,-1},{-31,-7},{-23,-6},{-13,-12},{0,-15},{16,-10},{30,-11},{42,-5},{30,-4},{19,-5},{8,1},{-5,4},{-18,1},{-28,4}},'salt2')
  poly(c,{{-35,-2},{-22,-4},{-13,-9},{-1,-12},{11,-9},{5,-5},{-8,-2},{-18,-3},{-28,0}},'salt3')
  poly(c,{{-25,4},{-14,2},{-7,4},{-2,4},{-7,7},{-19,6}},'salt1')
  c:layer(6);fracture(c,-21,-2,8);fracture(c,3,-6,9)
  line(c,12,4,21,1,'salt2');line(c,23,-2,31,-3,'salt3')
end
out.salt_crust_1=function(c)
  c:layer(3)
  poly(c,{{-39,4},{-28,-3},{-16,-5},{-9,-10},{1,-12},{18,-10},{29,-4},{37,-2},{33,5},{20,8},{7,10},{-8,8},{-24,9}},'salt2')
  poly(c,{{-29,3},{-18,-2},{-8,-3},{-3,-7},{9,-8},{24,-3},{25,1},{14,5},{2,6},{-11,3},{-20,6}},'salt0')
  poly(c,{{-24,3},{-16,0},{-6,0},{0,-5},{9,-6},{20,-3},{20,0},{13,2},{1,4},{-9,1}},'salt1')
  c:layer(6)
  line(c,-28,-3,-16,-5,'salt3');line(c,-8,-10,0,-12,'salt3')
  line(c,2,-12,10,-11,'salt3');line(c,17,7,25,5,'salt0')
  fracture(c,-18,0,8)
end
out.salt_crust_2=function(c)
  c:layer(3)
  for _,p in ipairs({{-30,2,14},{-12,-5,21},{10,-6,18},{-6,9,13}}) do
    local x,y,w=p[1],p[2],p[3]
    poly(c,{{x,y},{x+6,y-4},{x+w,y-5},{x+w+6,y-2},{x+w,y},{x+8,y+1}},'salt2')
    line(c,x+2,y+1,x+9,y+2,'salt0');line(c,x+7,y-4,x+w-2,y-5,'salt3')
  end
end

-- A low, layered sedimentary formation, with the light breaking across the
-- upper plane and wide quiet side faces, rather than concentric stone bands.
local function rock(c,variant)
  c:shadow(22,5);c:layer(3)
  local k=variant==1 and -4 or 0
  poly(c,{{-24,0},{-21,-13},{-13,-26+k},{-1,-32+k},{10,-30+k},{18,-21},{21,-12},{25,-5},{22,4},{8,8},{-10,6}},'earth0')
  poly(c,{{-22,-5},{-19,-16},{-11,-26+k},{-1,-30+k},{9,-28+k},{13,-21},{5,-16},{-5,-14},{-11,-6}},'earth3')
  poly(c,{{5,-16},{13,-21},{17,-20},{20,-11},{16,-5},{19,3},{7,6},{-3,2},{-3,-7}},'earth1')
  poly(c,{{-22,-5},{-11,-6},{-5,-14},{5,-16},{-3,-7},{-3,2},{7,6},{-9,5},{-21,0}},'earth2')
  c:layer(6)
  line(c,-17,-19,-10,-25+k,'earth4');line(c,-10,-25+k,-2,-28+k,'earth4')
  line(c,-20,-6,-11,-7,'earth1');line(c,-11,-7,-6,-10,'earth1')
  line(c,-5,-10,3,-12,'earth0');line(c,3,-12,9,-10,'earth0')
  line(c,10,-8,17,-10,'earth2');line(c,10,-3,17,-5,'earth0')
  line(c,-15,0,-7,-2,'earth3');line(c,-17,2,-11,3,'earth1')
  poly(c,{{-23,6},{-26,2},{-23,-1},{-19,0},{-16,4},{-17,7}},'earth1')
  line(c,-25,2,-22,0,'earth3')
end
out.salt_rock=function(c) rock(c,0) end
out.salt_rock_1=function(c) rock(c,1) end
out.salt_rock_2=function(c)
  c:shadow(23,5);c:layer(3)
  poly(c,{{-25,0},{-20,-12},{-6,-20},{4,-18},{15,-24},{23,-15},{25,1},{12,7},{-10,5}},'earth0')
  poly(c,{{-24,-2},{-18,-13},{-6,-18},{3,-15},{-4,-9},{-3,-2},{-12,1}},'earth3')
  poly(c,{{-3,-2},{-4,-9},{3,-15},{14,-22},{20,-16},{13,-9},{9,2}},'earth2')
  poly(c,{{13,-9},{20,-16},{23,-14},{24,0},{12,5},{9,2}},'earth1')
  c:layer(6);line(c,-17,-12,-8,-17,'earth4');line(c,5,-16,13,-21,'earth3')
  line(c,-19,-1,-11,-4,'earth1');line(c,-11,-4,-5,-3,'earth1')
  line(c,0,1,6,-2,'earth0');line(c,15,-6,20,-8,'earth0')
end

out.reed_clump=function(c)
  c:shadow(16,4);c:layer(3)
  poly(c,{{-18,0},{-12,-5},{-5,-4},{3,-7},{12,-3},{18,1},{11,5},{-5,5}},'moss0')
  for _,v in ipairs({{-12,0,-15,-15},{-8,2,-10,-25},{-3,1,-3,-30},{3,0,7,-27},{7,2,13,-19},{12,2,19,-9}}) do
    local x,y,tx,ty=table.unpack(v)
    poly(c,{{x-2,y},{tx-1,ty},{tx+1,ty+5},{x+2,y}},'moss0')
    line(c,x,y,tx,ty,'reed1');line(c,x-1,y-2,tx-1,ty+1,'reed2')
    poly(c,{{x,y-3},{x+6,y-13},{x+4,y-4}},'moss1')
    poly(c,{{tx-2,ty+1},{tx-3,ty-4},{tx-1,ty-5},{tx+1,ty}},'reed1')
    line(c,tx-2,ty-3,tx-1,ty,'reed3')
  end
  c:layer(6);line(c,-9,1,-3,3,'moss1');line(c,2,3,8,1,'moss1')
end

local function wreck_ground(c)
  c:layer(1)
  poly(c,{{-52,1},{-35,-10},{-16,-11},{-3,-18},{28,-8},{51,2},{45,10},{19,14},{-9,12},{-31,9}},'cast')
  c:layer(2)
  poly(c,{{-48,0},{-32,-8},{-14,-8},{3,-12},{32,-4},{45,3},{33,8},{13,10},{-14,7},{-32,6}},'earth0')
end
local function debris(c)
  c:layer(6)
  poly(c,{{-45,5},{-36,1},{-28,4},{-33,8}},'rust1')
  line(c,-43,4,-35,2,'rust2')
  poly(c,{{24,8},{31,4},{40,7},{32,11}},'rust1');line(c,27,7,32,5,'rust3')
  line(c,-20,7,-13,9,'earth2');line(c,8,11,17,10,'salt2')
end
local function wear(c,x,y,mat)
  local p={{x,y},{x+5,y-1},{x+8,y+1},{x+4,y+2},{x+1,y+2}}
  local q={};for _,v in ipairs(p) do q[#q+1]={c.cx+v[1],c.cy+v[2]} end
  c:overpaint(q,mat..'0',{mat..'1',mat..'2',mat..'3'})
  local p2={{x+1,y},{x+5,y-1},{x+6,y},{x+2,y+1}}
  q={};for _,v in ipairs(p2) do q[#q+1]={c.cx+v[1],c.cy+v[2]} end
  c:overpaint(q,mat..'1',{mat..'0'})
end
out.salvage=function(c)
  wreck_ground(c);c:layer(3)
  -- Tug stern and ripped-open hold. No gun barrel or whole vehicle silhouette.
  poly(c,{{-45,-15},{-18,-31},{33,-17},{46,-8},{32,6},{6,9},{-24,1},{-43,-6}},'rust0')
  poly(c,{{-45,-15},{-31,-16},{-16,-6},{6,0},{32,-3},{46,-8},{32,6},{6,9},{-24,1},{-43,-6}},'rust1')
  poly(c,{{-45,-15},{-18,-31},{33,-17},{46,-8},{32,-3},{6,0},{-16,-6},{-31,-16}},'rust2')
  poly(c,{{-29,-18},{-16,-26},{18,-18},{27,-11},{10,-5},{-8,-9}},'ink')
  poly(c,{{-23,-18},{-16,-23},{17,-16},{20,-12},{9,-8},{-7,-12}},'steel0')
  for _,p in ipairs({{-20,-22,-10,-11},{-10,-22,0,-10},{1,-20,9,-9}}) do
    line(c,p[1],p[2],p[3],p[4],'rust1',2);line(c,p[1],p[2],p[3],p[4],'rust3')
  end
  -- Torn starboard rail, salt-stained lower hull, exposed frames and holes.
  poly(c,{{5,0},{15,-4},{19,-1},{27,-5},{32,-3},{38,-7},{39,-2},{30,6},{7,8}},'rust2')
  poly(c,{{-36,-5},{-24,1},{5,8},{16,8},{18,6},{-6,2},{-24,-3}},'earth1')
  line(c,-43,-14,-32,-15,'ivory2');line(c,-31,-15,-18,-7,'ivory1')
  line(c,25,-16,35,-12,'rust3');line(c,36,-10,42,-8,'rust3')
  for _,x in ipairs({-30,-22,-13}) do line(c,x,-4+(x+30)/4,x+2,-1+(x+30)/4,'rust0',2) end
  c:layer(4)
  -- A collapsed wheelhouse, with one intact window and one dark broken frame.
  poly(c,{{-34,-24},{-34,-40},{-21,-47},{-5,-39},{-5,-21},{-18,-16}},'ivory0')
  poly(c,{{-34,-40},{-21,-47},{-5,-39},{-18,-31}},'ivory2')
  poly(c,{{-33,-39},{-18,-31},{-18,-17},{-33,-24}},'ivory1')
  poly(c,{{-17,-30},{-5,-38},{-5,-22},{-17,-17}},'rust1')
  poly(c,{{-30,-36},{-22,-32},{-22,-25},{-30,-29}},'glass0')
  line(c,-29,-35,-23,-32,'glass2')
  poly(c,{{-14,-30},{-8,-34},{-8,-27},{-14,-23}},'ink')
  line(c,-34,-40,-22,-46,'ivory3');line(c,-21,-46,-14,-43,'ivory3')
  poly(c,{{-14,-43},{-6,-40},{-9,-37},{-18,-39}},'ivory0')
  c:layer(5)
  line(c,-20,-46,-24,-62,'steel0',2);line(c,-21,-46,-25,-61,'steel2')
  line(c,-25,-62,-18,-65,'steel1');line(c,-25,-57,-37,-59,'steel1')
  line(c,-24,-55,-33,-35,'reed0')
  -- Salvageable copper sheets carry the same restrained warm read at every site.
  poly(c,{{15,-5},{26,-9},{35,-4},{23,1}},'rust0')
  poly(c,{{15,-7},{26,-11},{35,-6},{23,-1}},'rust3')
  line(c,16,-7,25,-10,'rust4');line(c,24,-1,32,-5,'rust2')
  debris(c)
  for _,p in ipairs({{-40,-10},{-29,-9},{-9,0},{24,-1}}) do wear(c,p[1],p[2],'rust') end
  wear(c,-29,-39,'ivory');wear(c,-15,-42,'ivory')
  line(c,-33,-22,-28,-20,'earth2');line(c,-24,-18,-20,-17,'earth2')
end
out.salvage_turbine=function(c)
  wreck_ground(c);c:layer(3)
  poly(c,{{-44,-24},{-36,-39},{-19,-47},{5,-41},{28,-25},{27,-5},{9,8},{-12,2},{-36,-10}},'steel0')
  poly(c,{{-43,-24},{-35,-39},{-18,-45},{5,-39},{24,-26},{9,-28},{-4,-32},{-18,-29},{-28,-19}},'steel2')
  poly(c,{{-42,-22},{-28,-18},{-15,-22},{-13,-6},{5,6},{-12,1},{-35,-11}},'steel1')
  line(c,-35,-38,-20,-44,'steel3');line(c,-17,-44,-8,-42,'steel3')
  line(c,-34,-33,-14,-22,'steel0',2);line(c,-27,-40,-7,-29,'steel1',2)
  c:layer(4)
  oval(c,10,-16,25,21,'rust0');oval(c,9,-18,24,20,'steel2')
  oval(c,10,-17,19,16,'steel0');oval(c,11,-16,15,12,'ink')
  -- Curled impeller blades converge into a recessed, non-emissive hub.
  for _,p in ipairs({{{9,-18},{-2,-29},{7,-29},{13,-21}},{{13,-20},{24,-25},{25,-19},{16,-14}},{{15,-14},{25,-7},{17,-5},{10,-11}},{{9,-11},{3,-5},{-2,-10},{5,-18}}}) do
    poly(c,p,'steel1')
  end
  oval(c,10,-16,5,4,'steel0');oval(c,9,-18,4,3,'steel3');oval(c,10,-18,2,2,'steel1')
  -- Missing casing crown and gouged lower rim avoid a perfect ring.
  poly(c,{{1,-40},{7,-33},{18,-35},{24,-29},{19,-24},{7,-28},{-1,-27}},'rust0')
  line(c,-11,-20,-9,-28,'steel3');line(c,-9,-28,-3,-32,'steel3')
  line(c,30,-15,28,-6,'steel1');line(c,22,0,13,3,'rust2')
  c:layer(5)
  poly(c,{{-34,-14},{-24,-19},{-17,-15},{-15,-3},{-6,2},{-10,6},{-26,0}},'rust0')
  line(c,-32,-14,-24,-17,'rust3',2);line(c,-24,-17,-20,-13,'rust2',3)
  line(c,-20,-13,-18,-4,'rust1',3);line(c,-18,-3,-9,3,'rust2',3)
  poly(c,{{26,0},{36,-5},{45,0},{36,5}},'rust3');line(c,28,0,36,-4,'rust4')
  debris(c)
  for _,p in ipairs({{-37,-27},{-27,-39},{-11,-35},{-9,-15},{17,-1}}) do wear(c,p[1],p[2],'steel') end
  line(c,-30,-10,-23,-8,'earth2');line(c,-8,4,0,7,'earth1')
end
out.salvage_barge=function(c)
  wreck_ground(c);c:layer(3)
  poly(c,{{-46,-11},{-24,-27},{26,-15},{48,-2},{37,8},{4,10},{-33,2}},'rust0')
  poly(c,{{-43,-12},{-24,-25},{27,-13},{45,-2},{34,3},{2,3},{-29,-3}},'steel1')
  for i=0,4 do line(c,-29+i*10,-16+i*2,-12+i*10,-5+i*2,'steel0',2) end
  c:layer(4)
  poly(c,{{-32,-18},{-31,-44},{-9,-55},{-4,-52},{-2,-46},{3,-47},{4,-43},{12,-44},{22,-41},{27,-17},{8,-4}},'rust0')
  poly(c,{{-31,-43},{-10,-54},{-5,-52},{-3,-45},{3,-46},{4,-42},{12,-43},{22,-41},{7,-29}},'steel2')
  poly(c,{{-31,-41},{7,-26},{8,-5},{-32,-18}},'rust1')
  poly(c,{{9,-27},{22,-39},{25,-18},{10,-8}},'rust2')
  for i=0,5 do
    local x=-27+i*5;local y=-38+i*2
    line(c,x,y,x,y+17,'rust0');line(c,x+1,y+1,x+1,y+16,'rust2')
  end
  poly(c,{{-18,-42},{-8,-49},{10,-41},{1,-35}},'steel0')
  poly(c,{{-9,-50},{-6,-51},{-5,-45},{2,-43},{-1,-41}},'steel3')
  line(c,-31,-43,-11,-53,'steel3');line(c,-28,-42,-10,-35,'ivory1')
  poly(c,{{-22,-24},{-15,-22},{-13,-18},{-20,-20}},'ivory1')
  -- Crumpled door slab spills into the foreground, detached from its hinges.
  c:layer(5)
  poly(c,{{7,-6},{23,-18},{34,-9},{36,-1},{21,5}},'rust0')
  poly(c,{{9,-8},{23,-19},{30,-11},{23,-6},{22,2}},'rust3')
  line(c,10,-9,23,-18,'rust4');line(c,17,-6,27,-14,'rust2')
  line(c,21,-3,29,-9,'rust1');line(c,23,-6,33,-4,'rust2')
  debris(c)
  for _,p in ipairs({{-28,-27},{-22,-34},{-9,-21},{14,-26},{-18,-15}}) do wear(c,p[1],p[2],'rust') end
  poly(c,{{-32,-18},{-27,-18},{-24,-14},{-14,-12},{-12,-9},{-23,-12}},'earth1')
  line(c,-26,-17,-23,-15,'earth2');line(c,-17,-11,-12,-9,'earth2')
end
return out
