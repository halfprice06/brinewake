-- Native impact and wreck-burst keys. Short flashes give way to separated
-- fragments, then smoke; no uniform expanding disc or screen-wide flash.
local out={}
local function poly(c,points,color)local q={};for _,p in ipairs(points)do q[#q+1]={c.cx+p[1],c.cy+p[2]}end;c:poly(q,color)end
local function line(c,x,y,xx,yy,col,w)c:line({c.cx+x,c.cy+y},{c.cx+xx,c.cy+yy},col,w)end
local function puff(c,x,y,r)
  c:ellipse(c.cx+x,c.cy+y,r,math.max(2,r-1),'steel0')
  c:ellipse(c.cx+x-1,c.cy+y-2,math.max(2,r-2),math.max(2,r-3),'steel1')
  line(c,x-r+2,y-3,x-1,y-r+2,'steel2')
end
for i=0,3 do
  local phase=i
  out['fx_impact_'..i]=function(c)
    c:layer(5)
    if phase==0 then
      poly(c,{{-8,-4},{-2,-3},{0,-11},{3,-3},{10,-6},{5,0},{9,6},{2,3},{-1,10},{-3,3},{-11,4},{-5,-1}},'rust0')
      poly(c,{{-7,-3},{-1,-2},{0,-8},{2,-2},{8,-4},{3,0},{6,4},{1,2},{-1,7},{-2,2},{-8,3},{-3,-1}},'amber3')
      poly(c,{{-3,-2},{1,-4},{4,0},{0,4},{-4,1}},'ivory4')
    elseif phase==1 then
      line(c,-5,-3,-10,-7,'amber3',2);line(c,5,-3,10,-9,'ivory4')
      line(c,-4,4,-10,8,'rust3');line(c,5,3,12,6,'amber3')
      poly(c,{{-3,-3},{1,-5},{4,-1},{2,3},{-2,4},{-5,1}},'rust2')
      line(c,-2,-3,2,-2,'ivory4')
    elseif phase==2 then
      line(c,-10,-6,-13,-8,'rust3');line(c,9,-8,11,-11,'amber3')
      line(c,9,5,13,7,'rust2');line(c,-8,8,-11,10,'amber2')
      line(c,-3,-2,0,-4,'steel2')
    else
      line(c,-12,-7,-13,-7,'rust1');line(c,11,-9,12,-10,'rust3')
      line(c,11,7,13,7,'steel2')
    end
  end
end
for i=0,2 do
  local phase=i
  out['fx_muzzle_'..i]=function(c)
    c:layer(5)
    if phase==0 then
      poly(c,{{-10,-1},{-3,-3},{-4,-8},{2,-4},{8,-7},{5,0},{11,3},{3,4},{0,10},{-3,4},{-8,5},{-5,1}},'rust3')
      poly(c,{{-6,-1},{0,-5},{5,-2},{6,3},{0,6},{-4,3}},'amber4')
    elseif phase==1 then
      poly(c,{{-7,-1},{-2,-3},{1,-7},{3,-2},{8,1},{3,3},{-1,7},{-3,2}},'amber2')
      line(c,-3,-1,3,1,'ivory4',2)
    else
      line(c,-5,-2,-2,-1,'steel2');line(c,2,3,5,4,'rust2')
    end
  end
end
for i=0,5 do
  local phase=i
  out['fx_wreck_'..i]=function(c)
    c:layer(2)
    if phase<4 then c:ellipse(c.cx,c.cy-1,10+phase*2,3,'cast')end
    c:layer(3)
    if phase==0 then
      poly(c,{{-4,-6},{-12,-11},{-5,-15},{-9,-23},{0,-20},{5,-29},{8,-19},{17,-21},{12,-12},{18,-7},{7,-4},{3,2}},'rust1')
      poly(c,{{-4,-7},{-8,-13},{-3,-15},{-5,-21},{1,-18},{6,-25},{8,-16},{13,-18},{10,-11},{14,-8},{6,-5}},'amber3')
      poly(c,{{-3,-11},{0,-19},{5,-20},{9,-12},{5,-6},{0,-7}},'ivory4')
    elseif phase==1 then
      puff(c,-7,-15,8);puff(c,7,-16,9);puff(c,0,-25,8)
      poly(c,{{-12,-5},{-13,-13},{-5,-17},{0,-23},{7,-20},{13,-14},{13,-6},{5,-1},{-4,-1}},'rust2')
      poly(c,{{-8,-5},{-8,-12},{-3,-14},{1,-19},{6,-14},{10,-11},{8,-5},{2,-3}},'amber3')
      poly(c,{{-3,-5},{-4,-11},{1,-14},{5,-9},{3,-4}},'amber4')
    elseif phase==2 then
      puff(c,-10,-13,8);puff(c,9,-14,8);puff(c,1,-26,8);puff(c,-9,-26,6)
      poly(c,{{-6,-5},{-9,-12},{-3,-14},{0,-20},{5,-13},{11,-10},{7,-4}},'rust2')
      line(c,-3,-7,0,-13,'amber3',2)
      line(c,-17,-6,-21,-9,'rust3');line(c,17,-8,21,-12,'ivory3')
    elseif phase==3 then
      puff(c,-11,-21,8);puff(c,9,-22,8);puff(c,-2,-29,6)
      line(c,-4,-10,-1,-13,'rust1',2);line(c,8,-9,10,-12,'rust2')
      line(c,-21,-2,-23,-4,'steel2');line(c,21,-4,23,-6,'steel3')
    elseif phase==4 then
      puff(c,-12,-26,5);puff(c,10,-27,5);puff(c,0,-32,3)
      line(c,-4,-16,-1,-18,'steel1',2)
    else
      puff(c,-13,-29,3);puff(c,11,-30,3)
      line(c,0,-22,3,-24,'steel1')
    end
  end
end
return out
