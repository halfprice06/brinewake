-- Root-authored salt, silt, dressed stone and brackish water tiles.
local out={}
local function ground(c,color)
  c:layer(3)
  for y=-8,8 do c:line({c.cx-(16-2*math.abs(y)),c.cy+y},{c.cx+(16-2*math.abs(y)),c.cy+y},color) end
end
local function line(c,x,y,xx,yy,color) c:line({c.cx+x,c.cy+y},{c.cx+xx,c.cy+yy},color) end
local function poly(c,points,color)
  local q={};for _,p in ipairs(points) do q[#q+1]={c.cx+p[1],c.cy+p[2]} end;c:poly(q,color)
end
for i=0,3 do
  local phase=i
  out['terrain_salt_'..i]=function(c)
    ground(c,'salt1');c:layer(6)
    if phase==1 then
      poly(c,{{-11,0},{-8,-2},{-2,-3},{4,-2},{8,0},{5,1},{-2,0},{-7,1}},'salt2')
      line(c,-7,3,-1,2,'salt0')
    elseif phase==2 then
      line(c,-7,-3,-2,-3,'salt2');line(c,-2,-3,3,-1,'salt2');line(c,3,-1,8,-1,'salt2')
      line(c,-9,2,-5,3,'salt0')
      line(c,-5,3,-1,3,'salt0')
    elseif phase==3 then
      poly(c,{{-8,1},{-5,-1},{0,-2},{4,-1},{8,1},{4,2},{-3,3}},'salt0')
      line(c,-5,-1,0,-2,'salt2');line(c,0,-2,4,-1,'salt2')
      line(c,5,3,7,2,'salt2')
    end
  end
  out['terrain_silt_'..i]=function(c)
    ground(c,'earth1');c:layer(6)
    if phase>0 then
      line(c,-8,0,-3,-2,'earth0');line(c,-3,-2,3,-2,'earth0');line(c,3,-2,8,0,'earth0')
      line(c,-5,2,1,1,'earth2')
      if phase==3 then line(c,3,4,7,2,'earth2') end
    end
  end
  out['terrain_water_'..i]=function(c)
    ground(c,'sea1');c:layer(6)
    if phase>0 then
      local dx=phase-2
      line(c,-9+dx,-1,-4+dx,-3,'sea2');line(c,-4+dx,-3,1+dx,-3,'sea2')
      line(c,1+dx,3,7+dx,1,'sea0')
      line(c,3+dx,3,7+dx,2,'sea2')
    end
  end
  out['terrain_shallow_'..i]=function(c)
    ground(c,'sea2');c:layer(6)
    local x=phase-2
    poly(c,{{-8+x,1},{-3+x,-1},{4+x,-1},{8+x,1},{2+x,3}},'sea1')
    line(c,-8+x,-1,-2+x,-3,'sea3')
    line(c,-2+x,-3,4+x,-3,'sea3')
    line(c,1+x,4,7+x,2,'sea3')
  end
  out['terrain_causeway_'..i]=function(c)
    ground(c,'earth3');c:layer(6)
    line(c,-8,-4,8,4,'earth1')
    line(c,-7,-4,9,4,'earth4')
    line(c,-1,-7,-8,-4,'earth2')
    if phase==2 then
      line(c,0,0,3,-1,'earth0');line(c,3,-1,5,0,'earth1')
    elseif phase==3 then line(c,-7,2,-3,4,'earth2') end
  end
end

out.reed_clump=function(c)
  c:shadow(12,3);c:layer(3)
  for _,p in ipairs({{-9,1,15},{-5,-1,20},{-1,2,17},{3,0,23},{7,1,18},{10,2,11}}) do
    local x,y,h=p[1],p[2],p[3]
    line(c,x,y,x-2,y-h,'reed0')
    line(c,x-1,y,x-3,y-h+1,'reed2')
    line(c,x-2,y-h,x-3,y-h-4,'reed1')
    line(c,x-1,y-h,x-2,y-h-4,'reed3')
    line(c,x,y-2,x+4,y-8,'jade1')
    line(c,x,y-4,x-5,y-10,'jade2')
  end
  c:layer(6)
  line(c,-6,-18,-7,-22,'reed3');line(c,0,-21,-1,-25,'reed3')
end

out.salt_rock=function(c)
  c:shadow(15,4);c:layer(3)
  poly(c,{{-15,-1},{-11,-10},{-4,-17},{5,-15},{14,-7},{16,0},{6,5},{-6,4}},'earth0')
  poly(c,{{-15,-1},{-11,-10},{-4,-17},{5,-15},{2,-8},{-6,-4}},'earth3')
  poly(c,{{2,-8},{5,-15},{14,-7},{16,0},{6,5},{4,-1}},'earth1')
  poly(c,{{-15,-1},{-6,-4},{2,-8},{4,-1},{6,5},{-6,4}},'earth2')
  c:layer(6)
  line(c,-10,-10,-4,-15,'earth4');line(c,-4,-15,2,-14,'earth4')
  line(c,-11,-2,-5,-3,'earth1');line(c,-5,-3,0,-6,'earth1')
  line(c,5,-5,10,-3,'earth0');line(c,10,-3,13,-4,'earth0')
  line(c,-5,1,1,0,'earth3')
end
-- Matching edge strips: the same 2:1 native steps connect at tile corners.
for _,direction in ipairs({'e','s','w','n'}) do
  for variation=0,2 do
    local dir,variant=direction,variation
    out['shore_'..dir..'_'..variant]=function(c)
      c:layer(3)
      local a,b
      if dir=='e' then a={16,0};b={0,8}
      elseif dir=='s' then a={0,8};b={-16,0}
      elseif dir=='w' then a={-16,0};b={0,-8}
      else a={0,-8};b={16,0}end
      local front=dir=='e' or dir=='s'
      local depth=front and 5 or 2
      poly(c,{{a[1],a[2]},{b[1],b[2]},{b[1],b[2]+depth},{a[1],a[2]+depth}},front and 'earth0' or 'wet0')
      line(c,a[1],a[2],b[1],b[2],'earth3')
      if front then
        line(c,a[1],a[2]+1,b[1],b[2]+1,'earth1')
        local t=0.18+variant*0.17
        local x=a[1]+(b[1]-a[1])*t;local y=a[2]+(b[2]-a[2])*t
        line(c,x,y+2,x+(b[1]-a[1])*0.25,y+(b[2]-a[2])*0.25+2,'earth2')
        line(c,x+2,y+4,x+5,y+4,'wet0')
      end
      -- Broken foam rests on the water side, below the earthy lip.
      local breaks=({{{0.05,0.31},{0.63,0.88}},{{0.22,0.57}},{{0.0,0.12},{0.76,1.0}}})[variant+1]
      for _,range in ipairs(breaks)do
        local x0=a[1]+(b[1]-a[1])*range[1];local y0=a[2]+(b[2]-a[2])*range[1]
        local x1=a[1]+(b[1]-a[1])*range[2];local y1=a[2]+(b[2]-a[2])*range[2]
        local foam_offset=front and depth+1 or -2
        line(c,x0,y0+foam_offset,x1,y1+foam_offset,front and (variant==1 and 'sea3' or 'foam') or 'sea2')
      end
    end
  end
end
out.salt_bush=function(c)
  c:shadow(14,4);c:layer(3)
  poly(c,{{-14,0},{-10,-7},{-6,-8},{-4,-14},{0,-11},{5,-15},{10,-9},{13,-6},{15,1},{7,4},{-6,4}},'moss0')
  poly(c,{{-12,-2},{-8,-7},{-4,-5},{0,-11},{3,-8},{8,-12},{11,-7},{8,-3},{3,-4},{-2,0}},'moss1')
  line(c,-8,-6,-5,-7,'reed2');line(c,1,-9,4,-11,'reed2');line(c,7,-8,10,-9,'reed3')
  line(c,-6,1,-1,-1,'moss1');line(c,5,1,10,-2,'moss1')
end
out.marsh_grass=function(c)
  c:shadow(13,3);c:layer(3)
  for _,v in ipairs({{-12,0,-4,-13},{-8,1,-10,-19},{-3,2,-2,-23},{2,1,7,-18},{7,2,13,-11},{10,1,16,-6}})do
    poly(c,{{v[1],v[2]},{v[3],v[4]},{v[3]-2,v[4]+7},{v[1]-3,v[2]+1}},'moss0')
    line(c,v[1],v[2],v[3],v[4],'moss1')
  end
  line(c,-1,-19,0,-25,'reed1',2);line(c,5,-13,7,-20,'reed2',2)
  line(c,-10,-10,-7,-14,'reed2');line(c,9,-4,13,-6,'reed2')
end
out.drift_scrap=function(c)
  c:shadow(13,3);c:layer(3)
  poly(c,{{-14,-1},{-9,-7},{3,-4},{8,-6},{15,-1},{8,4},{0,3},{-6,5}},'earth0')
  poly(c,{{-12,-2},{-8,-6},{0,-4},{3,-5},{10,-2},{6,0},{0,-1},{-5,2}},'rust0')
  line(c,-8,-5,3,-1,'rust2')
  line(c,-4,-3,9,3,'steel1',2)
  line(c,-3,-4,10,2,'steel2')
  line(c,2,2,7,1,'earth2')
end
out.pebbles=function(c)
  c:layer(3)
  for _,v in ipairs({{-9,0,5},{-1,3,4},{7,-2,6},{10,4,3}})do
    local x,y,w=v[1],v[2],v[3]
    poly(c,{{x-w/2,y},{x-1,y-3},{x+w/2,y-2},{x+w/2+1,y+1},{x,y+2}},'salt0')
    poly(c,{{x-w/2,y},{x-1,y-3},{x+w/2,y-2},{x,y}},'salt2')
  end
end
return out
