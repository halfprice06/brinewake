-- Quiet authored native terrain. Seams are unaccented; texture supports units.
local out={}
local function diamond(c,col)
 c:layer('body');for y=-8,8 do local r=16-2*math.abs(y);c:line(c.cx-r,c.cy+y,c.cx+r,c.cy+y,col)end
end
for i=0,3 do
 local variant=i
 out['terrain_salt_'..i]=function(c)
  diamond(c,'ground1');c:layer('details')
  if variant==1 then
   c:poly({{c.cx-10,c.cy},{c.cx-6,c.cy-2},{c.cx+1,c.cy-2},{c.cx+5,c.cy},{c.cx-2,c.cy+1}},'ground2')
   c:line(c.cx-7,c.cy+3,c.cx-2,c.cy+3,'ground0')
  elseif variant==2 then
   c:line(c.cx-5,c.cy-3,c.cx-1,c.cy-3,'ground2');c:line(c.cx-1,c.cy-3,c.cx+3,c.cy-1,'ground2')
   c:line(c.cx+3,c.cy-1,c.cx+7,c.cy-1,'ground2');c:line(c.cx-8,c.cy+2,c.cx-5,c.cy+2,'ground0')
  elseif variant==3 then
   c:poly({{c.cx-6,c.cy},{c.cx-3,c.cy-1},{c.cx+4,c.cy},{c.cx+6,c.cy+2},{c.cx+1,c.cy+3}},'ground0')
   c:line(c.cx-3,c.cy-1,c.cx+3,c.cy,'ground2');c:pixel(c.cx+4,c.cy+3,'ground2')
  end
 end
 out['terrain_silt_'..i]=function(c)
  diamond(c,'silt0');c:layer('details')
  if variant%2==0 then c:line(c.cx-8,c.cy+1,c.cx-2,c.cy-1,'ground0');c:line(c.cx-2,c.cy-1,c.cx+5,c.cy-1,'ground0')end
  if variant>=2 then c:line(c.cx+1,c.cy+4,c.cx+5,c.cy+2,'ground1')end
 end
 out['terrain_water_'..i]=function(c)
  diamond(c,'water1');c:layer('details')
  if variant>0 then
   local x=c.cx+variant-2
   c:line(x-6,c.cy-2,x-2,c.cy-3,'water2');c:line(x-2,c.cy-3,x+3,c.cy-3,'water2')
   c:line(x+1,c.cy+2,x+6,c.cy+1,'water0')
  end
 end
 out['terrain_shallow_'..i]=function(c)
  diamond(c,'water2');c:layer('details')
  local x=c.cx+(variant%3)-1;c:line(x-7,c.cy-2,x-2,c.cy-2,'water3');c:line(x-2,c.cy-2,x+1,c.cy-1,'water3')
  c:poly({{x+1,c.cy+2},{x+5,c.cy},{x+9,c.cy+1},{x+4,c.cy+3}},'water1')
 end
 out['terrain_causeway_'..i]=function(c)
  diamond(c,'silt2');c:layer('details')
  c:line(c.cx-8,c.cy-3,c.cx+8,c.cy+5,'ground1');c:line(c.cx-2,c.cy-6,c.cx-10,c.cy-2,'silt3')
  if variant==2 then c:line(c.cx+2,c.cy-1,c.cx+5,c.cy-1,'silt0');c:line(c.cx+5,c.cy-1,c.cx+7,c.cy,'silt0')end
 end
end
out.reed_clump=function(c)
 c:shadow(9,3);c:layer('body')
 for _,v in ipairs({{-7,0,11},{-3,1,17},{1,-2,15},{5,0,20},{8,2,12}})do
  c:line3({v[1],v[2],0},{v[1]-2,v[2],v[3]},'straw0',2)
  c:line3({v[1]-2,v[2],v[3]},{v[1]-1,v[2],v[3]+4},'straw1')
  c:line3({v[1],v[2],4},{v[1]+3,v[2],8},'jade0')
 end
 c:layer('details');c:line3({3,0,19},{4,0,23},'straw2')
end
out.salt_rock=function(c)
 c:shadow(12,4);c:layer('body')
 c:poly3({{-12,-4,0},{-6,-7,7},{2,-6,11},{11,-2,5},{12,5,0},{-5,8,0}},'silt0')
 c:poly3({{-12,-4,0},{-6,-7,7},{2,-6,11},{4,0,7},{-5,4,3}},'ground3')
 c:poly3({{4,0,7},{2,-6,11},{11,-2,5},{12,5,0},{5,5,1}},'ground0')
 c:layer('details');c:line3({-6,-7,7},{2,-6,11},'silt2');c:line3({-8,0,1},{-1,1,3},'silt0')
end
return out
