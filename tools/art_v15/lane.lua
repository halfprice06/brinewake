-- Root-authored v15 lane memory: four drying stages for a sluice lane that
-- has just been drained. The floor is an exact copy of the matching dry lane
-- tile; a damp wash darkens the floor, standing pools sit between the
-- crossbars and shrink, and a salt rime blooms as the brine evaporates.
-- Stage 0 is freshly drained; stage 3 is nearly dry. Same 32x16 canvas and
-- 16,8 anchor as the lane tiles it replaces for the first minute.
local root=assert(app.params.root)
local C=dofile(root..'/tools/art_v15/common.lua')
local m,packer=C.manifest()
local entries={};local out=root..'/output/art-v15/lane';os.execute('mkdir -p '..out)
local layers={'lane floor','damp wash','standing pools','salt rime'}
local FLOOR=C.rgba.salt1
local function hash(x,y,s) return ((x+40)*73+(y+40)*151+(x+40)*(y+40)*7+s*31)%100 end
-- The wash retreats as connected regions, never as speckle: stage 0 covers
-- the floor, stage 1 keeps the lower half and a tongue, stage 2 keeps a
-- band around the last pool, stage 3 keeps only two damp shadows.
local function wet(stage,x,y,v)
 local dx,dy=x-16,y-8
 if stage==0 then return true end
 if stage==1 then return dy>=-1 or (dx>=-9 and dx<=2 and dy>=-4) end
 if stage==2 then return math.abs(dx+(v%2))+2*math.abs(dy-1)<=9 end
 return (dx>=-11 and dx<=-6 and dy>=1 and dy<=3) or (dx>=4 and dx<=9 and dy>=-3 and dy<=-1)
end
local frames={}
for stage=0,3 do
 for v=0,3 do
  frames[#frames+1]={key='terrain_lane_drying_'..stage..'_'..v,duration=1,draw=function(c)
   c.clip=nil
   c:layer(1);c:stamp(m,'terrain_lane_dry_'..v,0,0)
   local floor=c.ims[1]
   -- Damp wash over the floor colour only; the crossbars keep their tone.
   c:layer(2)
   for y=0,15 do for x=0,31 do
    if floor:getPixel(x,y)==FLOOR and wet(stage,x,y,v) then
     local h=hash(x,y,v)
     -- Darker streaks run along the lane axis in short connected runs.
     local streak=((x+2*y+v*3)%11)<3 and stage<=1
     c:raw(x-16,y-8,C.rgba[streak and 'earth0' or 'damp'])
    end
   end end
   c.clip=C.lane_clip
   -- Pools between the bars: two, then one, then a last puddle, then none.
   c:layer(3)
   local pools={
    {{{-9,-1},{-3,-3},{3,-2},{1,1},{-6,2}},{{2,1},{8,-1},{11,1},{6,3}}},
    {{{-7,-1},{-3,-2},{1,-1},{-1,1},{-5,1}},{{4,1},{8,0},{9,2},{6,3}}},
    {{{-4,0},{-1,-1},{2,0},{0,2},{-3,2}}},
    {},
   }
   for _,p in ipairs(pools[stage+1]) do
    local shifted={};for _,q in ipairs(p) do shifted[#shifted+1]={q[1]+(v%2)*2-1,q[2]+(v>1 and 1 or 0)} end
    c:poly(shifted,'wet0')
    local sx,sy=shifted[2][1],shifted[2][2]+1;c:pixel(sx,sy,'sea2')
   end
   -- Rime: pale crystals along the drying edges and the bars.
   c:layer(4)
   if stage>=1 then
    c:pixel(-8+v,1,'salt3');c:pixel(9-v,-2,'salt3');c:pixel(0,-4,'salt3')
   end
   if stage>=2 then
    c:line(-10,2,-5,2,'ivory2');c:line(3,-3,8,-3,'ivory2');c:pixel(-1,3,'ivory2');c:pixel(6,0,'ivory2')
   end
   if stage==3 then
    c:line(-12,0,-6,0,'ivory2');c:line(-2,-2,4,-2,'ivory3');c:line(1,3,7,3,'ivory2');c:pixel(10,0,'ivory3');c:pixel(-4,-4,'ivory2')
   end
  end}
 end
end
C.document('lane_drying',32,16,16,8,layers,frames,'lane_drying_stages',m,packer,out,entries)
C.write(root..'/tools/art_v15/sources-lane.json',json.encode(entries))
C.save_manifest(m)
print('Authored '..#entries..' drying lane tiles; manifest now has '..C.count(m)..' entries.')
