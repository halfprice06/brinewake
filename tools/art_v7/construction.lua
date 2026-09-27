-- Root-authored construction keys: laid-out site, load-bearing frame, fit-out.
-- Every upright ends at a foundation point. No completed roof floats over an
-- empty site, and the last frame keeps the finished building's native anchors.
local M={}
M.spec={
  union_works={x=2,y=5,w=43,d=44,h=40},
  assembly_works={x=0,y=4,w=37,d=40,h=37},
  dropoff={x=0,y=5,w=36,d=40,h=42},
  condenser={x=0,y=5,w=30,d=33,h=48},
  tower={x=0,y=5,w=30,d=31,h=56},
}
local function poly(c,p,col)
  local q={};for _,v in ipairs(p) do q[#q+1]={c.cx+v[1],c.cy+v[2]} end;c:poly(q,col)
end
local function line(c,x,y,xx,yy,col,w) c:line({c.cx+x,c.cy+y},{c.cx+xx,c.cy+yy},col,w) end
local function block(c,x,y,w,d,h,mat)
  poly(c,{{x-w,y-w/2-h},{x,y-h},{x,y},{x-w,y-w/2}},mat..'2')
  poly(c,{{x,y-h},{x+d,y-d/2-h},{x+d,y-d/2},{x,y}},mat..'1')
  poly(c,{{x-w,y-w/2-h},{x-w+d,y-(w+d)/2-h},{x+d,y-d/2-h},{x,y-h}},mat..'3')
  line(c,x-w,y-w/2,x,y,mat..'0');line(c,x,y,x+d,y-d/2,mat..'0')
  line(c,x-w,y-w/2-h,x,y-h,mat..'4')
end
local function post(c,x,y,h,mat)
  block(c,x+2,y,3,3,h,mat)
  block(c,x+2,y+1,5,5,2,'earth')
end
local function beam(c,a,b,col)
  line(c,a[1],a[2]+1,b[1],b[2]+1,'steel0',2)
  line(c,a[1],a[2],b[1],b[2],col,2)
  line(c,a[1],a[2],b[1],b[2],'steel3')
end
function M.paint(c,name,stage)
  local s=M.spec[name];local x,y,w,d,h=s.x,s.y,s.w,s.d,s.h
  local woven=name=='assembly_works';local mat=woven and 'reed' or 'steel'
  local front={x,y};local left={x-w,y-w/2};local back={x-w+d,y-(w+d)/2};local right={x+d,y-d/2}
  if stage<2 then
    c:layer(2)
    block(c,x,y,w,d,2,'earth')
    -- Laid-out conduits follow the footprint, with open space for machinery.
    line(c,left[1]+4,left[2]-3,front[1],front[2]-3,'steel0',2)
    line(c,front[1],front[2]-3,right[1]-4,right[2]-3,'steel0',2)
    line(c,left[1]+5,left[2]-4,front[1],front[2]-4,'steel2')
    block(c,-16,-9,13,9,6,woven and 'reed' or 'rust')
    block(c,-15,-15,10,7,3,'ivory')
    for i=0,2 do block(c,12+i*4,-18+i*2,4,18,3,mat) end
    c:layer(3)
    -- Machinery cores distinguish the sites even before cladding is fitted.
    if name=='condenser' then
      c:cylinder(-12,-18,0,7,stage==0 and 9 or 24,'steel')
      c:cylinder(13,-10,0,9,stage==0 and 6 or 31,'steel')
    elseif name=='tower' then
      block(c,1,-9,15,16,stage==0 and 6 or 22,'steel')
      if stage==1 then block(c,1,-31,10,11,13,'rust') end
    elseif woven then
      block(c,8,-18,14,14,stage==0 and 5 or 18,'jade')
      if stage==1 then c:cylinder(6,-20,0,5,19,'amber') end
    else
      block(c,9,-21,16,12,stage==0 and 4 or 14,'steel')
      if stage==1 then block(c,8,-35,8,9,6,'rust') end
    end
  end
  c:layer(stage==2 and 6 or 4)
  local height=stage==0 and 8 or h
  if stage<2 then
    -- Back frame first; braces meet actual post tops and bases.
    post(c,back[1],back[2],height,mat)
    post(c,left[1],left[2],height,mat)
    beam(c,{left[1],left[2]-height},{back[1],back[2]-height},mat..'2')
    if stage==1 then beam(c,{left[1],left[2]-2},{back[1],back[2]-height},mat..'1') end
  end
  -- Open front scaffolds do not paint a solid box around the building.
  local lx,ly=left[1]-3,left[2]+3;local rx,ry=right[1]+2,right[2]+3
  post(c,lx,ly,height,mat);post(c,rx,ry,height,mat)
  if stage>0 then
    local fy=y+2
    post(c,x,fy,height*0.6,mat)
    beam(c,{lx,ly-height*0.6},{x,fy-height*0.6},mat..'2')
    beam(c,{x,fy-height*0.6},{rx,ry-height*0.6},mat..'2')
    if stage==1 then
      beam(c,{lx,ly-height},{back[1],back[2]-height},mat..'2')
      beam(c,{back[1],back[2]-height},{rx,ry-height},mat..'2')
    end
    -- A ladder has stable feet and supports, not disconnected floating rungs.
    line(c,lx+4,ly,lx+10,ly-height*0.7,'reed0',2)
    line(c,lx+9,ly+2,lx+15,ly-height*0.7+2,'reed0',2)
    for i=2,math.floor(height*0.7)-2,5 do
      local xx=lx+4+i*6/(height*0.7)
      line(c,xx,ly-i,xx+5,ly-i+2,'reed2')
    end
  end
  c:layer(6)
  -- A tied safety panel and tools remain low enough to preserve the silhouette.
  local py=left[2]+5
  block(c,left[1]+15,py,12,3,6,woven and 'reed' or 'rust')
  for i=0,2 do line(c,left[1]+5+i*3,py-6+i*1.5,left[1]+7+i*3,py-5+i*1.5,'ivory2') end
  if stage==2 then
    -- A folded canvas cover on a front scaffold shows final installation work.
    poly(c,{{x-15,y-h*0.6-5},{x-4,y-h*0.6},{x-4,y-h*0.6+8},{x-10,y-h*0.6+6},{x-15,y-h*0.6+4}},woven and 'reed1' or 'rust1')
    line(c,x-14,y-h*0.6-4,x-5,y-h*0.6,'ivory2')
  end
end
return M
