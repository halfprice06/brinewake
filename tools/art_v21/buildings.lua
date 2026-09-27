-- The Saltglass Compact's buildings, in the v18 structures vocabulary
-- (screen-space 2:1 planes, the front corner of a footprint at (x,y) below
-- the anchor) with the Compact's materials: cobalt-violet glaze walls on
-- short stilts, salt-white crust slabs and caps, copper fittings, glass
-- roofs, and mirrors that gather the sun. Mirrors and lenses are drawn in
-- the machines' local 3D (compact.lua) on a painter set to facing 1, whose
-- ground axes run along the screen's 2:1 diagonals.
-- No shadow is drawn beneath a building: the contact layer stays empty.
--   Kiln (compact_hq), Glassworks (compact_works), Drydock (compact_drydock),
--   Rake shed (compact_dropoff), Mirror nest (compact_tower): 160x144,
--   anchor 80,128. Palisade (compact_palisade): 64x64, anchor 32,50.
local root=assert(app.params.root)
local K=dofile(root..'/tools/art_v21/compact.lua')
local out={}
-- Budget: copper in two shades, no black: the darkest glaze is the ink.
out.BUDGET={rust0='glaze0',rust3='rust2',rust4='amber3',glass0='glaze0',glass4='glint',ink='glaze0'}

local function point(c,x,y) return {c.cx+x,c.cy+y} end
local function poly(c,p,color)
  local q={};for _,v in ipairs(p) do q[#q+1]=point(c,v[1],v[2]) end;c:poly(q,color)
end
local function line(c,x,y,xx,yy,color,width) c:line(point(c,x,y),point(c,xx,yy),color,width) end
local function rect(c,x,y,w,h,color) poly(c,{{x,y},{x+w,y},{x+w,y+h},{x,y+h}},color) end
local function px(c,x,y,color) c:pixel(c.cx+x,c.cy+y,color) end
local function ellipse(c,x,y,rx,ry,col) c:ellipse(c.cx+x,c.cy+y,rx,ry,col) end
local function block(c,x,y,w,d,h,mat)
  poly(c,{{x-w,y-w/2-h},{x,y-h},{x,y},{x-w,y-w/2}},mat..'2')
  poly(c,{{x,y-h},{x+d,y-d/2-h},{x+d,y-d/2},{x,y}},mat..'1')
  poly(c,{{x-w,y-w/2-h},{x-w+d,y-(w+d)/2-h},{x+d,y-d/2-h},{x,y-h}},mat..'3')
  line(c,x-w,y-w/2,x,y,mat..'0')
  line(c,x,y,x+d,y-d/2,mat..'0')
  line(c,x-w,y-w/2-h,x,y-h,mat..'4')
  line(c,x,y-h,x+d,y-d/2-h,mat..'2')
  if h>1 then line(c,x,y-h+1,x,y-1,mat..'0') end
end
-- A glazed wall block with the Compact's salt seam at its foot.
local function glazed(c,x,y,w,d,h)
  block(c,x,y,w,d,h,'glaze')
  line(c,x-w+1,y-w/2-1,x-1,y-1,'crust1')
  line(c,x+1,y-1,x+d-1,y-d/2-1,'crust0')
end
local function panel(c,x,y,length,h,side,col,rim)
  poly(c,{{x,y},{x+side*length,y-length/2},{x+side*length,y-length/2-h},{x,y-h}},col)
  if rim then
    line(c,x,y-h,x+side*length,y-length/2-h,rim)
    line(c,x,y,x+side*length,y-length/2,rim)
  end
end
local function window(c,x,y,length,h,side)
  panel(c,x,y,length,h,side,'glaze0')
  panel(c,x+side,y-1,length-2,h-2,side,'glass1')
  panel(c,x+side*2,y-h+2,math.max(1,length-4),1,side,'glass3')
end
local function cylinder(c,x,y,r,h,mat)
  local ry=math.floor(r/2)
  for dx=-r,r do
    local e=math.floor(math.sqrt(math.max(0,1-(dx/r)^2))*ry+.5)
    local shade=dx<-r*0.6 and 2 or (dx<-r*0.1 and 3 or (dx<r*0.45 and 2 or 1))
    line(c,x+dx,y-h+e,x+dx,y+e,mat..shade)
    px(c,x+dx,y+e,mat..'0')
  end
  ellipse(c,x,y-h,r,ry,mat..'3')
  line(c,x-r+2,y-h-1,x-1,y-h-ry+1,mat..'4')
end
local function band(c,x,y,r,col) -- a ring round a cylinder's front half
  for dx=-r,r do local dy=math.floor(math.sqrt(math.max(0,1-(dx/r)^2))*r/2+.5);px(c,x+dx,y+dy,col) end
end
local function pipe(c,x,y,xx,yy,mat,width)
  line(c,x+1,y+1,xx+1,yy+1,'glaze0',(width or 3)+1)
  line(c,x,y,xx,yy,mat..'1',width or 3)
  line(c,x,y,xx,yy,mat..'3')
end
-- A stilt: a glazed leg with a copper collar and a salt shoe.
local function stilt(c,x,y,h)
  line(c,x,y,x,y-h,'glaze1',2)
  line(c,x,y-h,x,y-1,'glaze1');line(c,x-1,y-h,x-1,y-1,'glaze3')
  px(c,x,y-math.floor(h/2),'rust2');px(c,x-1,y-math.floor(h/2),'rust1')
  line(c,x-3,y,x+2,y,'glaze0');line(c,x-2,y-1,x+1,y-1,'crust1')
end
-- The salt pad every Compact building stands on: a crust slab.
-- Pale but not bright: the top is laid in tiles with darker joints.
local function pad(c,x,y,w,d)
  y=y+2;w=w+3;d=d+3
  local h=3
  poly(c,{{x-w,y-w/2-h},{x,y-h},{x,y},{x-w,y-w/2}},'crust0')
  poly(c,{{x,y-h},{x+d,y-d/2-h},{x+d,y-d/2},{x,y}},'glaze2')
  poly(c,{{x-w,y-w/2-h},{x-w+d,y-(w+d)/2-h},{x+d,y-d/2-h},{x,y-h}},'crust1')
  line(c,x-w,y-w/2,x,y,'glaze0');line(c,x,y,x+d,y-d/2,'glaze0')
  line(c,x-w,y-w/2-h,x,y-h,'crust2');line(c,x,y-h,x+d,y-d/2-h,'crust0')
  for a=12,w-6,12 do line(c,x-a+1,y-a/2-h-1,x-a+d-1,y-(a+d)/2-h,'crust0') end
  for a=12,d-6,12 do line(c,x+a-1,y-a/2-h-1,x+a-w+1,y-(a+w)/2-h,'crust0') end
end
-- A salt pan: a shallow crust-rimmed dish of brine on the pad.
local function salt_pan(c,x,y,w,d,dry)
  poly(c,{{x-w,y-w/2},{x,y},{x+d,y-d/2},{x+d-w,y-(w+d)/2}},'crust2')
  poly(c,{{x-w+2,y-w/2},{x,y-1},{x+d-2,y-d/2},{x+d-w,y-(w+d)/2+1}},dry and 'crust1' or 'glass1')
  if not dry then line(c,x-w+4,y-w/2-1,x-2,y-3,'glass2');px(c,x-w+5,y-w/2-2,'glint') end
  line(c,x-w,y-w/2,x,y,'crust0');line(c,x,y,x+d,y-d/2,'glaze1')
end
-- A roof of glass panes between copper ribs, on the up-right slope.
local function glass_roof(c,x,y,w,d,rise)
  poly(c,{{x-w,y-w/2},{x,y},{x+d,y-d/2},{x+d-w,y-(w+d)/2}},'glass1')
  for a=0,w,6 do line(c,x-a,y-a/2,x-a+d,y-(a+d)/2,'rust1') end
  for a=3,w-2,6 do line(c,x-a+2,y-a/2-2,x-a+math.floor(d/2),y-a/2-math.floor(d/4)-1,'glass2') end
  line(c,x-w,y-w/2,x,y,'glaze0');line(c,x,y,x+d,y-d/2,'glaze0')
  line(c,x-w+1,y-w/2-1,x-w+5,y-w/2-3,'glint')
end

-- Local 3D on a facing-1 painter from a screen offset: the ground point
-- under (sx,sy), for mirrors and lenses.
local function ground(sx,sy)
  local a,b=sx/0.7071,sy/0.35355
  return (a-b)/2,(a+b)/2
end
-- A mirror with a sheen streak across its glass, lower left to upper
-- right, so it reads as a mirror and not a ball.
local function mirror_at(c,sx,sy,z,n,r,opts)
  local f,s=ground(sx,sy)
  local face=K.mirror(c,{f,s,z},n,r,opts)
  if face then
    local p=c:point(f,s,z)
    local k=math.floor(r*0.7)
    c:overpaint({{p[1]-k,p[2]+k/2+1},{p[1]-k,p[2]+k/2-1},{p[1]+k,p[2]-k/2-2},{p[1]+k,p[2]-k/2}},'glass3',{'glass1','glass2'})
    c:overpaint({{p[1]-k+2,p[2]+k/2+3},{p[1]-k+2,p[2]+k/2+2},{p[1]+k-2,p[2]-k/2+1},{p[1]+k-2,p[2]-k/2+2}},'glass3',{'glass1','glass2'})
  end
  return face
end
local function lens_at(c,sx,sy,z,r)
  local f,s=ground(sx,sy)
  K.lens(c,f,s,z,r)
end
-- A heliostat on a stilt: a mirror on a copper yoke atop a glazed post.
-- n is the local normal (the painter's facing-1 axes: +f up-right, +s
-- down-right on screen).
local function heliostat(c,sx,sy,h,n,r)
  stilt(c,sx,sy,h)
  -- The fork: a copper yoke from the post top to the dish's sides.
  line(c,sx,sy-h,sx-r+2,sy-h-r+2,'rust1');line(c,sx,sy-h,sx+r-2,sy-h-r+2,'rust1')
  line(c,sx-r+2,sy-h-r+2,sx-r+2,sy-h-r,'rust2');line(c,sx+r-2,sy-h-r+2,sx+r-2,sy-h-r,'rust2')
  mirror_at(c,sx,sy-h,r,n,r,{rim='rust1'})
  px(c,sx-1,sy-h+1,'rust2');px(c,sx,sy-h+1,'rust2')
end

------------------------------------------------------------------------------
-- The Kiln: the Compact's seat. A great bottle kiln of violet glaze banded
-- with copper and capped with salt, its arched mouth glowing, fed by two
-- heliostats on stilts that turn the sun into its fire, with a glass-roofed
-- packing shed behind.
out.compact_hq=function(c,phase)
  c:layer(2)
  pad(c,0,6,50,50)
  -- Salt pans drying along the left front edge.
  -- The packing shed behind, to the right.
  glazed(c,34,-20,18,20,18)
  glass_roof(c,34,-38,18,20)
  window(c,37,-24,14,8,1)
  panel(c,16,-29,8,10,-1,'glaze0')
  c:layer(3)
  -- The bottle: a wide glazed body, a crust shoulder and a tapering neck.
  cylinder(c,-2,-14,26,32,'glaze')
  for a=-18,18,12 do line(c,-2+a,-14-32+2,-2+a,-14+math.floor(math.sqrt(1-(a/26)^2)*13)-1,'glaze1') end
  band(c,-2,-24,26,'rust1');band(c,-2,-25,26,'rust2')
  band(c,-2,-40,26,'crust1')
  cylinder(c,-2,-46,22,5,'crust')
  cylinder(c,-2,-51,17,10,'glaze')
  band(c,-2,-56,17,'rust2')
  cylinder(c,-2,-61,12,10,'glaze')
  cylinder(c,-2,-71,8,4,'crust')
  cylinder(c,-2,-75,6,10,'glaze')
  cylinder(c,-2,-85,7,2,'crust')
  ellipse(c,-2,-87,4,1,'glaze0')
  -- The mouth: an arch on the front, the fire inside.
  local glow=phase and ({0,1,2,1})[phase+1] or 0
  poly(c,{{-12,-6},{-12,-16},{-9,-21},{-3,-23},{3,-21},{6,-16},{6,-6}},'glaze0')
  poly(c,{{-10,-7},{-10,-15},{-8,-19},{-3,-21},{2,-19},{4,-15},{4,-7}},'amber2')
  poly(c,{{-7,-7},{-7,-13},{-3,-16-glow},{1,-13},{1,-7}},'amber3')
  line(c,-5,-8,-1,-8,'glint')
  line(c,-13,-5,7,-5,'crust2');line(c,-13,-4,7,-4,'crust0')
  -- Peep holes round the bottle.
  for _,p in ipairs({{-20,-12},{14,-12}}) do rect(c,p[1],p[2],2,3,'glaze0');px(c,p[1],p[2]+2,'amber2') end
  c:layer(4)
  -- Heat off the neck: a shimmer that rises and thins.
  if phase then
    local rows=({{{-1,3},{0,2}},{{-2,4},{-1,3},{0,1}},{{-1,3},{1,3},{2,1}},{{0,3},{2,2}}})[phase+1]
    for j,r in ipairs(rows) do for i=0,r[2]-1 do px(c,-2+r[1]+i,-89-j*2-phase,(j==1) and 'crust1' or 'crust2') end end
  end
  -- Salt pans drying on the pad in front of the kiln.
  salt_pan(c,-34,-5,12,8);salt_pan(c,-46,-12,8,8,true);salt_pan(c,14,2,8,10)
  c:layer(5)
  -- Two heliostats at the corners, their mirrors turned in on the mouth.
  heliostat(c,-46,-18,30,{0.1,0.45,0.88},10)
  heliostat(c,44,-14,28,{-0.5,0.2,0.84},9)
  -- The sun thrown into the mouth: a thin light line when the kiln works.
  if phase and phase%2==0 then
    for i=0,28,2 do px(c,-40+i,-44+i,(i%4==0) and 'crust2' or 'crust1') end
    px(c,-40,-44,'glint')
    for i=0,34,2 do px(c,40-i,-46+math.floor(i*30/36),(i%4==0) and 'crust2' or 'crust1') end
  end
  c:layer(6)
  line(c,-26,-40,-26,-12,'glaze4')
  line(c,-17,-58,-17,-52,'glaze4')
  px(c,-12,-68,'glaze4');px(c,-11,-69,'glaze4')
end
out.compact_hq_lamps=function(c)
  c:layer(6)
  for _,p in ipairs({{-20,-12},{14,-12}}) do px(c,p[1],p[2],'amber3');px(c,p[1]+1,p[2]+1,'amber2') end
  for _,p in ipairs({{40,-28},{45,-31},{49,-33}}) do px(c,p[1],p[2],'amber3');px(c,p[1],p[2]+1,'amber2') end
  px(c,-2,-86,'amber3')
end

------------------------------------------------------------------------------
-- The Glassworks: a long glazed hall raised on stilts under a glass roof,
-- a tall thin chimney banded in copper, a furnace door, and a cooling rack
-- of new panes in front.
out.compact_works=function(c,phase)
  c:layer(2)
  pad(c,0,6,50,50)
  -- The chimney at the back.
  cylinder(c,24,-40,5,44,'glaze')
  band(c,24,-56,5,'rust2');band(c,24,-70,5,'rust2')
  cylinder(c,24,-84,6,3,'crust')
  c:layer(3)
  -- Stilts under the hall, then the hall.
  for _,p in ipairs({{-40,-12},{-6,4},{36,-14},{0,-30}}) do stilt(c,p[1],p[2],9) end
  glazed(c,2,-6,42,40,20)
  -- Plate seams on the long wall.
  for a=10,36,13 do line(c,2-a,-6-a/2-2,2-a,-6-a/2-19,'glaze1') end
  glass_roof(c,2,-26,42,40)
  -- The ridge: a crust cap along the roof.
  line(c,-19,-38,21,-58,'crust2');line(c,-19,-37,21,-57,'crust0')
  c:layer(4)
  window(c,-30,-16,12,8,-1)
  window(c,6,-12,14,9,1)
  window(c,24,-21,12,9,1)
  -- The furnace door on the front corner, glowing.
  local glow=phase and ({0,1,2,1})[phase+1] or 0
  panel(c,-14,-9,10,12,-1,'glaze0')
  panel(c,-15,-10,8,10,-1,'amber2')
  panel(c,-16,-11,5,6+glow,-1,'amber3')
  c:layer(5)
  -- The cooling rack: two stilts and a copper bar hung with new panes.
  local rx,ry=-46,-6
  stilt(c,rx,ry,21);stilt(c,rx+22,ry-11,21)
  line(c,rx-1,ry-21,rx+23,ry-33,'rust2');line(c,rx-1,ry-22,rx+23,ry-34,'rust1')
  local slide=phase and ({0,1,2,1})[phase+1] or 0
  for i=0,2 do
    local x=rx+3+i*6+((i==1) and slide or 0)
    local yb=ry-20-math.floor((x-rx)/2)+9
    panel(c,x,yb,4,8,1,'glass1');panel(c,x+1,yb-1,2,6,1,'glass2');px(c,x+1,yb-7,'glint')
  end
  -- Smoke off the chimney.
  if phase then
    c:layer(6)
    local rows=({{{0,3},{1,2}},{{-1,4},{0,3},{1,1}},{{0,3},{2,3},{3,1}},{{1,3},{3,2}}})[phase+1]
    for j,r in ipairs(rows) do for i=0,r[2]-1 do px(c,24+r[1]+i-1,-89-j*2-phase,(j==1) and 'crust0' or 'crust1') end end
  end
  c:layer(6)
  line(c,-38,-30,-38,-18,'glaze4')
  line(c,20,-46,20,-80,'glaze4')
end
out.compact_works_lamps=function(c)
  c:layer(6)
  for _,p in ipairs({{-33,-20},{-37,-18},{10,-18},{15,-20},{28,-26},{32,-28}}) do px(c,p[1],p[2],'amber3') end
  px(c,-15,-15,'glint')
end

------------------------------------------------------------------------------
-- The Drydock: an open slip under a stilted gantry, a new machine standing
-- in its cradle, and a glazed store with a glass roof behind.
out.compact_drydock=function(c,phase)
  c:layer(2)
  pad(c,2,6,48,48)
  glazed(c,32,-20,16,16,20)
  glass_roof(c,32,-40,16,16)
  window(c,35,-25,11,8,1)
  c:layer(3)
  -- The slip: a sunken bay of brine between crust kerbs, rails along it.
  poly(c,{{-34,-8},{-2,8},{22,-4},{-10,-20}},'glaze1')
  poly(c,{{-30,-8},{-2,6},{18,-4},{-10,-18}},'glass1')
  line(c,-24,-6,6,-21,'glass2');line(c,-14,-1,12,-14,'glass2')
  line(c,-34,-8,-2,8,'crust1');line(c,-2,8,22,-4,'crust0')
  -- The machine in the cradle: a new walker's glazed pod on half-built
  -- stilts, its crust cap on and its lens still dark.
  for _,l in ipairs({{-18,-8},{-6,-2},{-2,-16},{-14,-20}}) do line(c,l[1],l[2],l[1],l[2]-14,'glaze1');line(c,l[1]-1,l[2],l[1]-1,l[2]-14,'glaze3') end
  glazed(c,-8,-16,12,12,9)
  block(c,-8,-25,8,8,3,'crust')
  px(c,-1,-21,'glass1');px(c,0,-22,'glass1')
  -- Stacked plates and salt blocks waiting on the pad.
  c:layer(4)
  block(c,26,4,8,10,3,'glaze');block(c,26,1,8,10,3,'glaze');block(c,26,-2,7,9,2,'crust')
  -- A heliostat on the right corner, lighting the slip.
  c:layer(5)
  heliostat(c,46,-12,26,{-0.6,0.15,0.8},9)
  c:layer(5)
  -- The gantry: two stilted legs, a copper beam, and a hook that works.
  pipe(c,-40,-6,-40,-58,'glaze',3)
  pipe(c,8,-26,8,-72,'glaze',3)
  line(c,-43,-6,-37,-6,'glaze0');line(c,-42,-7,-38,-7,'crust1')
  line(c,5,-26,11,-26,'glaze0');line(c,6,-27,10,-27,'crust1')
  pipe(c,-40,-58,8,-72,'rust',4)
  local drop=phase and ({0,3,6,3})[phase+1] or 0
  line(c,-14,-65,-14,-44+drop,'glaze0')
  rect(c,-16,-44+drop,5,3,'rust2');px(c,-14,-41+drop,'rust1')
  if phase and phase==2 then
    -- A weld: the lens torch flaring on the pod.
    px(c,-14,-26,'glint');px(c,-15,-26,'amber3');px(c,-13,-27,'amber3');px(c,-14,-28,'amber2');px(c,-16,-24,'amber2')
  end
  c:layer(6)
  line(c,-42,-50,-42,-12,'glaze4')
  line(c,20,-40,20,-28,'glaze4')
end

------------------------------------------------------------------------------
-- The Rake shed: a crust-roofed canopy on four stilts over a heap of
-- salvage, a copper hopper on one side and a spare rake leaning on a post.
out.compact_dropoff=function(c,phase)
  c:layer(2)
  pad(c,0,6,46,46)
  c:layer(3)
  -- Back posts, the heap, then front posts and the roof.
  stilt(c,0,-40,40);stilt(c,-40,-18,40);stilt(c,40,-18,40)
  -- The heap: copper scrap and glass cullet in a crust bin.
  block(c,0,-4,26,26,6,'glaze')
  poly(c,{{-22,-18},{-12,-26},{2,-28},{16,-24},{24,-18},{8,-12},{-4,-10}},'rust1')
  poly(c,{{-16,-18},{-8,-24},{2,-25},{12,-21},{6,-16},{-4,-14}},'rust2')
  for _,p in ipairs({{-10,-20},{4,-22},{10,-19},{-4,-17},{-14,-19}}) do px(c,p[1],p[2],'glass3');px(c,p[1]+1,p[2],'glass2') end
  px(c,-2,-24,'crust2');px(c,8,-23,'crust2')
  c:layer(4)
  -- The hopper on the right, a chute into the bin.
  block(c,34,-6,10,10,16,'rust')
  block(c,34,-22,12,12,3,'crust')
  line(c,24,-12,14,-10,'glaze0',2)
  c:layer(5)
  stilt(c,0,4,40)
  -- The roof: a low crust pitch.
  block(c,0,-36,46,46,3,'glaze')
  for a=8,40,8 do line(c,-a,-39-a/2,-a+45,-39-(a+45)/2,'glaze2') end
  line(c,-46,-62,0,-39,'crust2');line(c,-46,-61,0,-38,'crust1')
  line(c,-23,-50,23,-73,'crust2');line(c,-23,-51,23,-74,'crust1')
  -- The spare rake leaning on the front post.
  line(c,-8,2,-20,-34,'rust2');line(c,-9,2,-21,-34,'rust1')
  line(c,-25,-36,-15,-33,'glaze0');for i=0,3 do px(c,-24+i*3,-35,'crust1') end
  -- A raker's lamp-lens on the front post, winking when salvage lands.
  lens_at(c,1,-20,0,1)
  if phase then px(c,1,-20,'glint') end
end

------------------------------------------------------------------------------
-- The Mirror nest: the Compact's defence. A tall stilted tripod carrying a
-- glazed platform and a cluster of mirror dishes round a focusing lens.
out.compact_tower=function(c)
  c:layer(2)
  pad(c,0,6,30,30)
  stilt(c,0,-26,46)
  c:layer(3)
  stilt(c,-24,-8,46);stilt(c,22,-10,46)
  -- Cross braces.
  line(c,-24,-30,0,-44,'rust1');line(c,0,-44,22,-32,'rust1')
  c:layer(4)
  stilt(c,-2,6,46)
  -- The platform.
  glazed(c,-1,-40,16,16,6)
  block(c,-1,-46,12,12,2,'crust')
  c:layer(5)
  -- The dishes: a big one turned to the front, two smaller at its sides.
  mirror_at(c,-18,-58,0,{0.2,0.5,0.84},9,{rim='rust1',concave=true})
  mirror_at(c,17,-60,0,{-0.55,0.1,0.83},9,{rim='rust1',concave=true})
  mirror_at(c,-1,-60,8,{-0.3,0.35,0.88},14,{rim='rust1',concave=true})
  -- The lens on its boom, where the light meets.
  line(c,-1,-52,-1,-84,'rust1');line(c,0,-52,0,-84,'rust2')
  lens_at(c,-1,-86,0,3)
  c:layer(6)
  line(c,-26,-44,-26,-16,'glaze4')
end

------------------------------------------------------------------------------
-- The Palisade: salt blocks mortared between glazed posts, one segment
-- along the up-right diagonal, capped in glaze.
out.spec={
  compact_hq={x=0,y=6,w=50,d=50,h=40},
  compact_works={x=0,y=6,w=46,d=44,h=36},
  compact_drydock={x=2,y=6,w=46,d=46,h=36},
  compact_dropoff={x=0,y=6,w=44,d=44,h=36},
  compact_tower={x=0,y=6,w=28,d=28,h=46},
  compact_palisade={x=-11,y=6,w=4,d=26,h=13},
}
out.compact_palisade=function(c)
  c:layer(2)
  block(c,-11,8,6,28,2,'crust')
  c:layer(3)
  block(c,-11,6,4,26,12,'crust')
  -- The courses: blocks laid with offset joints.
  for i=0,2 do
    local y=4-i*4
    line(c,-10,y,15,y-12,'crust0')
    for j=0,3 do local o=(i%2==0) and 0 or 3;local x=-8+o+j*6;px(c,x,y-x/2-4+j*0,'crust0') end
  end
  block(c,-11,-6,5,27,2,'glaze')
  c:layer(5)
  block(c,-14,8,4,4,16,'glaze');block(c,12,-5,4,4,16,'glaze')
  block(c,-14,-8,4,4,1,'crust');block(c,12,-21,4,4,1,'crust')
  c:layer(6)
  line(c,-10,-9,14,-21,'glaze4')
  px(c,-7,1,'crust2');px(c,6,-6,'crust2')
end

-- Construction: the salt pad laid out, a stilted frame, then the finished
-- drawing. Uprights end at foundation points.
function out.construction(c,name,stage)
  local s=out.spec[name];local x,y,w,d,h=s.x,s.y,s.w,s.d,s.h
  if stage==2 then out[name](c,nil);return end
  local left={x-w,y-w/2};local right={x+d,y-d/2};local back={x-w+d,y-(w+d)/2}
  c:layer(2)
  if w>20 then pad(c,x,y,w,d) else block(c,x,y,w,d,2,'glaze') end
  line(c,left[1]+4,left[2]-3,x,y-3,'glaze1',2)
  line(c,x,y-3,right[1]-4,right[2]-3,'glaze1',2)
  c:layer(3)
  local core=stage==0 and math.floor(h/5) or math.floor(h/2)
  if w>20 then
    block(c,x+4,y-8,math.floor(w*0.5),math.floor(d*0.5),core,'glaze')
    for i=0,2 do block(c,x+14+i*4,y-16+i*2,4,12,3,'glaze') end
    block(c,x-24,y-10,6,6,4,'crust')
  else
    block(c,x,y,w,d,core,'crust')
  end
  if stage==1 then
    c:layer(5)
    for _,p in ipairs({left,right,back}) do stilt(c,p[1]+2,p[2],h) end
    line(c,left[1]+2,left[2]-h,back[1]+2,back[2]-h,'rust1',2)
    line(c,back[1]+2,back[2]-h,right[1]+2,right[2]-h,'rust1',2)
  end
end
return out
