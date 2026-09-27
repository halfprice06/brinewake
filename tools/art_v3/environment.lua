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
return out
