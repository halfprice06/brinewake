-- Root-authored v15 drowned town: six submerged silhouettes for the deep
-- water of the basin. Drawn in the darkest sea shades on a transparent
-- 64x32 canvas anchored at a cell centre; the engine paints them over the
-- water fill and under the travelling crests. Outlines are broken in short
-- runs so the shapes read as things under water rather than as islands.
local root=assert(app.params.root)
local C=dofile(root..'/tools/art_v15/common.lua')
local m,packer=C.manifest()
local entries={};local out=root..'/output/art-v15/submerged';os.execute('mkdir -p '..out)
local layers={'deep silhouette','softer edge'}
-- Dimetric helpers: (f,s) are ground units along the two tile axes.
local function iso(f,s) return f-s, (f+s)/2 end
local function ipath(c,pts,col,gap)
 for i=2,#pts do
  local ax,ay=iso(pts[i-1][1],pts[i-1][2]);local bx,by=iso(pts[i][1],pts[i][2])
  local steps=math.max(math.abs(bx-ax),math.abs(by-ay),1)
  for t=0,steps do
   if not gap or (t+i*3)%gap~=0 then
    c:pixel(ax+(bx-ax)*t/steps,ay+(by-ay)*t/steps,col)
   end
  end
 end
end
local function ipoly(c,pts,col)
 local p={};for _,v in ipairs(pts) do local x,y=iso(v[1],v[2]);p[#p+1]={x,y} end;c:poly(p,col)
end
local shapes={
 {key='submerged_wall',draw=function(c)
  -- A long quay wall along one axis, breached in the middle, with coping.
  c:layer(1)
  ipoly(c,{{-26,-2},{-4,-2},{-4,1},{-26,1}},'sea0');ipoly(c,{{2,-2},{24,-2},{24,1},{2,1}},'sea0')
  c:layer(2)
  ipath(c,{{-26,-3},{-4,-3}},'sea_ink',5);ipath(c,{{2,-3},{24,-3}},'sea_ink',5)
  c:pixel(-3,2,'sea_ink');c:pixel(0,3,'sea_ink');c:pixel(2,2,'sea_ink')
 end},
 {key='submerged_roof',draw=function(c)
  -- A roof ridge of a drowned house: two pitched planes seen from above.
  c:layer(1)
  ipoly(c,{{-12,-8},{12,-8},{12,8},{-12,8}},'sea0')
  c:layer(2)
  ipath(c,{{-12,0},{12,0}},'sea_ink',4)
  ipath(c,{{-12,-8},{12,-8},{12,8},{-12,8},{-12,-8}},'sea_ink',6)
  c:pixel(6,-3,'sea_ink');c:pixel(7,-3,'sea_ink')
 end},
 {key='submerged_hull',draw=function(c)
  -- A sunken hull lying on its side, keel line and a few ribs.
  c:layer(1)
  ipoly(c,{{-20,-4},{-6,-7},{14,-6},{22,-1},{16,4},{-8,5},{-20,1}},'sea0')
  c:layer(2)
  ipath(c,{{-18,-1},{20,-2}},'sea_ink',5)
  for i=-3,3 do local f=i*5;ipath(c,{{f,-6},{f,4}},'sea_ink',3) end
 end},
 {key='submerged_posts',draw=function(c)
  -- Mooring posts in a line with the lean of old pilings.
  c:layer(1)
  for i=0,4 do local f=-22+i*11;ipoly(c,{{f,-1},{f+3,-1},{f+3,2},{f,2}},'sea0') end
  c:layer(2)
  for i=0,4 do local f=-22+i*11;local x,y=iso(f+4,3);c:pixel(x,y,'sea_ink');x,y=iso(f+1,-2);c:pixel(x,y,'sea_ink') end
  ipath(c,{{-22,4},{26,4}},'sea_ink',7)
 end},
 {key='submerged_stair',draw=function(c)
  -- A ferry stair descending into the deep: four treads.
  c:layer(1)
  for i=0,3 do local s=-8+i*4;ipoly(c,{{-10+i*2,s},{10-i*2,s},{10-i*2,s+3},{-10+i*2,s+3}},'sea0') end
  c:layer(2)
  for i=0,3 do local s=-8+i*4;ipath(c,{{-10+i*2,s},{10-i*2,s}},'sea_ink',4) end
 end},
 {key='submerged_ring',draw=function(c)
  -- A cistern ring, its rim broken where the wall fell in.
  c:layer(1)
  local pts={};for a=0,23 do local t=a/24*2*math.pi;pts[#pts+1]={math.cos(t)*14,math.sin(t)*14} end;pts[#pts+1]=pts[1]
  ipath(c,pts,'sea0');local inner={};for a=0,23 do local t=a/24*2*math.pi;inner[#inner+1]={math.cos(t)*11,math.sin(t)*11} end;inner[#inner+1]=inner[1]
  ipath(c,inner,'sea0')
  c:layer(2)
  local x,y=iso(0,-12);c:line(x-3,y,x+3,y,'sea1');x,y=iso(10,8);c:pixel(x,y,'sea_ink')
 end},
}
local frames={}
for _,s in ipairs(shapes) do frames[#frames+1]={key=s.key,duration=1,draw=s.draw} end
C.document('submerged',64,32,32,16,layers,frames,'submerged_shapes',m,packer,out,entries)
C.write(root..'/tools/art_v15/sources-submerged.json',json.encode(entries))
C.save_manifest(m)
print('Authored '..#entries..' submerged silhouettes; manifest now has '..C.count(m)..' entries.')
