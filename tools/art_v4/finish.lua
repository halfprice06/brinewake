-- Root's material overpaint. Each contour is attached to a specific existing
-- surface. Masks protect silhouettes, glazing and foreground machinery.
local out={}
local ivory={'ivory1','ivory2','ivory3','ivory4'}
local rust={'rust1','rust2','rust3'}
local jade={'jade1','jade2','jade3','jade4'}
local steel={'steel1','steel2','steel3','steel4'}
local function patch(c,points,color,allowed)
  local p={};for _,v in ipairs(points) do p[#p+1]={c.cx+v[1],c.cy+v[2]} end
  c:overpaint(p,color,allowed)
end
local function stroke(c,x,y,w,color,allowed)
  patch(c,{{x,y},{x+w,y-2},{x+w,y},{x,y+2}},color,allowed)
end
local function scar(c,x,y,color,allowed)
  patch(c,{{x,y},{x+4,y-1},{x+6,y},{x+4,y+2},{x+1,y+2}},color,allowed)
end
local function roof(c,x,y,w,d)
  -- Recessed maintenance panel with a light seam and a broken lower lip.
  patch(c,{{x-w,y-w/2},{x-w+d,y-(w+d)/2},{x+d,y-d/2},{x,y}},'ivory2',ivory)
  patch(c,{{x-w+2,y-w/2},{x-w+d,y-(w+d)/2+1},{x+d-2,y-d/2},{x,y-1}},'ivory3',{'ivory2'})
  stroke(c,x-w+3,y-w/2,math.max(4,w-5),'ivory1',ivory)
end
local function hull(c,f,s,z,w,h,mat)
  local side=c.co>=0 and 1 or -1
  s=s*side
  c:overpaint3({{f-w,s,z},{f+w,s,z},{f+w,s,z+h},{f-w,s,z+h}},mat..'0',{mat..'1',mat..'2'})
  c:overpaint3({{f-w+1,s,z+1},{f+w-1,s,z+1},{f+w-1,s,z+h-1},{f-w+1,s,z+h-1}},mat..'2',{mat..'0'})
  c:line3({f-w+1,s,z+h},{f-1,s,z+h},mat..'3')
end

out.union_hq=function(c)
  c:layer(6)
  roof(c,-17,-68,16,27)
  roof(c,-22,-38,25,18)
  roof(c,27,-48,20,19)
  -- The raised wheelhouse casts a coherent shadow onto the lower roof.
  patch(c,{{-22,-60},{0,-64},{20,-46},{3,-37},{-5,-41}},'ivory1',ivory)
  patch(c,{{-21,-60},{-6,-62},{10,-47},{-2,-42}},'ivory2',{'ivory1'})
  for _,p in ipairs({{-31,-80},{-4,-79},{-45,-46},{-28,-41},{31,-54},{42,-44}}) do scar(c,p[1],p[2],'ivory1',ivory) end
  patch(c,{{-55,-17},{-22,-4},{-22,-12},{-48,-26}},'rust1',rust)
  scar(c,-41,-28,'rust0',rust);scar(c,34,-24,'rust0',rust)
  stroke(c,-47,-26,6,'rust3',rust)
  patch(c,{{-25,-18},{-24,-18},{-24,-8},{-25,-8}},'rust0',rust)
  -- Close panels and fasteners are grouped around the service opening.
  stroke(c,8,-30,7,'steel4',steel)
  scar(c,0,-20,'steel1',steel)
end
out.union_works=function(c)
  c:layer(6)
  roof(c,-17,-61,13,24);roof(c,9,-47,10,22)
  patch(c,{{-34,-58},{-11,-69},{0,-47},{-21,-35}},'ivory1',ivory)
  for _,p in ipairs({{-27,-59},{-5,-55},{19,-49},{29,-48},{-34,-38}}) do scar(c,p[1],p[2],'ivory1',ivory) end
  patch(c,{{-40,-14},{0,4},{0,-5},{-40,-23}},'rust1',rust)
  for _,p in ipairs({{-36,-23},{-17,-13},{29,-19},{37,-12}}) do scar(c,p[1],p[2],'rust0',rust) end
  stroke(c,-27,-29,8,'rust3',rust)
end
out.assembly_hq=function(c)
  c:layer(6)
  -- Overlapping composite plates follow the vault's curve and retain a broad
  -- lit side. Their seams stop before the cream leading rib.
  patch(c,{{-47,-27},{-36,-38},{-29,-60},{-40,-54}},'jade1',jade)
  patch(c,{{-32,-64},{-18,-69},{-4,-87},{-15,-82}},'jade2',jade)
  patch(c,{{10,-83},{20,-69},{34,-54},{35,-61},{21,-80}},'jade1',jade)
  patch(c,{{37,-45},{43,-21},{50,-15},{48,-33}},'jade0',jade)
  stroke(c,-33,-61,8,'jade4',jade)
  stroke(c,-24,-74,9,'jade3',jade)
  for _,p in ipairs({{-42,-39},{-26,-67},{18,-77},{37,-39}}) do scar(c,p[1],p[2],'jade1',jade) end
  -- Pressure-glass reflections are narrow and interrupted by the metal belt.
  patch(c,{{-5,-47},{0,-49},{0,-26},{-3,-23}},'amber3',{'amber2','amber3'})
  scar(c,-30,-18,'reed0',{'reed1','reed2','reed3'})
end
out.assembly_works=function(c)
  c:layer(6)
  patch(c,{{-42,-38},{-25,-40},{-12,-58},{-20,-60}},'jade2',jade)
  patch(c,{{0,-63},{15,-55},{30,-30},{18,-24}},'jade1',jade)
  stroke(c,-32,-44,10,'jade4',jade)
  for _,p in ipairs({{-30,-45},{-19,-56},{13,-51},{30,-35}}) do scar(c,p[1],p[2],'jade1',jade) end
  scar(c,-33,-21,'reed0',{'reed1','reed2','reed3'})
  scar(c,28,-28,'reed0',{'reed1','reed2','reed3'})
end
out.dropoff=function(c)
  c:layer(6)
  for _,p in ipairs({{-20,-24},{-9,-20},{20,-23},{16,-38},{31,-37}}) do scar(c,p[1],p[2],'rust1',rust) end
  patch(c,{{-26,-21},{-13,-14},{-13,-24},{-26,-31}},'rust0',rust)
  stroke(c,12,-35,7,'rust4',rust)
end
out.condenser=function(c)
  c:layer(6)
  patch(c,{{-28,-46},{-24,-47},{-24,-16},{-27,-13}},'steel3',steel)
  patch(c,{{21,-39},{28,-40},{29,-11},{24,-10}},'steel0',steel)
  for _,p in ipairs({{-26,-20},{-19,-37},{24,-19},{7,-42}}) do scar(c,p[1],p[2],'steel1',steel) end
  stroke(c,7,-8,10,'steel4',steel)
end
out.tower=function(c)
  c:layer(6)
  roof(c,1,-61,12,14)
  patch(c,{{-19,-46},{-2,-37},{-2,-41},{-17,-49}},'rust0',rust)
  for _,p in ipairs({{-16,-22},{-6,-17},{12,-18},{0,-64}}) do scar(c,p[1],p[2],'steel1',steel) end
  stroke(c,17,-44,12,'steel4',steel)
end
out.gate=function(c)
  c:layer(6)
  local concrete={'earth1','earth2','earth3','earth4','ivory2','ivory3'}
  patch(c,{{-43,-37},{-34,-33},{-34,-5},{-44,-10}},'earth1',concrete)
  patch(c,{{34,-41},{42,-44},{43,-9},{35,-4}},'earth0',concrete)
  for _,p in ipairs({{-38,-29},{-31,-18},{25,-28},{39,-41},{-23,-55},{12,-55}}) do scar(c,p[1],p[2],'earth0',concrete) end
  stroke(c,-24,-58,14,'earth4',concrete)
end
out.salvage=function(c)
  c:layer(6)
  for _,p in ipairs({{-18,-25},{-27,-18},{-10,-9},{26,-10},{30,-5}}) do scar(c,p[1],p[2],'rust0',rust) end
  patch(c,{{-10,-34},{5,-32},{18,-26},{10,-23}},'ivory0',ivory)
  stroke(c,1,-35,8,'ivory3',ivory)
end
out.well=function(c)
  c:layer(6)
  scar(c,-16,-25,'steel0',steel)
  stroke(c,-15,-28,7,'steel4',steel)
end

for _,name in ipairs({'hook','riveter','bulwark','sounder','wick','skipper','reedguard','loom'}) do
  out[name]=function(c)
    c:layer(6)
    if name=='hook' then
      hull(c,-7,7,12,4,5,'rust')
      c:overpaint3({{-3,-2,35},{3,-2,37},{6,-2,37},{1,-2,35}},'ivory1',ivory)
      c:line3({10,-3,15},{12,-3,15},'glass3')
    elseif name=='riveter' then
      hull(c,-9,6,16,4,5,'rust')
      c:line3({12,-2,19},{18,-2,19},'steel3')
    elseif name=='bulwark' then
      local front=c.si>=0 and 15 or 9
      c:overpaint3({{front,-9,8},{front,9,8},{front,7,13},{front,-8,11}},'rust1',rust)
      c:overpaint3({{front,-7,17},{front,-4,17},{front,-3,20},{front,-7,21}},'ivory1',ivory)
    elseif name=='sounder' then
      hull(c,-3,6,12,5,3,'rust')
    elseif name=='wick' then
      hull(c,-3,8,12,5,4,'jade')
    elseif name=='skipper' then
      c:overpaint3({{-12,-8,16},{-4,-11,16},{1,-10,16},{-8,-6,16}},'jade2',jade)
      c:overpaint3({{-9,5,16},{2,8,16},{8,5,16},{1,10,16}},'jade1',jade)
    elseif name=='reedguard' then
      hull(c,-3,9,14,5,4,'jade')
    elseif name=='loom' then
      c:overpaint3({{-10,-6,23},{-10,6,23},{-7,6,30},{-7,-6,30}},'jade1',jade)
    end
  end
end
return out
